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

use crate::app::App;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl App {
  pub(crate) fn spawn_error(&self, error: String) {
    let error_list = Arc::clone(&self.error_list);
    
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    tokio::spawn(async move {
      error_list.lock().unwrap().push_back((id, error));

      sleep(Duration::from_secs(5)).await;

      error_list.lock().unwrap().retain(|(entry_id, _)| *entry_id != id);
    });
  }

  pub(crate) fn spawn_message(&self, message: String) {
    let message_list = Arc::clone(&self.message_list);
    
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    tokio::spawn(async move {
      message_list.lock().unwrap().push_back((id, message));

      sleep(Duration::from_secs(10)).await;

      message_list.lock().unwrap().retain(|(entry_id, _)| *entry_id != id);
    });
  }
}
