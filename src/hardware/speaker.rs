use esp_hal::{
    Blocking,
    gpio::{
        Level, Output, OutputConfig, OutputSignal,
        interconnect::PeripheralOutput as _,
    },
    i2s::master::{Config, DataFormat, I2s},
    peripherals::{DMA_CH0, GPIO37, GPIO38, GPIO39, GPIO40, I2S0},
    time::Rate,
};

// The 'a lifetime is used because the I2s interface the function
// returns can only last for as long as the gpio interfaces do.
/// Intiailize the i2s speaker.
pub(super) fn init<'a>(
    i2s0: I2S0<'a>,
    dma: DMA_CH0<'a>,
    gpio37: GPIO37<'a>,
    gpio38: GPIO38<'a>,
    gpio39: GPIO39<'a>,
    gpio40: GPIO40<'a>,
) -> I2s<'a, Blocking> {
    // for some really weird reason, sd on this chip stands for
    // shutdown, and not serial data (which is what the i2s
    // protocol uses, but this chip calls it din)
    //
    // enable the amplifier by setting the shutdown pin to high
    let _shutdown =
        Output::new(gpio37, Level::High, OutputConfig::default());

    // connect physical output pins to the i2s signal pins
    gpio38.connect_peripheral_to_output(OutputSignal::I2S0O_SD);
    gpio39.connect_peripheral_to_output(OutputSignal::I2S0O_BCK);
    gpio40.connect_peripheral_to_output(OutputSignal::I2S0O_WS);

    // make I2S config. sample rate of 44.1kHz
    let config = Config::default()
        .with_sample_rate(Rate::from_hz(44_100))
        // the data format specifies that each frame will be 16 bits,
        // with no padding. from my understanding of i2s, the
        // data line alternates between the left and
        // right channel.
        .with_data_format(DataFormat::Data16Channel16)
        .with_tx_config(Default::default());

    // create I2S.
    // the dma channel means that we are able to occassionally write
    // to a memory buffer that is read directy by the i2s device,
    // instead of having the cpu bitbang out a signal
    I2s::new(i2s0, dma, config).unwrap()
}
