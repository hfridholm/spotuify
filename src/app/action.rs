/*     _        _   _             
 *    / \   ___| |_(_) ___  _ __  
 *   / _ \ / __| __| |/ _ \| '_ \ 
 *  / ___ \ (__| |_| | (_) | | | |
 * /_/   \_\___|\__|_|\___/|_| |_|
 */

use crate::api::{
  Player,
  Queue,
  Search,
  SimplifiedTracks,
  SpotifyApi,
  User
};

use super::{
  App,
  AppWindow,
  AppFocus,
};

use crossterm::event::{
  KeyCode,
  KeyEvent,
};

#[derive(Debug, Clone)]
pub enum AppAction {
  Quit,

  Tick,
  Render,

  Initialize,
  Refresh,

  KeyPress { key: KeyEvent },

  Up,
  Down,
  Right,
  Left,

  NavbarEnter,
  EnterHome,

  ListNext { count: usize, max: usize },
  ListPrev { count: usize },

  PauseResume,

  Next,
  Prev,

  VolumeUp,
  VolumeDown,
  Mute,

  ToggleShuffle,
  ToggleRepeat,

  QueueNext { count: usize },
  QueuePrev { count: usize },
  QueueFirst,
  QueueLast,

  TableNext { count: usize },
  TablePrev { count: usize },
  TableFirst,
  TableLast,

  PlayQueue,
  PlaySearch,
  PlayHistory,

  QueueHistory,

  ShowHomePlaylist,

  Search,
  SearchInsert { to_insert: char },
  SearchDelete,
  ShowSearchTrack,
  ShowSearchArtist,
  ShowSearchAlbum,

  AlbumTracksUpdated { tracks: SimplifiedTracks },

  ApiUpdated     { api: SpotifyApi  },
  SearchUpdated  { search:  Search  },
  PlayerUpdated  { player:  Player  },
  QueueUpdated   { queue:   Queue   },
  UserUpdated    { user:    User    },
  /*
  HistoryUpdated { history: History },
  */

  SetWindow { window: AppWindow },
  SetFocus  { focus:  AppFocus  },

  SetFocusAndTabIndex { focus: AppFocus, tab_index: usize },

  Error(String),
  Message(String),
}

impl App {
  pub(crate) fn update(&mut self, action: AppAction) {
    match action {
      AppAction::Quit => {
        self.exit = true;
      }

      AppAction::KeyPress { key } => {
        self.handle_key(key);
      }

      AppAction::Tick => {}

      AppAction::Render => {}

      AppAction::Refresh => {
        self.spawn_refresh();
      }

      AppAction::Initialize => {
        self.spawn_initialize();
      }

      AppAction::Up | AppAction::Down | AppAction::Right | AppAction::Left => {
        self.move_focus(action);
      }

      AppAction::EnterHome => {
        self.spawn_fetch_user();
      }

      AppAction::NavbarEnter => {
        const TABS: [(AppWindow, &str); 2] = [
          (AppWindow::Home,   "Home"),
          (AppWindow::Search, "Search"),
        ];

        if let Some((window, _)) = TABS.get(self.list_index) {
          self.set_window(*window);
        }
      }

      AppAction::ListNext { count: _, max } => {
        self.list_index = self.list_index.saturating_add(1).min(max.saturating_sub(1));
      }

      AppAction::ListPrev { count: _ } => {
        self.list_index = self.list_index.saturating_sub(1);
      }

      AppAction::PauseResume => {
        if self.player.is_playing {
          self.spawn_pause();
        } else {
          self.spawn_resume();
        }
      }

      AppAction::Next => {
        self.spawn_next();
      }

      AppAction::Prev => {
        self.spawn_prev();
      }

      AppAction::VolumeUp => {
        self.spawn_volume_up();
      }

      AppAction::VolumeDown => {
        self.spawn_volume_down();
      }

      AppAction::Mute => {
        self.spawn_mute();
      }

      AppAction::ToggleShuffle => {
        self.spawn_shuffle();
      }

      AppAction::ToggleRepeat => {
        self.spawn_repeat();
      }

      AppAction::TableNext { count: _ } => {
        self.table_state.select_next();
      }

      AppAction::TablePrev { count: _ } => {
        self.table_state.select_previous();
      }

      AppAction::TableFirst => {
        self.table_state.select_first();
      }

      AppAction::TableLast => {
        self.table_state.select_last();
      }

      AppAction::QueueNext { count: _ } => {
        self.queue_state.select_next();
      }

      AppAction::QueuePrev { count: _ } => {
        self.queue_state.select_previous();
      }

      AppAction::QueueFirst => {
        self.queue_state.select_first();
      }

      AppAction::QueueLast => {
        self.queue_state.select_last();
      }

      AppAction::PlayQueue => {
        self.spawn_play_queue();
      }

      AppAction::PlayHistory => {
        self.spawn_play_history();
      }

      AppAction::PlaySearch => {
        self.spawn_play_search();
      }

      AppAction::QueueHistory => {
        self.spawn_queue_history();
      }

      AppAction::Search => {
        self.spawn_search();
      }

      AppAction::SearchInsert { to_insert } => {
        self.search_buffer.push(to_insert);
      }

      AppAction::SearchDelete => {
        self.search_buffer.pop();
      }

      AppAction::SearchUpdated { search } => {
        self.search = search;

        self.tab_index = 0;
        self.set_focus(AppFocus::SearchTrack);
      }

      AppAction::PlayerUpdated { player } => {
        self.player = player;
        self.last_player_update = std::time::Instant::now();
      }

      AppAction::UserUpdated { user } => {
        self.user = user;
      }

      AppAction::ApiUpdated { api } => {
        self.spotify = api;
      }

      AppAction::QueueUpdated { queue } => {
        self.queue = Some(queue);

        if let Some(queue) = &self.queue {
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

      AppAction::ShowSearchTrack => {
        let Some(index) =
          self.table_state.selected()
        else {
          return;
        };

        if let Some(track) = self.search.tracks.items.get(index) {
          self.track = Some(track.clone());
          self.set_window(AppWindow::Track);
        }
      }

      AppAction::ShowSearchArtist => {
        let Some(index) =
          self.table_state.selected()
        else {
          return;
        };

        if let Some(artist) = self.search.artists.items.get(index) {
          self.artist = Some(artist.clone());
          self.set_window(AppWindow::Artist);
        }
      }

      AppAction::ShowSearchAlbum => {
        self.spawn_show_search_album();
      }

      AppAction::AlbumTracksUpdated { tracks } => {
        self.album_tracks = Some(tracks);
      }

      AppAction::ShowHomePlaylist => {
        let Some(index) =
          self.table_state.selected()
        else {
          return;
        };

        if let Some(playlist) = self.user_playlists.items.get(index) {
          self.playlist = playlist.clone();
          self.set_window(AppWindow::Playlist);
        }
      }

      AppAction::Error(error) => {
        self.spawn_error(error);
      }

      AppAction::Message(message) => {
        self.spawn_message(message);
      }

      AppAction::SetWindow { window } => {
        self.set_window(window);
      }

      AppAction::SetFocus { focus } => {
        self.set_focus(focus);
      }

      AppAction::SetFocusAndTabIndex { focus, tab_index } => {
        self.set_focus(focus);
        self.tab_index = tab_index;
      }
    }
  }
}
