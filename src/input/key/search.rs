/*  ____                      _     
 * / ___|  ___  __ _ _ __ ___| |__  
 * \___ \ / _ \/ _` | '__/ __| '_ \ 
 *  ___) |  __/ (_| | | | (__| | | |
 * |____/ \___|\__,_|_|  \___|_| |_|
 */

use crossterm::event::{
  KeyCode,
  KeyEvent,
};

use super::{
  AppAction,
  AppFocus
};

pub(crate) fn parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Enter => {
      Some(AppAction::Search)
    }

    KeyCode::Char(to_insert) => {
      Some(AppAction::SearchInsert { to_insert })
    }

    KeyCode::Backspace => {
      Some(AppAction::SearchDelete)
    }

    _ => None
  }
}

pub(crate) fn track_parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Enter => {
      Some(AppAction::PlaySearch)
    }

    KeyCode::Char('i') => {
      Some(AppAction::ShowSearchTrack)
    }

    KeyCode::Tab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::SearchArtist, tab_index: 1 })
    }

    _ => None
  }
}

pub(crate) fn artist_parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Enter => {
      Some(AppAction::ShowSearchArtist)
    }

    KeyCode::Tab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::SearchAlbum, tab_index: 2 })
    }

    KeyCode::BackTab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::SearchTrack, tab_index: 0 })
    }

    _ => None
  }
}

pub(crate) fn album_parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Enter => {
      Some(AppAction::ShowSearchAlbum)
    }

    KeyCode::Tab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::SearchPlaylist, tab_index: 3 })
    }

    KeyCode::BackTab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::SearchArtist, tab_index: 1 })
    }

    _ => None
  }
}

pub(crate) fn playlist_parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::BackTab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::SearchAlbum, tab_index: 2 })
    }

    _ => None
  }
}
