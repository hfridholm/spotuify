/*  ____  _        _             
 * / ___|| |_ __ _| |_ _   _ ___ 
 * \___ \| __/ _` | __| | | / __|
 *  ___) | || (_| | |_| |_| \__ \
 * |____/ \__\__,_|\__|\__,_|___/
 */

use super::{
  App,
  AppFocus,
};

use crate::input::{
  InputMode,
};

use ratatui::{
  buffer::Buffer,
  layout::Rect,
  text::{
    Line,
    Span,
  },
  widgets::{
    Paragraph,
    Widget,
    Wrap
  },
  style::{
    Color,
    Style,
    Stylize,
  },
};

impl App {
  pub(crate) fn render_status(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let string = self.input.string();

    let line = match self.mode {
      InputMode::Normal => {
        Line::from(vec![
          Span::raw("("),
          Span::styled("NORMAL", Style::default().fg(Color::LightBlue)),
          Span::raw(format!(") {}", self.input.string())),
        ])
      }

      InputMode::Insert => {
        Line::from(vec![
          Span::raw("("),
          Span::from("INPUT".light_green()),
          Span::raw(")"),
        ])
      }

      InputMode::Command => {
        Line::from(vec![
          Span::raw("("),
          Span::from("COMMAND".light_yellow()),
          Span::raw(format!(") :{}", self.command)),
        ])
      }
    };

    Paragraph::new(line)
      .left_aligned()
      .wrap(Wrap { trim: true })
      .render(area, buf);
  }
}
