/*     _        _   _             
 *    / \   ___| |_(_) ___  _ __  
 *   / _ \ / __| __| |/ _ \| '_ \ 
 *  / ___ \ (__| |_| | (_) | | | |
 * /_/   \_\___|\__|_|\___/|_| |_|
 */

use crate::app::App;
use crate::app::AppWindow;
use crate::app::AppFocus;

use crate::api::{
    Player, Queue, Search, SimplifiedTracks,
    SpotifyApi, User,
};

#[derive(Debug, Clone)]
pub enum Action {
  Quit,

  Tick,
  Render,

  Initialize,
  Refresh,

  Up,
  Down,
  Right,
  Left,

  NavbarEnter,
  EnterHome,

  ListNext { max: usize },
  ListPrev,

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
  pub(crate) fn update(&mut self, action: Action) {
    match action {
      Action::Quit => {
        self.exit = true;
      }

      Action::Tick => {}

      Action::Render => {}

      Action::Refresh => {
        self.spawn_refresh();
      }

      Action::Initialize => {
        self.spawn_initialize();
      }

      Action::Up | Action::Down | Action::Right | Action::Left => {
        self.move_focus(action);
      }

      Action::EnterHome => {
        self.spawn_fetch_user();
      }

      Action::NavbarEnter => {
        const TABS: [(AppWindow, &str); 2] = [
          (AppWindow::Home,   "Home"),
          (AppWindow::Search, "Search"),
        ];

        if let Some((window, _)) = TABS.get(self.list_index) {
          self.set_window(*window);
        }
      }

      Action::ListNext { max } => {
        self.list_index = self.list_index.saturating_add(1).min(max.saturating_sub(1));
      }

      Action::ListPrev => {
        self.list_index = self.list_index.saturating_sub(1);
      }

      Action::PauseResume => {
        if self.player.is_playing {
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

      Action::PlayQueue => {
        self.spawn_play_queue();
      }

      Action::PlayHistory => {
        self.spawn_play_history();
      }

      Action::PlaySearch => {
        self.spawn_play_search();
      }

      Action::QueueHistory => {
        self.spawn_queue_history();
      }

      Action::Search => {
        self.spawn_search();
      }

      Action::SearchInsert { to_insert } => {
        self.search_buffer.push(to_insert);
      }

      Action::SearchDelete => {
        self.search_buffer.pop();
      }

      Action::SearchUpdated { search } => {
        self.search = search;

        self.tab_index = 0;
        self.set_focus(AppFocus::SearchTrack);
      }

      Action::PlayerUpdated { player } => {
        self.player = player;
        self.last_player_update = std::time::Instant::now();
      }

      Action::UserUpdated { user } => {
        self.user = user;
      }

      Action::ApiUpdated { api } => {
        self.spotify = api;
      }

      Action::QueueUpdated { queue } => {
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

      Action::ShowSearchTrack => {
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

      Action::ShowSearchArtist => {
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

      Action::ShowSearchAlbum => {
        self.spawn_show_search_album();
      }

      Action::AlbumTracksUpdated { tracks } => {
        self.album_tracks = Some(tracks);
      }

      Action::ShowHomePlaylist => {
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

      Action::Error(error) => {
        self.spawn_error(error);
      }

      Action::Message(message) => {
        self.spawn_message(message);
      }

      Action::SetWindow { window } => {
        self.set_window(window);
      }

      Action::SetFocus { focus } => {
        self.set_focus(focus);
      }

      Action::SetFocusAndTabIndex { focus, tab_index } => {
        self.set_focus(focus);
        self.tab_index = tab_index;
      }
    }
  }
}
