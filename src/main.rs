use axum::{http::{header::CONTENT_TYPE, Method, StatusCode}, middleware, response::{Html, IntoResponse}, routing::{get, post}, serve, Extension, Json, Router};
use router_state::{RouterState, UserProfile};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

mod logger;
mod env;
mod router_state;
mod routes;
mod auth;
mod errors;
mod middlewares;
mod constants;

#[tokio::main]
async fn main() {
  log::set_logger(&logger::LOGGER).expect("failed to set logger");
  log::set_max_level(log::LevelFilter::Trace);

  let env: env::Env = env::parse_env();
  let database_url = format!("postgres://{}:{}@{}:5432/postgres",
    env.database_usr, env.database_pwd, env.database_url);

  log::trace!("connecting to database");
  let pool = PgPoolOptions::new()
    .max_connections(5)
    .acquire_timeout(std::time::Duration::from_secs(5))
    .connect(&database_url)
    .await.expect("failed to connect to database");
  log::trace!("connected to database");

  log::trace!("running migrations");
  sqlx::migrate!()
    .run(&pool)
    .await.expect("failed to run migrations");
  log::trace!("migrations ran successfully");

  let router_state = router_state::RouterState::new(pool, &env);

  let protected_router: Router<RouterState> = Router::new()
    .route("/", get(protected_page));

  let client = auth::build_oauth_client(&env);

  let auth_router: Router<RouterState> = Router::new()
    .route("/login", get(routes::oauth::get_login_url))
    .route("/google_callback", post(routes::oauth::google_callback))
    .route("/validate", get(routes::oauth::validate))
    .route("/logout", post(routes::oauth::logout))
    .layer(Extension(env.google_client_id.clone()));

  let app: Router = Router::new()
    .nest("/shader", routes::shader::build_shader_router())
    .nest("/auth", auth_router)
    .nest("/protected", protected_router)
    .with_state(router_state.clone())
    .layer(middleware::from_fn_with_state(router_state.clone(), middlewares::token_refresh::token_refresh_middleware))
    .layer(Extension(client))
    .layer(build_cors_layer(&env));

  let url = format!("0.0.0.0:{}", env.backend_port);
  let listener = TcpListener::bind(&url).await.expect(format!("failed to bind to {}", url).as_str());

  log::trace!("listening on {}", url);
  serve(listener, app).await.expect("failed to start server");
}

async fn protected_page(profile: UserProfile) -> impl IntoResponse {
  (StatusCode::OK, Json(profile))
}

fn build_cors_layer(env: &env::Env) -> CorsLayer {
  let origins = [
    env.frontend_url.parse().unwrap(),
  ];

  let headers = [
    CONTENT_TYPE,
  ];

  CorsLayer::new()
    .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_origin(origins)
    .allow_credentials(true)
    .allow_headers(headers)
}
