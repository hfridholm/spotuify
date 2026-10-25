/*  ___                   _   
 * |_ _|_ __  _ __  _   _| |_ 
 *  | || '_ \| '_ \| | | | __|
 *  | || | | | |_) | |_| | |_ 
 * |___|_| |_| .__/ \__,_|\__|
 *           |_|              
 */

mod keymap;

use crossterm::event::{
  self,
  Event,
  KeyEventKind
};

use crate::app::{
  Action, 
  AppFocus,
};

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use tokio::sync::{
  mpsc::UnboundedSender,
  watch,
};

pub fn spawn_event_handler(
  tx: UnboundedSender<Action>,
  app_focus: Arc<Mutex<AppFocus>>,
  mut shutdown: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
  tokio::task::spawn_blocking(move || {
    // Stop if shutdown has been requested.
    while !*shutdown.borrow() {
      let ready = match event::poll(Duration::from_millis(100)) {
        Ok(ready) => ready,
        Err(error) => {
          let _ = tx.send(Action::Error(format!(
                "Terminal event error: {error}"
          )));
          return;
        }
      };

      if !ready {
        continue;
      }

      let event = match event::read() {
        Ok(event) => event,
        Err(error) => {
          let _ = tx.send(Action::Error(format!(
                "Terminal event error: {error}"
          )));
          return;
        }
      };

      let Event::Key(key) = event else {
        continue;
      };

      if key.kind != KeyEventKind::Press {
        continue;
      }

      let focus = match app_focus.lock() {
        Ok(focus) => *focus,
        Err(_) => return,
      };

      if tx.send(keymap::key_to_action(focus, key)).is_err() {
        return;
      }
    }
  })
}
