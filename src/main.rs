use std::io;

use std::{thread, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
  buffer::Buffer,
  layout::{Constraint, Layout, Rect},
  style::Stylize,
  symbols::border,
  text::{Line, Text},
  widgets::{Block, BorderType, Paragraph, Wrap, Widget},
  DefaultTerminal, Frame,
};

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
  access_token:  String,
  scope:         String,
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

  // 1. Generate PKCE values
  let verifier = generate_code_verifier();
  let challenge = generate_code_challenge(&verifier);

  // 2. Build authorization URL
  let auth_url = build_authorization_url(
    &client_id,
    &challenge,
  );

  // 3. Start callback server BEFORE opening browser
  // Try to open the browser automatically.
  if let Err(error) = open::that(&auth_url) {
    eprintln!("Could not open browser automatically: {error}");
  }

  // 4. Wait for Spotify to redirect back to us
  let code = wait_for_callback()?;

  // 5. Exchange code for access token
  let token = exchange_code(
    &client,
    &client_id,
    &code,
    &verifier,
  )?;

  let refresh_token = token
    .refresh_token
    .ok_or("Spotify did not return a refresh token")?;

  std::fs::write(token_path, &refresh_token)?;

  Ok(token.access_token)
}

#[derive(Default, Debug, Deserialize)]
struct Spotify {
  #[serde(skip)]
  client:       reqwest::blocking::Client,
  access_token: String,
  player:       Player,
  track:        Option<Track>,
  queue:        Option<Queue>,
}

#[derive(Default, Debug, Deserialize)]
struct Player {
  device:        Option<Device>,
  is_playing:    bool,
  repeat_state:  String,
  shuffle_state: bool,
}

#[derive(Default, Debug, Deserialize)]
struct Device {
  id:             String,
  name:           String,
  volume_percent: u8,
  is_active:      bool,
}

#[derive(Debug, Deserialize)]
struct Playing {
  item: Option<Track>,
}

#[derive(Default, Debug, Deserialize)]
struct Queue {
  currently_playing: Option<Track>,
  queue:             Vec<Track>,
}

#[derive(Default, Debug, Deserialize)]
struct Track {
  id:      String,
  name:    String,
  artists: Vec<Artist>,
}

#[derive(Default, Debug, Deserialize)]
struct Artist {
  id:   String,
  name: String,
}

impl Spotify {
  fn new() -> Result<Self, Error> {
    let client_id = env::var("CLIENT_ID")?;
    let client = reqwest::blocking::Client::new();

    let access_token = get_access_token(
      &client,
      &client_id,
    )?;

    let player = Player::get(
      &client,
      &access_token,
    )?;

    Ok(Self {
      client:       client,
      access_token: access_token,
      player:       player,
      track:        None,
      queue:        None,
    })
  }

  fn update_track(&mut self) -> Result<(), Error> {
    let track = self.player.get_track(
      &self.client,
      &self.access_token,
    )?;

    self.track = Some(track);

    Ok(())
  }

  fn update_queue(&mut self) -> Result<(), Error> {
    let queue = self.player.get_queue(
      &self.client,
      &self.access_token,
    )?;

    self.queue = Some(queue);

    Ok(())
  }

  fn resume(&mut self) -> Result<(), Error> {
    self.player.resume(&self.client, &self.access_token)
  }

  fn pause(&mut self) -> Result<(), Error> {
    self.player.pause(&self.client, &self.access_token)
  }

  fn next(&mut self) -> Result<(), Error> {
    self.player.next(&self.client, &self.access_token)?;

    // Need to wait for the next track to start, before reading it
    thread::sleep(Duration::from_millis(500));

    self.update_track()
  }

  fn prev(&mut self) -> Result<(), Error> {
    self.player.prev(&self.client, &self.access_token)?;

    // Need to wait for the next track to start, before reading it
    thread::sleep(Duration::from_millis(500));

    self.update_track()
  }

  fn toggle_repeat(&mut self) -> Result<(), Error> {
    self.player.toggle_repeat(&self.client, &self.access_token)
  }

  fn toggle_shuffle(&mut self) -> Result<(), Error> {
    self.player.toggle_shuffle(&self.client, &self.access_token)
  }
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

  fn get_queue(
    &mut self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<Queue, Error> {
    let response = client
      .get("https://api.spotify.com/v1/me/player/queue")
      .bearer_auth(access_token)
      .send()?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("Queue is empty".into());
    }

    let queue: Queue = response
      .error_for_status()?
      .json()?;

    Ok(queue)
  }

  fn resume(
    &mut self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(client, access_token, "https://api.spotify.com/v1/me/player/play")?;

    self.is_playing = true;

    Ok(())
  }

  fn pause(
    &mut self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.put(client, access_token, "https://api.spotify.com/v1/me/player/pause")?;

    self.is_playing = false;

    Ok(())
  }

  fn next(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.post(client, access_token, "https://api.spotify.com/v1/me/player/next")
  }

