/*     _                
 *    / \   _ __  _ __  
 *   / _ \ | '_ \| '_ \ 
 *  / ___ \| |_) | |_) |
 * /_/   \_\ .__/| .__/ 
 *         |_|   |_|    
 */

mod actions;
mod focus;
mod helpers;
mod notifications;
mod spawn;

pub use actions::Action;
pub use focus::{
  AppFocus, AppWindow
};

use crate::error::Error;

use crate::api::{
  Album, Artist, History, Player, Playlist, Playlists,
  Queue, Search, SimplifiedTracks, SpotifyApi, Track, User,
};

use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
};

use tokio::{
  sync::mpsc::{
    UnboundedReceiver,
    UnboundedSender,
  },
  time::{
    self,
    Duration
  },
};

use ratatui::{
  widgets::TableState,
  DefaultTerminal,
};

pub struct App {
  pub(crate) spotify: SpotifyApi,
  pub(crate) last_player_update: std::time::Instant,

  pub(crate) focus: Arc<Mutex<AppFocus>>,
  pub(crate) window: Arc<Mutex<AppWindow>>,

  pub(crate) user: User,
  pub(crate) user_playlists: Playlists,

  pub(crate) player: Player,
  pub(crate) queue: Option<Queue>,
  pub(crate) history: Option<History>,

  pub(crate) search: Search,
  pub(crate) search_buffer: String,

  pub(crate) track: Option<Track>,
  pub(crate) artist: Option<Artist>,
  pub(crate) playlist: Option<Playlist>,

  pub(crate) album: Option<Album>,
  pub(crate) album_tracks: Option<SimplifiedTracks>,

  pub(crate) queue_state: TableState,
  pub(crate) table_state: TableState,

  pub(crate) list_index: usize,
  pub(crate) tab_index: usize,

  pub(crate) action_tx: UnboundedSender<Action>,

  pub(crate) error_list: Arc<Mutex<VecDeque<(u64, String)>>>,
  pub(crate) message_list: Arc<Mutex<VecDeque<(u64, String)>>>,
  pub(crate) exit: bool,
}

impl App {
  pub(crate) async fn new(
    action_tx: UnboundedSender<Action>,
  ) -> Result<Self, Error> {
    Ok(Self {
      spotify: SpotifyApi::default(), // Maybe rename this to "api"

      focus:  Arc::new(Mutex::new(AppFocus::Player)),
      window: Arc::new(Mutex::new(AppWindow::Home)),

      user:           User::default(),
      user_playlists: Playlists::default(),

      player:  Player::default(),
      last_player_update: std::time::Instant::now(),

      history: None,
      queue:   None,

      search:        Search::default(),
      search_buffer: String::default(),

      track:    None,
      artist:   None,
      playlist: None,

      album:        None,
      album_tracks: None,

      queue_state: TableState::default(),
      table_state: TableState::default(),

      list_index: 0,
      tab_index:  0,

      action_tx,

      error_list:   Arc::new(Mutex::new(VecDeque::new())),
      message_list: Arc::new(Mutex::new(VecDeque::new())),

      exit: false,
    })
  }

  pub(crate) async fn run(
    &mut self,
    terminal: &mut DefaultTerminal,
    mut action_rx: UnboundedReceiver<Action>,
  ) -> Result<(), Error> {
    // Immediately fetch Spotify state
    self.action_tx.send(Action::Initialize)?;

    // Render playing track progress bar every 33 ms
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
          self.action_tx.send(Action::Refresh)?;
        }
      }
    }

    Ok(())
  }

  // Artificially (locally) update the progress time of the playing track
  pub(crate) fn update_progress(&mut self) {
    if !self.player.is_playing {
      return;
    }

    let elapsed = self.last_player_update
      .elapsed()
      .as_millis() as u32;

    // Artificially add the elapsed time,
    // without needing to fetch new data
    self.player.progress_ms = self.player.progress_ms.saturating_add(elapsed);

    self.last_player_update = std::time::Instant::now();

    // Clamp the progress time to the duration,
    // so it doesn't overflow
    if let Ok(duration) = self.player.get_duration() {
      self.player.progress_ms = self.player.progress_ms.min(duration);
    }
  }
}
