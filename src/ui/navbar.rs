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
  BORDER_COLOR,
  HEADER_COLOR,
  BORDER_TYPE,
  TAB_COLOR,
};

use ratatui::{
  buffer::Buffer,
  widgets::{
    Block,
    BorderType,
    Paragraph,
    Widget,
    Tabs,
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
    let in_focus = self.focus.is(AppFocus::Navbar);

    const TABS: [(AppWindow, &str); 2] = [
      (AppWindow::Home,   "Home"),
      (AppWindow::Search, "Search"),
    ];

    let block = Block::bordered()
      .border_type(BORDER_TYPE.get(in_focus))
      .border_style(Style::default()
        .fg(BORDER_COLOR.get(in_focus, in_focus))
      );

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

    // Creates tabs from every string TABS[i].1
    let strings: [&_; TABS.len()] = std::array::from_fn(|i| TABS[i].1);

    Tabs::new(strings)
      .select(self.list_index)
      .style(
        Style::default()
        .fg(TAB_COLOR.get(false, in_focus))
      )
      .highlight_style(
        Style::default()
        .fg(TAB_COLOR.get(true, in_focus))
      )
      .divider("")
      .render(centered[1], buf);
  }
}
