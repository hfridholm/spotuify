/*  _   _             _                
 * | \ | | __ ___   _| |__   __ _ _ __ 
 * |  \| |/ _` \ \ / / '_ \ / _` | '__|
 * | |\  | (_| |\ V /| |_) | (_| | |   
 * |_| \_|\__,_| \_/ |_.__/ \__,_|_|   
 */

use super::{
  App,
  AppFocus,
  AppWindow,
};

use ratatui::{
  buffer::Buffer,
  widgets::{
    Block,
    BorderType,
    Paragraph,
    Widget
  },
  style::{
    Color,
    Style,
    Modifier,
  },
  layout::{
    Constraint,
    Layout,
    Rect,
  },
};

impl App {
  pub(crate) fn render_navbar(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(focus) => *focus,
      Err(_) => return,
    };

    let (color, border_type) = if app_focus == AppFocus::Navbar {
      (Color::Green, BorderType::Thick)
    } else {
      (Color::White, BorderType::Plain)
    };

    const TABS: [(AppWindow, &str); 2] = [
      (AppWindow::Home,   "Home"),
      (AppWindow::Search, "Search"),
    ];

    let app_window = match self.window.lock() {
      Ok(window) => *window,
      Err(_) => AppWindow::Home,
    };

    let block = Block::bordered()
      .border_type(border_type)
      .border_style(Style::default().fg(color));

    let inner = block.inner(area);
    block.render(area, buf);

    // Calculate the width needed for all tabs.
    let width = TABS
      .iter()
      .map(|(_, name)| name.len() as u16)
      .sum::<u16>()
      + (TABS.len() as u16 - 1) * 3;

    let centered = Layout::horizontal([
      Constraint::Fill(1),
      Constraint::Length(width),
      Constraint::Fill(1),
    ])
      .split(inner);

    let mut x = centered[1].x;

    for (i, (window, name)) in TABS.iter().enumerate() {
      // List selected item is only visable if navbar is in focus
      let is_list_selected = i == self.list_index && app_focus == AppFocus::Navbar;
      let is_current_window = *window == app_window;

      let style = match (is_list_selected, is_current_window) {
        // Both states are active.
        (true, true) => Style::default()
          .fg(Color::White)
          .bg(Color::Blue)
          .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),

        // Selected list item.
        (true, false) => Style::default()
          .fg(Color::White)
          .bg(Color::Blue),

        // Current application window.
        (false, true) => Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),

        // Neither state is active.
        (false, false) => Style::default()
          .fg(Color::Gray),
      };

      let tab_area = Rect {
        x,
        y: centered[1].y,
        width: name.len() as u16,
        height: centered[1].height,
      };

      Paragraph::new(*name)
        .style(style)
        .render(tab_area, buf);

      x += name.len() as u16 + 3;
    }
  }
}