  fn prev(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<(), Error> {
    self.post(client, access_token, "https://api.spotify.com/v1/me/player/previous")
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

  fn post(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
    url: &str,
  ) -> Result<(), Error> {
    let device = self.device()?;

    client
      .post(url)
      .query(&[("device_id", device.id.as_str())])
      .bearer_auth(access_token)
      .send()?
      .error_for_status()?;

    Ok(())
  }

  fn toggle_shuffle(
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

  fn toggle_repeat(
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

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("Nothing is currently playing".into());
    }

    let playing: Playing = response
      .error_for_status()?
      .json()?;

    playing
      .item
      .ok_or_else(|| "No currently playing track".into())
  }

  fn device(&self) -> Result<&Device, Error> {
    self.device
      .as_ref()
      .ok_or("No Spotify device found".into())
  }
}

#[derive(Debug, Default)]
pub struct App {
  spotify: Spotify,
  error:   Option<String>,
  exit:    bool,
}

impl App {
  fn new() -> Result<Self, Error> {
    let spotify = Spotify::new()?;

    Ok(Self {
      spotify: spotify,
      error:   None,
      exit:    false,
    })
  }

  // runs the application's main loop until the user quits
  pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
    while !self.exit {
      terminal.draw(|frame| self.draw(frame))?;
      self.handle_events()?;
    }
    Ok(())
  }

  fn draw(&self, frame: &mut Frame) {
    frame.render_widget(self, frame.area());
  }

  fn exit(&mut self) {
    self.exit = true;
  }

  fn handle_key_event(&mut self, key_event: KeyEvent) {
    match key_event.code {
      KeyCode::Char('q') => self.exit(),
      KeyCode::Enter => {
        if let Err(err) = self.spotify.update_track() {
          self.error = Some(err.to_string());
        }

        if let Err(err) = self.spotify.update_queue() {
          self.error = Some(err.to_string());
        }
      },
      KeyCode::Char(' ') => {
        if self.spotify.player.is_playing {
          self.spotify.pause();
        } else {
          self.spotify.resume();
        }
      },
      KeyCode::Char('l') => {
        if let Err(err) = self.spotify.next() {
          self.error = Some(err.to_string());
        }
      },
      KeyCode::Char('h') => {
        if let Err(err) = self.spotify.prev() {
          self.error = Some(err.to_string());
        }
      },
      KeyCode::Char('s') => {
        if let Err(err) = self.spotify.toggle_shuffle() {
          self.error = Some(err.to_string());
        }
      },
      KeyCode::Char('r') => {
        if let Err(err) = self.spotify.toggle_repeat() {
          self.error = Some(err.to_string());
        }
      },
      _ => {}
    }
  }

  /// updates the application's state based on user input
  fn handle_events(&mut self) -> io::Result<()> {
    match event::read()? {
      // it's important to check that the event is a key press event as
      // crossterm also emits key release and repeat events on Windows.
      Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
        self.handle_key_event(key_event)
      }
      _ => {}
    };
    Ok(())
  }
}

// Render
impl Widget for &App {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let [player_area, queue_area, instructions_area, error_area] = Layout::vertical([
      Constraint::Min(1),
      Constraint::Min(1),
      Constraint::Length(2),
      Constraint::Length(3),
    ])
      .margin(1)
      .areas(area);

    // Instructions
    let instructions = Line::from(
    "_ Pause/Resume | r Refresh | h,<- Previous | l,-> Next | s Toggle Shuffle | r Toggle Repeat"
      .blue());

    // Error
    let error = Line::from(
      self.error
      .as_deref()
      .unwrap_or("")
      .red(),
    );

    // Queue block
    let queue_block = Block::bordered()
      .title(Line::from(" Queue ".bold()).left_aligned())
      .border_type(BorderType::Thick)
      .white();

    // Queue text
    let queue_text = match &self.spotify.queue {
      Some(queue) => {
        let tracks = queue.queue
          .iter()
          .map(|track| track.name.as_str())
          .collect::<Vec<_>>()
          .join("\n");

        Text::from(tracks)
      }
      None => Text::from("No queue"),
    }
      .white();

    // Player block
    let player_block = Block::bordered()
      .title(Line::from(" Player ".bold()).left_aligned())
      .border_type(BorderType::Thick)
      .green();

    let track_text = match &self.spotify.track {
      Some(track) => {
        let artists = track
          .artists
          .iter()
          .map(|artist| artist.name.as_str())
          .collect::<Vec<_>>()
          .join(", ");

        Text::from(vec![
          Line::from(track.name.clone().bold()),
          Line::from(artists),
        ])
      }
      None => Text::from("Nothing playing"),
    }
      .white();

    Paragraph::new(track_text)
      .centered()
      .block(player_block)
      .render(player_area, buf);

    Paragraph::new(queue_text)
      .centered()
      .block(queue_block)
      .render(queue_area, buf);

    Paragraph::new(instructions)
      .centered()
      .wrap(Wrap { trim: true })
      .render(instructions_area, buf);

    Paragraph::new(error)
      .wrap(Wrap { trim: true })
      .render(error_area, buf);
  }
}

// Main
fn main() -> Result<(), Error> {
  dotenvy::dotenv().ok();

  let mut app = App::new()?;

  ratatui::run(|terminal| app.run(terminal))?;

  Ok(())
}
