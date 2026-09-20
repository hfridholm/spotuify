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
    List,
    ListItem,
    ListState,
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
  let min = (ms / 1000) / 60;

  format!("{:>2}m", min)
}

// ============================================================================
// AppWindow
// ============================================================================

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum AppWindow {
  #[default]
  History,
  Search,

  Home,
  /*
  HomePlaylists,
  HomeFollowing,

  Artist,
  ArtistAlbums,
  ArtistPlaylists,
  */
}

// ============================================================================
// AppFocus - the fields the user can interact with
// ============================================================================

// Note: the difference between AppFocus and AppWindow is that
//       there can exist multiple focuses on one window
//       example: ArtistAlbums, ArtistSongs
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum AppFocus {
  #[default]
  Player,
  Queue,
  History,

  /*
  Search,
  SearchArtists,
  SearchAlbums,
  SearchAccounts,
  */

  Home,
  /*
  HomePlaylists,
  HomeFollowing,

  Artist,
  ArtistAlbums,
  ArtistPlaylists,
  ArtistSongs,
  */
}

// ============================================================================
// AppBlock
// ============================================================================

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum AppBlock {
  #[default]
  Player,
  Queue,
  Main,
}

impl AppBlock {
  fn next(&self) -> Self {
    match self {
      Self::Player => Self::Main,
      Self::Queue  => Self::Player,
      Self::Main   => Self::Queue,
    }
  }

