/*  _____                    
 * |  ___|__   ___ _   _ ___ 
 * | |_ / _ \ / __| | | / __|
 * |  _| (_) | (__| |_| \__ \
 * |_|  \___/ \___|\__,_|___/
 */

use super::{
  App,
  AppAction
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
  pub(crate) fn is(&self, window: AppWindow) -> bool {
    *self == window
  }

  pub(crate) fn get_focus(&self) -> AppFocus {
    match self {
      Self::Search   => AppFocus::Search,
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
  pub(crate) fn is(&self, focus: AppFocus) -> bool {
    *self == focus
  }

  pub(crate) fn in_window(&self, window: AppWindow) -> bool {
    self.get_window() == Some(window)
  }

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
  pub(crate) fn move_focus(&mut self, action: AppAction) {
    let new_focus = match self.focus {
      AppFocus::Search => {
        match action {
          AppAction::Down => {
            match self.tab_index {
              0 => AppFocus::SearchTrack,
              1 => AppFocus::SearchArtist,
              2 => AppFocus::SearchAlbum,
              3 => AppFocus::SearchPlaylist,
              _ => self.focus,
            }
          },
          AppAction::Left => AppFocus::Queue,
          AppAction::Up   => AppFocus::Navbar,
          _ => self.focus,
        }
      },

      AppFocus::SearchTrack | AppFocus::SearchAlbum | AppFocus::SearchArtist | AppFocus::SearchPlaylist => {
        match action {
          AppAction::Up   => AppFocus::Search,
          AppAction::Down => AppFocus::Player,
          AppAction::Left => AppFocus::Queue,
          _ => self.focus,
        }
      },

      AppFocus::Track | AppFocus::Artist | AppFocus::Playlist => {
        match action {
          AppAction::Up   => AppFocus::Navbar,
          AppAction::Down => AppFocus::Player,
          AppAction::Left => AppFocus::Queue,
          _ => self.focus,
        }
      },

      AppFocus::Home => {
        match action {
          AppAction::Down => {
            match self.tab_index {
              0 => AppFocus::HomeHistory,
              1 => AppFocus::HomePlaylists,
              _ => self.focus,
            }
          },
          AppAction::Left => AppFocus::Queue,
          AppAction::Up   => AppFocus::Navbar,
          _ => self.focus,
        }
      },

      AppFocus::HomePlaylists | AppFocus::HomeHistory | AppFocus::HomeFollowing => {
        match action {
          AppAction::Up   => AppFocus::Home,
          AppAction::Down => AppFocus::Player,
          AppAction::Left => AppFocus::Queue,
          _ => self.focus,
        }
      },

      AppFocus::Player => {
        match action {
          AppAction::Up => self.window.get_focus(),
          _ => self.focus,
        }
      }

      AppFocus::Queue => {
        match action {
          AppAction::Down  => AppFocus::Player,
          AppAction::Right => self.window.get_focus(),
          _ => self.focus,
        }
      }

      AppFocus::Navbar => {
        match action {
          AppAction::Left => AppFocus::Queue,
          AppAction::Down => self.window.get_focus(),
          _ => self.focus,
        }
      }

      _ => self.focus,
    };

    self.set_focus(new_focus);
  }

  pub(crate) fn set_focus(&mut self, focus: AppFocus) {
    self.focus = focus;

    // Update the window to belong to the current focus
    if let Some(focus_window) = focus.get_window()
      && !focus_window.is(self.window)
    {
      self.set_window(focus_window);
    }

    // Update tab_index to correspond to the current focus
    self.tab_index = match focus {
      AppFocus::HomeHistory
      | AppFocus::SearchTrack  => 0,

      AppFocus::HomePlaylists
      | AppFocus::SearchArtist => 1,

      AppFocus::SearchAlbum    => 2,

      AppFocus::SearchPlaylist => 3,

      _ => self.tab_index,
    };
  }
  
  pub(crate) fn set_window(&mut self, window: AppWindow) {
    let tx = self.action_tx.clone();

    self.window = window;

    // Let the window initialize itself
    match window {
      AppWindow::Home => {
        let _ = tx.send(AppAction::EnterHome);
      }
      _ => {}
    }

    // If the current focus is not in the window,
    // set focus to window's default focus and reset tab_index
    if let Some(focus_window) = self.focus.get_window()
      && !focus_window.is(window)
    {
      self.set_focus(window.get_focus());
      self.tab_index = 0;
    }
  }
}
