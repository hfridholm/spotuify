/*  ____                             
 * / ___| _ __   __ ___      ___ __  
 * \___ \| '_ \ / _` \ \ /\ / / '_ \ 
 *  ___) | |_) | (_| |\ V  V /| | | |
 * |____/| .__/ \__,_| \_/\_/ |_| |_|
 *       |_|                         
 */

use crate::api::SpotifyApi;

use super::{Action, App};
use crate::app::AppWindow;

impl App {
  pub(crate) fn spawn_pause(&self) {
    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.pause(&player).await {
        let _ = tx.send(Action::Error(error.to_string()));
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_fetch_user(&self) {
    let spotify = self.spotify.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      match spotify.get_user().await {
        Ok(user) => {
          let _ = tx.send(
            Action::UserUpdated { user }
          );
        }
        Err(err) => {
          let _ = tx.send(
            Action::Error(err.to_string())
          );
          return;
        }
      }
    });
  }

  pub(crate) fn spawn_show_search_album(&mut self) {
    let Some(index) =
      self.table_state.selected()
    else {
      return;
    };

    if let Some(album) = self.search.albums.items.get(index).cloned() {
      self.album = Some(album.clone());
      self.set_window(AppWindow::Album);

      let spotify = self.spotify.clone();
      let tx      = self.action_tx.clone();

      tokio::spawn(async move {
        match album.get_tracks(&spotify).await {
          Ok(tracks) => {
            let _ = tx.send(
              Action::AlbumTracksUpdated { tracks },
            );
          }

          Err(err) => {
            let _ = tx.send(
              Action::Error(err.to_string()),
            );
          }
        }
      });
    }
  } 

