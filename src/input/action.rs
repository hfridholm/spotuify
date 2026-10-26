/*     _        _   _             
 *    / \   ___| |_(_) ___  _ __  
 *   / _ \ / __| __| |/ _ \| '_ \ 
 *  / ___ \ (__| |_| | (_) | | | |
 * /_/   \_\___|\__|_|\___/|_| |_|
 */

// pub mod home;
// pub mod navbar;
// pub mod player;
// pub mod queue;
pub mod search;

use super::{
  AppAction,
  AppFocus,
  InputAction,
};

pub(crate) fn parse_action(
  app_focus: AppFocus,
  action: InputAction,
) -> Option<AppAction> {
  /*
  match app_focus {
    AppFocus::Navbar => {
      navbar::parse_action(action)
    }

    AppFocus::Player => {
      player::parse_action(action)
    }

    AppFocus::Queue => {
      queue::parse_action(action)
    }

    AppFocus::Home => {
      home::parse_action(action)
    }

    AppFocus::HomeHistory => {
      home::history_parse_action(action)
    }

    AppFocus::HomePlaylists => {
      home::playlist_parse_action(action)
    }

    AppFocus::Search => {
      search::parse_action(action)
    }

    AppFocus::SearchTrack => {
      search::track_parse_action(action)
    }

    AppFocus::SearchAlbum => {
      search::album_parse_action(action)
    }

    AppFocus::SearchPlaylist => {
      search::playlist_parse_action(action)
    }

    AppFocus::SearchArtist => {
      search::artist_parse_action(action)
    }

    _ => None
  }
  */

  None
}
