
use axum::{extract::{Path, Request, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Json};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::router_state::{RouterState, UserProfile};

#[derive(Debug, Serialize, Deserialize)]
pub struct ShaderData {
  pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct NewShaderData {
  pub name: String,
  pub description: String,
  pub data: ShaderData,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Shader {
  pub id: String,
  pub name: String,
  pub description: String,
  pub data: sqlx::types::Json<ShaderData>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

async fn generate_shader_id(
  router_state: &RouterState
) -> Result<String, (axum::http::StatusCode, &'static str)> {
  loop {
    let id = nanoid!(6);
    let shader: Option<Shader> = sqlx::query_as("SELECT id FROM shaders WHERE id = $1")
      .bind(&id)
      .fetch_optional(&router_state.db)
      .await
      .map_err(|e| {
        log::error!("failed to execute query: {:?}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query")
      })?;

    if shader.is_none() {
      return Ok(id);
    }
  }
}

async fn get_shader_by_id(
  router_state: &RouterState,
  id: &str,
  profile: &UserProfile,
) -> Result<Shader, (axum::http::StatusCode, &'static str)> {
  let shader: Result<Shader, _> = sqlx::query_as("SELECT * FROM shaders WHERE id = $1")
    .bind(id)
    .fetch_one(&router_state.db)
    .await;

  match shader {
    Ok(shader) => Ok(shader),
    Err(e) => {
      log::error!("failed to execute query: {:?}", e);
      Err((axum::http::StatusCode::NOT_FOUND, "shader not found"))
    }
  }
}

pub async fn add_shader(
  State(router_state): State<RouterState>,
  profile: UserProfile,
  Json(new_shader): Json<NewShaderData>
) -> Result<impl IntoResponse, impl IntoResponse> {
  let id = generate_shader_id(&router_state).await?;

  let result = sqlx::query(
    "INSERT INTO shaders (user_id, id, name, description, data) VALUES (
      (SELECT id FROM users WHERE email = $1 LIMIT 1), $2, $3, $4, $5)"
    )
    .bind(&profile.email)
    .bind(&id)
    .bind(&new_shader.name)
    .bind(&new_shader.description)
    .bind(sqlx::types::Json(new_shader.data))
    .execute(&router_state.db)
    .await;

  match result {
    Err(e) => return {
      log::error!("failed to execute query: {:?}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
    },
    _ => (),
  };

  let shader: Result<Shader, _> = get_shader_by_id(&router_state, &id, &profile).await;

  match shader {
    Ok(shader) => Ok(Json(shader)),
    Err(e) => {
      log::error!("failed to execute query: {:?}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
    }
  }
}

pub async fn get_shaders(
  State(router_state): State<RouterState>
) -> Result<impl IntoResponse, impl IntoResponse> {
  let shaders: Result<Vec<Shader>, _> = sqlx::query_as("SELECT * FROM shaders")
    .fetch_all(&router_state.db)
    .await;

  match shaders {
    Ok(shaders) => Ok(Json(shaders)),
    Err(e) => {
      log::error!("failed to execute query: {:?}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
    }
  }
}

pub async fn get_shader(
  Path(id): Path<String>,
  profile: UserProfile,
  State(router_state): State<RouterState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  get_shader_by_id(&router_state, &id, &profile).await
    .map(|shader| Json(shader))
}

pub fn build_shader_router() -> axum::Router<RouterState> {
  axum::Router::new()
    .route("/", post(add_shader))
    .route("/all", get(get_shaders))
    .route("/:id", get(get_shader))
}

