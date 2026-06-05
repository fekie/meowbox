use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex,
};
use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcConfig, AdcPin, Attenuation},
    peripherals::{ADC2, GPIO17, GPIO18},
};

pub static THUMBWHEELS: Mutex<
    CriticalSectionRawMutex,
    Option<Thumbwheels>,
> = Mutex::new(None);

pub struct Thumbwheels {
    adc: Adc<'static, ADC2<'static>, Blocking>,
    left: AdcPin<GPIO17<'static>, ADC2<'static>>,
    right: AdcPin<GPIO18<'static>, ADC2<'static>>,
}

impl Thumbwheels {
    pub fn new(
        adc2: ADC2<'static>,
        left: GPIO17<'static>,
        right: GPIO18<'static>,
    ) -> Self {
        let mut adc_config = AdcConfig::new();
        let left = adc_config.enable_pin(left, Attenuation::_11dB);
        let right = adc_config.enable_pin(right, Attenuation::_11dB);
        let adc = Adc::new(adc2, adc_config);

        Self { adc, left, right }
    }

    pub fn left_raw(&mut self) -> u16 {
        self.adc.read_blocking(&mut self.left)
    }

    pub fn right_raw(&mut self) -> u16 {
        self.adc.read_blocking(&mut self.right)
    }

    pub fn raw_values(&mut self) -> ThumbwheelValues {
        ThumbwheelValues {
            left: self.left_raw(),
            right: self.right_raw(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbwheelValues {
    pub left: u16,
    pub right: u16,
}

pub struct ThumbwheelHandle;

impl ThumbwheelHandle {
    pub async fn left_raw() -> Option<u16> {
        THUMBWHEELS.lock().await.as_mut().map(Thumbwheels::left_raw)
    }

    pub async fn right_raw() -> Option<u16> {
        THUMBWHEELS
            .lock()
            .await
            .as_mut()
            .map(Thumbwheels::right_raw)
    }

    pub async fn raw_values() -> Option<ThumbwheelValues> {
        THUMBWHEELS
            .lock()
            .await
            .as_mut()
            .map(Thumbwheels::raw_values)
    }
}
