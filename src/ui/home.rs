/*  _   _                      
 * | | | | ___  _ __ ___   ___ 
 * | |_| |/ _ \| '_ ` _ \ / _ \
 * |  _  | (_) | | | | | |  __/
 * |_| |_|\___/|_| |_| |_|\___|
 */

use super::{
  App,
  AppFocus,
  format_ms,
  BORDER_COLOR,
  HEADER_COLOR,
  TEXT_COLOR,
  BORDER_TYPE,
};

use ratatui::{
  buffer::Buffer,
  text::Line,
  widgets::{
    Block,
    BorderType,
    Paragraph,
    Widget,
    Table,
    Row,
    Tabs,
    StatefulWidget,
    Cell
  },
  symbols::merge::MergeStrategy,
  style::{
    Color,
    Style,
    Stylize,
    Modifier,
  },
  layout::{
    Constraint,
    Layout,
    Rect,
    Spacing,
  },
};

impl App {
  pub(crate) fn render_home(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let in_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus == AppFocus::Home,
      Err(_) => return,
    };

    let [header_area, main_area] = Layout::vertical([
      Constraint::Length(5),
      Constraint::Min(1),
    ])
      .spacing(Spacing::Overlap(1))
      .areas(area);

    let header_block = Block::bordered()
      .title(
        Line::from(format!(" Home ").bold())
        .left_aligned(),
      )
      .merge_borders(MergeStrategy::Exact)
      .border_type(BORDER_TYPE.get(in_focus))
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus)),
      );

    let inner_area = header_block.inner(header_area);

    Paragraph::new(
      Line::from(format!("{}", self.user.display_name.to_string()))
    )
      .style(Style::default()
        .fg(TEXT_COLOR.get(in_focus))
      )
      .centered()
      .render(inner_area, buf);

    // If focus is the main area, render header border on bottom
    if !in_focus {
      header_block.clone().render(header_area, buf);
    }

    match self.tab_index {
      0 => Self::render_home_history(self, main_area, buf),
      1 => Self::render_home_playlists(self, main_area, buf),
      _ => {},
    }
    
    // If focus is the header, render header border on top
    if in_focus {
      header_block.clone().render(header_area, buf);
    }

    let rect = Rect {
      x: header_area.x,
      y: header_area.y + header_area.height - 1,
      width: header_area.width,
      height: 1,
    };

    Self::render_home_tabs(self, rect, buf);
  }

  fn render_home_tabs(
    &mut self,
    rect: Rect,
    buf: &mut Buffer,
  ) {
    const TABS: [(AppFocus, &str); 2] = [
      (AppFocus::HomeHistory,   "History"),
      (AppFocus::HomePlaylists, "Playlist"),
    ];

    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let selected = TABS.iter().position(|(focus, _)| *focus == app_focus);

    let width: u16 = TABS.iter().map(|t| t.1.len() as u16).sum::<u16>()
      + (TABS.len() as u16 - 1) * 2 + 2;

    let centered = Layout::horizontal([
      Constraint::Fill(1),
      Constraint::Length(width),
      Constraint::Fill(1),
    ])
      .split(rect);

    // Creates tabs from every string TABS[i].1
    let strings: [&_; TABS.len()] = std::array::from_fn(|i| TABS[i].1);

    Tabs::new(strings)
      .select(selected)
      .style(
        Style::default()
        .fg(Color::Gray)
      )
      .highlight_style(
        Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD),
      )
      .divider("")
      .render(centered[1], buf);
  }

  fn render_home_playlists(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let in_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus == AppFocus::HomePlaylists,
      Err(_) => return,
    };

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("NAME"),
      Cell::from("ARTISTS"),
    ])
      .style(
        Style::default()
        .fg(HEADER_COLOR.get(in_focus)),
      )
      .bold();

    let rows = if !self.user_playlists.items.is_empty() {
      self.user_playlists.items
        .iter()
        .enumerate()
        .filter_map(|(index, playlist)| {
          let playlist = playlist.as_ref()?;

          Some(Row::new([
              Cell::from(format!("{:>2}", index + 1)),
              Cell::from(playlist.name.as_str()),
              Cell::from(playlist.owner.display_name.as_str()),
          ]))
        }
      )
        .collect::<Vec<_>>()
    } else {
      vec![
        Row::new([
          Cell::from(""),
          Cell::from("No tracks"),
          Cell::from(""),
        ])
      ]
    };

    let block = Block::bordered()
      .border_type(BORDER_TYPE.get(in_focus))
      .merge_borders(MergeStrategy::Exact)
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus)),
      );

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
      Constraint::Fill(1),
    ])
      .header(header)
      .column_spacing(2)
      .style(
        Style::default()
        .fg(Color::White)
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray)
        .fg(Color::Black)
      )
      .block(block);

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.table_state,
    );
  }

  fn render_home_history(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let in_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus == AppFocus::HomeHistory,
      Err(_) => return,
    };

    let block = Block::bordered()
      .border_type(BORDER_TYPE.get(in_focus))
      .merge_borders(MergeStrategy::Exact)
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus)),
      );

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("TRACK"),
      Cell::from("AGO"),
    ])
      .style(
        Style::default()
        .fg(HEADER_COLOR.get(in_focus)),
      )
      .bold();

    let no_history = || {
      Row::new([
        Cell::from(""),
        Cell::from("No history"),
        Cell::from(""),
      ])
    };

    let rows = self.history
      .as_ref()
      .map(|history| {
        history.items
          .iter()
          .filter_map(|played_track| {
            played_track.track
              .as_ref()
              .map(|track| (played_track, track))
          })
        .enumerate()
          .map(|(index, (played_track, track))| {
            Row::new([
              Cell::from(format!("{:>2}", index + 1)),
              Cell::from(track.name.as_str()),
              Cell::from(format_ms(played_track.get_elapsed())),
            ])
          })
        .collect::<Vec<_>>()
      })
    .filter(|items| !items.is_empty())
      .unwrap_or_else(|| vec![no_history()]);

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
      Constraint::Length(3),
    ])
      .header(header)
      .column_spacing(2)
      .block(block)
      .style(
        Style::default()
        .fg(Color::White),
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray),
      );

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.table_state,
    );
  }
}
