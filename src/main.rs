use std::io;

use std::{thread, time::Duration};
use chrono::{DateTime, Utc};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
  buffer::Buffer,
  layout::{Constraint, Layout, Rect},
  style::{Stylize, Color},
  symbols::border,
  text::{Line, Text},
  widgets::{
    Block, BorderType, Cell, Paragraph, Row, Table, Widget, Wrap,
  },
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
    .append_pair("scope", "user-modify-playback-state user-read-recently-played")
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
  playing:      Option<Playing>,
  queue:        Option<Queue>,
  history:      Option<History>,
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
  device:      Option<Device>,
  progress_ms: u32,
  item:        Option<Track>,
}

impl Playing {
  fn get_remaining(&self) -> u32 {
    let progress_ms = self.progress_ms;

    let duration_ms = self.item
      .as_ref()
      .map(|track| track.duration_ms)
      .unwrap_or(0);

    duration_ms.saturating_sub(progress_ms)
  }
}

#[derive(Default, Debug, Deserialize)]
struct History {
  items: Vec<PlayedTrack>,
}

#[derive(Default, Debug, Deserialize)]
struct PlayedTrack {
  track:     Option<Track>,
  played_at: String,
}

impl PlayedTrack {
  fn get_elapsed(&self) -> u32{
    let then = DateTime::parse_from_rfc3339(&self.played_at)
      .unwrap()
      .with_timezone(&Utc);

    let now = Utc::now();
    let elapsed = now.signed_duration_since(then);

    elapsed.num_milliseconds() as u32
  }
}

fn format_ms(ms: u32) -> String {
  let sec = (ms / 1000) % 60;
  let min = (ms / 1000) / 60;

  let sec_str = sec.to_string();
  let min_str = min.to_string();

  format!("{:>2}m {:0>2}s", min_str, sec_str)
}

#[derive(Default, Debug, Deserialize)]
struct Queue {
  currently_playing: Option<Track>,
  queue:             Vec<Track>,
}

