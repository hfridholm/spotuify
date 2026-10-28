/*     _         _   _     _   
 *    / \   _ __| |_(_)___| |_ 
 *   / _ \ | '__| __| / __| __|
 *  / ___ \| |  | |_| \__ \ |_ 
 * /_/   \_\_|   \__|_|___/\__|
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
  style::{
    Color, 
    Style,
    Stylize
  },
  text::Line,
  widgets::{
    Block,
    BorderType,
    Widget
  },
};

impl App {
  pub(crate) fn render_artist(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let in_focus = self.focus.is(AppFocus::Artist);

    let Some(artist) = &self.artist else {
      return;
    };

    let block = Block::bordered()
      .title(
        Line::from(format!(" {} ", artist.name).bold())
        .left_aligned()
        .style(Style::default()
          .fg(HEADER_COLOR.get(in_focus, in_focus))
        )
      )
      .border_type(BORDER_TYPE.get(in_focus))
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus, in_focus))
      )
      .render(area, buf);
  }
}
