/*  _   _       _   _  __ _           _   _                 
 * | \ | | ___ | |_(_)/ _(_) ___ __ _| |_(_) ___  _ __  ___ 
 * |  \| |/ _ \| __| | |_| |/ __/ _` | __| |/ _ \| '_ \/ __|
 * | |\  | (_) | |_| |  _| | (_| (_| | |_| | (_) | | | \__ \
 * |_| \_|\___/ \__|_|_| |_|\___\__,_|\__|_|\___/|_| |_|___/
 */

use std::sync::{
  Arc,
  atomic::{AtomicU64, Ordering},
};

use tokio::time::{sleep, Duration};

use super::{
  App,
  AppAction,
};

pub struct AppNotification {
  pub(crate) id:       u64,
  pub(crate) is_error: bool,
  pub(crate) text:     String,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl App {
  pub(crate) fn spawn_error(&self, error: String) {
    let tx = self.action_tx.clone();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    tokio::spawn(async move {
      if tx.send(AppAction::AddError { id, error }).is_err() {
        return;
      }

      sleep(Duration::from_secs(5)).await;

      let _ = tx.send(AppAction::RemoveNotification { id });
    });
  }

  pub(crate) fn spawn_info(&self, info: String) {
    let tx = self.action_tx.clone();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    tokio::spawn(async move {
      if tx.send(AppAction::AddInfo { id, info }).is_err() {
        return;
      }

      sleep(Duration::from_secs(5)).await;

      let _ = tx.send(AppAction::RemoveNotification { id });
    });
  }
}
