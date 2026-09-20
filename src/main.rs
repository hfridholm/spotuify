/*                  _         _  __       
 *  ___ _ __   ___ | |_ _   _(_)/ _|_   _ 
 * / __| '_ \ / _ \| __| | | | | |_| | | |
 * \__ \ |_) | (_) | |_| |_| | |  _| |_| |
 * |___/ .__/ \___/ \__|\__,_|_|_|  \__, |
 *     |_|                          |___/ 
 */

use std::{
  env,
  io,
  sync::{ Arc, Mutex },
  thread,
  time::Duration,
  collections::VecDeque,
};

use base64::{
  engine::general_purpose::URL_SAFE_NO_PAD,
  Engine,
};

use chrono::{DateTime, Utc};

use crossterm::event::{
  self,
  Event,
  KeyCode,
  KeyEvent,
  KeyEventKind,
};

use rand::{
  RngExt,
  distr::Alphanumeric,
};

use ratatui::{
  buffer::Buffer,
  layout::{
    Constraint,
    Layout,
    Rect,
    Direction,
  },
  style::{
    Color,
    Style,
    Stylize,
    Modifier,
  },
  symbols::border,
  text::{
    Line,
    Text,
    Span,
  },
  widgets::{
    Block,
    BorderType,
    Cell,
    LineGauge,
    Paragraph,
    Row,
    StatefulWidget,
    Table,
    TableState,
    Tabs,
    Widget,
    Wrap,
    Borders,
    Clear,
  },
  DefaultTerminal,
  Frame,
};

use reqwest::{
  Client,
  header::CONTENT_LENGTH,
};

use serde::Deserialize;

use sha2::{
  Digest,
  Sha256,
};

use tiny_http::{
  Response,
  Server,
};

use tokio::{
  sync::mpsc::{
    self,
    UnboundedReceiver,
    UnboundedSender,
  },
  time,
};

use url::Url;

type Error = Box<dyn std::error::Error + Send + Sync>;

const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";


// ============================================================================
// Spotify OAuth
// ============================================================================

#[derive(Debug, Deserialize)]
struct SpotifyToken {
  access_token:  String,
  #[allow(dead_code)]
  scope:         String,
  #[allow(dead_code)]
  expires_in:    u64,
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
      "user-read-playback-state user-modify-playback-state user-read-recently-played",
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

#[derive(Default, Debug, Clone)]
struct SpotifyState {
  player:  Player,
  queue:   Option<Queue>,
  history: Option<History>,
}

#[derive(Debug, Clone)]
struct SpotifyApi {
  client:       Client,
  access_token: Arc<str>,
}

impl SpotifyApi {
  async fn new() -> Result<Self, Error> {
    let client_id = env::var("CLIENT_ID")?;

    let access_token = tokio::task::spawn_blocking(move || {
      get_access_token_blocking(&client_id)
    }).await??;

    Ok(Self {
      client:       Client::new(),
      access_token: Arc::from(access_token),
    })
  }

  async fn fetch_state(
    &self,
  ) -> Result<SpotifyState, Error> {
    let player = Player::get(
      &self.client,
      &self.access_token,
    ).await?;

    let queue = player.get_queue(
      &self.client,
      &self.access_token,
    ).await?;

    /*
    let history = player.get_history(
      &self.client,
      &self.access_token,
    ).await?;
    */

    Ok(SpotifyState {
      player,
      queue: Some(queue),
      history: None,
    })
  }

  async fn refresh(
    &self,
  ) -> Result<SpotifyState, Error> {
    self.fetch_state().await
  }

  async fn pause(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.pause(
      &self.client,
      &self.access_token,
    ).await
  }

  async fn resume(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.resume(
      &self.client,
      &self.access_token,
    ).await
  }

  async fn next(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.next(
      &self.client,
      &self.access_token,
    ).await
  }

  async fn previous(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.prev(
      &self.client,
      &self.access_token,
    ).await
  }

