/*  _   _       _   _  __ _           _   _                 
 * | \ | | ___ | |_(_)/ _(_) ___ __ _| |_(_) ___  _ __  ___ 
 * |  \| |/ _ \| __| | |_| |/ __/ _` | __| |/ _ \| '_ \/ __|
 * | |\  | (_) | |_| |  _| | (_| (_| | |_| | (_) | | | \__ \
 * |_| \_|\___/ \__|_|_| |_|\___\__,_|\__|_|\___/|_| |_|___/
 */

use crate::app::App;

use ratatui::{
    buffer::Buffer,
    layout::{Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Paragraph, Widget, Clear, Wrap},
};

impl App {
  pub(crate) fn render_error_and_message(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let error_list = match self.error_list.lock() {
      Ok(error_list) => error_list,
      Err(_) => return,
    };

    let message_list = match self.message_list.lock() {
      Ok(message_list) => message_list,
      Err(_) => return,
    };

    #[derive(PartialEq)]
    enum InfoType {
      Error,
      Message,
    }

    let mut info_list: Vec<(InfoType, String)> = Vec::new();

    info_list.extend(
      error_list.iter().map(|(_, s)| (InfoType::Error, s.clone()))
    );

    info_list.extend(
      message_list.iter().map(|(_, s)| (InfoType::Message, s.clone()))
    );

    let mut y = area.y;

    for (index, (info_type, text)) in info_list.iter().enumerate() {
      let is_error = *info_type == InfoType::Error;

      let text_width = text.len() as u16;

      // +2 for left/right borders +2 padding
      let width = (text_width + 4).min(area.width);

      // Width available for text inside the borders.
      let content_width = width.saturating_sub(2).max(1);

      // Number of lines required when wrapped.
      let lines = text_width.div_ceil(content_width);

      // +3 for top/bottom borders + "Error"
      let height_add = if is_error { 3 } else { 2 };
      let height = (lines + height_add).min(area.height.saturating_sub(y - area.y));

      let rect = Rect {
        x: area.x + area.width - width,
        y,
        width,
        height,
      };

      // Clears rect from any previously rendered text
      Clear.render(rect, buf);

      let block = if is_error {
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
          .fg(Color::White)
        )
      };

      if is_error {
        Paragraph::new(Text::from(vec![
          Line::from("Error".to_string().red()),
          Line::from(text.as_str().bold().white()),
        ]))
      } else {
        Paragraph::new(Text::from(vec![
          Line::from(text.as_str().bold().white()),
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
