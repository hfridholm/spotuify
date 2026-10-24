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

use super::Action;

pub(crate) fn navbar_key_action(
  key: KeyEvent,
) -> Action {
  match key.code {
    KeyCode::Enter => {
      Action::NavbarEnter
    }

    KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
      Action::ListNext { max: 2 }
    }

    KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
      Action::ListPrev
    }

    _ => Action::Render,
  }
}
