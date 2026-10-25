/*                  _         _  __       
 *  ___ _ __   ___ | |_ _   _(_)/ _|_   _ 
 * / __| '_ \ / _ \| __| | | | | |_| | | |
 * \__ \ |_) | (_) | |_| |_| | |  _| |_| |
 * |___/ .__/ \___/ \__|\__,_|_|_|  \__, |
 *     |_|                          |___/ 
 */

mod app;
mod input;
mod ui;
mod api;
mod error;

use error::Error;
use app::{
  Action,
  App
};

use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Error> {
  let home_dir = dirs::home_dir()
    .ok_or("Could not determine home directory")?;

  let env_path = home_dir.join(".spotuify").join(".env");
  dotenvy::from_path(&env_path)?;

  let (action_tx, action_rx) =
    mpsc::unbounded_channel::<Action>();

  let mut app = App::new(action_tx.clone()).await?;

  let (shutdown_tx, shutdown_rx) =
    tokio::sync::watch::channel(false);

  let event_task = input::spawn_event_handler(
    action_tx.clone(),
    app.focus.clone(),
    shutdown_rx,
  );

  let mut terminal = ratatui::init();

  let result = app.run(&mut terminal, action_rx).await;

  // Stop the event handler and wait for it to exit.
  let _ = shutdown_tx.send(true);
  let _ = event_task.await;

  ratatui::restore();

  result
}
