use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distr::Alphanumeric, RngExt};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;
use tiny_http::{Response, Server};
use url::Url;

type Error = Box<dyn std::error::Error + Send + Sync>;

const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";

#[derive(Debug, Deserialize)]
struct SpotifyToken {
  access_token: String,
  scope: String,
  expires_in: u64,
  refresh_token: Option<String>,
}

fn generate_code_verifier() -> String {
  rand::rng()
    .sample_iter(Alphanumeric)
    .take(64)
    .map(char::from)
    .collect()
}

fn generate_code_challenge(verifier: &str) -> String {
  let hash = Sha256::digest(verifier.as_bytes());
  URL_SAFE_NO_PAD.encode(hash)
}

fn build_authorization_url(
  client_id: &str,
  challenge: &str,
) -> String {
  let mut url = Url::parse("https://accounts.spotify.com/authorize")
    .expect("Invalid Spotify authorization URL");

  url.query_pairs_mut()
    .append_pair("response_type", "code")
    .append_pair("client_id", client_id)
    .append_pair("scope", "user-modify-playback-state")
    .append_pair("code_challenge_method", "S256")
    .append_pair("code_challenge", challenge)
    .append_pair("redirect_uri", REDIRECT_URI);

  url.to_string()
}

fn wait_for_callback() -> Result<String, Error> {
  let server = Server::http("127.0.0.1:8888")?;

  println!("Waiting for Spotify callback...");

  let request = server.recv()?;

  let url = Url::parse(&format!("http://127.0.0.1{}", request.url()))?;

  let code = url
    .query_pairs()
    .find(|(key, _)| key == "code")
    .map(|(_, value)| value.to_string());

  let response = Response::from_string(
    "Spotify authorization complete. You can close this browser window.",
  );

  request.respond(response)?;

  code.ok_or_else(|| "Spotify did not return an authorization code".into())
}

fn exchange_code(
  client: &reqwest::blocking::Client,
  client_id: &str,
  code: &str,
  verifier: &str,
) -> Result<SpotifyToken, Error> {
  let token = client
    .post("https://accounts.spotify.com/api/token")
    .form(&[
      ("grant_type", "authorization_code"),
      ("code", code),
      ("redirect_uri", REDIRECT_URI),
      ("client_id", client_id),
      ("code_verifier", verifier),
    ])
    .send()?
    .error_for_status()?
    .json::<SpotifyToken>()?;

  Ok(token)
}

fn refresh_access_token(
  client: &reqwest::blocking::Client,
  client_id: &str,
  refresh_token: &str,
) -> Result<SpotifyToken, Error> {
  let token = client
    .post("https://accounts.spotify.com/api/token")
    .form(&[
      ("grant_type", "refresh_token"),
      ("refresh_token", refresh_token),
      ("client_id", client_id),
    ])
    .send()?
    .error_for_status()?
    .json::<SpotifyToken>()?;

  Ok(token)
}

fn get_access_token(
  client: &reqwest::blocking::Client,
  client_id: &str,
) -> Result<String, Error> {
  let token_path = "refresh_token";

  // Try existing refresh token first.
  if let Ok(refresh_token) = std::fs::read_to_string(token_path) {
    println!("Refreshing Spotify access token...");

    let token = refresh_access_token(
      client,
      client_id,
      refresh_token.trim(),
    )?;

    // Spotify can return a new refresh token.
    if let Some(new_refresh_token) = &token.refresh_token {
      std::fs::write(token_path, new_refresh_token)?;
    }

    return Ok(token.access_token);
  }

  // No refresh token: perform initial authorization.
  println!("No Spotify credentials found.");
  println!("Starting Spotify authorization...");

  // 1. Generate PKCE values
  let verifier = generate_code_verifier();
  let challenge = generate_code_challenge(&verifier);

  // 2. Build authorization URL
  let auth_url = build_authorization_url(
    &client_id,
    &challenge,
  );

  // 3. Start callback server BEFORE opening browser
  println!("Open this URL in your browser:");
  println!("{auth_url}");

  // Try to open the browser automatically.
  if let Err(error) = open::that(&auth_url) {
    eprintln!("Could not open browser automatically: {error}");
  }

  // 4. Wait for Spotify to redirect back to us
  let code = wait_for_callback()?;

  println!("Received authorization code.");

  // 5. Exchange code for access token
  let token = exchange_code(
    &client,
    &client_id,
    &code,
    &verifier,
  )?;

  println!("Access token obtained!");
  println!("Expires in: {} seconds", token.expires_in);
  println!("Scopes: {}", token.scope);

  let refresh_token = token
    .refresh_token
    .ok_or("Spotify did not return a refresh token")?;

  std::fs::write(token_path, &refresh_token)?;

  Ok(token.access_token)
}

