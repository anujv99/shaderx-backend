use axum::{body::Body, extract::{FromRequestParts, Request, State}, http::{header::{COOKIE, SET_COOKIE}, request::Parts}, middleware::Next, response::{IntoResponse, IntoResponseParts, Response}, Extension};
use axum_extra::extract::PrivateCookieJar;
use chrono::{Duration, Local, Utc};
use cookie::Cookie;
use oauth2::{basic::{BasicClient, BasicTokenType}, reqwest::async_http_client, EmptyExtraTokenFields, RefreshToken, StandardTokenResponse, TokenResponse};

use crate::{constants::{ACCESS_COOKIE_EXPIRE_TIME, ACCESS_TOKEN_EXPIRE_TIME}, errors::ApiError, router_state::{RouterState, UserProfile}};

#[derive(Debug, sqlx::FromRow)]
struct Session {
  pub id: i32,
  pub user_id: i32,
  pub session_id: String,
  pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct RefreshTokenEntry {
  pub id: i32,
  pub user_id: i32,
  pub refresh_token: String,
}

async fn default_req(parts: Parts, body: Body, next: Next) -> Response {
  let req = Request::from_parts(parts, body);
  next.run(req).await
}

async fn refresh_access_token(
  client: &BasicClient,
  refresh_token: &RefreshToken,
) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, ApiError> {
  let token = client
    .exchange_refresh_token(refresh_token)
    .request_async(async_http_client)
    .await?;

  Ok(token)
}
  

pub async fn token_refresh_middleware(
  State(state): State<RouterState>,
  Extension(client): Extension<BasicClient>,
  req: Request,
  next: Next,
) -> Response {
  let (mut parts, body) = req.into_parts();

  let mut cookie_jar: PrivateCookieJar = match PrivateCookieJar::from_request_parts(&mut parts, &state).await {
    Ok(jar) => jar,
    Err(_) => return default_req(parts, body, next).await,
  };

  let Some(cookie) = cookie_jar.get("sid").map(|cookie| cookie.value().to_owned()) else {
    return default_req(parts, body, next).await;
  };

  let session = match sqlx::query_as::<_, Session>("
    SELECT * FROM sessions WHERE session_id = $1
  ").bind(cookie).fetch_one(&state.db).await {
    Ok(session) => session,
    Err(_) => return default_req(parts, body, next).await,
  };

  if Utc::now() >= session.expires_at {
    let refresh_token = match sqlx::query_as::<_, RefreshTokenEntry>("
      SELECT * FROM refresh_tokens WHERE user_id = $1
      ").bind(session.user_id).fetch_one(&state.db).await {
        Ok(refresh_token) => refresh_token,
        Err(_) => return default_req(parts, body, next).await,
      };

    match refresh_access_token(&client, &RefreshToken::new(refresh_token.refresh_token)).await {
      Ok(access_token) => {
        let token = access_token.access_token().secret().to_owned();

        let Some(secs) = access_token.expires_in() else {
          return default_req(parts, body, next).await;
        };

        let secs: i64 = secs.as_secs().try_into().unwrap();

        let max_age = Local::now().naive_utc() + Duration::seconds(
          std::cmp::min(secs, ACCESS_TOKEN_EXPIRE_TIME),
        );

        log::error!("updating session token to: {}", token);
        sqlx::query("UPDATE sessions SET session_id = $1, expires_at = $2 WHERE id = $3")
          .bind(&token)
          .bind(max_age)
          .bind(session.id)
          .execute(&state.db)
          .await
          .unwrap();

        let cookie = Cookie::build(("sid", token.clone()))
          .domain("localhost")
          .path("/")
          .secure(true)
          .http_only(true)
          .max_age(cookie::time::Duration::seconds(ACCESS_COOKIE_EXPIRE_TIME));

        cookie_jar = cookie_jar.add(cookie.clone());
        assert_eq!(cookie_jar.get("sid").unwrap().value(), token);

        // insert new cookie in request
        let encoded_cookie = cookie_jar.into_response();
        encoded_cookie.headers().get(SET_COOKIE).map(|cookie| {
          parts.headers.insert(COOKIE, cookie.clone());
        });

        // insert new cookie in response
        let mut res = next.run(Request::from_parts(parts, body)).await;
        res.headers_mut().insert(SET_COOKIE, encoded_cookie.headers().get(SET_COOKIE).unwrap().clone());

        return res;
      },
      Err(_) => return default_req(parts, body, next).await,
    };
  }

  default_req(parts, body, next).await
}
