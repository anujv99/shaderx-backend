
use axum::{extract::{Query, State}, http::StatusCode, response::{IntoResponse, Redirect}, Extension};
use axum_extra::extract::{cookie::Cookie, PrivateCookieJar};
use chrono::{Duration, Local};
use oauth2::{basic::BasicClient, reqwest::async_http_client, AuthorizationCode, TokenResponse};
use serde::Deserialize;

use crate::{constants::{ACCESS_COOKIE_EXPIRE_TIME, ACCESS_TOKEN_EXPIRE_TIME}, errors::ApiError, router_state::{RouterState, UserProfile}};

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
  code: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenIdProfile {
  pub email: String,
  pub name: String,
}

pub async fn google_callback(
  State(state): State<RouterState>,
  jar: PrivateCookieJar,
  Query(query): Query<AuthRequest>,
  Extension(oauth_client): Extension<BasicClient>,
) -> Result<impl IntoResponse, ApiError> {
  let token = oauth_client
    .exchange_code(AuthorizationCode::new(query.code))
    .request_async(async_http_client)
    .await?;

  let profile = state.ctx.get("https://openidconnect.googleapis.com/v1/userinfo")
    .bearer_auth(token.access_token().secret().to_owned())
    .send()
    .await?;

  let profile = profile.json::<OpenIdProfile>().await.unwrap();

  let Some(secs) = token.expires_in() else {
    return Err(ApiError::OptionError);
  };

  let secs: i64 = secs.as_secs().try_into().unwrap();

  let max_age = Local::now().naive_utc() + Duration::seconds(
    std::cmp::min(secs, ACCESS_TOKEN_EXPIRE_TIME)
  );

  let cookie = Cookie::build(("sid", token.access_token().secret().to_owned()))
    .domain(state.env.frontend_domain)
    .path("/")
    .secure(true)
    .http_only(true)
    .max_age(cookie::time::Duration::days(ACCESS_COOKIE_EXPIRE_TIME));

  sqlx::query("INSERT INTO users (email, name) VALUES ($1, $2) ON CONFLICT (email) DO NOTHING")
    .bind(profile.email.clone())
    .bind(profile.name.clone())
    .execute(&state.db)
    .await?;

  sqlx::query("INSERT INTO sessions (user_id, session_id, expires_at) VALUES (
    (SELECT id FROM users WHERE email = $1 LIMIT 1),
    $2, $3)
    ON CONFLICT (user_id) DO UPDATE SET
    session_id = excluded.session_id,
    expires_at = excluded.expires_at")
    .bind(&profile.email)
    .bind(token.access_token().secret().to_owned())
    .bind(max_age)
    .execute(&state.db)
    .await?;

  let refresh_token = token.refresh_token().unwrap().secret().to_owned();
  let refresh_token_expires_at = Local::now().naive_utc() + Duration::days(7);

  sqlx::query("INSERT INTO refresh_tokens (user_id, refresh_token, expires_at) VALUES (
    (SELECT id FROM users WHERE email = $1 LIMIT 1),
    $2, $3)
    ON CONFLICT (user_id) DO UPDATE SET
    refresh_token = excluded.refresh_token,
    expires_at = excluded.expires_at")
    .bind(&profile.email)
    .bind(refresh_token)
    .bind(refresh_token_expires_at)
    .execute(&state.db)
    .await?;

  Ok((
    jar.add(cookie),
    Redirect::to("/protected")
  ))
}

pub async fn get_login_url(
  State(state): State<RouterState>,
  Extension(oauth_id): Extension<String>,
) -> impl IntoResponse {
  let redirect_url = state.env.frontend_url;
  format!("https://accounts.google.com/o/oauth2/v2/auth?access_type=offline&prompt=consent&scope=openid%20profile%20email&client_id={oauth_id}&response_type=code&redirect_uri={redirect_url}/auth/google_callback").into_response()
}

pub async fn validate(
  profile: UserProfile,
) -> impl IntoResponse {
  (StatusCode::OK, axum::Json(profile))
}

pub async fn logout(
  profile: UserProfile,
  jar: PrivateCookieJar,
  State(state): State<RouterState>,
) -> Result<impl IntoResponse, ApiError> {
  // remove from sessions
  let _ = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
    .bind(&profile.id)
    .execute(&state.db)
    .await;

  // remove from refresh_tokens
  let _ = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
    .bind(&profile.id)
    .execute(&state.db)
    .await;

  Ok((
    jar.remove(Cookie::from("sid")),
    StatusCode::OK
  ))
}
