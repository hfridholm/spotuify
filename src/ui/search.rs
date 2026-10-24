/*  ____                      _     
 * / ___|  ___  __ _ _ __ ___| |__  
 * \___ \ / _ \/ _` | '__/ __| '_ \ 
 *  ___) |  __/ (_| | | | (__| | | |
 * |____/ \___|\__,_|_|  \___|_| |_|
 */

use crate::app::App;
use crate::ui::AppFocus;

use ratatui::{
  buffer::Buffer,
  text::{Line},
  widgets::{Block, BorderType, Paragraph, Widget, Table, Row, Cell, Tabs, StatefulWidget},
  symbols::{
    merge::MergeStrategy,
  },
  style::{
    Color,
    Style,
    Stylize,
    Modifier,
  },
  layout::{
    Constraint,
    Layout,
    Rect,
    Spacing,
  },
};

impl App {
  pub(crate) fn render_search(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::Search {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let [header_area, main_area] = Layout::vertical([
      Constraint::Length(3),
      Constraint::Min(1),
    ])
      .spacing(Spacing::Overlap(1))
      .areas(area);

    let header_block = Block::bordered()
      .title(
        Line::from(format!(" Search ").bold())
        .left_aligned(),
      )
      .merge_borders(MergeStrategy::Exact)
      .border_type(border_type)
      .border_style(
        Style::default()
        .fg(block_color),
      );

    let input_area = header_block.inner(header_area);

    Paragraph::new(
      Line::from(format!("{}", self.search_buffer))
    )
      .style(Style::default()
        .bg(Color::Gray)
        .fg(Color::Black)
      )
      .left_aligned()
      .render(input_area, buf);

    // If focus is the main area, render header border on bottom
    if app_focus != AppFocus::Search {
      header_block.clone().render(header_area, buf);
    }

    match self.tab_index {
      0 => Self::render_search_tracks(self, main_area, buf),
      1 => Self::render_search_artists(self, main_area, buf),
      2 => Self::render_search_albums(self, main_area, buf),
      3 => Self::render_search_playlists(self, main_area, buf),
      _ => {},
    }
    
    // If focus is the header, render header border on top
    if app_focus == AppFocus::Search {
      header_block.clone().render(header_area, buf);
    }

    let rect = Rect {
      x: header_area.x,
      y: header_area.y + header_area.height - 1,
      width: header_area.width,
      height: 1,
    };

    Self::render_search_types(self, rect, buf);
  }

  fn render_search_types(
    &mut self,
    rect: Rect,
    buf: &mut Buffer,
  ) {
    const TABS: [(AppFocus, &str); 4] = [
      (AppFocus::SearchTrack,    "Track"), 
      (AppFocus::SearchArtist,   "Artist"),
      (AppFocus::SearchAlbum,    "Album"),
      (AppFocus::SearchPlaylist, "Playlist"),
    ];

    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let selected = TABS.iter().position(|(focus, _)| *focus == app_focus);

    let width: u16 = TABS.iter().map(|t| t.1.len() as u16).sum::<u16>()
      + (TABS.len() as u16 - 1) * 2 + 2;

    let centered = Layout::horizontal([
      Constraint::Fill(1),
      Constraint::Length(width),
      Constraint::Fill(1),
    ])
      .split(rect);

    // Creates tabs from every string TABS[i].1
    let strings: [&_; TABS.len()] = std::array::from_fn(|i| TABS[i].1);

    Tabs::new(strings)
      .select(selected)
      .style(
        Style::default()
        .fg(Color::Gray)
      )
      .highlight_style(
        Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD),
      )
      .divider("")
      .render(centered[1], buf);
  }

  fn render_search_tracks(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::SearchTrack {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("NAME"),
      Cell::from("ARTISTS"),
    ])
      .style(
        Style::default()
        .fg(header_color),
      )
      .bold();

    let rows = if !self.search.tracks.items.is_empty() {
      self.search.tracks.items
        .iter()
        .enumerate()
        .map(|(index, track)| {
          Row::new([
            Cell::from(format!("{:>2}", index + 1)),
            Cell::from(track.name.as_str()),
            Cell::from(track.artists
              .iter()
              .map(|artist| artist.name.as_str())
              .collect::<Vec<_>>()
              .join(", ")
            )
          ])
        },
      )
        .collect::<Vec<_>>()
    } else {
      vec![
        Row::new([
          Cell::from(""),
          Cell::from("No tracks"),
          Cell::from(""),
        ])
      ]
    };

