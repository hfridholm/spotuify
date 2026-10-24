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

use crate::Action;
use crate::input::AppFocus;

pub(crate) fn search_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::Search
    }

    KeyCode::Char(to_insert) => {
      Action::SearchInsert { to_insert }
    }

    KeyCode::Backspace => {
      Action::SearchDelete
    }

    _ => Action::Render,
  }
}

pub(crate) fn search_track_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::PlaySearch
    }

    KeyCode::Char('i') => {
      Action::ShowSearchTrack
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::TableNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::TablePrevious
    }

    KeyCode::Tab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::SearchArtist, tab_index: 1 }
    }

    _ => Action::Render,
  }
}

pub(crate) fn search_artist_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::ShowSearchArtist
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::TableNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::TablePrevious
    }

    KeyCode::Tab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::SearchAlbum, tab_index: 2 }
    }

    KeyCode::BackTab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::SearchTrack, tab_index: 0 }
    }

    _ => Action::Render,
  }
}

pub(crate) fn search_album_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::ShowSearchAlbum
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::TableNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::TablePrevious
    }

    KeyCode::Tab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::SearchPlaylist, tab_index: 3 }
    }

    KeyCode::BackTab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::SearchArtist, tab_index: 1 }
    }

    _ => Action::Render,
  }
}

pub(crate) fn search_playlist_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Down | KeyCode::Char('j') => {
      Action::TableNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::TablePrevious
    }

    KeyCode::BackTab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::SearchAlbum, tab_index: 2 }
    }

    _ => Action::Render,
  }
}
