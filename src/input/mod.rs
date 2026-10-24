/*  ___                   _   
 * |_ _|_ __  _ __  _   _| |_ 
 *  | || '_ \| '_ \| | | | __|
 *  | || | | | |_) | |_| | |_ 
 * |___|_| |_| .__/ \__,_|\__|
 *           |_|              
 */

mod keymap;

use std::sync::{Arc, Mutex};

use crossterm::event::{self, Event, KeyEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{Action, AppFocus, AppWindow};

pub fn spawn_event_handler(
  tx: UnboundedSender<Action>,
  app_focus:  Arc<Mutex<AppFocus>>,
  app_window: Arc<Mutex<AppWindow>>,
) {
  tokio::spawn(async move {
    loop {
      let event_result = tokio::task::spawn_blocking(event::read).await;

      let event = match event_result {
        Ok(Ok(event)) => event,

        Ok(Err(error)) => {
          let _ = tx.send(
            Action::Error(format!("Terminal event error: {error}")),
          );
          return;
        }

        Err(error) => {
          let _ = tx.send(
            Action::Error(format!("Event task error: {error}")),
          );
          return;
        }
      };

      let Event::Key(key) = event else {
        continue;
      };

      if key.kind != KeyEventKind::Press {
        continue;
      }

      let app_focus = match app_focus.lock() {
        Ok(app_focus) => *app_focus,
        Err(_) => return,
      };

      let action = keymap::key_to_action(app_focus, key);

      if tx.send(action).is_err() {
        return;
      }
    }
  });
}
