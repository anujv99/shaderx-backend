use sqlx::{Pool, Postgres};


#[derive(Debug, Clone)]
pub struct RouterState {
  pub db: Pool<Postgres>,
}

impl RouterState {
  pub fn new(db: Pool<Postgres>) -> Self {
    Self { db }
  }
}
