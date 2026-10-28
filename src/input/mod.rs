/*  ___                   _   
 * |_ _|_ __  _ __  _   _| |_ 
 *  | || '_ \| '_ \| | | | __|
 *  | || | | | |_) | |_| | |_ 
 * |___|_| |_| .__/ \__,_|\__|
 *           |_|              
 */

mod key;
mod action;

use crossterm::event::{
  self,
  Event,
  KeyEvent,
  KeyCode,
  KeyModifiers,
  KeyEventKind
};

use crate::app::{
  App,
  AppAction, 
  AppFocus,
};

use std::{
  time::Duration,
};

use tokio::sync::{
  mpsc::UnboundedSender,
  watch,
};

impl App {
  pub(crate) fn spawn_event_handler(
    &self,
    mut shutdown: watch::Receiver<bool>,
  ) -> tokio::task::JoinHandle<()> {
    let tx = self.action_tx.clone();

    tokio::task::spawn_blocking(move || {
      // Stop if shutdown has been requested.
      while !*shutdown.borrow() {
        let ready = match event::poll(Duration::from_millis(100)) {
          Ok(ready) => ready,
          Err(error) => {
            let _ = tx.send(AppAction::Error(format!(
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
            let _ = tx.send(AppAction::Error(format!(
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

        // Now, we are processing an actual keypress
        if tx.send(AppAction::KeyPress { key }).is_err() {
          return;
        }
      }
    })
  }
}

/*
 * I have to think about what and how I can use this
 * Vim-type input buffer
 *
 * It is useful for:
 * - moving multiple lines in lists MoveUp { 3 } -> 3x ListPrev
 * - entering input mode on input fields
 *   + dd  -> remove all buffer
 *   + i/I -> enter input mode
 * - '/' search for stuff (SEARCH mode)
 * - ':' enter command    (COMMAND mode)
 * - gg -> move to top of screen
 * - G  -> move to bottom of screen
 * - '_ee' -> open and close queue
 *
 * Scheme:
 * 1. Input key (KeyEvent)
 * 2. If InputBuffer is empty,
 *    Let active AppFocus parse KeyEvent
 *    (because each window can have special buttons)
 * 3. If focus didn't have anything special for key,
 *    Push key to InputBuffer - building the input buffer command
 * 4. If InputBuffer has a completed command,
 *    Let active AppFocus parse InputBuffer InputAction
 *    (an AppFocus don't have to accept all InputAction)
 *
 * Implementation:
 * 1. 
 */

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
  JumpUp,
  JumpDown,
  JumpLeft,
  JumpRight,
  JumpTop,
  JumpBottom,
  MoveUp    { count: usize },
  MoveDown  { count: usize },
  MoveLeft  { count: usize },
  MoveRight { count: usize },
  MoveTop,
  MoveBottom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputStatus {
  Pending,
  Complete(InputAction),
  Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
  Normal,
  Insert,
  Command,
}

#[derive(Debug, Default)]
pub struct InputBuffer {
  keys:  Vec<KeyEvent>,
  count: Option<usize>,
}

impl InputBuffer {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn keys(&self) -> &[KeyEvent] {
    &self.keys
  }

  pub fn count(&self) -> usize {
    self.count.unwrap_or(1)
  }

  pub fn clear(&mut self) {
    self.keys.clear();
    self.count = None;
  }

  pub fn push(&mut self, key: KeyEvent) -> InputStatus {
    // Escape cancels the current sequence.
    if key.code == KeyCode::Esc {
      self.clear();
      return InputStatus::Invalid;
    }

    // Parse a leading numeric count.
    if let KeyCode::Char(c @ '1'..='9') = key.code {
      if self.keys.is_empty() {
        let digit = c.to_digit(10).unwrap() as usize;
        self.count = Some(
          self.count
          .unwrap_or(0)
          .saturating_mul(10)
          .saturating_add(digit)
          .min(999_999),
        );
        return InputStatus::Pending;
      }
    }

    // Zero is a count only after another count digit.
    if key.code == KeyCode::Char('0')
      && (self.count.is_some() || !self.keys.is_empty())
        && self.keys.is_empty()
    {
      self.count = Some(
        self.count.unwrap_or(0)
        .saturating_mul(10)
        .min(999_999),
      );
      return InputStatus::Pending;
    }

    self.keys.push(key);
    self.resolve()
  }

  fn resolve(&self) -> InputStatus {
    let count = self.count();

    // Only plain, unmodified character keys match these commands.
    let chars: Option<String> = self.keys.iter().map(|key| {
      if key.modifiers == KeyModifiers::NONE
        || key.modifiers == KeyModifiers::SHIFT
      {
        if let KeyCode::Char(c) = key.code {
          return Some(c);
        }
      }
      None
    }).collect();

    let Some(chars) = chars else {
      return InputStatus::Invalid;
    };

    match chars.as_str() {
      "j"  => InputStatus::Complete(InputAction::MoveDown  { count }),
      "k"  => InputStatus::Complete(InputAction::MoveUp    { count }),
      "h"  => InputStatus::Complete(InputAction::MoveLeft  { count }),
      "l"  => InputStatus::Complete(InputAction::MoveRight { count }),
      "H"  => InputStatus::Complete(InputAction::MoveTop),
      "L"  => InputStatus::Complete(InputAction::MoveBottom),
      "gg" => InputStatus::Complete(InputAction::JumpTop),
      "G"  => InputStatus::Complete(InputAction::JumpBottom),
      _ if ["gg", "dd", "j", "k"]
        .iter()
        .any(|cmd| cmd.starts_with(chars.as_str())) =>
      {
        InputStatus::Pending
      }
      _ => InputStatus::Invalid,
    }
  }

  pub fn string(&self) -> String {
    let mut string = self.count
      .filter(|n| *n > 0)
      .map(|n| n.to_string())
      .unwrap_or_default();

    for key in &self.keys {
      match key.code {
        KeyCode::Char(c)   => string.push(c),
        KeyCode::Esc       => string.push_str("<Esc>"),
        KeyCode::Enter     => string.push_str("<Enter>"),
        KeyCode::Backspace => string.push_str("<BS>"),
        KeyCode::Up        => string.push_str("<Up>"),
        KeyCode::Down      => string.push_str("<Down>"),
        _ => string.push('?'),
      }
    }

    string
  }
}

impl App {
  pub(crate) fn handle_key(&mut self, key: KeyEvent) {
    match self.mode {
      InputMode::Normal  => self.handle_normal(key),
      InputMode::Insert  => self.handle_insert(key),
      InputMode::Command => self.handle_command(key),
    }
  }

  fn handle_normal(&mut self, key: KeyEvent) {
    match key.code {
      KeyCode::Char(':') => {
        self.input.clear();
        self.command.clear();
        self.mode = InputMode::Command;
      }
      _ => {
        self.parse_key(key);
      }
    }
  }

  fn handle_insert(&mut self, key: KeyEvent) {
    match key.code {
      KeyCode::Esc => {
        self.mode = InputMode::Normal;
      }
      KeyCode::Char(c) => {
        // self.insert_text(&c.to_string());
      }
      KeyCode::Enter => {
        // self.insert_text("\n");
      }
      KeyCode::Backspace => {
        // self.delete_text();
      }
      _ => {
        // Handle arrows, Home, End, Delete, etc.
      }
    }
  }

  fn handle_command(&mut self, key: KeyEvent) {
    match key.code {
      KeyCode::Esc => {
        self.command.clear();
        self.mode = InputMode::Normal;
      }
      KeyCode::Enter => {
        self.parse_command();
        self.command.clear();
        self.mode = InputMode::Normal;
      }
      KeyCode::Backspace => {
        self.command.pop();
      }
      KeyCode::Char(c) => {
        self.command.push(c);
      }
      _ => {}
    }
  }

  fn parse_key(&mut self, key: KeyEvent) {
    let tx = self.action_tx.clone();

    if let Some(app_action) = key::parse_key(self.focus, key) {
      if tx.send(app_action).is_err() {
        return;
      }
    } else {
      match self.input.push(key) {
        InputStatus::Pending => {}

        InputStatus::Complete(input_action) => {
          self.input.clear();

          if let Some(app_action) = action::parse_action(self.focus, input_action) {
            if tx.send(app_action).is_err() {
              return;
            }
          } else {
            // Something
          }
        }

        InputStatus::Invalid => {
          self.input.clear();
        }
      }
    }
  }

  fn insert_text(&mut self, _text: &str) {
    // Insert into your document model at the cursor.
  }

  fn delete_text(&mut self) {
    // Delete the previous character safely.
  }

  fn parse_command(&mut self) {
    let tx = self.action_tx.clone();

    if let Some(app_action) = match self.command.as_str() {
      "q" | "quit" => Some(AppAction::Quit),

      "next"   => Some(AppAction::Next),
      "prev"   => Some(AppAction::Prev),
      "resume" => Some(AppAction::Resume),
      "pause"  => Some(AppAction::Pause),

      _ => None
    }
    {
      if tx.send(app_action).is_err() {
        return;
      }
    }
  }
}
