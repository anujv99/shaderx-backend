
use dotenv::dotenv;

#[derive(Debug)]
pub struct Env {
  pub database_url: String,
  pub database_pwd: String,
  pub database_usr: String,
}

pub fn parse_env() -> Env {
  dotenv().ok();
  Env {
    database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
    database_pwd: std::env::var("DATABASE_PWD").expect("DATABASE_PWD must be set"),
    database_usr: std::env::var("DATABASE_USR").expect("DATABASE_USR must be set"),
  }
}
