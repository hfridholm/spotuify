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

  pub(crate) fn panel(
    title: &str,
    color: Color,
    border_type: BorderType,
  ) -> Block<'static> {
    Block::bordered()
      .title(
        Line::from(format!(" {title} ").bold())
        .left_aligned(),
      )
      .border_type(border_type)
      .border_style(
        Style::default()
        .fg(color),
      )
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
    ] =
      Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(5),
        Constraint::Length(2),
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

    self.render_error_and_message(
      info_area,
      buf,
    );
  }
}
