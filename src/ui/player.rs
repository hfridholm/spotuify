/*  ____  _                       
 * |  _ \| | __ _ _   _  ___ _ __ 
 * | |_) | |/ _` | | | |/ _ \ '__|
 * |  __/| | (_| | |_| |  __/ |   
 * |_|   |_|\__,_|\__, |\___|_|   
 *                |___/           
 */

/*
♪
⏮
⏭
▶
Ⅱ
×
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
  text::{
    Line,
    Text
  },
  symbols,
  widgets::{
    BorderType,
    Block,
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
    let in_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus == AppFocus::Player,
      Err(_) => return,
    };

    let block = Block::bordered()
      .title(
        Line::from(format!("  Player  ").bold())
        .left_aligned(),
      )
      .border_type(BORDER_TYPE.get(in_focus))
      .border_style(
        Style::default()
        .fg(BORDER_COLOR.get(in_focus, in_focus)),
      );

    block.clone().render(area, buf);

    let inner = block.inner(area);

    let [tracks_area, timeline_area, stats_area] = Layout::vertical([
      Constraint::Length(2),
      Constraint::Length(1),
      Constraint::Length(1),
    ])
      .areas(inner);

    self.render_player_tracks(tracks_area, buf);
    self.render_player_timeline(timeline_area, buf);
    self.render_player_stats(stats_area, buf);
  }

  fn render_player_stats(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let volume_percent = self.get_volume_percent()
      .unwrap_or(0);

    Paragraph::new(format!("♪ {volume_percent}%"))
      .render(area, buf);
  }

  fn render_player_tracks(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
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
          Line::from(track.name.as_str())
            .style(Style::default().fg(Color::White)),
          Line::from(artists)
            .style(Style::default().fg(Color::Gray)),
        ])
      }

      None => {
        Text::from("No track playing")
      }
    };

    // Render previous track
    // Render next track

    Paragraph::new(text)
      .centered()
      .render(area, buf);

  }

  fn render_player_timeline(
    &self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let Some(app_focus) = self.get_focus() else {
      return;
    };

    let in_focus = app_focus.is(AppFocus::Player);

    let color = if in_focus {
      Color::Green
    } else {
      Color::DarkGray
    };

    let [elapsed_area, gauge_area, duration_area] = Layout::horizontal([
      Constraint::Length(6),
      Constraint::Min(1),
      Constraint::Length(6),
    ])
      .areas(area);

    let duration_ms = self.player.get_duration().unwrap_or(0);
    let elapsed_ms  = self.player.progress_ms.min(duration_ms);

    let ratio = if duration_ms == 0 { 0.0 } else {
      elapsed_ms as f64 / duration_ms as f64
    };

    Paragraph::new(format!("{}", format_time(elapsed_ms as u64 / 1000)))
      .left_aligned()
      .render(elapsed_area, buf);

    Paragraph::new(format!("{}", format_time(duration_ms as u64 / 1000)))
      .right_aligned()
      .render(duration_area, buf);

    LineGauge::default()
      .ratio(ratio)
      .label("")
      .filled_style(
        Style::default()
        .fg(color),
      )
      .filled_symbol(symbols::shade::FULL)
      .unfilled_symbol(symbols::shade::LIGHT)
      .unfilled_style(
        Style::default()
        .fg(Color::DarkGray),
      )
      .render(gauge_area, buf);
  }
}

fn format_time(seconds: u64) -> String {
  format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
