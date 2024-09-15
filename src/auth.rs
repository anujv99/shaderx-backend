use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

use crate::env::Env;

pub fn build_oauth_client(env: &Env) -> BasicClient {
  let redirect_url = format!("{}/auth/google_callback", env.frontend_url);

  let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string())
    .expect("failed to parse auth url");
  let token_url = TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
    .expect("failed to parse token url");

  BasicClient::new(
    ClientId::new(env.google_client_id.clone()),
    Some(ClientSecret::new(env.google_client_secret.clone())),
    auth_url,
    Some(token_url),
  ).set_redirect_uri(RedirectUrl::new(redirect_url).expect("failed to parse redirect url"))
}

