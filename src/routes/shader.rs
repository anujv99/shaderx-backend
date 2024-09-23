
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{delete, get, post, put}, Json};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Execute};

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

#[derive(Debug, Deserialize)]
pub struct UpdateShaderData {
  pub name: Option<String>,
  pub description: Option<String>,
  pub data: Option<ShaderData>,
  pub access: Option<AccessLevel>,
  pub tags: Option<sqlx::types::JsonValue>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "access_level", rename_all = "lowercase")]
pub enum AccessLevel {
  Public,
  Unlisted,
  Private,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Shader {
  pub id: String,
  pub name: String,
  pub description: String,
  pub access: AccessLevel,
  pub tags: sqlx::types::JsonValue,
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
  let shader: Result<Shader, _> = sqlx::query_as("SELECT * FROM shaders WHERE id = $1 AND user_id = $2 AND deleted = false")
    .bind(id)
    .bind(&profile.user_id)
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
      (SELECT user_id FROM users WHERE user_id = $1 LIMIT 1), $2, $3, $4, $5)"
    )
    .bind(&profile.user_id)
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

pub async fn update_shader(
  State(router_state): State<RouterState>,
  Path(id): Path<String>,
  profile: UserProfile,
  Json(update_shader): Json<UpdateShaderData>
) -> Result<impl IntoResponse, impl IntoResponse> {
  let shader = get_shader_by_id(&router_state, &id, &profile).await?;

  let mut query_builder = sqlx::QueryBuilder::new("UPDATE shaders SET");
  let mut updated = false;

  if let Some(name) = update_shader.name {
    query_builder.push(" name = ");
    query_builder.push_bind(name);
    updated = true;
  }

  if let Some(description) = update_shader.description {
    query_builder.push(" description = ");
    query_builder.push_bind(description);
    updated = true;
  }

  if let Some(data) = update_shader.data {
    query_builder.push(" data = ");
    query_builder.push_bind(sqlx::types::Json(data));
    updated = true;
  }

  if let Some(access) = update_shader.access {
    query_builder.push(" access = ");
    query_builder.push_bind(access);
    updated = true;
  }

  if let Some(tags) = update_shader.tags {
    query_builder.push(" tags = ");
    query_builder.push_bind(tags);
    updated = true;
  }

  if !updated {
    return Ok(Json(shader));
  }

  query_builder.push(" WHERE id = ");
  query_builder.push_bind(&id);
  query_builder.push(" AND user_id = ");
  query_builder.push_bind(&profile.user_id);
  query_builder.push(" AND deleted = false");
  query_builder.push(" RETURNING *");

  let query = query_builder.build();
  let result = query
    .execute(&router_state.db)
    .await;

  match result {
    Err(e) => {
      log::error!("failed to execute query: {:?}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
    },
    Ok(_) => {
      let shader: Result<Shader, _> = get_shader_by_id(&router_state, &id, &profile).await;

      match shader {
        Ok(shader) => Ok(Json(shader)),
        Err(e) => {
          log::error!("failed to execute query: {:?}", e);
          Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
        }
      }
    }
  }
}

pub async fn get_shaders(
  State(router_state): State<RouterState>
) -> Result<impl IntoResponse, impl IntoResponse> {
  // select all shaders where deleted = false and public = true
  let shaders: Result<Vec<Shader>, _> = sqlx::query_as("SELECT * FROM shaders WHERE deleted = false AND access = 'public'")
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

pub async fn get_my_shaders(
  profile: UserProfile,
  State(router_state): State<RouterState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  // select all shaders where user_id = profile.user_id and deleted = false
  let shaders: Result<Vec<Shader>, _> = sqlx::query_as(
    "SELECT * FROM shaders WHERE user_id = $1 AND deleted = false"
  )
    .bind(&profile.user_id)
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

pub async fn get_my_deleted_shaders(
  profile: UserProfile,
  State(router_state): State<RouterState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  // select all shaders where user_id = profile.user_id and deleted = true
  let shaders: Result<Vec<Shader>, _> = sqlx::query_as(
    "SELECT * FROM shaders WHERE user_id = $1 AND deleted = true"
  )
    .bind(&profile.user_id)
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

pub async fn delete_shader(
  Path(id): Path<String>,
  profile: UserProfile,
  State(router_state): State<RouterState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  let result = sqlx::query(
    "UPDATE shaders SET deleted = true WHERE id = $1 AND user_id = $2"
  )
    .bind(&id)
    .bind(&profile.user_id)
    .execute(&router_state.db)
    .await;

  match result {
    Ok(_) => Ok(StatusCode::NO_CONTENT),
    Err(e) => {
      log::error!("failed to execute query: {:?}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
    }
  }
}

pub async fn force_delete_shader(
  Path(id): Path<String>,
  profile: UserProfile,
  State(router_state): State<RouterState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  let result = sqlx::query(
    "DELETE FROM shaders WHERE id = $1 AND user_id = $2"
  )
    .bind(&id)
    .bind(&profile.user_id)
    .execute(&router_state.db)
    .await;

  match result {
    Ok(_) => Ok(StatusCode::NO_CONTENT),
    Err(e) => {
      log::error!("failed to execute query: {:?}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
    }
  }
}

pub async fn restore_shader(
  Path(id): Path<String>,
  profile: UserProfile,
  State(router_state): State<RouterState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  let result = sqlx::query(
    "UPDATE shaders SET deleted = false WHERE id = $1 AND user_id = $2"
  )
    .bind(&id)
    .bind(&profile.user_id)
    .execute(&router_state.db)
    .await;

  match result {
    Ok(_) => Ok(StatusCode::NO_CONTENT),
    Err(e) => {
      log::error!("failed to execute query: {:?}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to execute query"))
    }
  }
}

pub fn build_shader_router() -> axum::Router<RouterState> {
  axum::Router::new()
    .route("/", post(add_shader))
    .route("/all", get(get_shaders))
    .route("/my", get(get_my_shaders))
    .route("/archive", get(get_my_deleted_shaders))
    .route("/:id", get(get_shader))
    .route("/:id", put(update_shader))
    .route("/:id/delete", post(delete_shader))
    .route("/:id/restore", post(restore_shader))
    .route("/:id", delete(force_delete_shader))
}

