use core::f32::consts::PI;

use defmt::warn;
use embassy_executor::SendSpawner;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Instant};
use esp_hal::{
    Async, Blocking,
    clock::CpuClock,
    dma::DmaDescriptor,
    gpio::{
        Level, Output, OutputConfig, OutputSignal,
        interconnect::PeripheralOutput as _,
    },
    i2s::master::{Config, DataFormat, I2s, I2sTx},
    peripherals::{DMA_CH0, GPIO37, GPIO38, GPIO39, GPIO40, I2S0},
    rng::Rng,
    time::Rate,
};
use micromath::F32Ext;
use static_cell::StaticCell;

pub static _A: Channel<CriticalSectionRawMutex, (), 20> =
    Channel::new();

// NOTE: the system module is used for playing system sounds, and the
// user channel is used for playing user sounds (i.e. programs playing
// their own sounds).
pub mod system;
pub mod user;

// NOTE: later on, it would be nice to have some kind of mixer to
// where multiple sounds can be played at once.

/// The speaker can buffer two sounds.
const SPEAKER_BUFFER_CMD_SIZE: usize = 2;

/// A channel to send commands to the speaker.
pub static SPEAKER_CHANNEL: Channel<
    CriticalSectionRawMutex,
    SpeakerCommand,
    SPEAKER_BUFFER_CMD_SIZE,
> = Channel::new();

#[derive(Clone)]
pub enum SpeakerCommand {
    Sine440Hz(embassy_time::Duration),
}

pub(super) type SpeakerType = I2s<'static, Async>;
type SpeakerTxType = I2sTx<'static, Async>;

pub const SPEAKER_SAMPLE_RATE: u32 = 44_100;

// From my understanding, this involves the memory that we write to
// that the i2s speaker directly reads from. So basically we are
// occassionally filling a buffer.
static DESCRIPTORS: StaticCell<[DmaDescriptor; 8]> =
    StaticCell::new();
static BUFFER: StaticCell<[u8; 2048]> = StaticCell::new();

// The 'a lifetime is used because the I2s interface the function
// returns can only last for as long as the gpio interfaces do.
/// Intiailize the i2s speaker.``
pub(super) fn init(
    i2s0: I2S0<'static>,
    dma: DMA_CH0<'static>,
    gpio37: GPIO37<'static>,
    gpio38: GPIO38<'static>,
    gpio39: GPIO39<'static>,
    gpio40: GPIO40<'static>,
) -> SpeakerType {
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
        .with_sample_rate(Rate::from_hz(SPEAKER_SAMPLE_RATE))
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
    I2s::new(i2s0, dma, config).unwrap().into_async()
}

/// The speaker contains 4 channels that will be mixed.
/// The first two are used for system sounds. The last
/// two are used for playing extra sounds specified by
/// other parts of the program.
struct SpeakerChannelPool {}

/// A task that waits until a speaker command is sent. After receiving
/// a channel input, it will play that sound.
#[embassy_executor::task]
pub async fn speaker_task(speaker: SpeakerType) {
    // TODO: clean up all of this cause like, what the hell
    // initialize static cell buffers
    let descriptors = DESCRIPTORS.init([DmaDescriptor::EMPTY; 8]);
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) =
        esp_hal::dma_buffers!(4096, 4096);
    //let buffer: &mut [u8; 2048] = BUFFER.init([0u8; 2048]);

    let mut speaker_tx: SpeakerTxType =
        speaker.i2s_tx.build(descriptors);

    //loop {
    let cmd = SPEAKER_CHANNEL.receive().await;

    // do a check to see if the command was to switch operating
    // modes (which requires ownership). Otherwise, execute the
    // command as normal.
    match cmd {
        SpeakerCommand::Sine440Hz(duration) => {
            play_sine440hz_async(speaker_tx, Duration::from_secs(10))
                .await;
        } // }
    }
}

fn play_sine440hz(
    speaker_tx: &mut SpeakerTxType,
    buffer: &mut [u8; 32000],
    duration: Duration,
) {
    let mut phase = 0.0f32;
    let sample_rate = SPEAKER_SAMPLE_RATE as f32;
    let freq = 440.0; // A4 tone

    let start = Instant::now();

    let _ = speaker_tx.write_dma(buffer).unwrap();

    loop {
        for chunk in buffer.chunks_exact_mut(4) {
            let sample = (phase.sin() * 8000.0) as i16;

            // stereo
            chunk[0] = sample as u8;
            chunk[1] = (sample >> 8) as u8;
            chunk[2] = sample as u8;
            chunk[3] = (sample >> 8) as u8;

            phase += 2.0 * PI * freq / sample_rate;
            if phase > 2.0 * PI {
                phase -= 2.0 * PI;
            }
        }

        // send to I2S
        let foo = speaker_tx.write_dma(buffer).unwrap();

        let _ = foo.wait();

        // The sound will likely last longer a btt longer than the
        // duration, as the i2s is reading directly from
        // memory.
        if start.elapsed() > duration {
            break;
        }
    }
}

async fn play_sine440hz_async(
    speaker_tx: SpeakerTxType,
    duration: Duration,
) {
    const BUF_SIZE: usize = 512;

    let mut buf_a = [0u8; BUF_SIZE];

    let mut phase = 0.0f32;
    let sample_rate = SPEAKER_SAMPLE_RATE as f32;
    let freq = 440.0;

    let start = Instant::now();

    //fill_sine(&mut buf_a, &mut phase, freq, sample_rate);

    let mut ring_buffer = [0u8; 2048];

    let mut transfer = speaker_tx
        .write_dma_circular_async(&mut ring_buffer)
        .unwrap();

    let mut i = 0;

    while start.elapsed() < duration {
        // fill next buffer
        fill_sine(&mut buf_a, &mut phase, freq, sample_rate);

        let bytes_available_count =
            transfer.available().await.unwrap_or_default();

        let end_index = i + bytes_available_count % (buf_a.len() + 1);

        if let Err(e) = transfer.push(&buf_a[i..end_index]).await {
            warn!("speaker error");
        }

        i += bytes_available_count;

        //apply_fade_edges(fill, FADE_SAMPLES);

        //transfer.unwrap();

        //core::mem::swap(&mut current, &mut next);
        // core::mem::swap(&mut next, &mut fill);
    }
}

fn fill_sine(
    buffer: &mut [u8],
    phase: &mut f32,
    freq: f32,
    sample_rate: f32,
) {
    for chunk in buffer.chunks_exact_mut(4) {
        let sample = (phase.sin() * 8000.0) as i16;
        let s = sample.to_le_bytes();

        // LEFT channel
        chunk[0] = s[0];
        chunk[1] = s[1];

        // RIGHT channel (silence)
        chunk[2] = 0;
        chunk[3] = 0;

        *phase += 2.0 * PI * freq / sample_rate;
        if *phase >= 2.0 * PI {
            *phase -= 2.0 * PI;
        }
    }
}

fn apply_fade_edges(buffer: &mut [u8], fade_samples: usize) {
    let samples = buffer.len() / 4;

    for i in 0..fade_samples {
        let gain = i as f32 / fade_samples as f32;

        let idx = i * 4;

        let sample =
            i16::from_le_bytes([buffer[idx], buffer[idx + 1]]);
        let scaled = (sample as f32 * gain) as i16;

        let bytes = scaled.to_le_bytes();
        buffer[idx] = bytes[0];
        buffer[idx + 1] = bytes[1];
        buffer[idx + 2] = bytes[0];
        buffer[idx + 3] = bytes[1];
    }
}
