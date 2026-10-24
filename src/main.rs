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
  dotenvy::dotenv().ok();

  let (action_tx, action_rx) =
    mpsc::unbounded_channel::<Action>();

  let mut app = App::new(action_tx.clone()).await?;

  input::spawn_event_handler(
    action_tx.clone(),
    app.focus.clone(),
  );

  let mut terminal = ratatui::init();

  let result = app.run(
    &mut terminal,
    action_rx,
  ).await;

  ratatui::restore();

  result
}
