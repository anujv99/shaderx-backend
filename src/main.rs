use axum::{http::StatusCode, response::{Html, IntoResponse}, routing::{get, post}, serve, Extension, Router};
use router_state::{RouterState, UserProfile};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

mod logger;
mod env;
mod router_state;
mod routes;
mod auth;
mod errors;

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

  let router_state = router_state::RouterState::new(pool);

  let protected_router: Router<RouterState> = Router::new()
    .route("/", get(protected_page));

  let homepage_router: Router<RouterState> = Router::new()
    .route("/", get(homepage))
    .layer(Extension(env.google_client_id.clone()));

  let auth_router: Router<RouterState> = Router::new()
    .route("/auth/google_callback", get(routes::oauth::google_callback));

  let client = auth::build_oauth_client(&env);

  let app: Router = Router::new()
    .route("/shader", post(routes::shader::add_shader))
    .route("/shader/all", get(routes::shader::get_shaders))
    .route("/shader/:id", get(routes::shader::get_shader))
    .nest("/api", auth_router)
    .nest("/", homepage_router)
    .nest("/protected", protected_router)
    .with_state(router_state)
    .layer(Extension(client));

  let listener = TcpListener::bind("0.0.0.0:3000").await.expect("failed to bind to port 3000");
  serve(listener, app).await.expect("failed to start server");
}

// testing
async fn homepage(
  Extension(oauth_id): Extension<String>,
) -> Html<String> {
  Html(format!("
      <p>Welcome!</p>

      <a href=\"https://accounts.google.com/o/oauth2/v2/auth?scope=openid%20profile%20email&client_id={oauth_id}&response_type=code&redirect_uri=http://localhost:3000/api/auth/google_callback\">
        Login with Google
      </a>
    ")
  )
}


async fn protected_page(profile: UserProfile) -> impl IntoResponse {
  (StatusCode::OK, profile.email)
}