    let block = Block::bordered()
      .border_type(border_type)
      .merge_borders(MergeStrategy::Exact)
      .border_style(
        Style::default()
        .fg(block_color),
      );

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
      Constraint::Fill(1),
    ])
      .header(header)
      .column_spacing(2)
      .style(
        Style::default()
        .fg(Color::White)
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray)
        .fg(Color::Black)
      )
      .block(block);

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.table_state,
    );
  }

  fn render_search_albums(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::SearchAlbum {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("NAME"),
    ])
      .style(
        Style::default()
        .fg(header_color),
      )
      .bold();

    let rows = if !self.search.albums.items.is_empty() {
      self.search.albums.items
        .iter()
        .enumerate()
        .map(|(index, album)| {
          Row::new([
            Cell::from(format!("{:>2}", index + 1)),
            Cell::from(album.name.as_str()),
          ])
        },
      )
        .collect::<Vec<_>>()
    } else {
      vec![
        Row::new([
          Cell::from(""),
          Cell::from("No albums"),
        ])
      ]
    };

    let block = Block::bordered()
      .border_type(border_type)
      .merge_borders(MergeStrategy::Exact)
      .border_style(
        Style::default()
        .fg(block_color),
      );

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
    ])
      .header(header)
      .column_spacing(2)
      .style(
        Style::default()
        .fg(Color::White),
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray),
      )
      .block(block);

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.table_state,
    );
  }

  fn render_search_artists(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::SearchArtist {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("NAME"),
    ])
      .style(
        Style::default()
        .fg(header_color),
      )
      .bold();

    let rows = if !self.search.artists.items.is_empty() {
      self.search.artists.items
        .iter()
        .enumerate()
        .map(|(index, artist)| {
          Row::new([
            Cell::from(format!("{:>2}", index + 1)),
            Cell::from(artist.name.as_str()),
          ])
        },
      )
        .collect::<Vec<_>>()
    } else {
      vec![
        Row::new([
          Cell::from(""),
          Cell::from("No artists"),
        ])
      ]
    };

    let block = Block::bordered()
      .border_type(border_type)
      .merge_borders(MergeStrategy::Exact)
      .border_style(
        Style::default()
        .fg(block_color),
      );

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
    ])
      .header(header)
      .column_spacing(2)
      .style(
        Style::default()
        .fg(Color::White),
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray),
      )
      .block(block);

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.table_state,
    );
  }

  fn render_search_playlists(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let app_focus = match self.focus.lock() {
      Ok(app_focus) => *app_focus,
      Err(_) => return,
    };

    let (block_color, header_color, border_type) = if app_focus == AppFocus::SearchPlaylist {
      (Color::Green, Color::Yellow, BorderType::Thick)
    } else {
      (Color::White, Color::White, BorderType::Plain)
    };

    let header = Row::new([
      Cell::from(" #"),
      Cell::from("NAME"),
    ])
      .style(
        Style::default()
        .fg(header_color),
      )
      .bold();

    let rows = if !self.search.playlists.items.is_empty() {
      self.search.playlists.items
        .iter()
        .enumerate()
        .filter_map(|(index, playlist)| {
          let playlist = playlist.as_ref()?;

          Some(Row::new([
              Cell::from(format!("{:>2}", index + 1)),
              Cell::from(playlist.name.as_str()),
          ]))
        }
      )
        .collect::<Vec<_>>()
    } else {
      vec![
        Row::new([
          Cell::from(""),
          Cell::from("No playlists"),
        ])
      ]
    };

    let block = Block::bordered()
      .border_type(border_type)
      .merge_borders(MergeStrategy::Exact)
      .border_style(
        Style::default()
        .fg(block_color),
      );

    let table = Table::new(rows, [
      Constraint::Length(2),
      Constraint::Min(10),
    ])
      .header(header)
      .column_spacing(2)
      .style(
        Style::default()
        .fg(Color::White),
      )
      .row_highlight_style(
        Style::default()
        .bg(Color::Gray),
      )
      .block(block);

    StatefulWidget::render(
      table,
      area,
      buf,
      &mut self.table_state,
    );
  }
}
