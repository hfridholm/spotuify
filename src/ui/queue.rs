/*   ___                        
 *  / _ \ _   _  ___ _   _  ___ 
 * | | | | | | |/ _ \ | | |/ _ \
 * | |_| | |_| |  __/ |_| |  __/
 *  \__\_\\__,_|\___|\__,_|\___|
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
  text::{
    Line,
  },
  widgets::{
    BorderType,
    Block,
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
  pub(crate) fn render_queue(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let Some(app_focus) = self.get_focus() else {
      return;
    };

    let in_focus = app_focus.is(AppFocus::Queue);

    let block = Block::bordered()
      .title(
        Line::from(format!("  Queue  ").bold())
        .left_aligned(),
      )
      .border_type(BORDER_TYPE.get(in_focus))
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus, in_focus)),
      );

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("TRACK"),
      Cell::from("IN"),
    ])
      .style(
        Style::default()
        .fg(HEADER_COLOR.get(in_focus, in_focus)),
      )
      .bold();

    let mut remaining_ms = self.player.get_remaining()
      .unwrap_or(0);

    let selected_index = match self.queue_state.selected() {
      Some(index) => index,
      None        => 1,
    };

    let rows = match &self.queue {
      Some(queue) if !queue.queue.is_empty() => {
        queue.queue
          .iter()
          .enumerate()
          .map(|(index, track)| {
            let time_str = format_ms(remaining_ms);

            remaining_ms = remaining_ms.saturating_add(track.duration_ms);

            let rel_index: i64 = if index != selected_index {
              (index as i64 - selected_index as i64).abs()
            } else {
              (selected_index + 1) as i64
            };

            Row::new([
              Cell::from(format!("{:>2}", rel_index)),
              Cell::from(track.name.as_str()),
              Cell::from(time_str),
            ])
          },
        )
          .collect::<Vec<_>>()
      }

      _ => {
        vec![
          Row::new([
            Cell::from(""),
            Cell::from("No queue"),
            Cell::from(""),
          ])
        ]
      }
    };

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
        .fg(TEXT_COLOR.get(in_focus, in_focus))
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray)
        .fg(Color::Black)
      );

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.queue_state,
    );
  }
}