#[derive(Debug, Deserialize)]
struct Player {
  device: Option<Device>,
  is_playing: bool,
  repeat_state: String,
  shuffle_state: bool,
}

#[derive(Debug, Deserialize)]
struct Device {
  id: String,
  name: String,
  volume_percent: u8,
  is_active: bool,
}

#[derive(Debug, Deserialize)]
struct Playing {
  item: Option<Track>,
}

#[derive(Debug, Deserialize)]
struct Track {
  id: String,
  name: String,
  artists: Vec<Artist>,
}

#[derive(Debug, Deserialize)]
struct Artist {
  id: String,
  name: String,
}

impl Player {
  fn get(
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<Self, Error> {
    let response = client
      .get("https://api.spotify.com/v1/me/player")
      .bearer_auth(access_token)
      .send()?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("Nothing is currently playing".into());
    }

    Ok(response.error_for_status()?.json()?)
  }

  fn resume(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(client, access_token, "https://api.spotify.com/v1/me/player/play")
  }

  fn pause(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(client, access_token, "https://api.spotify.com/v1/me/player/pause")
  }

  fn next(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(client, access_token, "https://api.spotify.com/v1/me/player/next")
  }

  fn prev(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(client, access_token, "https://api.spotify.com/v1/me/player/previous")
  }

  fn put(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
    url: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .put(url)
      .query(&[("device_id", device.id.as_str())])
      .bearer_auth(access_token)
      .send()?
      .error_for_status()?;

    Ok(())
  }

  fn switch_shuffle(
    &mut self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    let next_state = !self.shuffle_state;

    client
      .put("https://api.spotify.com/v1/me/player/shuffle")
      .query(&[
        ("state", next_state.to_string()),
        ("device_id", device.id.clone())
      ])
      .bearer_auth(access_token)
      .send()?
      .error_for_status()?;

    self.shuffle_state = next_state;

    Ok(())
  }

  fn switch_repeat(
    &mut self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    let states = ["track", "context", "off"];

    let index = states.iter().position(|&s| s == self.repeat_state).unwrap_or(0);

    let next_state = states[(index + 1) % 3];

    client
      .put("https://api.spotify.com/v1/me/player/repeat")
      .query(&[
        ("state", next_state),
        ("device_id", device.id.as_str())
      ])
      .bearer_auth(access_token)
      .send()?
      .error_for_status()?;

    self.repeat_state = next_state.to_string();

    Ok(())
  }

  fn get_track(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<Track, Error> {
    let response = client
      .get("https://api.spotify.com/v1/me/player/currently-playing")
      .bearer_auth(access_token)
      .send()?;

    let status = response.status();
    let text = response.text()?;

    println!("Response: {}", text);

    if status == reqwest::StatusCode::OK {
      let playing: Playing = serde_json::from_str(&text)?;

      println!("Playing: {:?}", playing);

      return playing
        .item
        .ok_or_else(|| "No currently playing track".into());
    }
    
    Err("No currently playing track".into())
  }

  fn device(&self) -> Result<&Device, Error> {
    self.device
      .as_ref()
      .ok_or("No Spotify device found".into())
  }
}

fn main() -> Result<(), Error> {
  dotenvy::dotenv().ok();

  let client_id = env::var("CLIENT_ID")?;
  let client = reqwest::blocking::Client::new();

  let access_token = get_access_token(
    &client,
    &client_id,
  )?;

  let mut player = Player::get(
    &client,
    &access_token,
  )?;

  let track = player.get_track(
    &client,
    &access_token,
  )?;

  println!("Track : {}", track.name);

  Ok(())
}
