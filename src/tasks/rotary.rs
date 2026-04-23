use defmt::println;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;
use esp_println::dbg;
use rotary_encoder_embedded::{
    Direction, quadrature::QuadratureTableMode,
};

use super::{
    BUZZER_SIGNAL, BuzzerSequence, LED_ROTATION_SIGNAL,
    LEDRotationParams, hardware,
};
use crate::{
    //hardware::{BLUE_LED, GREEN_LED, RED_LED, YELLOW_LED},
    leds::LightRing,
    menu::MenuStatusHandle,
    tasks::neopixel::{NEOPIXEL_CH, NeoPixelHandle, NeopixelCommand},
};

#[task]
pub async fn rotary_switch_left_event(
    rotary_switch: &'static hardware::RotarySwitchType,
    buzzer: &'static hardware::BuzzerType,
    //led: &'static hardware::ButtonLEDType,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        rotary_switch
            .lock()
            .await
            .as_mut()
            .unwrap()
            .wait_for_falling_edge()
            .await;

        //let params = LEDRotationParams::default();
        //LED_ROTATION_SIGNAL.signal(params);

        buzzer.lock().await.as_mut().unwrap().set_high();

        println!("wah");

        Timer::after(Duration::from_millis(100)).await;

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        //buzzer.lock().await.as_mut().unwrap().set_low();
        buzzer.lock().await.as_mut().unwrap().set_low();
    }
}

/// meow
#[task]
pub async fn rotary_switch_right_event(
    rotary_switch: &'static hardware::RotarySwitchType,
    buzzer: &'static hardware::BuzzerType,
) {
    let neopixel_handle = NeoPixelHandle::new();

    let mut hue = 0;
    let brightness = 20;

    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        rotary_switch
            .lock()
            .await
            .as_mut()
            .unwrap()
            .wait_for_falling_edge()
            .await;

        buzzer.lock().await.as_mut().unwrap().set_high();

        //neopixel_handle.cycle_all_hues(3).await;

        // play simple tone
        //BUZZER_SIGNAL.signal(BuzzerSequence::SimpleTone200ms);

        //println!("{}", hue);

        // NEOPIXEL_CH
        //     .send(NeopixelCommand::ActivateWithHSV {
        //         hue,
        //         brightness,
        //     })
        //     .await;

        // hue = hue.wrapping_add(10);

        Timer::after(Duration::from_millis(2000)).await;

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        //buzzer.lock().await.as_mut().unwrap().set_low();
        buzzer.lock().await.as_mut().unwrap().set_low();
    }
}

#[task]
pub async fn left_rotary_rotation_watcher(
    left_rotary_a: Input<'static>,
    left_rotary_b: Input<'static>,
) {
    //let mut light_ring = LightRing::new().await;

    // start an encoder that we set the values of manually

    let mut raw_encoder = QuadratureTableMode::new(3);
    let _dir = raw_encoder.update(false, false);

    loop {
        // whenever this happens, update the state of the encoder
        let dir = raw_encoder
            .update(left_rotary_b.is_low(), left_rotary_a.is_low());

        match dir {
            Direction::Clockwise => {
                // YELLOW_LED.lock().await.as_mut().unwrap().
                // set_high(); RED_LED.lock().await.
                // as_mut().unwrap().set_low();
                //light_ring.forward().await;
                //menu_scroll_down();
            }
            Direction::Anticlockwise => {
                // RED_LED.lock().await.as_mut().unwrap().set_high();
                // YELLOW_LED.lock().await.as_mut().unwrap().
                // set_low();
                //light_ring.backward().await;
                //menu_scroll_up();
            }
            Direction::None => {}
        }

        Timer::after(Duration::from_micros(1000)).await; // 1 kHz
    }
}

#[task]
pub async fn right_rotary_rotation_watcher(
    right_rotary_a: Input<'static>,
    right_rotary_b: Input<'static>,
) {
    let neopixel_handle = NeoPixelHandle::new();

    // start an encoder that we set the values of manually
    // this used to be 4, but it would sometimes miss count.
    // even still, any value of this usually overcounts
    let mut raw_encoder = QuadratureTableMode::new(3);
    let _dir = raw_encoder.update(false, false);

    loop {
        // whenever this happens, update the state of the encoder
        let dir = raw_encoder
            .update(right_rotary_b.is_low(), right_rotary_a.is_low());

        match dir {
            Direction::Clockwise => {
                // BLUE_LED.lock().await.as_mut().unwrap().set_high();
                // GREEN_LED.lock().await.as_mut().unwrap().set_low();

                info!("clockwise");
                neopixel_handle.increment_neopixel_hue(5).await;
                //Timer::after(Duration::from_millis(200)).await;

                // Increment some value
            }
            Direction::Anticlockwise => {
                // GREEN_LED.lock().await.as_mut().unwrap().
                // set_high(); BLUE_LED.lock().await.
                // as_mut().unwrap().set_low();
                // Timer::after(Duration::from_millis(200)).await;

                info!("counterclockwise");
                neopixel_handle.increment_neopixel_hue(-5).await;

                // Decrement some value
            }
            Direction::None => {
                //info!("nothing");
                // Do nothing
            }
        }

        Timer::after(Duration::from_micros(1000)).await; // 1 kHz
    }
}

// Scrolls down the menu by 1 (which increments the scroll offset)
fn menu_scroll_down() {
    let menu_status_handle = MenuStatusHandle::new();

    let mut scroll = menu_status_handle.scroll();
    scroll = (scroll + 1) % menu_status_handle.current_layer_size();
    menu_status_handle.set_scroll(scroll);
    menu_status_handle.set_needs_update(true);
}

// Scrolls up the menu by 1 (which decrements the scroll offset)
fn menu_scroll_up() {
    let menu_status_handle = MenuStatusHandle::new();

    let mut scroll = menu_status_handle.scroll();
    if scroll == 0 {
        scroll = menu_status_handle.current_layer_size() - 1;
    } else {
        scroll -= 1;
    }
    menu_status_handle.set_scroll(scroll);
    menu_status_handle.set_needs_update(true);
}
