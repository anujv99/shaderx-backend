use axum::{routing::get, serve, Router};
// use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

mod logger;

#[tokio::main]
async fn main() {
  log::set_logger(&logger::LOGGER).expect("failed to set logger");
  log::set_max_level(log::LevelFilter::Trace);

  log::trace!("connecting to database");
  /*
  let pool = PgPoolOptions::new()
    .max_connections(5)
    .acquire_timeout(std::time::Duration::from_secs(5))
    .connect("postgres://admin:root@192.168.1.16:5432/postgres")
    .await.expect("failed to connect to database");
  */
  log::trace!("connected to database");

  let app: Router = Router::new()
    .route("/", get(root));

  let listener = TcpListener::bind("0.0.0.0:3000").await.expect("failed to bind to port 3000");
  serve(listener, app).await.expect("failed to start server");
}

async fn root() -> &'static str {
  "Hello, World! 123"
}
