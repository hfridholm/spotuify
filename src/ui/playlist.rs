/*  ____  _             _ _     _   
 * |  _ \| | __ _ _   _| (_)___| |_ 
 * | |_) | |/ _` | | | | | / __| __|
 * |  __/| | (_| | |_| | | \__ \ |_ 
 * |_|   |_|\__,_|\__, |_|_|___/\__|
 *                |___/             
 */

use super::{
  App,
  AppFocus,
  BORDER_COLOR,
  HEADER_COLOR,
  BORDER_TYPE,
};

use ratatui::{
  buffer::Buffer,
  text::Line,
  widgets::{
    Block,
    BorderType,
    Table,
    Row,
    Cell,
    StatefulWidget
  },
  style::{
    Color,
    Style,
    Stylize,
  },
  layout::{
    Constraint,
    Rect,
  },
};

impl App {
  pub(crate) fn render_playlist(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let in_focus = self.focus.is(AppFocus::Playlist);

    let Some(playlist) = &self.playlist else {
      return;
    };

    let block = Block::bordered()
      .border_type(BORDER_TYPE.get(in_focus))
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus, in_focus)),
      )
      .title(
        Line::from(format!(" {} ", playlist.name.to_string()).bold())
        .left_aligned(),
      );

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("TRACK"),
      Cell::from("AGO"),
    ])
      .style(
        Style::default()
        .fg(HEADER_COLOR.get(in_focus, in_focus)),
      )
      .bold();

    let no_tracks = || {
      Row::new([
        Cell::from(""),
        Cell::from("No Tracks"),
        Cell::from(""),
      ])
    };

    /*
    let rows = playlist
      .as_ref()
      .map(|playlist| {
        playlist.items.items
          .iter()
          .enumerate()
          .map(|(index, track)| {
            Row::new([
              Cell::from(format!("{:>2}", index + 1)),
              Cell::from(track.name.as_str()),
              Cell::from(track.name.as_str()),
            ])
          })
        .collect::<Vec<_>>()
      })
    .filter(|items| !items.is_empty())
      .unwrap_or_else(|| vec![no_tracks()]);
    */

    let rows = vec![no_tracks()];

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