  fn prev(&self) -> Self {
    match self {
      Self::Player => Self::Queue,
      Self::Queue  => Self::Main,
      Self::Main   => Self::Player,
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

  BlockNext,
  BlockPrevious,

  PauseResume,

  Next,
  Previous,

  VolumeUp,
  VolumeDown,
  Mute,

  ToggleShuffle,
  ToggleRepeat,

  QueueNext,
  QueuePrevious,
  QueueFirst,
  QueueLast,

  TableNext,
  TablePrevious,
  TableFirst,
  TableLast,

  List1Next,
  List1Previous,
  List1First,
  List1Last,

  PlayQueue,

  PlayHistory,

  QueueHistory,

  SpotifyStateUpdated {
    state: SpotifyState,
  },

  HomeEnter,

  EnterWindow {
    window: AppWindow,
  },

  Error(String),
}

// ============================================================================
// App
// ============================================================================

struct App {
  spotify:       SpotifyApi,
  state:         SpotifyState,

  focus:         Arc<Mutex<AppFocus>>,
  block:         Arc<Mutex<AppBlock>>,
  window:        Arc<Mutex<AppWindow>>,

  queue_state: TableState,
  table_state: TableState,

  list1_state: ListState,
  list2_state: ListState,

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

      block:  Arc::new(Mutex::new(AppBlock::Player)),
      focus:  Arc::new(Mutex::new(AppFocus::Player)),
      window: Arc::new(Mutex::new(AppWindow::Home)),

      queue_state: TableState::default(),
      table_state: TableState::default(),

      list1_state: ListState::default(),
      list2_state: ListState::default(),

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
  
  fn set_focus(&mut self, focus: AppFocus) {
    match self.focus.lock() {
      Ok(mut app_focus) => *app_focus = focus,
      Err(_) => return,
    };
  }
  
  fn set_window(&mut self, window: AppWindow) {
    match self.window.lock() {
      Ok(mut app_window) => *app_window = window,
      Err(_) => return,
    };

    match window {
      AppWindow::Home => self.set_focus(AppFocus::Home),

      AppWindow::History => self.set_focus(AppFocus::History),

      _ => {},
    };
  }
  
  fn update_focus(&mut self) {
    let app_window = match self.window.lock() {
      Ok(app_window) => *app_window,
      Err(_) => return,
    };

    let app_block = match self.block.lock() {
      Ok(app_block) => *app_block,
      Err(_) => return,
    };

    match app_block {
      AppBlock::Player => self.set_focus(AppFocus::Player),

      AppBlock::Queue  => self.set_focus(AppFocus::Queue),

      AppBlock::Main   => {
        match app_window {
          AppWindow::History => self.set_focus(AppFocus::History),

          _ => self.set_window(AppWindow::Home),
        };
      }
    };
  }

  fn update(&mut self, action: Action) {
    match action {
      Action::Quit => {
        self.exit = true;
      }

      Action::Tick => {}

      Action::Render => {}

      Action::BlockNext => {
        match self.block.lock() {
          Ok(mut block) => *block = block.next(),
          Err(_) => return,
        };

        self.update_focus();
      }

      Action::BlockPrevious => {
        match self.block.lock() {
          Ok(mut block) => *block = block.prev(),
          Err(_) => return,
        };
        
        self.update_focus();
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

      Action::TableNext => {
        self.table_state.select_next();
      }

      Action::TablePrevious => {
        self.table_state.select_previous();
      }

      Action::TableFirst => {
        self.table_state.select_first();
      }

      Action::TableLast => {
        self.table_state.select_last();
      }

      Action::QueueNext => {
        self.queue_state.select_next();
      }

      Action::QueuePrevious => {
        self.queue_state.select_previous();
      }

      Action::QueueFirst => {
        self.queue_state.select_first();
      }

      Action::QueueLast => {
        self.queue_state.select_last();
      }

      Action::List1Next => {
        self.list1_state.select_next();
      }

      Action::List1Previous => {
        self.list1_state.select_previous();
      }

      Action::List1First => {
        self.list1_state.select_first();
      }

      Action::List1Last => {
        self.list1_state.select_last();
      }

      Action::HomeEnter => {
        let Some(index) =
          self.list1_state.selected()
        else {
          return;
        };

        match index {
          0 => self.set_window(AppWindow::History),

          1 => self.set_window(AppWindow::History),

          _ => {},
        }
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

      Action::EnterWindow { window } => {
        self.set_window(window);
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
      self.list1_state.selected()
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
      self.list1_state.selected()
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
  ) {
    const TABS: [(AppWindow, &str); 2] = [
      (AppWindow::Home,   "Home"), 
      (AppWindow::Search, "Search"),
    ];

    let app_window = match self.window.lock() {
      Ok(app_window) => *app_window,
      Err(_) => AppWindow::Home,
    };

    let selected = TABS.iter().position(|(window, _)| *window == app_window);

    let width: u16 = TABS.iter().map(|t| t.1.len() as u16).sum::<u16>()
      + (TABS.len() as u16 - 1) * 3 + 1;

    let block = Block::bordered();

    block.clone().render(area, buf);

    let centered = Layout::horizontal([
      Constraint::Fill(1),
      Constraint::Length(width),
      Constraint::Fill(1),
    ])
      .split(block.inner(area));

    // Creates tabs from every string TABS[i].1
    let strings: [&_; TABS.len()] = std::array::from_fn(|i| TABS[i].1);

    Tabs::new(strings)
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
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (color, border_type) = if app_focus == AppFocus::Player {
      (Color::Green, BorderType::Thick)
    } else {
      (Color::White, BorderType::Plain)
    };

    let block = Self::panel(" Player ", color, border_type);

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
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::Queue {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let block = Self::panel(" Queue ", block_color, border_type);

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

  fn render_window(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_window = match self.window.lock() {
      Ok(app_window) => *app_window,
      Err(_) => return,
    };

    match app_window {
      AppWindow::History => self.render_history(area, buf),

      AppWindow::Home    => self.render_home(area, buf),

      _ => {},
    };
  }

  fn render_home(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (color, border_type) = if app_focus == AppFocus::Home {
      (Color::Green, BorderType::Thick)
    } else {
      (Color::White, BorderType::Plain)
    };

    let block = Self::panel(" Home ", color, border_type);

    block.clone().render(area, buf);

    let inner = block.inner(area);

    let items = ["History", "History"];

    let list = List::new(
      items.iter()
      .map(|item| ListItem::new(*item))
      .collect::<Vec<_>>(),
    )
      .highlight_style(
        Style::default()
        .bg(Color::Blue)
        .fg(Color::White),
      )
      .highlight_symbol(">> ");

    StatefulWidget::render(
      list,
      inner,
      buf,
      &mut self.list1_state,
    );
  }

  fn render_history(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::History {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let block = Self::panel(" History ", block_color, border_type);

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
      &mut self.table_state,
    );
  }

  fn render_instructions(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let instructions = match app_focus {
      AppFocus::Player => {
        Line::from(
          "Space Pause/Resume | Enter Refresh | h/← Previous | l/→ Next | ↑/↓ Volume | m Mute | s Shuffle | r Repeat",
        )
      }

      AppFocus::Queue => {
        Line::from(
          "Enter Play | ↑/k Up | ↓/j Down | Home/End Jump",
        )
      }

      AppFocus::History => {
        Line::from(
          "Enter Play | h/← Queue | ↑/k Up | ↓/j Down | Home/End Jump",
        )
      }

      AppFocus::Home => {
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
          BorderType::Plain
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
    border_type: BorderType,
  ) -> Block<'static> {
    Block::bordered()
      .title(
        Line::from(format!(" {title} ").bold())
        .left_aligned(),
      )
      .border_type(border_type)
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

    let [ navbar_area, window_area ] = Layout::default()
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
    );

    self.render_player(
      player_area,
      buf,
    );

    self.render_queue(
      queue_area,
      buf,
    );

    self.render_window(
      window_area,
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

fn home_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::HomeEnter
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::List1Next
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::List1Previous
    }

    KeyCode::Home => {
      Action::List1First
    }

    KeyCode::End => {
      Action::List1Last
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
      Action::QueueNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::QueuePrevious
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
      Action::TableNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::TablePrevious
    }

    _ => Action::Render,
  }
}

fn key_to_action(
  app_focus: AppFocus,
  app_block: AppBlock,
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
      return Action::BlockNext;
    }

    KeyCode::BackTab => {
      return Action::BlockPrevious;
    }

    _ => {}
  }

  if app_block == AppBlock::Main {
    match key.code {
      KeyCode::Char('H') => {
        return Action::EnterWindow { window: AppWindow::Home };
      }

      KeyCode::Char('S') => {
        return Action::EnterWindow { window: AppWindow::Home };
      }

      _ => {},
    }
  }

  match app_focus {
    AppFocus::Player => {
      player_key_action(key)
    }

    AppFocus::Queue => {
      queue_key_action(key)
    }

    AppFocus::History => {
      history_key_action(key)
    }

    AppFocus::Home => {
      home_key_action(key)
    }
  }
}

// ============================================================================
// Crossterm event task
// ============================================================================

fn spawn_event_handler(
  tx: UnboundedSender<Action>,
  app_focus:  Arc<Mutex<AppFocus>>,
  app_window: Arc<Mutex<AppWindow>>,
  app_block:  Arc<Mutex<AppBlock>>,
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

      let app_focus = match app_focus.lock() {
        Ok(app_focus) => *app_focus,
        Err(_) => return,
      };

      let app_block = match app_block.lock() {
        Ok(app_block) => *app_block,
        Err(_) => return,
      };

      let action = key_to_action(app_focus, app_block, key);

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
    app.window.clone(),
    app.block.clone(),
  );

  let mut terminal = ratatui::init();

  let result = app.run(
    &mut terminal,
    action_rx,
  ).await;

  ratatui::restore();

  result
}
