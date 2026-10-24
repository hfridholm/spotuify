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
    let is_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus == AppFocus::Track,
      Err(_) => return,
    };

    let track = match &self.track {
      Some(track) => track,
      None => return,
    };

    let (block_color, header_color, border_type) = if is_focus {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let block = Block::bordered()
      .title(
        Line::from(format!(" {} ", track.name).bold())
        .left_aligned()
        .style(Style::default().fg(header_color))
      )
      .border_type(border_type)
      .border_style(
        Style::default()
        .fg(block_color)
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
