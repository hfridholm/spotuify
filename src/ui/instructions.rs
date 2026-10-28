/*  ___           _                   _   _                 
 * |_ _|_ __  ___| |_ _ __ _   _  ___| |_(_) ___  _ __  ___ 
 *  | || '_ \/ __| __| '__| | | |/ __| __| |/ _ \| '_ \/ __|
 *  | || | | \__ \ |_| |  | |_| | (__| |_| | (_) | | | \__ \
 * |___|_| |_|___/\__|_|   \__,_|\___|\__|_|\___/|_| |_|___/
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
  layout::Rect,
  style::Stylize,
  text::Line,
  widgets::{
    Paragraph,
    Widget,
    Wrap
  },
};

impl App {
  pub(crate) fn render_instructions(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let instructions = match self.focus {
      AppFocus::Player => {
        Line::from(
          "_ Pause/Resume | ↵ Refresh | h/← Previous | l/→ Next | ↑/↓ Volume | m Mute | s Shuffle | r Repeat",
        )
      }

      AppFocus::Queue => {
        Line::from(
          "↵ Play | ↑/k Up | ↓/j Down | Home/End Jump | ⌦ Remove",
        )
      }

      AppFocus::HomeHistory => {
        Line::from(
          "↵ Play | h/← Queue | ↑/k Up | ↓/j Down | Home/End Jump",
        )
      }

      AppFocus::Home => {
        Line::from(
          "↑/k Up | ↓/j Down | Home/End Jump",
        )
      }

      AppFocus::Search => {
        Line::from(
          "↵ Search",
        )
      }

      AppFocus::SearchTrack => {
        Line::from(
          "↵ Play | ↑/k Up | ↓/j Down | Home/End Jump",
        )
      }
      _ => Line::from(""),
    }
      .blue();

    Paragraph::new(instructions)
      .centered()
      .wrap(Wrap { trim: true })
      .render(area, buf);
  }
}
