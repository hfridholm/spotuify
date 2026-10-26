/*  ____                      _     
 * / ___|  ___  __ _ _ __ ___| |__  
 * \___ \ / _ \/ _` | '__/ __| '_ \ 
 *  ___) |  __/ (_| | | | (__| | | |
 * |____/ \___|\__,_|_|  \___|_| |_|
 */

use super::{
  AppAction,
  AppFocus,
  InputAction,
};

pub(crate) fn track_parse_action(
  action: InputAction,
) -> Option<AppAction> {
  match action {
    InputAction::MoveDown { count } => {
      Some(AppAction::TableNext { count })
    }

    InputAction::MoveUp { count } => {
      Some(AppAction::TablePrev { count })
    }

    _ => None
  }
}

pub(crate) fn artist_parse_action(
  action: InputAction
) -> Option<AppAction> {
  match action {
    InputAction::MoveDown { count } => {
      Some(AppAction::TableNext { count })
    }

    InputAction::MoveUp { count } => {
      Some(AppAction::TablePrev { count })
    }

    _ => None
  }
}

pub(crate) fn album_parse_action(
  action: InputAction,
) -> Option<AppAction> {
  match action {
    InputAction::MoveDown { count } => {
      Some(AppAction::TableNext { count })
    }

    InputAction::MoveUp { count } => {
      Some(AppAction::TablePrev { count })
    }

    _ => None
  }
}

pub(crate) fn playlist_parse_action(
  action: InputAction,
) -> Option<AppAction> {
  match action {
    InputAction::MoveDown { count } => {
      Some(AppAction::TableNext { count })
    }

    InputAction::MoveUp { count } => {
      Some(AppAction::TablePrev { count })
    }

    _ => None
  }
}
