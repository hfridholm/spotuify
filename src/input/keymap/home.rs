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

use crate::Action;
use crate::input::AppFocus;

pub(crate) fn home_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Tab | KeyCode::Right | KeyCode::Char('j') => {
      Action::ListNext { max: 2 }
    }

    KeyCode::BackTab | KeyCode::Left | KeyCode::Char('k') => {
      Action::ListPrev
    }

    _ => Action::Render,
  }
}

pub(crate) fn home_playlists_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::ShowHomePlaylist
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::TableNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::TablePrevious
    }

    KeyCode::BackTab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::HomeHistory, tab_index: 0 }
    }

    _ => Action::Render,
  }
}

pub(crate) fn home_history_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::PlayHistory
    }

    KeyCode::Left | KeyCode::Char('h') => {
      Action::QueueHistory
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::TableNext
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::TablePrevious
    }

    KeyCode::Tab => {
      Action::SetFocusAndTabIndex { focus: AppFocus::HomePlaylists, tab_index: 1 }
    }

    _ => Action::Render,
  }
}
