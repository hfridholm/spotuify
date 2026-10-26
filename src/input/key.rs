/*  _  __                                
 * | |/ /___ _   _ 
 * | ' // _ \ | | |
 * | . \  __/ |_| |
 * |_|\_\___|\__, |
 *           |___/
 */

pub mod home;
pub mod navbar;
pub mod player;
pub mod queue;
pub mod search;

use crossterm::event::{
  KeyCode,
  KeyEvent,
  KeyModifiers,
};

use super::{
  AppAction,
  AppFocus
};

pub(crate) fn parse_key(
  app_focus: AppFocus,
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Char('c') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(AppAction::Quit);
      }
    }

    KeyCode::Char('j') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(AppAction::Down);
      }
    }

    KeyCode::Char('k') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(AppAction::Up);
      }
    }

    KeyCode::Char('h') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(AppAction::Left);
      }
    }

    KeyCode::Char('l') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(AppAction::Right);
      }
    }

    KeyCode::F(1) => {
      return Some(AppAction::SetFocus { focus: AppFocus::Search });
    }

    KeyCode::F(2) => {
      return Some(AppAction::SetFocus { focus: AppFocus::HomePlaylists });
    }

    _ => {}
  }

  match app_focus {
    AppFocus::Navbar => {
      navbar::parse_key(key)
    }

    AppFocus::Player => {
      player::parse_key(key)
    }

    AppFocus::Queue => {
      queue::parse_key(key)
    }

    AppFocus::Home => {
      home::parse_key(key)
    }

    AppFocus::HomeHistory => {
      home::history_parse_key(key)
    }

    AppFocus::HomePlaylists => {
      home::playlist_parse_key(key)
    }

    AppFocus::Search => {
      search::parse_key(key)
    }

    AppFocus::SearchTrack => {
      search::track_parse_key(key)
    }

    AppFocus::SearchAlbum => {
      search::album_parse_key(key)
    }

    AppFocus::SearchPlaylist => {
      search::playlist_parse_key(key)
    }

    AppFocus::SearchArtist => {
      search::artist_parse_key(key)
    }

    _ => None
  }
}

