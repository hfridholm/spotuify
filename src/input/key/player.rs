/*  ____  _                       
 * |  _ \| | __ _ _   _  ___ _ __ 
 * | |_) | |/ _` | | | |/ _ \ '__|
 * |  __/| | (_| | |_| |  __/ |   
 * |_|   |_|\__,_|\__, |\___|_|   
 *                |___/           
 */

use crossterm::event::{
  KeyCode,
  KeyEvent,
};

use super::AppAction;

pub(crate) fn parse_key(
  key: KeyEvent,
) -> Option<AppAction> {
  match key.code {
    KeyCode::Enter => {
      Some(AppAction::Refresh)
    }

    KeyCode::Char(' ') => {
      Some(AppAction::PauseResume)
    }

    KeyCode::Right | KeyCode::Char('l') => {
      Some(AppAction::Next)
    }

    KeyCode::Left | KeyCode::Char('h') => {
      Some(AppAction::Prev)
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Some(AppAction::VolumeDown)
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Some(AppAction::VolumeUp)
    }

    KeyCode::Char('m') => {
      Some(AppAction::Mute)
    }

    KeyCode::Char('s') => {
      Some(AppAction::ToggleShuffle)
    }

    KeyCode::Char('r') => {
      Some(AppAction::ToggleRepeat)
    }

    _ => None
  }
}