#[derive(Default, Debug, Deserialize)]
struct Track {
  id:          String,
  name:        String,
  duration_ms: u32,
  artists:     Vec<Artist>,
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
      playing:      None,
      queue:        None,
      history:      None,
    })
  }

  fn update(&mut self) -> Result<(), Error> {
    self.update_playing()?;
    self.update_queue()?;
    self.update_history()
  }

  fn update_playing(&mut self) -> Result<(), Error> {
    let playing = self.player.get_playing(
      &self.client,
      &self.access_token,
    )?;

    self.playing = Some(playing);

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

  fn update_history(&mut self) -> Result<(), Error> {
    let history = self.player.get_history(
      &self.client,
      &self.access_token,
    )?;

    self.history = Some(history);

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

    self.update()
  }

  fn prev(&mut self) -> Result<(), Error> {
    self.player.prev(&self.client, &self.access_token)?;

    // Need to wait for the next track to start, before reading it
    thread::sleep(Duration::from_millis(500));

    self.update()
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

  fn get_history(
    &mut self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<History, Error> {
    let response = client
      .get("https://api.spotify.com/v1/me/player/recently-played")
      .bearer_auth(access_token)
      .send()?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("No recently played tracks".into());
    }

    let history: History = response
      .error_for_status()?
      .json()?;

    Ok(history)
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

  fn get_playing(
    &self,
    client: &reqwest::blocking::Client,
    access_token: &str,
  ) -> Result<Playing, Error> {
    let response = client
      .get("https://api.spotify.com/v1/me/player/currently-playing")
      .bearer_auth(access_token)
      .send()?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
      return Err("Nothing is currently playing".into());
    }

    Ok(response.error_for_status()?.json()?)
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
        if let Err(err) = self.spotify.update() {
          self.error = Some(err.to_string());
        }
      },
      KeyCode::Char(' ') => {
        if self.spotify.player.is_playing {
          if let Err(err) = self.spotify.pause() {
            self.error = Some(err.to_string());
          }
        } else {
          if let Err(err) = self.spotify.resume() {
            self.error = Some(err.to_string());
          }
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

  fn render_player(&self, area: Rect, buf: &mut Buffer) {
    let block = Self::panel(" Player ", Color::Green);

    let text = match &self.spotify.playing {
      Some(playing) => {
        match &playing.item {
          Some(track) => {
            let artists = track
              .artists
              .iter()
              .map(|artist| artist.name.as_str())
              .collect::<Vec<_>>()
              .join(", ");

            Text::from(vec![
              Line::from(track.name.as_str().bold()),
              Line::from(artists),
            ])
          }
          None => Text::from("No track playing"),
        }
      }
      None => Text::from("Nothing playing"),
    };

    Paragraph::new(text)
      .centered()
      .white()
      .block(block)
      .render(area, buf);
  }

  fn render_queue(&self, area: Rect, buf: &mut Buffer) {
    let block = Self::panel(" Queue ", Color::White);

    let header = Row::new([
      Cell::from("  #".to_uppercase().cyan()),
      Cell::from("Track".to_uppercase().cyan()),
      Cell::from("Playing in".to_uppercase().cyan()),
    ])
      .bold();

    let mut remaining_ms = self.spotify.playing
      .as_ref()
      .map(|playing| playing.get_remaining())
      .unwrap_or(0);

    let rows = match &self.spotify.queue {
      Some(queue) if !queue.queue.is_empty() =>
        queue.queue
        .iter()
        .enumerate()
        .map(|(index, track)| {
          let time_str = format_ms(remaining_ms);

          remaining_ms += track.duration_ms;

          let order_str = (index + 1).to_string();

          Row::new([
            Cell::from(format!("+{:0>2}", order_str)),
            Cell::from(track.name.as_str()),
            Cell::from(time_str.clone()),
          ])
        })
      .collect::<Vec<_>>(),

      _ => vec![Row::new([
        Cell::from(""),
        Cell::from("No queue"),
        Cell::from(""),
      ])],
    };

    Table::new(
      rows,
      [
      Constraint::Length(4),
      Constraint::Min(1),
      Constraint::Length(12),
      ],
    )
      .header(header)
      .column_spacing(1)
      .block(block)
      .render(area, buf);
    }

  fn render_history(&self, area: Rect, buf: &mut Buffer) {
    let block = Self::panel(" History ", Color::White);

    let header = Row::new([
      Cell::from("  #".to_uppercase().cyan()),
      Cell::from("Track".to_uppercase().cyan()),
      Cell::from("Played ago".to_uppercase().cyan()),
    ])
      .bold();

    let rows = match &self.spotify.history {
      Some(history) if !history.items.is_empty() => {
        let items = history
          .items
          .iter()
          .filter_map(|played_track| {
            played_track
              .track
              .as_ref()
              .map(|track| (played_track, track))
          })
        .enumerate()
          .map(|(index, (played_track, track))| {
            let order_str = (index + 1).to_string();

            let time_str = format_ms(played_track.get_elapsed());

            Row::new([
              Cell::from(format!("-{:0>2}", order_str)),
              Cell::from(track.name.as_str()),
              Cell::from(time_str.clone()),
            ])
          })
        .collect::<Vec<_>>();

        if items.is_empty() {
          vec![Row::new([
            Cell::from(""),
            Cell::from("No history"),
            Cell::from(""),
          ])]
        } else {
          items
        }
      }

      _ => vec![Row::new([
        Cell::from(""),
        Cell::from("No history"),
        Cell::from(""),
      ])],
    };

    Table::new(
      rows,
      [
      Constraint::Length(4),
      Constraint::Min(1),
      Constraint::Length(30),
      ],
    )
      .header(header)
      .column_spacing(1)
      .block(block)
      .render(area, buf);
    }

  fn queue_position(&self, index: usize) -> String {
    // TODO: Replace this with the actual calculation once
    // the Spotify track duration/playback fields are available.

    if index == 0 {
      "next".to_string()
    } else {
      format!("+{index}")
    }
  }

  fn render_instructions(&self, area: Rect, buf: &mut Buffer) {
    let instructions = Line::from(
      "_ Pause/Resume | r Refresh | h/← Previous | l/→ Next | \
             s Toggle Shuffle | R Toggle Repeat",
    )
      .blue();

    Paragraph::new(instructions)
      .centered()
      .wrap(Wrap { trim: true })
      .render(area, buf);
  }

  fn render_error(&self, area: Rect, buf: &mut Buffer) {
    let error = self.error.as_deref().unwrap_or("").red();

    Paragraph::new(Line::from(error))
      .wrap(Wrap { trim: true })
      .render(area, buf);
  }

  fn panel(title: &str, color: Color) -> Block<'static> {
    Block::bordered()
      .title(Line::from(format!(" {title} ").bold()).left_aligned())
      .border_type(BorderType::Thick)
      .fg(color)
  }
}

impl Widget for &App {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let [player_area, content_area, instructions_area, error_area] =
      Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(5),
        Constraint::Length(2),
        Constraint::Length(3),
      ])
      .margin(1)
      .areas(area);

    // Queue + history side-by-side.
    let [queue_area, history_area] = Layout::horizontal([
      Constraint::Percentage(60),
      Constraint::Percentage(40),
    ])
      .areas(content_area);

    self.render_player(player_area, buf);
    self.render_queue(queue_area, buf);
    self.render_history(history_area, buf);
    self.render_instructions(instructions_area, buf);
    self.render_error(error_area, buf);
  }
}

// Main
fn main() -> Result<(), Error> {
  dotenvy::dotenv().ok();

  let mut app = App::new()?;

  ratatui::run(|terminal| app.run(terminal))?;

  Ok(())
}
