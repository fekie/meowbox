use core::f32::consts::PI;

use defmt::warn;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Instant};
use esp_hal::{
    Async,
    dma::DmaDescriptor,
    gpio::{
        Level, Output, OutputConfig, OutputSignal,
        interconnect::PeripheralOutput as _,
    },
    i2s::master::{
        Config, DataFormat, I2s, I2sTx,
        asynch::I2sWriteDmaTransferAsync,
    },
    peripherals::{DMA_CH0, GPIO39, GPIO40, GPIO41, GPIO42, I2S0},
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

pub static MEOW_PCM: &[u8] =
    include_bytes!("../../../sounds/meow.pcm");

pub struct Cry {
    pub pokemon_id: u16,
    pub name: &'static str,
    pub samples: &'static [u8],
}

pub static CRIES: &[Cry] =
    include!(concat!(env!("OUT_DIR"), "/cries.rs"));

#[derive(Clone)]
pub enum SpeakerCommand {
    Sine440Hz(embassy_time::Duration),
    PlayWaveform {
        waveform: Waveform,
        frequency_hz: u16,
        duration: embassy_time::Duration,
    },
    Silence,
    PlayPcm(&'static [u8]),
    PlayPcmWithVolume {
        samples: &'static [u8],
        volume_multiplier: f32,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
}

impl Waveform {
    pub const COUNT: usize = 4;

    pub fn index(self) -> usize {
        match self {
            Self::Sine => 0,
            Self::Square => 1,
            Self::Saw => 2,
            Self::Triangle => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::Sine,
            1 => Self::Square,
            2 => Self::Saw,
            _ => Self::Triangle,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Sine => "Sine",
            Self::Square => "Square",
            Self::Saw => "Saw",
            Self::Triangle => "Triangle",
        }
    }
}

pub(super) type SpeakerType = I2s<'static, Async>;
type SpeakerTxType = I2sTx<'static, Async>;

pub const SPEAKER_SAMPLE_RATE: u32 = 44_100;

// From my understanding, this involves the memory that we write to
// that the i2s speaker directly reads from. So basically we are
// occassionally filling a buffer.
static DESCRIPTORS: StaticCell<[DmaDescriptor; 8]> =
    StaticCell::new();

// The 'a lifetime is used because the I2s interface the function
// returns can only last for as long as the gpio interfaces do.
/// Intiailize the i2s speaker.``
pub(super) fn init(
    i2s0: I2S0<'static>,
    dma: DMA_CH0<'static>,
    sd: GPIO39<'static>,
    din: GPIO40<'static>,
    bclk: GPIO41<'static>,
    lrclk: GPIO42<'static>,
) -> SpeakerType {
    // for some really weird reason, sd on this chip stands for
    // shutdown, and not serial data (which is what the i2s
    // protocol uses, but this chip calls it din)
    //
    // enable the amplifier by setting the shutdown pin to high
    let _shutdown =
        Output::new(sd, Level::High, OutputConfig::default());

    // connect physical output pins to the i2s signal pins
    din.connect_peripheral_to_output(OutputSignal::I2S0O_SD);
    bclk.connect_peripheral_to_output(OutputSignal::I2S0O_BCK);
    lrclk.connect_peripheral_to_output(OutputSignal::I2S0O_WS);

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
    // let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) =
    //     esp_hal::dma_buffers!(4096, 4096);
    //let buffer: &mut [u8; 2048] = BUFFER.init([0u8; 2048]);

    let speaker_tx: SpeakerTxType = speaker.i2s_tx.build(descriptors);
    let mut dma_buffer = [0u8; 4096];
    let mut transfer = speaker_tx
        .write_dma_circular_async(&mut dma_buffer)
        .unwrap();
    let mut waveform_phase = 0.0f32;

    loop {
        let cmd = SPEAKER_CHANNEL.receive().await;

        match cmd {
            SpeakerCommand::Sine440Hz(duration) => {
                let mut buffer = [0u8; 2048];
                let mut phase = 0.0f32;
                let sample_rate = SPEAKER_SAMPLE_RATE as f32;
                let start = Instant::now();

                while start.elapsed() < duration {
                    fill_sine(
                        &mut buffer,
                        &mut phase,
                        440.0,
                        sample_rate,
                    );
                    push_all(&mut transfer, &buffer).await;
                }

                buffer.fill(0);
                for _ in 0..2 {
                    push_all(&mut transfer, &buffer).await;
                }
            }
            SpeakerCommand::PlayWaveform {
                waveform,
                frequency_hz,
                duration,
            } => {
                let mut buffer = [0u8; 2048];
                let sample_rate = SPEAKER_SAMPLE_RATE as f32;
                let start = Instant::now();

                while start.elapsed() < duration {
                    fill_waveform(
                        &mut buffer,
                        &mut waveform_phase,
                        waveform,
                        frequency_hz as f32,
                        sample_rate,
                    );
                    push_all(&mut transfer, &buffer).await;
                }
            }
            SpeakerCommand::Silence => {
                waveform_phase = 0.0;
                let silence = [0u8; 2048];
                for _ in 0..2 {
                    push_all(&mut transfer, &silence).await;
                }
            }
            SpeakerCommand::PlayPcm(samples) => {
                for buffer in samples.chunks(2048) {
                    push_all(&mut transfer, buffer).await;
                }

                let silence = [0u8; 2048];
                for _ in 0..2 {
                    push_all(&mut transfer, &silence).await;
                }
            }
            SpeakerCommand::PlayPcmWithVolume {
                samples,
                volume_multiplier,
            } => {
                let mut scaled_buffer = [0u8; 2048];

                for buffer in samples.chunks(scaled_buffer.len()) {
                    let output = &mut scaled_buffer[..buffer.len()];
                    scale_pcm_s16le(
                        buffer,
                        output,
                        volume_multiplier,
                    );
                    push_all(&mut transfer, output).await;
                }

                let silence = [0u8; 2048];
                for _ in 0..2 {
                    push_all(&mut transfer, &silence).await;
                }
            }
        }
    }
}

fn scale_pcm_s16le(input: &[u8], output: &mut [u8], multiplier: f32) {
    output.copy_from_slice(input);

    for (input_sample, output_sample) in
        input.chunks_exact(2).zip(output.chunks_exact_mut(2))
    {
        let sample =
            i16::from_le_bytes([input_sample[0], input_sample[1]]);
        let scaled = (sample as f32 * multiplier)
            .clamp(i16::MIN as f32, i16::MAX as f32)
            as i16;
        output_sample.copy_from_slice(&scaled.to_le_bytes());
    }
}

async fn push_all(
    transfer: &mut I2sWriteDmaTransferAsync<'_, &mut [u8; 4096]>,
    buffer: &[u8],
) {
    let mut offset = 0;

    while offset < buffer.len() {
        match transfer.push(&buffer[offset..]).await {
            Ok(0) => {}
            Ok(written) => offset += written,
            Err(_) => {
                warn!("speaker error");
                return;
            }
        }
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
            let sample = (phase.sin() * 16000.0) as i16;

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

fn fill_sine(
    buffer: &mut [u8],
    phase: &mut f32,
    freq: f32,
    sample_rate: f32,
) {
    for chunk in buffer.chunks_exact_mut(4) {
        let sample = (phase.sin() * 16000.0) as i16;
        let s = sample.to_le_bytes();

        chunk[0] = s[0];
        chunk[1] = s[1];
        chunk[2] = s[0];
        chunk[3] = s[1];

        *phase += 2.0 * PI * freq / sample_rate;
        if *phase >= 2.0 * PI {
            *phase -= 2.0 * PI;
        }
    }
}

fn fill_waveform(
    buffer: &mut [u8],
    phase: &mut f32,
    waveform: Waveform,
    freq: f32,
    sample_rate: f32,
) {
    for chunk in buffer.chunks_exact_mut(4) {
        let sample =
            (waveform_sample(waveform, *phase) * 14000.0) as i16;
        let s = sample.to_le_bytes();

        chunk[0] = s[0];
        chunk[1] = s[1];
        chunk[2] = s[0];
        chunk[3] = s[1];

        *phase += freq / sample_rate;
        while *phase >= 1.0 {
            *phase -= 1.0;
        }
    }
}

pub fn waveform_sample(waveform: Waveform, phase: f32) -> f32 {
    let phase = phase - phase as u32 as f32;

    match waveform {
        Waveform::Sine => (phase * 2.0 * PI).sin(),
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Saw => phase * 2.0 - 1.0,
        Waveform::Triangle => {
            if phase < 0.5 {
                phase * 4.0 - 1.0
            } else {
                3.0 - phase * 4.0
            }
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
