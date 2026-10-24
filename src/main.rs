/*                  _         _  __       
 *  ___ _ __   ___ | |_ _   _(_)/ _|_   _ 
 * / __| '_ \ / _ \| __| | | | | |_| | | |
 * \__ \ |_) | (_) | |_| |_| | |  _| |_| |
 * |___/ .__/ \___/ \__|\__,_|_|_|  \__, |
 *     |_|                          |___/ 
 */

/*
use std::{
  sync::{
    Arc,
    Mutex,
    atomic::{AtomicU64, Ordering},
  },
  collections::VecDeque,
};

use crossterm::event::{
  self,
  Event,
  KeyCode,
  KeyEvent,
  KeyEventKind,
  KeyModifiers,
};

use ratatui::{
  buffer::Buffer,
  layout::{
    Constraint,
    Layout,
    Rect,
    Direction,
    Spacing,
  },
  style::{
    Color,
    Style,
    Stylize,
    Modifier,
  },
  symbols::{
    border,
    merge::MergeStrategy,
  },
  symbols,
  text::{
    Line,
    Text,
    Span,
  },
  widgets::{
    Block,
    BorderType,
    Cell,
    LineGauge,
    Paragraph,
    Row,
    StatefulWidget,
    Table,
    TableState,
    Tabs,
    Widget,
    Wrap,
    Borders,
    Clear,
  },
  DefaultTerminal,
  Frame,
};

use tokio::{
  sync::mpsc::{
    self,
    UnboundedReceiver,
    UnboundedSender,
  },
  time,
  time::{sleep, Duration},
};

mod api;
use crate::api::{
  SpotifyApi,
  User,
  Player,
  History,
  Queue,
  Search,
  SimplifiedTracks,
  Tracks,
  Track,
  Artists,
  Artist,
  Albums,
  Album,
  Playlists,
  Playlist,
};
*/

mod app;
mod input;
mod ui;
mod api;
mod error;


use crate::error::Error;

use app::App;
use crate::app::Action;

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
    app.window.clone(),
  );

  let mut terminal = ratatui::init();

  let result = app.run(
    &mut terminal,
    action_rx,
  ).await;

  ratatui::restore();

  result
}
