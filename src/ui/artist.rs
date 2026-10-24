/*     _         _   _     _   
 *    / \   _ __| |_(_)___| |_ 
 *   / _ \ | '__| __| / __| __|
 *  / ___ \| |  | |_| \__ \ |_ 
 * /_/   \_\_|   \__|_|___/\__|
 */

use crate::app::App;
use crate::ui::AppFocus;

use ratatui::{
    buffer::Buffer,
    layout::{Rect},
    style::{Color, Style, Stylize},
    text::{Line},
    widgets::{Block, BorderType, Widget},
};

impl App {
  pub(crate) fn render_artist(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let artist = match &self.artist {
      Some(artist) => artist,
      None => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::Artist {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let block = Block::bordered()
      .title(
        Line::from(format!(" {} ", artist.name).bold())
        .left_aligned()
        .style(Style::default().fg(header_color))
      )
      .border_type(border_type)
      .border_style(
        Style::default()
        .fg(block_color)
      )
      .render(area, buf);
  }
}
