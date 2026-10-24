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

use super::Action;

pub(crate) fn player_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::Refresh
    }

    KeyCode::Char(' ') => {
      Action::PauseResume
    }

    KeyCode::Right | KeyCode::Char('l') => {
      Action::Next
    }

    KeyCode::Left | KeyCode::Char('h') => {
      Action::Previous
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Action::VolumeDown
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Action::VolumeUp
    }

    KeyCode::Char('m') => {
      Action::Mute
    }

    KeyCode::Char('s') => {
      Action::ToggleShuffle
    }

    KeyCode::Char('r') => {
      Action::ToggleRepeat
    }

    _ => Action::Render,
  }
}
