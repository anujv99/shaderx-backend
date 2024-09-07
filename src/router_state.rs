use axum::extract::{FromRef, FromRequest, FromRequestParts, Request};
use axum_extra::extract::{cookie::Key, PrivateCookieJar};
use sqlx::{Pool, Postgres};
use reqwest::Client as ReqwestClient;
use serde::Deserialize;

use crate::errors::ApiError;

#[derive(Debug, Clone)]
pub struct RouterState {
  pub db: Pool<Postgres>,
  pub key: Key,
  pub ctx: ReqwestClient,
}

#[derive(Debug, Deserialize, sqlx::FromRow, Clone)]
pub struct UserProfile {
  pub email: String,
}

impl RouterState {
  pub fn new(db: Pool<Postgres>) -> Self {
    Self {
      db,
      key: Key::generate(),
      ctx: ReqwestClient::new(),
    }
  }
}

impl FromRef<RouterState> for Key {
  fn from_ref(state: &RouterState) -> Self {
    state.key.clone()
  }
}

#[axum::async_trait]
impl FromRequest<RouterState> for UserProfile {
  type Rejection = ApiError;
  async fn from_request(req: Request, state: &RouterState) -> Result<Self, Self::Rejection> {
    let state = state.to_owned();
    let (mut parts, _body) = req.into_parts();
    let cookie_jar: PrivateCookieJar = PrivateCookieJar::from_request_parts(&mut parts, &state).await?;

    let Some(cookie) = cookie_jar.get("sid").map(|cookie| cookie.value().to_owned()) else {
      return Err(ApiError::Unauthorized);
    };

    let res = sqlx::query_as::<_, UserProfile>("
      SELECT users.email FROM sessions
      LEFT JOIN users ON sessions.user_id = users.id
      WHERE sessions.session_id = $1 LIMIT 1
    ").bind(cookie).fetch_one(&state.db).await?;

    Ok(Self { email: res.email })
  }
}
