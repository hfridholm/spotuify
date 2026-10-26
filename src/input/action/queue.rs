/*   ___                        
 *  / _ \ _   _  ___ _   _  ___ 
 * | | | | | | |/ _ \ | | |/ _ \
 * | |_| | |_| |  __/ |_| |  __/
 *  \__\_\\__,_|\___|\__,_|\___|
 */

use super::{
  AppAction,
  AppFocus,
  InputAction,
};

pub(crate) fn parse_action(
  action: InputAction
) -> Option<AppAction> {
  match action {
    InputAction::MoveDown { count } => {
      Some(AppAction::QueueNext { count })
    }

    InputAction::MoveUp { count } => {
      Some(AppAction::QueuePrev { count })
    }

    InputAction::MoveTop => {
      Some(AppAction::QueueFirst)
    }

    InputAction::MoveBottom => {
      Some(AppAction::QueueLast)
    }

    _ => None
  }
}
