/*  _   _                      
 * | | | | ___  _ __ ___   ___ 
 * | |_| |/ _ \| '_ ` _ \ / _ \
 * |  _  | (_) | | | | | |  __/
 * |_| |_|\___/|_| |_| |_|\___|
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
    KeyCode::Tab | KeyCode::Right | KeyCode::Char('j') => {
      Some(AppAction::ListNext { count: 1, max: 2 })
    }

    KeyCode::BackTab | KeyCode::Left | KeyCode::Char('k') => {
      Some(AppAction::ListPrev { count: 1 })
    }

    _ => None
  }
}

pub(crate) fn playlist_parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Enter => {
      Some(AppAction::ShowHomePlaylist)
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Some(AppAction::TableNext { count: 1 })
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Some(AppAction::TablePrev { count: 1 })
    }

    KeyCode::BackTab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::HomeHistory, tab_index: 0 })
    }

    _ => None
  }
}

pub(crate) fn history_parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Enter => {
      Some(AppAction::PlayHistory)
    }

    KeyCode::Left | KeyCode::Char('h') => {
      Some(AppAction::QueueHistory)
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Some(AppAction::TableNext { count: 1 })
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Some(AppAction::TablePrev { count: 1 })
    }

    KeyCode::Tab => {
      Some(AppAction::SetFocusAndTabIndex { focus: AppFocus::HomePlaylists, tab_index: 1 })
    }

    _ => None
  }
}
