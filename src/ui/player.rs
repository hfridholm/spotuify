/*  ____  _                       
 * |  _ \| | __ _ _   _  ___ _ __ 
 * | |_) | |/ _` | | | |/ _ \ '__|
 * |  __/| | (_| | |_| |  __/ |   
 * |_|   |_|\__,_|\__, |\___|_|   
 *                |___/           
 */

use super::{
  App,
  AppFocus,
};

use ratatui::{
  buffer::Buffer,
  text::{
    Line,
    Text
  },
  widgets::{
    BorderType,
    LineGauge,
    Paragraph,
    Widget
  },
  style::{
    Color,
    Style,
    Stylize,
  },
  layout::{
    Constraint,
    Layout,
    Rect,
  },
};

impl App {
  pub(crate) fn render_player(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (color, border_type) = if app_focus == AppFocus::Player {
      (Color::Green, BorderType::Thick)
    } else {
      (Color::White, BorderType::Plain)
    };

    let block = Self::panel(" Player ", color, border_type);

    block.clone().render(area, buf);

    let inner = block.inner(area);

    let [text_area, gauge_area] = Layout::vertical([
      Constraint::Min(1),
      Constraint::Length(1),
    ])
      .areas(inner);

    let text = match &self.player.item {
      Some(track) => {
        let artists =
          track
          .artists
          .iter()
          .map(|artist| {
            artist.name.as_str()
          })
        .collect::<Vec<_>>()
          .join(", ");

        Text::from(vec![
          Line::from(track.name.as_str().bold()),
          Line::from(artists),
        ])
      }

      None => {
        Text::from("No track playing")
      }
    };

    let volume_percent = self.get_volume_percent()
      .unwrap_or(0);

    Paragraph::new(format!("{volume_percent}%"))
      .right_aligned()
      .render(text_area, buf);

    Paragraph::new(text)
      .centered()
      .render(text_area, buf);

    let ratio = if let Ok(duration_ms) = self.player.get_duration() {
      if duration_ms == 0 {
        0.0
      } else {
        (self.player.progress_ms.min(duration_ms)) as f64 / duration_ms as f64
      }
    } else {
      0.0
    };

    let gauge = LineGauge::default()
      .filled_style(
        Style::default()
        .fg(color),
      )
      .unfilled_style(
        Style::default()
        .fg(Color::DarkGray),
      )
      .ratio(ratio);

    gauge.render(gauge_area, buf);
  }
}