  pub(crate) fn spawn_initialize(&self) {
    let spotify = self.spotify.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      // 1. Initialize the API if needed.
      let spotify = if spotify.access_token.is_empty() {
        match SpotifyApi::new().await {
          Ok(api) => {
            let _ = tx.send(Action::ApiUpdated {
              api: api.clone(),
            });

            api
          }
          Err(err) => {
            let _ = tx.send(Action::Error(err.to_string()));
            return;
          }
        }
      } else {
        spotify
      };

      // 2. Fetch player, queue and user concurrently.
      let tx_player = tx.clone();
      let tx_queue = tx.clone();
      let tx_user = tx.clone();

      let player_task = async {
        match spotify.get_player().await {
          Ok(player) => {
            let _ = tx_player.send(Action::PlayerUpdated { player });
          }
          Err(err) => {
            let _ = tx_player.send(Action::Error(err.to_string()));
          }
        }
      };

      let queue_task = async {
        match spotify.get_queue().await {
          Ok(queue) => {
            let _ = tx_queue.send(Action::QueueUpdated { queue });
          }
          Err(err) => {
            let _ = tx_queue.send(Action::Error(err.to_string()));
          }
        }
      };

      let user_task = async {
        match spotify.get_user().await {
          Ok(user) => {
            let _ = tx_user.send(Action::UserUpdated { user });
          }
          Err(err) => {
            let _ = tx_user.send(Action::Error(err.to_string()));
          }
        }
      };

      tokio::join!(player_task, queue_task, user_task);
    });
  }

  pub(crate) fn spawn_refresh(&self) {
    let spotify = self.spotify.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      // 1. Initialize the API if needed.
      let spotify = if spotify.access_token.is_empty() {
        match SpotifyApi::new().await {
          Ok(api) => {
            let _ = tx.send(Action::ApiUpdated {
              api: api.clone(),
            });

            api
          }
          Err(err) => {
            let _ = tx.send(Action::Error(err.to_string()));
            return;
          }
        }
      } else {
        spotify
      };

      // 2. Fetch player and queue concurrently.
      let tx_player = tx.clone();
      let tx_queue = tx.clone();

      let player_task = async {
        match spotify.get_player().await {
          Ok(player) => {
            let _ = tx_player.send(Action::PlayerUpdated { player });
          }
          Err(err) => {
            let _ = tx_player.send(Action::Error(err.to_string()));
          }
        }
      };

      let queue_task = async {
        match spotify.get_queue().await {
          Ok(queue) => {
            let _ = tx_queue.send(Action::QueueUpdated { queue });
          }
          Err(err) => {
            let _ = tx_queue.send(Action::Error(err.to_string()));
          }
        }
      };

      tokio::join!(player_task, queue_task);
    });
  }

  pub(crate) fn spawn_resume(&self) {
    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.resume(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_next(&self) {
    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.next(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_previous(&self) {
    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.previous(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_volume_up(&self) {
    let volume = self.get_volume_percent()
      .unwrap_or(0)
      .saturating_add(10)
      .min(100);

    self.spawn_volume(volume);
  }

  pub(crate) fn spawn_volume_down(&self) {
    let volume = self.get_volume_percent()
      .unwrap_or(0)
      .saturating_sub(10);

    self.spawn_volume(volume);
  }

  pub(crate) fn spawn_mute(&self) {
    self.spawn_volume(0);
  }

  pub(crate) fn spawn_volume(&self, volume: u8) {
    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.volume(&player, volume).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_shuffle(&self) {
    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.shuffle(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_repeat(&self) {
    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.repeat(&player).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_play_queue(&self) {
    let Some(index) =
      self.queue_state.selected()
    else {
      return;
    };

    let Some(track) = self.queue
      .as_ref()
      .and_then(|queue| {
        queue.queue.get(index)
      })
    else {
      return;
    };

    let track_uri = track.uri.clone();
    let album_uri = track.album.uri.clone();

    let _context_uri = self.get_context_uri()
      .unwrap_or_default();

    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.play(
        &player,
        &track_uri,
        &album_uri,
      ).await
      {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_play_history(&self) {
    let Some(track) = self.history
      .as_ref()
      .and_then(|history| {
        history.items
          .iter()
          .filter_map(|item| {
            item.track.as_ref()
          })
        .nth(self.list_index)
      })
    else {
      return;
    };

    let track_uri = track.uri.clone();
    let album_uri = track.album.uri.clone();

    let _context_uri = self.get_context_uri()
      .unwrap_or_default();

    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.play(
        &player,
        &track_uri,
        &album_uri,
      ).await
      {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_play_search(&self) {
    let Some(index) =
      self.table_state.selected()
    else {
      return;
    };

    let track = match &self.search.tracks.items.get(index) {
      Some(track) => *track,
      None => return,
    };

    let track_uri = track.uri.clone();
    let album_uri = track.album.uri.clone();

    let _context_uri = self.get_context_uri()
      .unwrap_or_default();

    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    // Create Action::Message
    let _ = tx.send(
      Action::Message(format!("Playing: {}\nTrack Uri: {}\nAlbum Uri: {}", track.name.to_string(), track.uri.to_string(), track.album.name.to_string()))
    );

    tokio::spawn(async move {
      if let Err(error) = spotify.play(
        &player,
        &track_uri,
        &album_uri,
      ).await
      {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_queue_history(&self) {
    let Some(track) = self.history
      .as_ref()
      .and_then(|history| {
        history
          .items
          .iter()
          .filter_map(|item| {
            item.track.as_ref()
          })
        .nth(self.list_index)
      })
    else {
      return;
    };

    let track_uri = track.uri.clone();

    let spotify = self.spotify.clone();
    let player = self.player.clone();
    let tx = self.action_tx.clone();

    tokio::spawn(async move {
      if let Err(error) = spotify.queue(&player, &track_uri).await {
        let _ = tx.send(
          Action::Error(error.to_string()),
        );
        return;
      }

      let _ = tx.send(Action::Refresh);
    });
  }

  pub(crate) fn spawn_search(&self) {
    let spotify = self.spotify.clone();
    let tx = self.action_tx.clone();
    let query = self.search_buffer.clone();

    tokio::spawn(async move {
      match spotify.get_search(&query).await {
        Ok(search) => {
          let _ = tx.send(
            Action::SearchUpdated { search },
          );
        }

        Err(err) => {
          let _ = tx.send(
            Action::Error(err.to_string()),
          );
        }
      }
    });
  }
}
