/*  _____                    
 * |  ___|__   ___ _   _ ___ 
 * | |_ / _ \ / __| | | / __|
 * |  _| (_) | (__| |_| \__ \
 * |_|  \___/ \___|\__,_|___/
 */

use super::{
  App,
  Action
};

// ============================================================================
// AppWindow
// ============================================================================

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum AppWindow {
  #[default]
  Search,
  Home,
  Track,
  Artist,
  Album,
  Playlist,
}

impl AppWindow {
  pub(crate) fn get_focus(&self) -> AppFocus {
    match self {
      Self::Search   => AppFocus::SearchTrack,
      Self::Home     => AppFocus::Home,
      Self::Track    => AppFocus::Track,
      Self::Artist   => AppFocus::Artist,
      Self::Album    => AppFocus::Album,
      Self::Playlist => AppFocus::Playlist,
    }
  }
}

// ============================================================================
// AppFocus - the fields the user can interact with
// ============================================================================

// Note: the difference between AppFocus and AppWindow is that
//       there can exist multiple focuses on one window
//       example: ArtistAlbums, ArtistTracks
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum AppFocus {
  #[default]
  Navbar,
  Player,
  Queue,
  History,

  Search,
  SearchTrack,
  SearchAlbum,
  SearchArtist,
  SearchPlaylist,

  Track,
  Album,
  Playlist,

  Home,
  HomePlaylists,
  HomeHistory,
  HomeFollowing,

  Artist,
  /*
  ArtistAlbums,
  ArtistPlaylists,
  ArtistTracks,
  */
}

impl AppFocus  {
  pub(crate) fn get_window(&self) -> Option<AppWindow> {
    match self {
      AppFocus::Search | AppFocus::SearchTrack | AppFocus::SearchArtist | AppFocus::SearchAlbum | AppFocus::SearchPlaylist => {
        Some(AppWindow::Search)
      }

      AppFocus::Home | AppFocus::HomeHistory | AppFocus::HomePlaylists => {
        Some(AppWindow::Home)
      }

      AppFocus::Track => {
        Some(AppWindow::Track)
      }

      AppFocus::Artist => {
        Some(AppWindow::Artist)
      }

      AppFocus::Album => {
        Some(AppWindow::Album)
      }

      AppFocus::Playlist => {
        Some(AppWindow::Playlist)
      }

      _ => None
    }
  }
}

impl App {
  pub(crate) fn move_focus(&mut self, action: Action) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let app_window = match self.window.lock() {
      Ok(app_window) => *app_window,
      Err(_) => return,
    };

    let new_focus = match app_focus {
      AppFocus::Search => {
        match action {
          Action::Down => {
            match self.tab_index {
              0 => AppFocus::SearchTrack,
              1 => AppFocus::SearchAlbum,
              2 => AppFocus::SearchArtist,
              3 => AppFocus::SearchPlaylist,
              _ => app_focus,
            }
          },
          Action::Left => AppFocus::Queue,
          Action::Up   => AppFocus::Navbar,
          _ => app_focus,
        }
      },

      AppFocus::SearchTrack | AppFocus::SearchAlbum | AppFocus::SearchArtist | AppFocus::SearchPlaylist => {
        match action {
          Action::Up   => AppFocus::Search,
          Action::Down => AppFocus::Player,
          Action::Left => AppFocus::Queue,
          _ => app_focus,
        }
      },

      AppFocus::Track | AppFocus::Artist | AppFocus::Playlist => {
        match action {
          Action::Up   => AppFocus::Navbar,
          Action::Down => AppFocus::Player,
          Action::Left => AppFocus::Queue,
          _ => app_focus,
        }
      },

      AppFocus::Home => {
        match action {
          Action::Down => {
            match self.tab_index {
              0 => AppFocus::HomeHistory,
              1 => AppFocus::HomePlaylists,
              _ => app_focus,
            }
          },
          Action::Left => AppFocus::Queue,
          Action::Up   => AppFocus::Navbar,
          _ => app_focus,
        }
      },

      AppFocus::HomePlaylists | AppFocus::HomeHistory | AppFocus::HomeFollowing => {
        match action {
          Action::Up   => AppFocus::Home,
          Action::Down => AppFocus::Player,
          Action::Left => AppFocus::Queue,
          _ => app_focus,
        }
      },

      AppFocus::Player => {
        match action {
          Action::Up => app_window.get_focus(),
          _ => app_focus,
        }
      }

      AppFocus::Queue => {
        match action {
          Action::Down  => AppFocus::Player,
          Action::Right => app_window.get_focus(),
          _ => app_focus,
        }
      }

      AppFocus::Navbar => {
        match action {
          Action::Left => AppFocus::Queue,
          Action::Down => app_window.get_focus(),
          _ => app_focus,
        }
      }

      _ => app_focus,
    };

    self.set_focus(new_focus);
  }

  pub(crate) fn set_focus(&mut self, focus: AppFocus) {
    match self.focus.lock() {
      Ok(mut app_focus) => *app_focus = focus,
      Err(_) => return,
    };

    let window = match self.window.lock() {
      Ok(mut app_window) => Some(*app_window),
      Err(_) => None,
    };

    if let Some(window) = window
      && let Some(focus_window) = focus.get_window()
      && focus_window != window
    {
      self.set_window(focus_window);
    }
  }
  
  pub(crate) fn set_window(&mut self, window: AppWindow) {
    let tx = self.action_tx.clone();

    match self.window.lock() {
      Ok(mut app_window) => *app_window = window,
      Err(_) => return,
    };

    // Let the window initialize itself
    match window {
      AppWindow::Home => {
        let _ = tx.send(Action::EnterHome);
      }
      _ => {}
    }

    let focus = match self.focus.lock() {
      Ok(mut app_focus) => Some(*app_focus),
      Err(_) => None,
    };

    // If the current focus is not in the window,
    // set focus to window's default focus and reset tab_index
    if let Some(focus) = focus
      && let Some(focus_window) = focus.get_window()
      && focus_window != window
    {
      self.set_focus(window.get_focus());
      self.tab_index = 0;
    }
  }
}
