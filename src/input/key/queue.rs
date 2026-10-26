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

    _ => None
  }
}
