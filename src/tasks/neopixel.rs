use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use esp_hal_smartled::SmartLedsAdapter;
use smart_leds::{RGB8, SmartLedsWrite};

pub static NEOPIXEL_CH: Channel<
    CriticalSectionRawMutex,
    NeopixelCommand,
    20,
> = Channel::new();

pub enum NeopixelCommand {
    ActivateWithHSV { hue: u16, brightness: u8 },
}

#[task]
pub async fn neopixel_command_listener(
    mut neopixel: SmartLedsAdapter<'static, 25>,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        let cmd = NEOPIXEL_CH.receive().await;

        match cmd {
            NeopixelCommand::ActivateWithHSV { hue, brightness } => {
                let rgb = hue_to_rgb(hue, brightness);

                neopixel.write([rgb]).unwrap();
            }
        }

        //button.lock().await.as_mut().unwrap().wait_for_low().await;
        //led.lock().await.as_mut().unwrap().set_low();

        //info!("right button triggered");
    }
}

/// Convert hue (0-360) and brightness (0-255)
/// to RGB.
pub fn hue_to_rgb(hue: u16, brightness: u8) -> RGB8 {
    // wrap hue to 0-359
    let hue = hue % 360;

    // convert hue into 0-255 range
    let hue_scaled = (hue * 255 / 360) as u8;

    let region = hue_scaled / 43; // 256 / 6 ~= 43
    let remainder = (hue_scaled - (region * 43)) * 6;

    let p = 0;
    let q =
        (brightness as u16 * (255 - remainder as u16) / 255) as u8;
    let t = (brightness as u16 * remainder as u16 / 255) as u8;

    match region {
        0 => RGB8 {
            r: brightness,
            g: t,
            b: p,
        },
        1 => RGB8 {
            r: q,
            g: brightness,
            b: p,
        },
        2 => RGB8 {
            r: p,
            g: brightness,
            b: t,
        },
        3 => RGB8 {
            r: p,
            g: q,
            b: brightness,
        },
        4 => RGB8 {
            r: t,
            g: p,
            b: brightness,
        },
        _ => RGB8 {
            r: brightness,
            g: p,
            b: q,
        },
    }
}
