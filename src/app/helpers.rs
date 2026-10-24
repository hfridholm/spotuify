/*  _   _      _                     
 * | | | | ___| |_ __   ___ _ __ ___ 
 * | |_| |/ _ \ | '_ \ / _ \ '__/ __|
 * |  _  |  __/ | |_) |  __/ |  \__ \
 * |_| |_|\___|_| .__/ \___|_|  |___/
 *              |_|           
 */

use crate::app::App;
use crate::error::Error;

impl App {
  pub(crate) fn get_volume_percent(&self) -> Result<u8, Error> {
    self.player.device
      .as_ref()
      .map(|device| device.volume_percent)
      .ok_or_else(|| {
        "No Spotify device found".into()
      })
  }

  pub(crate) fn get_context_uri(&self) -> Result<String, Error> {
    self.player.context
      .as_ref()
      .map(|context| context.uri.clone())
      .ok_or_else(|| {
        "No track context".into()
      })
  }
}
