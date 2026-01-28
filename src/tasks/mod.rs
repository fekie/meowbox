use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Timer};

use super::hardware;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub static BUZZER_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerSequence> = Signal::new();

pub enum BuzzerSequence {
    SimpleTone200ms,
    Intermittent10ms200ms,
    Intermittent5ms2000ms,
}

#[task]
pub async fn left_button_event(
    button: &'static hardware::ButtonType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
) {
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();

        info!("left button triggered");

        // wait 200ms
        for _ in 0..20 {
            buzzer.lock().await.as_mut().unwrap().toggle();
            Timer::after(Duration::from_millis(10)).await;
        }

        //Timer::after(Duration::from_millis(200)).await;
        buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn right_button_event(
    button: &'static hardware::ButtonType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();

        info!("right button triggered");

        // wait 200ms and alternate buzzer
        for _ in 0..200 {
            buzzer.lock().await.as_mut().unwrap().toggle();
            Timer::after(Duration::from_millis(1)).await;
        }

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn rotary_switch_left_event(
    rotary_switch: &'static hardware::RotarySwitchType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
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
        led.lock().await.as_mut().unwrap().set_low();

        Timer::after(Duration::from_millis(200)).await;

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        //buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn rotary_switch_right_event(
    rotary_switch: &'static hardware::RotarySwitchType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
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
        led.lock().await.as_mut().unwrap().set_low();

        // play simple tone
        BUZZER_SIGNAL.signal(BuzzerSequence::SimpleTone200ms);

        Timer::after(Duration::from_millis(200)).await;

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        //buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn play_sequence_listener(buzzer: &'static hardware::BuzzerType) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        let sequence = BUZZER_SIGNAL.wait().await;
        execute_sequence(buzzer, &sequence).await;
        // execute sequency here
    }
}

async fn execute_sequence(buzzer: &'static hardware::BuzzerType, sequence: &BuzzerSequence) {
    match sequence {
        BuzzerSequence::SimpleTone200ms => {
            buzzer.lock().await.as_mut().unwrap().set_high();
            Timer::after(Duration::from_millis(200)).await;

            buzzer.lock().await.as_mut().unwrap().set_low();
        }
        BuzzerSequence::Intermittent10ms200ms => {
            for _ in 0..20 {
                buzzer.lock().await.as_mut().unwrap().toggle();
                Timer::after(Duration::from_millis(10)).await;
            }

            buzzer.lock().await.as_mut().unwrap().set_low();
        }
        BuzzerSequence::Intermittent5ms2000ms => {
            for _ in 0..400 {
                buzzer.lock().await.as_mut().unwrap().toggle();
                Timer::after(Duration::from_millis(5)).await;
            }

            buzzer.lock().await.as_mut().unwrap().set_low();
        }
    }
}
