use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, Output};

#[task]
pub async fn left_button_event(
    button: &'static Mutex<CriticalSectionRawMutex, Option<Input<'static>>>,
    led: &'static Mutex<CriticalSectionRawMutex, Option<Output<'static>>>,
    buzzer: &'static Mutex<CriticalSectionRawMutex, Option<Output<'static>>>,
) {
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();

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
    button: &'static Mutex<CriticalSectionRawMutex, Option<Input<'static>>>,
    led: &'static Mutex<CriticalSectionRawMutex, Option<Output<'static>>>,
    buzzer: &'static Mutex<CriticalSectionRawMutex, Option<Output<'static>>>,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();

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
