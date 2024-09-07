
use axum::{extract::{Query, State}, response::{IntoResponse, Redirect}, Extension};
use axum_extra::extract::{cookie::Cookie, PrivateCookieJar};
use chrono::{Duration, Local};
use oauth2::{basic::BasicClient, reqwest::async_http_client, AuthorizationCode, TokenResponse};
use serde::Deserialize;

use crate::{errors::ApiError, router_state::{RouterState, UserProfile}};

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
  code: String,
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

  let profile = profile.json::<UserProfile>().await.unwrap();

  let Some(secs) = token.expires_in() else {
    return Err(ApiError::OptionError);
  };

  let secs: i64 = secs.as_secs().try_into().unwrap();

  let max_age = Local::now().naive_utc() + Duration::seconds(secs);

  let cookie = Cookie::build(("sid", token.access_token().secret().to_owned()))
    .domain("localhost")
    .path("/")
    .secure(false)
    .http_only(true)
    .max_age(cookie::time::Duration::seconds(secs));

  sqlx::query("INSERT INTO users (email) VALUES ($1) ON CONFLICT (email) DO NOTHING")
    .bind(profile.email.clone())
    .execute(&state.db)
    .await?;

  sqlx::query("INSERT INTO sessions (user_id, session_id, expires_at) VALUES (
    (SELECT id FROM users WHERE email = $1 LIMIT 1),
    $2, $3)
    ON CONFLICT (user_id) DO UPDATE SET
    session_id = excluded.session_id,
    expires_at = excluded.expires_at")
    .bind(profile.email)
    .bind(token.access_token().secret().to_owned())
    .bind(max_age)
    .execute(&state.db)
    .await?;

  Ok((
    jar.add(cookie),
    Redirect::to("/protected")
  ))
}
