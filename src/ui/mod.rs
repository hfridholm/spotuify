/*  _   _ ___ 
 * | | | |_ _|
 * | | | || | 
 * | |_| || | 
 *  \___/|___|
 */

mod album;
mod artist;
mod helpers;
mod home;
mod instructions;
mod navbar;
mod notifications;
mod player;
mod playlist;
mod queue;
mod search;
mod status;
mod track;

use crate::app::{
  App,
  AppWindow,
  AppFocus,
};

use helpers::format_ms;

use ratatui::{
  buffer::Buffer,
  layout::{
    Constraint,
    Layout,
    Rect,
    Direction
  },
  style::{
    Color,
    Style,
    Stylize
  },
  text::Line,
  widgets::{
    Block,
    BorderType,
    Widget
  },
  Frame,
};

pub(crate) struct UiBorderType {
  active:  BorderType,
  passive: BorderType,
}

pub(crate) const BORDER_TYPE: UiBorderType = UiBorderType {
  active:  BorderType::Thick,
  passive: BorderType::Plain,
};

impl UiBorderType {
  pub(crate) const fn get(&self, active: bool) -> BorderType {
    if active {
      self.active
    } else {
      self.passive
    }
  }
}

pub(crate) struct UiColor {
  in_focus:  Color,
  in_window: Color,
  other:     Color,
}

pub(crate) const TAB_COLOR: UiColor = UiColor {
  in_focus:  Color::LightBlue,
  in_window: Color::White,
  other:     Color::DarkGray,
};

pub(crate) const BORDER_COLOR: UiColor = UiColor {
  in_focus:  Color::Green,
  in_window: Color::White,
  other:     Color::DarkGray,
};

pub(crate) const TEXT_COLOR: UiColor = UiColor {
  in_focus:  Color::White,
  in_window: Color::White,
  other:     Color::DarkGray,
};

pub(crate) const HEADER_COLOR: UiColor = UiColor {
  in_focus:  Color::Yellow,
  in_window: Color::White,
  other:     Color::DarkGray,
};

impl UiColor {
  pub(crate) const fn get(&self, in_focus: bool, in_window: bool) -> Color {
    if in_window && in_focus {
      self.in_focus
    } else if in_window {
      self.in_window
    } else {
      self.other
    }
  }
}

impl App {
  pub(crate) fn draw(&mut self, frame: &mut Frame) {
    frame.render_widget(self, frame.area());
  }

  pub(crate) fn render_window(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_window = match self.window.lock() {
      Ok(app_window) => *app_window,
      Err(_) => return,
    };

    match app_window {
      AppWindow::Home     => self.render_home(area, buf),
      AppWindow::Search   => self.render_search(area, buf),
      AppWindow::Track    => self.render_track(area, buf),
      AppWindow::Artist   => self.render_artist(area, buf),
      AppWindow::Album    => self.render_album(area, buf),
      AppWindow::Playlist => self.render_playlist(area, buf),
    };
  }
}

// ============================================================================
// Widget implementation
// ============================================================================

impl Widget for &mut App {
  fn render(
    self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let [
      main_area,
      player_area,
      instructions_area,
      status_area,
    ] =
      Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(6),
        Constraint::Length(2),
        Constraint::Length(1),
      ])
      .margin(1)
      .areas(area);

    let [
      queue_area,
      right_area,
    ] =
      Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(80),
      ])
      .areas(main_area);

    let [ navbar_area, window_area ] = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length(3),
        Constraint::Min(5),
      ])
      .areas(right_area);

    let [ _, info_area ] = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
      ])
      .areas(area);

    self.render_navbar(
      navbar_area,
      buf,
    );

    self.render_player(
      player_area,
      buf,
    );

    self.render_queue(
      queue_area,
      buf,
    );

    self.render_window(
      window_area,
      buf,
    );

    self.render_instructions(
      instructions_area,
      buf,
    );

    self.render_status(
      status_area,
      buf,
    );

    self.render_error_and_message(
      info_area,
      buf,
    );
  }
}
