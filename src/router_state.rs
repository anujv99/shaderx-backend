use axum::{extract::{FromRef, FromRequestParts}, http::request::Parts};
use axum_extra::extract::{cookie::Key, PrivateCookieJar};
use sqlx::{Pool, Postgres};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};

use crate::{env::Env, errors::ApiError};

#[derive(Debug, Clone)]
pub struct RouterState {
  pub db: Pool<Postgres>,
  pub key: Key,
  pub ctx: ReqwestClient,
  pub env: Env,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserProfile {
  pub id: i32,
  pub user_id: sqlx::types::uuid::Uuid,
  pub email: String,
  pub name: String,
  pub username: Option<String>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl RouterState {
  pub fn new(db: Pool<Postgres>, env: &Env) -> Self {
    Self {
      db,
      key: Key::generate(),
      ctx: ReqwestClient::new(),
      env: env.clone(),
    }
  }
}

impl FromRef<RouterState> for Key {
  fn from_ref(state: &RouterState) -> Self {
    state.key.clone()
  }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for UserProfile
where
    RouterState: FromRef<S>,
    S: Send + Sync,
{
  type Rejection = ApiError;
  async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    let router_state = RouterState::from_ref(state).to_owned();
    let cookie_jar: PrivateCookieJar = PrivateCookieJar::from_request_parts(parts, &router_state).await?;

    let Some(cookie) = cookie_jar.get("sid").map(|cookie| cookie.value().to_owned()) else {
      return Err(ApiError::Unauthorized);
    };

    let res = sqlx::query_as::<_, UserProfile>("
      SELECT users.* FROM sessions
      LEFT JOIN users ON sessions.user_id = users.id
      WHERE sessions.session_id = $1 LIMIT 1
    ").bind(cookie).fetch_one(&router_state.db).await?;

    Ok(res)
  }
}

