use axum::{routing::{get, post}, serve, Router};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

mod logger;
mod env;
mod router_state;
mod routes;

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
  let app: Router = Router::new()
    .route("/shader", post(routes::shader::add_shader))
    .route("/shader/all", get(routes::shader::get_shaders))
    .route("/shader/:id", get(routes::shader::get_shader))
    .with_state(router_state);

  let listener = TcpListener::bind("0.0.0.0:3000").await.expect("failed to bind to port 3000");
  serve(listener, app).await.expect("failed to start server");
}

