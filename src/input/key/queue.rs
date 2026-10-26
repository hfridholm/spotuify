/*   ___                        
 *  / _ \ _   _  ___ _   _  ___ 
 * | | | | | | |/ _ \ | | |/ _ \
 * | |_| | |_| |  __/ |_| |  __/
 *  \__\_\\__,_|\___|\__,_|\___|
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
      Some(AppAction::PlayQueue)
    }

    KeyCode::Down | KeyCode::Char('j') => {
      Some(AppAction::QueueNext { count: 1 })
    }

    KeyCode::Up | KeyCode::Char('k') => {
      Some(AppAction::QueuePrev { count: 1 })
    }

    KeyCode::Home => {
      Some(AppAction::QueueFirst)
    }

    KeyCode::End => {
      Some(AppAction::QueueLast)
    }

    _ => None
  }
}
