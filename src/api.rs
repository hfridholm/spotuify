/*              _ 
 *   __ _ _ __ (_)
 *  / _` | '_ \| |
 * | (_| | |_) | |
 *  \__,_| .__/|_|
 *       |_|      
 */

use std::{
  env,
  io,
  sync::{
    Arc,
    Mutex,
    atomic::{AtomicU64, Ordering},
  },
  thread,
  collections::VecDeque,
};

use url::Url;

use serde::Deserialize;

use rand::{
  RngExt,
  distr::Alphanumeric,
};

use base64::{
  engine::general_purpose::URL_SAFE_NO_PAD,
  Engine,
};

use chrono::{DateTime, Utc};

use reqwest::{
  Client,
  header::CONTENT_LENGTH,
};

use sha2::{
  Digest,
  Sha256,
};

use tiny_http::{
  Response,
  Server,
};

type Error = Box<dyn std::error::Error + Send + Sync>;


// ============================================================================
// Spotify OAuth
// ============================================================================

const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";

#[derive(Debug, Deserialize)]
struct SpotifyToken {
  access_token:  String,
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
  let mut url = Url::parse(
    "https://accounts.spotify.com/authorize",
  )
    .expect("Invalid Spotify authorization URL");

  url.query_pairs_mut()
    .append_pair("response_type", "code")
    .append_pair("client_id", client_id)
    .append_pair(
      "scope",
      "user-read-playback-state user-modify-playback-state user-read-recently-played playlist-read-private playlist-modify-private playlist-modify-public playlist-read-collaborative user-follow-read user-follow-modify user-top-read",
    )
    .append_pair("code_challenge_method", "S256")
    .append_pair("code_challenge", challenge)
    .append_pair("redirect_uri", REDIRECT_URI);

  url.to_string()
}

fn wait_for_callback() -> Result<String, Error> {
  let server = Server::http("127.0.0.1:8888")?;

  let request = server.recv()?;

  let url = Url::parse(
    &format!("http://127.0.0.1{}", request.url())
  )?;

  let code = url
    .query_pairs()
    .find(|(key, _)| key == "code")
    .map(|(_, value)| value.to_string());

  let response = Response::from_string(
    "Spotify authorization complete. You can close this browser window.",
  );

  request.respond(response)?;

  code.ok_or_else(|| {
    "Spotify did not return an authorization code".into()
  })
}

