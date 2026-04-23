#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_time::{Duration, Timer};

use super::Meowbox;
use crate::states::State;

impl Meowbox {
    pub(super) async fn tick_error_state(&mut self) {
        if let State::ErrorState(etype) = self.state {
            //RED_LED.lock().await.as_mut().unwrap().toggle();
            error!("error state {:?}", etype);
            Timer::after(Duration::from_millis(200)).await;
        }
    }
}
