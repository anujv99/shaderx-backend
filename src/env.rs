
use dotenv::dotenv;

#[derive(Debug)]
pub struct Env {
  pub database_url: String,
  pub database_pwd: String,
  pub database_usr: String,
  pub google_client_id: String,
  pub google_client_secret: String,
  pub backend_port: u16,
}

const DEFAULT_PORT: u16 = 3000;

pub fn parse_env() -> Env {
  dotenv().ok();

  let port = std::env::var("BACKEND_PORT")
    .map(|port| port.parse().expect("BACKEND_PORT must be a number"))
    .unwrap_or(DEFAULT_PORT);

  Env {
    database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
    database_pwd: std::env::var("DATABASE_PWD").expect("DATABASE_PWD must be set"),
    database_usr: std::env::var("DATABASE_USR").expect("DATABASE_USR must be set"),
    google_client_id: std::env::var("GOOGLE_OAUTH_CLIENT_ID").expect("GOOGLE_OAUTH_CLIENT_ID must be set"),
    google_client_secret: std::env::var("GOOGLE_OAUTH_CLIENT_SECRET").expect("GOOGLE_OAUTH_CLIENT_SECRET must be set"),
    backend_port: port,
  }
}