fn exchange_code_blocking(
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

fn refresh_access_token_blocking(
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

fn get_access_token_blocking(
  client_id: &str,
) -> Result<String, Error> {
  let client = reqwest::blocking::Client::new();

  let token_path = "refresh_token";

  // Try existing refresh token first.
  if let Ok(refresh_token) = std::fs::read_to_string(token_path) {
    let token = refresh_access_token_blocking(
      &client,
      client_id,
      refresh_token.trim(),
    )?;

    if let Some(new_refresh_token) = &token.refresh_token {
      std::fs::write(
        token_path,
        new_refresh_token,
      )?;
    }

    return Ok(token.access_token);
  }

  // No refresh token: perform initial authorization.

  let verifier = generate_code_verifier();
  let challenge = generate_code_challenge(&verifier);

  let auth_url = build_authorization_url(
    client_id,
    &challenge,
  );

  // Start the callback server before opening the browser.
  //
  // wait_for_callback() is blocking, but this entire OAuth
  // operation is called from spawn_blocking by App::new().
  if let Err(error) = open::that(&auth_url) {
    eprintln!(
      "Could not open browser automatically: {error}"
    );

    eprintln!(
      "Open this URL manually:\n{auth_url}"
    );
  }

  let code = wait_for_callback()?;

  let token = exchange_code_blocking(
    &client,
    client_id,
    &code,
    &verifier,
  )?;

  let refresh_token = token
    .refresh_token
    .ok_or("Spotify did not return a refresh token")?;

  std::fs::write(
    token_path,
    &refresh_token,
  )?;

  Ok(token.access_token)
}

// ============================================================================
// Spotify data types
// ============================================================================

#[derive(Debug, Clone)]
pub struct SpotifyApi {
  client:       Client,
  access_token: Arc<str>,
}

impl SpotifyApi {
  pub async fn new() -> Result<Self, Error> {
    let client_id = env::var("CLIENT_ID")?;

    let access_token = tokio::task::spawn_blocking(move || {
      get_access_token_blocking(&client_id)
    }).await??;

    Ok(Self {
      client:       Client::new(),
      access_token: Arc::from(access_token),
    })
  }

  pub async fn get_search(&self, query: &str) -> Result<Search, Error> {
    Search::get(&self, &query).await
  }

  pub async fn get_player(&self) -> Result<Player, Error> {
    Player::get(&self).await
  }

  pub async fn get_queue(&self) -> Result<Queue, Error> {
    Queue::get(&self).await
  }

  pub async fn get_history(&self) -> Result<History, Error> {
    History::get(&self).await
  }

  pub async fn get_user(&self) -> Result<User, Error> {
    User::get(&self).await
  }

  pub async fn pause(&self, player: &Player) -> Result<(), Error> {
    player.pause(
      &self.client,
      &self.access_token,
    ).await
  }

  pub async fn resume(&self, player: &Player) -> Result<(), Error> {
    player.resume(
      &self.client,
      &self.access_token,
    ).await
  }

  pub async fn next(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.next(
      &self.client,
      &self.access_token,
    ).await
  }

  pub async fn previous(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.prev(
      &self.client,
      &self.access_token,
    ).await
  }

  pub async fn volume(
    &self,
    player: &Player,
    volume: u8,
  ) -> Result<(), Error> {
    player.volume(
      &self.client,
      &self.access_token,
      volume,
    ).await
  }

  pub async fn shuffle(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.toggle_shuffle(
      &self.client,
      &self.access_token,
    ).await
  }

  pub async fn repeat(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.toggle_repeat(
      &self.client,
      &self.access_token,
    ).await
  }

  pub async fn play(
    &self,
    player: &Player,
    uri: &str,
    context_uri: &str,
  ) -> Result<(), Error> {
    player.play(
      &self.client,
      &self.access_token,
      uri,
      context_uri,
    ).await
  }

  pub async fn queue(
    &self,
    player: &Player,
    uri: &str,
  ) -> Result<(), Error> {
    player.queue(
      &self.client,
      &self.access_token,
      uri,
    ).await
  }
}

// ============================================================================
// Spotify models
// ============================================================================

#[derive(Default, Debug, Clone, Deserialize)]
pub struct User {
  pub account_id:   String,
  pub display_name: String,
  pub uri:          String,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Player {
  pub device:        Option<Device>,
  pub progress_ms:   u32,
  pub context:       Option<Context>,
  pub item:          Option<Track>,
  pub is_playing:    bool,
  pub repeat_state:  String,
  pub shuffle_state: bool,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Device {
  pub id:             String,
  pub name:           String,
  pub volume_percent: u8,
  pub is_active:      bool,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct History {
  pub items: Vec<PlayedTrack>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct PlayedTrack {
  pub track:     Option<Track>,
  pub played_at: String,
}

impl PlayedTrack {
  pub fn get_elapsed(&self) -> u32 {
    let then = match DateTime::parse_from_rfc3339(&self.played_at) {
      Ok(value) => value.with_timezone(&Utc),
      Err(_) => return 0,
    };

    let now = Utc::now();

    let elapsed = now.signed_duration_since(then);

    elapsed
      .num_milliseconds()
      .max(0) as u32
  }
}

// Search result
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Search {
  pub tracks:    Tracks,
  pub artists:   Artists,
  pub albums:    Albums,
  pub playlists: Playlists,
}

// Spotify: Tracks
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Tracks {
  pub limit: u32,
  pub total: u32,
  pub items: Vec<Track>,
}

// Spotify: Artists
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Artists {
  pub limit: u32,
  pub total: u32,
  pub items: Vec<Artist>,
}

// Spotify: Albums
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Albums {
  pub limit: u32,
  pub total: u32,
  pub items: Vec<Album>,
}

// Spotify: Playlists
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Playlists {
  pub limit: u32,
  pub total: u32,
  pub items: Vec<Option<Playlist>>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Queue {
  pub currently_playing: Option<Track>,
  pub queue:             Vec<Track>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Context {
  pub uri: String,
}

// Spotify: TrackObject
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Track {
  pub album:       Album,
  pub id:          String,
  pub name:        String,
  pub uri:         String,
  pub duration_ms: u32,
  pub artists:     Vec<Artist>,
}

// Spotify ArtistObject
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Artist {
  pub id:   String,
  pub name: String,
}

// Spotify SimplifiedAlbumObject
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Album {
  pub album_type:   String,
  pub total_tracks: u32,
  pub release_date: String,
  pub id:           String,
  pub uri:          String,
  pub name:         String,
  pub artists:      Vec<Artist>,
}

// Spotify SimplifiedPlaylistObject
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Playlist {
  pub id:     String,
  pub name:   String,
  pub public: bool,
  pub items:  PlaylistItems,
  pub owner:  Owner,
}

// Spotify Owner
#[derive(Default, Debug, Clone, Deserialize)]
pub struct Owner {
  pub display_name: String,
  pub uri:          String,
}

// Sub-struct for getting items in playlist
#[derive(Default, Debug, Clone, Deserialize)]
pub struct PlaylistItems {
  pub href:  String,
  pub total: u32,
}

// ============================================================================
// Spotify API implementation
// ============================================================================

impl Playlists {
  pub async fn get(api: &SpotifyApi, user_id: &str) -> Result<Self, Error> {
    let response = api.client
      .get(format!("https://api.spotify.com/v1/users/{user_id}/playlists"))
      .bearer_auth(&api.access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("User got no playlists".into());
    }

    Ok(
      response
      .error_for_status()?
      .json::<Playlists>()
      .await?
    )
  }

  pub async fn get_current_users(api: &SpotifyApi) -> Result<Self, Error> {
    let response = api.client
      .get("https://api.spotify.com/v1/me/playlists")
      .bearer_auth(&api.access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("User got no playlists".into());
    }

    Ok(
      response
      .error_for_status()?
      .json::<Playlists>()
      .await?,
    )
  }
}

impl History {
  pub async fn get(
    api: &SpotifyApi,
  ) -> Result<Self, Error> {
    let response = api.client
      .get("https://api.spotify.com/v1/me/player/recently-played")
      .bearer_auth(&api.access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("No recently played tracks".into());
    }

    Ok(
      response
      .error_for_status()?
      .json::<History>()
      .await?,
    )
  }
}

impl Search {
  async fn get(
    api: &SpotifyApi,
    query: &str,
  ) -> Result<Self, Error> {
    let response = api.client
      .get("https://api.spotify.com/v1/search")
      .query(&[
        ("q", query),
        ("type", "album,artist,track,playlist"),
        ("limit", "5"),
      ])
      .bearer_auth(&api.access_token)
      .send()
      .await?
      .error_for_status()?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("Nothing is currently playing".into());
    }

    Ok(
      response
      .error_for_status()?
      .json::<Search>()
      .await?,
    )
  }
}

impl Queue {
  pub async fn get(
    api: &SpotifyApi,
  ) -> Result<Self, Error> {
    let response = api.client
      .get("https://api.spotify.com/v1/me/player/queue")
      .bearer_auth(&api.access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("Queue is empty".into());
    }

    Ok(
      response
      .error_for_status()?
      .json::<Queue>()
      .await?,
    )
  }

}

impl User {
  pub async fn get(
    api: &SpotifyApi,
  ) -> Result<Self, Error> {
    let response = api.client
      .get("https://api.spotify.com/v1/me")
      .bearer_auth(&api.access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("No user".into());
    }

    Ok(
      response
      .error_for_status()?
      .json::<User>()
      .await?,
    )
  }
}

impl Player {
  pub async fn get(
    api: &SpotifyApi,
  ) -> Result<Self, Error> {
    let response = api.client
      .get("https://api.spotify.com/v1/me/player")
      .bearer_auth(&api.access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("Nothing is currently playing".into());
    }

    Ok(
      response
      .error_for_status()?
      .json::<Player>()
      .await?,
    )
  }

  pub fn get_duration(&self) -> Result<u32, Error> {
    self.item
      .as_ref()
      .map(|track| track.duration_ms)
      .ok_or_else(|| "No track".into())
  }

  pub fn get_remaining(&self) -> Result<u32, Error> {
    let duration_ms = self.get_duration()?;

    Ok(duration_ms.saturating_sub(self.progress_ms))
  }

  pub async fn volume(
    &self,
    client: &Client,
    access_token: &str,
    volume_percent: u8,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .put("https://api.spotify.com/v1/me/player/volume")
      .query(&[
        ("device_id", &device.id.as_str()),
        ("volume_percent", &volume_percent.to_string().as_str()),
      ])
      .bearer_auth(access_token)
      .header(CONTENT_LENGTH, "0")
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  pub async fn queue(
    &self,
    client: &Client,
    access_token: &str,
    uri: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .post("https://api.spotify.com/v1/me/player/queue")
      .query(&[
        ("device_id", &device.id.as_str()),
        ("uri", &uri),
      ])
      .bearer_auth(access_token)
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  pub async fn play(
    &self,
    client: &Client,
    access_token: &str,
    uri: &str,
    context_uri: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    let body = if context_uri.is_empty() {
      serde_json::json!({
        "uris": [uri],
      })
    } else {
      serde_json::json!({
        "context_uri": context_uri,
        "offset": { "uri": uri },
      })
    };

    client
      .put("https://api.spotify.com/v1/me/player/play")
      .query(&[
        ("device_id", &device.id.as_str()),
      ])
      .json(&body)
      .bearer_auth(access_token)
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  pub async fn resume(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(
      client,
      access_token,
      "https://api.spotify.com/v1/me/player/play",
    ).await
  }

  pub async fn pause(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(
      client,
      access_token,
      "https://api.spotify.com/v1/me/player/pause",
    ).await
  }

  pub async fn next(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.post(
      client,
      access_token,
      "https://api.spotify.com/v1/me/player/next",
    ).await
  }

  pub async fn prev(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.post(
      client,
      access_token,
      "https://api.spotify.com/v1/me/player/previous",
    ).await
  }

  async fn put(
    &self,
    client: &Client,
    access_token: &str,
    url: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .put(url)
      .query(&[
        ("device_id", &device.id.as_str()),
      ])
      .bearer_auth(access_token)
      .header(CONTENT_LENGTH, "0")
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  async fn post(
    &self,
    client: &Client,
    access_token: &str,
    url: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .post(url)
      .query(&[
        ("device_id", &device.id.as_str()),
      ])
      .bearer_auth(access_token)
      .header(CONTENT_LENGTH, "0")
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  pub async fn toggle_shuffle(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    let next_state = !self.shuffle_state;

    client
      .put(
        "https://api.spotify.com/v1/me/player/shuffle",
      )
      .query(&[
        ("state", next_state.to_string()),
        ("device_id", device.id.clone()),
      ])
      .bearer_auth(access_token)
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  pub async fn toggle_repeat(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    let states = [
      "track",
      "context",
      "off",
    ];

    let index = states
      .iter()
      .position(|&state| {
        state == self.repeat_state
      })
      .unwrap_or(0);

    let next_state = states[(index + 1) % states.len()];

    client
      .put(
        "https://api.spotify.com/v1/me/player/repeat",
      )
      .query(&[
        ("state", next_state),
        ("device_id", &device.id.clone()),
      ])
      .bearer_auth(access_token)
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  fn device(&self) -> Result<&Device, Error> {
    self.device
      .as_ref()
      .ok_or_else(|| {
        "No Spotify device found".into()
      })
  }
}
