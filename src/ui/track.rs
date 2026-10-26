/*  _____               _    
 * |_   _| __ __ _  ___| | __
 *   | || '__/ _` |/ __| |/ /
 *   | || | | (_| | (__|   < 
 *   |_||_|  \__,_|\___|_|\_\
 */

use super::{
  App,
  AppFocus,
  format_ms,
  BORDER_COLOR,
  HEADER_COLOR,
  BORDER_TYPE,
};

use ratatui::{
  buffer::Buffer,
  layout::{
    Constraint,
    Rect
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
    Widget,
    Table,
    Row,
    Cell
  },
};

impl App {
  pub(crate) fn render_track(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let in_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus == AppFocus::Track,
      Err(_) => return,
    };

    let track = match &self.track {
      Some(track) => track,
      None => return,
    };

    let block = Block::bordered()
      .title(
        Line::from(format!(" {} ", track.name).bold())
        .left_aligned()
        .style(Style::default().fg(HEADER_COLOR.get(in_focus)))
      )
      .border_type(BORDER_TYPE.get(in_focus))
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus))
      );

    let artists =
      track
      .artists
      .iter()
      .map(|artist| {
        artist.name.as_str()
      })
    .collect::<Vec<_>>()
      .join(", ");

    let rows = vec![
      Row::new([
        Cell::from("Artists"),
        Cell::from(":"),
        Cell::from(format!("{}", artists)),
      ]),
      Row::new([
        Cell::from("Duration"),
        Cell::from(":"),
        Cell::from(format!("{}", format_ms(track.duration_ms))),
      ])
    ];

    Widget::render(
      Table::new(rows, [
        Constraint::Min(10),
        Constraint::Length(1),
        Constraint::Min(10),
      ])
        .column_spacing(2)
        .block(block)
        .style(
          Style::default()
          .fg(Color::White),
        ),
      area,
      buf,
    );
  }
}
