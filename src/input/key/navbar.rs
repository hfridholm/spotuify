/*  _   _             _                
 * | \ | | __ ___   _| |__   __ _ _ __ 
 * |  \| |/ _` \ \ / / '_ \ / _` | '__|
 * | |\  | (_| |\ V /| |_) | (_| | |   
 * |_| \_|\__,_| \_/ |_.__/ \__,_|_|   
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
      Some(AppAction::NavbarEnter)
    }

    KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
      Some(AppAction::ListNext { count: 1, max: 2 })
    }

    KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
      Some(AppAction::ListPrev { count: 1 })
    }

    _ => None
  }
}
