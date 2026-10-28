/*  _   _       _   _  __ _           _   _                 
 * | \ | | ___ | |_(_)/ _(_) ___ __ _| |_(_) ___  _ __  ___ 
 * |  \| |/ _ \| __| | |_| |/ __/ _` | __| |/ _ \| '_ \/ __|
 * | |\  | (_) | |_| |  _| | (_| (_| | |_| | (_) | | | \__ \
 * |_| \_|\___/ \__|_|_| |_|\___\__,_|\__|_|\___/|_| |_|___/
 */

use super::App;
use crate::app::AppNotification;

use ratatui::{
  buffer::Buffer,
  layout::Rect,
  style::{
    Color,
    Style, 
    Stylize
  },
  text::{
    Line,
    Text
  },
  widgets::{
    Block,
    BorderType,
    Paragraph,
    Widget,
    Clear,
    Wrap
  },
};

impl App {
  pub(crate) fn render_notifications(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let mut y = area.y;

    for (index, notification) in self.notifications.iter().enumerate() {
      let text_width = notification.text.len() as u16;

      // +2 for left/right borders +2 padding
      let width = (text_width + 4).min(area.width);

      // Width available for text inside the borders.
      let content_width = width.saturating_sub(2).max(1);

      // Number of lines required when wrapped.
      let lines = text_width.div_ceil(content_width);

      // +3 for top/bottom borders + "Error"
      let height = (lines + 3).min(area.height.saturating_sub(y - area.y));

      let rect = Rect {
        x: area.x + area.width - width,
        y,
        width,
        height,
      };

      // Clears rect from any previously rendered text
      Clear.render(rect, buf);

      let block = if notification.is_error {
        Block::bordered()
        .border_type(BorderType::Plain)
        .border_style(
          Style::default()
          .fg(Color::Red)
        )
      } else {
        Block::bordered()
        .border_type(BorderType::Plain)
        .border_style(
          Style::default()
          .fg(Color::Green)
        )
      };

      if notification.is_error {
        Paragraph::new(Text::from(vec![
          Line::from("Error".to_string().red()),
          Line::from(notification.text.as_str().bold().white()),
        ]))
      } else {
        Paragraph::new(Text::from(vec![
          Line::from("Info".to_string().green()),
          Line::from(notification.text.as_str().bold().white()),
        ]))
      }
        .centered()
        .wrap(Wrap { trim: false })
        .block(block)
        .render(rect, buf);

      y += height;
    }
  }
}
