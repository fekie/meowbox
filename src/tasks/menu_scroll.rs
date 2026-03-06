use core::sync::atomic::{
    AtomicBool, AtomicU8, AtomicU16, AtomicUsize, Ordering::SeqCst,
};

use defmt::println;
use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use esp_println::dbg;

pub static MENU_SCROLL_CH: Channel<
    CriticalSectionRawMutex,
    MenuScrollCommand,
    5,
> = Channel::new();

pub enum MenuScrollCommand {
    Forwards,
    Backwards,
}

pub static CURRENT_MENU_SCROLL: AtomicUsize = AtomicUsize::new(0);

#[task]
pub async fn menu_scroll_command_listener() {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel

    loop {
        let cmd = MENU_SCROLL_CH.receive().await;

        match cmd {
            MenuScrollCommand::Forwards => {
                let mut scroll = CURRENT_MENU_SCROLL.load(SeqCst);

                scroll = (scroll + 1) % 3;

                dbg!(scroll);

                CURRENT_MENU_SCROLL.store(scroll, SeqCst);
            }
            MenuScrollCommand::Backwards => {
                let mut scroll = CURRENT_MENU_SCROLL.load(SeqCst);

                if scroll == 0 {
                    scroll = 3 - 1;
                }

                CURRENT_MENU_SCROLL.store(scroll, SeqCst);
            }
        }

        //button.lock().await.as_mut().unwrap().wait_for_low().await;
        //led.lock().await.as_mut().unwrap().set_low();

        //info!("right button triggered");
    }
}
