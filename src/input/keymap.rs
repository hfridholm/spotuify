/*  _  __                                
 * | |/ /___ _   _ _ __ ___   __ _ _ __  
 * | ' // _ \ | | | '_ ` _ \ / _` | '_ \ 
 * | . \  __/ |_| | | | | | | (_| | |_) |
 * |_|\_\___|\__, |_| |_| |_|\__,_| .__/ 
 *           |___/                |_|
 */

pub mod home;
pub mod navbar;
pub mod player;
pub mod queue;
pub mod search;

use crossterm::event::{
  KeyCode, KeyEvent, KeyModifiers,
};

use crate::app::{Action, AppFocus};

pub(crate) fn key_to_action(
  app_focus: AppFocus,
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Char('c') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
      }
    }

    KeyCode::Char('j') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Down;
      }
    }

    KeyCode::Char('k') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Up;
      }
    }

    KeyCode::Char('h') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Left;
      }
    }

    KeyCode::Char('l') => {
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Right;
      }
    }

    _ => {}
  }

  match app_focus {
    AppFocus::Navbar => {
      navbar::navbar_key_action(key)
    }

    AppFocus::Player => {
      player::player_key_action(key)
    }

    AppFocus::Queue => {
      queue::queue_key_action(key)
    }

    AppFocus::Home => {
      home::home_key_action(key)
    }

    AppFocus::HomeHistory => {
      home::home_history_key_action(key)
    }

    AppFocus::HomePlaylists => {
      home::home_playlists_key_action(key)
    }

    AppFocus::Search => {
      search::search_key_action(key)
    }

    AppFocus::SearchTrack => {
      search::search_track_key_action(key)
    }

    AppFocus::SearchAlbum => {
      search::search_album_key_action(key)
    }

    AppFocus::SearchPlaylist => {
      search::search_playlist_key_action(key)
    }

    AppFocus::SearchArtist => {
      search::search_artist_key_action(key)
    }

    _ => Action::Render
  }
}