  async fn volume(
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

  async fn shuffle(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.toggle_shuffle(
      &self.client,
      &self.access_token,
    ).await
  }

  async fn repeat(
    &self,
    player: &Player,
  ) -> Result<(), Error> {
    player.toggle_repeat(
      &self.client,
      &self.access_token,
    ).await
  }

  async fn play(
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

  async fn queue(
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
struct Player {
  device:        Option<Device>,
  progress_ms:   u32,
  context:       Option<Context>,
  item:          Option<Track>,
  is_playing:    bool,
  repeat_state:  String,
  shuffle_state: bool,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct Device {
  id:             String,
  name:           String,
  volume_percent: u8,
  is_active:      bool,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct History {
  items: Vec<PlayedTrack>,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct PlayedTrack {
  track:     Option<Track>,
  played_at: String,
}

impl PlayedTrack {
  fn get_elapsed(&self) -> u32 {
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

#[derive(Default, Debug, Clone, Deserialize)]
struct Queue {
  currently_playing: Option<Track>,
  queue:             Vec<Track>,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct Context {
  uri: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct Track {
  id:          String,
  name:        String,
  uri:         String,
  duration_ms: u32,
  artists:     Vec<Artist>,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct Artist {
  id:   String,
  name: String,
}

// ============================================================================
// Spotify API implementation
// ============================================================================

impl Player {
  async fn get(
    client: &Client,
    access_token: &str,
  ) -> Result<Self, Error> {
    let response = client
      .get("https://api.spotify.com/v1/me/player")
      .bearer_auth(access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err(
        "Nothing is currently playing".into()
      );
    }

    Ok(
      response
      .error_for_status()?
      .json::<Player>()
      .await?,
    )
  }

  fn get_duration(&self) -> Result<u32, Error> {
    self.item
      .as_ref()
      .map(|track| track.duration_ms)
      .ok_or_else(|| "No track".into())
  }

  fn get_remaining(&self) -> Result<u32, Error> {
    let duration_ms = self.get_duration()?;

    Ok(duration_ms.saturating_sub(self.progress_ms))
  }

  async fn get_queue(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<Queue, Error> {
    let response = client
      .get(
        "https://api.spotify.com/v1/me/player/queue",
      )
      .bearer_auth(access_token)
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

  async fn get_history(
    &self,
    client: &Client,
    access_token: &str,
  ) -> Result<History, Error> {
    let response = client
      .get(
        "https://api.spotify.com/v1/me/player/recently-played",
      )
      .bearer_auth(access_token)
      .send()
      .await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err(
        "No recently played tracks".into()
      );
    }

    Ok(
      response
      .error_for_status()?
      .json::<History>()
      .await?,
    )
  }

  async fn volume(
    &self,
    client: &Client,
    access_token: &str,
    volume_percent: u8,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .put(
        "https://api.spotify.com/v1/me/player/volume",
      )
      .query(&[
        ("device_id", &device.id.as_str()),
        (
          "volume_percent",
          &volume_percent.to_string().as_str(),
        ),
      ])
      .bearer_auth(access_token)
      .header(CONTENT_LENGTH, "0")
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }

  async fn queue(
    &self,
    client: &Client,
    access_token: &str,
    uri: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .post(
        "https://api.spotify.com/v1/me/player/queue",
      )
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

  async fn play(
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
      .put(
        "https://api.spotify.com/v1/me/player/play",
      )
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

  async fn resume(
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

  async fn pause(
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

  async fn next(
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

  async fn prev(
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

  async fn toggle_shuffle(
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

  async fn toggle_repeat(
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

// ============================================================================
// Helpers
// ============================================================================

fn format_ms(ms: u32) -> String {
  // let sec = (ms / 1000) % 60;
  let min = (ms / 1000) / 60;

  // format!("{:>2}m {:0>2}s", min, sec)
  format!("{:>2}m", min)
}

// ============================================================================
// Focus
// ============================================================================

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum Focus {
  #[default]
  Player,
  Queue,
  History,
}

impl Focus {
  fn next(&self) -> Self {
    match self {
      Self::Player  => Self::History,
      Self::Queue   => Self::Player,
      Self::History => Self::Queue,
    }
  }

  fn prev(&self) -> Self {
    match self {
      Self::Player  => Self::Queue,
      Self::Queue   => Self::History,
      Self::History => Self::Player,
    }
  }
}

// ============================================================================
// Actions
// ============================================================================

#[derive(Debug, Clone)]
enum Action {
  Quit,
  Close,

  Tick,
  Render,

  Refresh,

  FocusNext,
  FocusPrevious,

  PauseResume,

  Next,
  Previous,

  VolumeUp,
  VolumeDown,
  Mute,

  ToggleShuffle,
  ToggleRepeat,

  QueueMoveNext,
  QueueMovePrevious,
  QueueFirst,
  QueueLast,

  HistoryMoveNext,
  HistoryMovePrevious,
  HistoryFirst,
  HistoryLast,

  PlayQueue,

  PlayHistory,

  QueueHistory,

  SpotifyStateUpdated {
    state: SpotifyState,
  },

  Error(String),
}

// ============================================================================
// App
// ============================================================================

struct App {
  spotify:       SpotifyApi,
  state:         SpotifyState,

  focus:         Arc<Mutex<Focus>>,

  queue_state:   TableState,
  history_state: TableState,

  action_tx:     UnboundedSender<Action>,

  error_list:    VecDeque<String>,
  exit:          bool,

  // Used to make the progress gauge move smoothly between
  // Spotify API refreshes.
  last_player_update: std::time::Instant,
}

impl App {
  async fn new(
    action_tx: UnboundedSender<Action>,
  ) -> Result<Self, Error> {
    let spotify = SpotifyApi::new().await?;

    Ok(Self {
      spotify,
      state: SpotifyState::default(),

      focus: Arc::new(
        Mutex::new(Focus::Player)
      ),

      queue_state:   TableState::default(),
      history_state: TableState::default(),

      action_tx,

      error_list: VecDeque::new(),
      exit:       false,

      last_player_update: std::time::Instant::now(),
    })
  }

  async fn run(
    &mut self,
    terminal: &mut DefaultTerminal,
    mut action_rx: UnboundedReceiver<Action>,
  ) -> Result<(), Error> {
    // Immediately fetch Spotify state.
    self.action_tx.send(Action::Refresh)?;

    // Render playing song progress bar every 33 ms
    let mut render_interval = time::interval(
      Duration::from_millis(33),
    );

    // Fetch new data from API every 10 s
    let mut refresh_interval = time::interval(
      Duration::from_secs(10),
    );

    // Avoid an immediate duplicate refresh from the interval.
    refresh_interval.tick().await;

    while !self.exit {
      tokio::select! {
        Some(action) = action_rx.recv() => {
          self.update(action);

          // Render immediately after a state/action update
          terminal.draw(|frame| { self.draw(frame) })?;
        }

          // The 30 FPS interval below also
          // keeps the progress gauge moving.
        _ = render_interval.tick() => {
          self.update_progress();

          terminal.draw(|frame| { self.draw(frame) })?;
        }

        _ = refresh_interval.tick() => {
          // self.action_tx.send(Action::Refresh)?;
        }
      }
    }

    Ok(())
  }

  // ------------------------------------------------------------------------
  // Action update
  // ------------------------------------------------------------------------

  fn update(&mut self, action: Action) {
    match action {
      Action::Quit => {
        self.exit = true;
      }

      Action::Tick => {}

      Action::Render => {}

      Action::FocusNext => {
        match self.focus.lock() {
          Ok(mut focus) => *focus = focus.next(),
          Err(_) => return,
        };
      }

      Action::FocusPrevious => {
        match self.focus.lock() {
          Ok(mut focus) => *focus = focus.prev(),
          Err(_) => return,
        };
      }

      Action::Close => {
        self.error_list.pop_front();
      }

      Action::Refresh => {
        self.spawn_refresh();
      }

      Action::PauseResume => {
        if self.state.player.is_playing {
          self.spawn_pause();
        } else {
          self.spawn_resume();
        }
      }

      Action::Next => {
        self.spawn_next();
      }

      Action::Previous => {
        self.spawn_previous();
      }

      Action::VolumeUp => {
        self.spawn_volume_up();
      }

      Action::VolumeDown => {
        self.spawn_volume_down();
      }

      Action::Mute => {
        self.spawn_mute();
      }

      Action::ToggleShuffle => {
        self.spawn_shuffle();
      }

      Action::ToggleRepeat => {
        self.spawn_repeat();
      }

      Action::QueueMoveNext => {
        self.queue_state.select_next();
      }

      Action::QueueMovePrevious => {
        self.queue_state.select_previous();
      }

      Action::QueueFirst => {
        self.queue_state.select_first();
      }

      Action::QueueLast => {
        self.queue_state.select_last();
      }

      Action::HistoryMoveNext => {
        self.history_state.select_next();
      }

      Action::HistoryMovePrevious => {
        self.history_state.select_previous();
      }

      Action::HistoryFirst => {
        self.history_state.select_first();
      }

      Action::HistoryLast => {
        self.history_state.select_last();
      }

      Action::PlayQueue => {
        self.spawn_play_queue();
      }

      Action::PlayHistory => {
        self.spawn_play_history();
      }

      Action::QueueHistory => {
        self.spawn_queue_history();
      }

      Action::SpotifyStateUpdated { state } => {
        self.state = state;

        self.last_player_update = std::time::Instant::now();

        self.clamp_selection();
      }

      Action::Error(error) => {
        self.error_list.push_back(error);
      }
    }
  }

  // ------------------------------------------------------------------------
  // Progress
  // ------------------------------------------------------------------------

  // Artificially (locally) update the progress time of the playing song
  fn update_progress(&mut self) {
    if !self.state.player.is_playing {
      return;
    }

    let elapsed = self.last_player_update
      .elapsed()
      .as_millis() as u32;

    // Artificially add the elapsed time,
    // without needing to fetch new data
    self.state.player.progress_ms = self.state.player.progress_ms.saturating_add(elapsed);

    self.last_player_update = std::time::Instant::now();

    // Clamp the progress time to the duration,
    // so it doesn't overflow
    if let Ok(duration) = self.state.player.get_duration() {
      self.state.player.progress_ms = self.state.player.progress_ms.min(duration);
    }
  }

  // ------------------------------------------------------------------------
  // Async Spotify operations
  // ------------------------------------------------------------------------

  fn spawn_refresh(&self) {
    let spotify = self.spotify.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      match spotify.refresh().await {
        Ok(state) => {
          let _ = tx.send(
            Action::SpotifyStateUpdated { state },
          );
        }

        Err(error) => {
          let _ = tx.send(
            Action::Error(format!("Refresh failed: {error}")),
          );
        }
      }
    });
  }

  fn spawn_pause(&self) {
    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.pause(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_resume(&self) {
    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.resume(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_next(&self) {
    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.next(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_previous(&self) {
    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.previous(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_volume_up(&self) {
    let volume = self.get_volume_percent()
      .unwrap_or(0)
      .saturating_add(10)
      .min(100);

    self.spawn_volume(volume);
  }

  fn spawn_volume_down(&self) {
    let volume = self.get_volume_percent()
      .unwrap_or(0)
      .saturating_sub(10);

    self.spawn_volume(volume);
  }

  fn spawn_mute(&self) {
    self.spawn_volume(0);
  }

  fn spawn_volume(&self, volume: u8) {
    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.volume(&player, volume).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_shuffle(&self) {
    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.shuffle(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_repeat(&self) {
    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.repeat(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_play_queue(&self) {
    let Some(index) =
      self.queue_state.selected()
    else {
      return;
    };

    let Some(track) = self.state
      .queue
      .as_ref()
      .and_then(|queue| {
        queue.queue.get(index)
      })
    else {
      return;
    };

    let uri = track.uri.clone();

    let context_uri = self.get_context_uri()
      .unwrap_or_default();

    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.play(
        &player,
        &uri,
        &context_uri,
      ).await
      {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_play_history(&self) {
    let Some(index) =
      self.history_state.selected()
    else {
      return;
    };

    let Some(track) = self.state
      .history
      .as_ref()
      .and_then(|history| {
        history
          .items
          .iter()
          .filter_map(|item| {
            item.track.as_ref()
          })
        .nth(index)
      })
    else {
      return;
    };

    let uri = track.uri.clone();

    let context_uri = self.get_context_uri()
      .unwrap_or_default();

    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.play(
        &player,
        &uri,
        &context_uri,
      ).await
      {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  fn spawn_queue_history(&self) {
    let Some(index) =
      self.history_state.selected()
    else {
      return;
    };

    let Some(track) = self.state
      .history
      .as_ref()
      .and_then(|history| {
        history
          .items
          .iter()
          .filter_map(|item| {
            item.track.as_ref()
          })
        .nth(index)
      })
    else {
      return;
    };

    let uri = track.uri.clone();

    let spotify = self.spotify.clone();
    let player = self.state.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.queue(&player, &uri).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  // ------------------------------------------------------------------------
  // Helpers
  // ------------------------------------------------------------------------

  fn get_volume_percent(&self) -> Result<u8, Error> {
    self.state
      .player
      .device
      .as_ref()
      .map(|device| device.volume_percent)
      .ok_or_else(|| {
        "No Spotify device found".into()
      })
  }

  fn get_context_uri(&self) -> Result<String, Error> {
    self.state
      .player
      .context
      .as_ref()
      .map(|context| context.uri.clone())
      .ok_or_else(|| {
        "No track context".into()
      })
  }

  fn clamp_selection(&mut self) {
    if let Some(queue) = &self.state.queue {
      if queue.queue.is_empty() {
        self.queue_state.select(None);
      }
      else if let Some(index) = self.queue_state.selected() {
        if index >= queue.queue.len() {
          self.queue_state.select(Some(queue.queue.len() - 1));
        }
      }
    }

    if let Some(history) = &self.state.history {
      let count = history
        .items
        .iter()
        .filter(|item| item.track.is_some())
        .count();

      if count == 0 {
        self.history_state.select(None);
      }
      else if let Some(index) = self.history_state.selected() {
        if index >= count {
          self.history_state.select(Some(count - 1));
        }
      }
    }
  }

  // ------------------------------------------------------------------------
  // Drawing
  // ------------------------------------------------------------------------

  fn draw(&mut self, frame: &mut Frame) {
    frame.render_widget(self, frame.area());
  }

  fn render_navbar(
    &self,
    area: Rect,
    buf: &mut Buffer,
    selected: usize
  ) {
    let tabs = ["Home", "Search"];

    let width: u16 = tabs.iter().map(|t| t.len() as u16).sum::<u16>()
      + (tabs.len() as u16 - 1) * 3 + 1;

    let block = Block::bordered();

    block.clone().render(area, buf);

    let centered = Layout::horizontal([
      Constraint::Fill(1),
      Constraint::Length(width),
      Constraint::Fill(1),
    ])
      .split(block.inner(area));

    Tabs::new(tabs)
      .select(selected)
      .style(
        Style::default()
        .fg(Color::Gray)
      )
      .highlight_style(
        Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD),
      )
      .render(centered[1], buf);
  }

  fn render_player(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let focus = match self.focus.lock() {
      Ok(focus) => *focus,
      Err(_) => return,
    };

    let color = if focus == Focus::Player {
      Color::Green
    } else {
      Color::White
    };

    let block = Self::panel(" Player ", color);

    block.clone().render(area, buf);

    let inner = block.inner(area);

    let [text_area, gauge_area] = Layout::vertical([
      Constraint::Min(1),
      Constraint::Length(1),
    ])
      .areas(inner);

    let text = match &self.state.player.item {
      Some(track) => {
        let artists =
          track
          .artists
          .iter()
          .map(|artist| {
            artist.name.as_str()
          })
        .collect::<Vec<_>>()
          .join(", ");

        Text::from(vec![
          Line::from(track.name.as_str().bold()),
          Line::from(artists),
        ])
      }

      None => {
        Text::from("No track playing")
      }
    };

    let volume_percent = self.get_volume_percent()
      .unwrap_or(0);

    Paragraph::new(format!("{volume_percent}%"))
      .right_aligned()
      .render(text_area, buf);

    Paragraph::new(text)
      .centered()
      .render(text_area, buf);

    let ratio = if let Ok(duration_ms) = self.state.player.get_duration() {
      if duration_ms == 0 {
        0.0
      } else {
        (self.state.player.progress_ms.min(duration_ms)) as f64 / duration_ms as f64
      }
    } else {
      0.0
    };

    let gauge = LineGauge::default()
      .filled_style(
        Style::default()
        .fg(color),
      )
      .unfilled_style(
        Style::default()
        .fg(Color::DarkGray),
      )
      .ratio(ratio);

    gauge.render(gauge_area, buf);
  }

  fn render_queue(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let focus = match self.focus.lock() {
      Ok(focus) => *focus,
      Err(_) => return,
    };

    let (block_color, header_color) = if focus == Focus::Queue {
      (Color::Green, Color::Yellow)
    } else {
      (Color::White, Color::White)
    };

    let block = Self::panel(" Queue ", block_color);

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("TRACK"),
      Cell::from("IN"),
    ])
      .style(
        Style::default()
        .fg(header_color),
      )
      .bold();

    let mut remaining_ms = self.state.player.get_remaining()
      .unwrap_or(0);

    let rows = match &self.state.queue {
      Some(queue) if !queue.queue.is_empty() => {
        queue.queue
          .iter()
          .enumerate()
          .map(|(index, track)| {
            let time_str = format_ms(remaining_ms);

            remaining_ms = remaining_ms.saturating_add(track.duration_ms);

            Row::new([
              Cell::from(format!("{:>2}", index + 1)),
              Cell::from(track.name.as_str()),
              Cell::from(time_str),
            ])
          },
        )
          .collect::<Vec<_>>()
      }

      _ => {
        vec![
          Row::new([
            Cell::from(""),
            Cell::from("No queue"),
            Cell::from(""),
          ])
        ]
      }
    };

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
      Constraint::Length(3),
    ])
      .header(header)
      .column_spacing(2)
      .block(block)
      .style(
        Style::default()
        .fg(Color::White),
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray),
      );

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.queue_state,
    );
  }

  fn render_history(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let focus = match self.focus.lock() {
      Ok(focus) => *focus,
      Err(_) => return,
    };

    let (block_color, header_color) = if focus == Focus::History {
      (Color::Green, Color::Yellow)
    } else {
      (Color::White, Color::White)
    };

    let block = Self::panel(" History ", block_color);

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("TRACK"),
      Cell::from("AGO"),
    ])
      .style(
        Style::default()
        .fg(header_color),
      )
      .bold();

    let rows = match &self.state.history {
      Some(history) if !history.items.is_empty() => {
        let items = history.items
          .iter()
          .filter_map(|played_track| {
            played_track.track
              .as_ref()
              .map(|track| {
                (played_track, track)
              })
            },
          )
          .enumerate()
          .map(|(index, (played_track, track))| {
            let time_str = format_ms(played_track.get_elapsed());

            Row::new([
              Cell::from(format!("{:>2}", index + 1)),
              Cell::from(track.name.as_str()),
              Cell::from(time_str),
            ])
          })
            .collect::<Vec<_>>();

        if items.is_empty() {
          vec![
            Row::new([
              Cell::from(""),
              Cell::from("No history"),
              Cell::from(""),
            ])
          ]
        } else {
          items
        }
      }

      _ => {
        vec![
          Row::new([
            Cell::from(""),
            Cell::from("No history"),
            Cell::from(""),
          ])
        ]
      }
    };

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
      Constraint::Length(3),
    ])
      .header(header)
      .column_spacing(2)
      .block(block)
      .style(
        Style::default()
        .fg(Color::White),
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray),
      );

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.history_state,
    );
  }

  fn render_instructions(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let focus = match self.focus.lock() {
      Ok(focus) => *focus,
      Err(_) => return,
    };

    let instructions = match focus {
      Focus::Player => {
        Line::from(
          "Space Pause/Resume | Enter Refresh | h/← Previous | l/→ Next | ↑/↓ Volume | m Mute | s Shuffle | r Repeat",
        )
      }

      Focus::Queue => {
        Line::from(
          "Enter Play | ↑/k Up | ↓/j Down | Home/End Jump",
        )
      }

      Focus::History => {
        Line::from(
          "Enter Play | h/← Queue | ↑/k Up | ↓/j Down | Home/End Jump",
        )
      }
    }
      .blue();

    Paragraph::new(instructions)
      .centered()
      .wrap(Wrap { trim: true })
      .render(area, buf);
  }

  fn render_error(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let mut y = area.y;

    for (index, error) in self.error_list.iter().enumerate() {
      let text_width = error.len() as u16;

      // +2 for left/right borders +2 padding
      let width = (text_width + 4).min(area.width);

      // Width available for text inside the borders.
      let content_width = width.saturating_sub(2).max(1);

      // Number of lines required when wrapped.
      let lines = text_width.div_ceil(content_width);

      // +3 for top/bottom borders + "Error"
      let height = (lines + 3).min(area.height.saturating_sub(y - area.y));

      let rect = Rect {
        x: area.x + area.width - width,
        y,
        width,
        height,
      };

      // Clears rect from any previously rendered text
      Clear.render(rect, buf);

      let mut block = Block::bordered()
        .border_type(
          BorderType::Thick
        )
        .border_style(
          Style::default()
          .fg(Color::Red)
        );


      if index == 0 {
        block = block.title(
          Line::from(vec![
            Span::styled(" c",    Style::default().fg(Color::Blue)),
            Span::styled("lose ", Style::default().fg(Color::White)),
          ])
          .right_aligned(),
        );
      }

      Paragraph::new(Text::from(vec![
        Line::from("Error".to_string().red()),
        Line::from(error.as_str().bold().white()),
      ]))
        .centered()
        .wrap(Wrap { trim: false })
        .block(block)
        .render(rect, buf);

      y += height;
    }
  }

  fn panel(
    title: &str,
    color: Color,
  ) -> Block<'static> {
    Block::bordered()
      .title(
        Line::from(format!(" {title} ").bold())
        .left_aligned(),
      )
      .border_type(
        BorderType::Thick
      )
      .border_style(
        Style::default()
        .fg(color),
      )
  }
}

// ============================================================================
// Widget implementation
// ============================================================================

impl Widget for &mut App {
  fn render(
    self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let [
      main_area,
      player_area,
      instructions_area,
    ] =
      Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(5),
        Constraint::Length(2),
      ])
      .margin(1)
      .areas(area);

    let [
      queue_area,
      right_area,
    ] =
      Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(80),
      ])
      .areas(main_area);

    let [ navbar_area, content_area ] = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length(3),
        Constraint::Min(5),
      ])
      .areas(right_area);

    let [ _, error_area ] = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
      ])
      .areas(area);

    self.render_navbar(
      navbar_area,
      buf,
      0,
    );

    self.render_player(
      player_area,
      buf,
    );

    self.render_queue(
      queue_area,
      buf,
    );

    self.render_history(
      content_area,
      buf,
    );

    self.render_instructions(
      instructions_area,
      buf,
    );

    self.render_error(
      error_area,
      buf,
    );
  }
}

// ============================================================================
// Keyboard handling
// ============================================================================

fn player_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::Refresh
    }

    KeyCode::Char('e') => {
      Action::Error(format!("Test Error"))
    }

    KeyCode::Char(' ') => {
      Action::PauseResume
    }

    KeyCode::Right | KeyCode::Char('l') => {
      Action::Next
    }

    KeyCode::Left | KeyCode::Char('h') => {
      Action::Previous
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::VolumeDown
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::VolumeUp
    }

    KeyCode::Char('m') => {
      Action::Mute
    }

    KeyCode::Char('s') => {
      Action::ToggleShuffle
    }

    KeyCode::Char('r') => {
      Action::ToggleRepeat
    }

    _ => Action::Render,
  }
}

fn queue_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::PlayQueue
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::QueueMoveNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::QueueMovePrevious
    }

    KeyCode::Home => {
      Action::QueueFirst
    }

    KeyCode::End => {
      Action::QueueLast
    }

    _ => Action::Render,
  }
}

fn history_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::PlayHistory
    }

    KeyCode::Left | KeyCode::Char('h') => {
      Action::QueueHistory
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::HistoryMoveNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::HistoryMovePrevious
    }

    KeyCode::Home => {
      Action::HistoryFirst
    }

    KeyCode::End => {
      Action::HistoryLast
    }

    _ => Action::Render,
  }
}

fn key_to_action(
  focus: Focus,
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Char('q') => {
      return Action::Quit;
    }

    KeyCode::Char('c') => {
      return Action::Close;
    }

    KeyCode::Tab => {
      return Action::FocusNext;
    }

    KeyCode::BackTab => {
      return Action::FocusPrevious;
    }

    _ => {}
  }

  match focus {
    Focus::Player => {
      player_key_action(key)
    }

    Focus::Queue => {
      queue_key_action(key)
    }

    Focus::History => {
      history_key_action(key)
    }
  }
}

// ============================================================================
// Crossterm event task
// ============================================================================

fn spawn_event_handler(
  tx: UnboundedSender<Action>,
  app_focus: Arc<Mutex<Focus>>,
) {
  tokio::spawn(async move {
    loop {
      let event_result = tokio::task::spawn_blocking(event::read).await;

      let event = match event_result {
        Ok(Ok(event)) => event,

        Ok(Err(error)) => {
          let _ = tx.send(
            Action::Error(
              format!(
                "Terminal event error: {error}"
              ),
            ),
          );

          return;
        }

        Err(error) => {
          let _ = tx.send(
            Action::Error(format!("Event task error: {error}")),
          );

          return;
        }
      };

      let Event::Key(key) = event else {
        continue;
      };

      if key.kind != KeyEventKind::Press {
        continue;
      }

      let focus = match app_focus.lock() {
        Ok(focus) => *focus,
        Err(_) => return,
      };

      let action = key_to_action(focus, key);

      if tx.send(action).is_err() {
        return;
      }
    }
  });
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Error> {
  dotenvy::dotenv().ok();

  let (action_tx, action_rx) =
    mpsc::unbounded_channel::<Action>();

  let mut app = App::new(action_tx.clone()).await?;

  spawn_event_handler(
    action_tx.clone(),
    app.focus.clone(),
  );

  let mut terminal = ratatui::init();

  let result = app.run(
    &mut terminal,
    action_rx,
  ).await;

  ratatui::restore();

  result
}
