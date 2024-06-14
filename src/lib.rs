// use dasp::Sample;
// use log::info;
use nih_plug::prelude::*;
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
// mod adpcm;
// use adpcm::{Decoder, Encoder};
mod composite;
mod filter;
mod fm_modulator;
mod transmission_line;
use composite::{CompositeSignal, RestoredSignal};
use fm_modulator::{FmDeModulator, FmModulator};
use transmission_line::TransmissionLine;
// use nih_plug_vizia::ViziaState;

mod utils {

    // pub fn downsample_f32(dst: &mut [f32], x: &[f32], factor: f64) {
    //     let x_len = x.len() as f64;
    //     // let mut dst = Vec::with_capacity((x_len / factor + 0.5) as usize);
    //     let mut i = 0.;
    //     let mut j = 0;
    //     let max = x.len() - 1;
    //     while (i + 0.5) < x_len && j < dst.len() {
    //         let idx_a = i.floor();
    //         let idx_b = i.ceil() as usize;
    //         let p = i - idx_a;
    //         let q = 1. - p;
    //         dst[j] = x[(idx_a as usize).min(max)] * q as f32
    //             + x[idx_b.min(max)] * p as f32;
    //         i += factor;
    //         j += 1;
    //     }
    // }
    pub fn downsample_f32(x: &[f32], factor: f64) -> Vec<f32> {
        let x_len = x.len() as f64;
        let mut dst = Vec::with_capacity((x_len / factor + 0.5) as usize);
        let mut i = 0.;
        let max = x.len() - 1;
        while (i + 0.5) < x_len {
            let idx_a = i.floor();
            let idx_b = i.ceil() as usize;
            let p = i - idx_a;
            let q = 1. - p;
            dst.push(
                x[(idx_a as usize).min(max)] * q as f32
                    + x[idx_b.min(max)] * p as f32,
            );
            i += factor;
        }
        dst
    }
}
const FM_CARRIER_FREQ: usize = 1_000_000;
const CUT_OFF: usize = 500_000;
const UPPER_SAMPLE_RATE: usize = FM_CARRIER_FREQ * 4;
struct FmRadio {
    params: Arc<Param>,
    sample_rate: f32,
    // FMシミュレーション
    fm_modulator: FmModulator,
    fm_demodulator: FmDeModulator,
    // 伝送路
    // transmission_line: TransmissionLine,
    // コンポジット
    composite: CompositeSignal,
    restore: RestoredSignal,
    // UpSampler
    up_sampler: Option<FastFixedIn<f32>>,
    // Buffer
    buffer: [Vec<f32>; 2],
}
#[derive(Params)]
struct Param {
    pub noise_gain: Arc<RwLock<f32>>,
    // pub fm_carrier: Arc<RwLock<f32>>,
}
impl Default for Param {
    fn default() -> Self {
        Self {
            noise_gain: Arc::new(RwLock::new(-std::f32::INFINITY)),
        }
    }
}
impl Default for FmRadio {
    fn default() -> Self {
        Self {
            params: Arc::new(Param::default()),
            sample_rate: 1.0,
            // FMシミュレーション
            fm_modulator: FmModulator::from(
                FM_CARRIER_FREQ as f64,
                UPPER_SAMPLE_RATE as f64,
            ),
            fm_demodulator: FmDeModulator::from(
                FM_CARRIER_FREQ as f64,
                UPPER_SAMPLE_RATE as f64,
                (FM_CARRIER_FREQ + CUT_OFF) as f64,
            ),
            // 伝送路
            // transmission_line: TransmissionLine::from_snr(-std::f32::INFINITY),
            // コンポジット
            composite: CompositeSignal::new(UPPER_SAMPLE_RATE as f32),
            restore: RestoredSignal::new(UPPER_SAMPLE_RATE as f32),
            // UpSampler
            up_sampler: None,
            buffer: [Vec::new(), Vec::new()],
        }
    }
}

impl Plugin for FmRadio {
    const NAME: &'static str = "FM Radio Simulator";
    const VENDOR: &'static str = "PARA-DISO";
    // You can use `env!("CARGO_PKG_HOMEPAGE")` to reference the homepage field from the
    // `Cargo.toml` file here
    const URL: &'static str = "https://github.com/PARA-DISO";
    const EMAIL: &'static str = "paradiso@ymail.ne.jp";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[],
            aux_output_ports: &[],

            // Individual ports and the layout as a whole can be named here. By default these names
            // are generated as needed. This layout will be called 'Stereo', while the other one is
            // given the name 'Mono' based no the number of input and output channels.
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    // Setting this to `true` will tell the wrapper to split the buffer up into smaller blocks
    // whenever there are inter-buffer parameter changes. This way no changes to the plugin are
    // required to support sample accurate automation and the wrapper handles all of the boring
    // stuff like making sure transport and other timing information stays consistent between the
    // splits.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn nih_plug::prelude::Params> {
        self.params.clone()
    }
    fn editor(
        &mut self,
        _async_executor: AsyncExecutor<Self>,
    ) -> Option<Box<dyn Editor>> {
        // editor::create(
        //     self.params.bypass.clone(),
        //     self.params.sample_rate.clone(),
        //     self.params.editor_state.clone(),
        // )
        None
    }
    // This plugin doesn't need any special initialization, but if you need to do anything expensive
    // then this would be the place. State is kept around when the host reconfigures the
    // plugin. If we do need special initialization, we could implement the `initialize()` and/or
    // `reset()` methods
    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        // self.composite =
        true
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.up_sampler.is_none() {
            self.up_sampler = Some(
                FastFixedIn::new(
                    UPPER_SAMPLE_RATE as f64 / self.sample_rate as f64,
                    UPPER_SAMPLE_RATE as f64 / self.sample_rate as f64,
                    PolynomialDegree::Linear,
                    buffer.samples(),
                    buffer.channels(),
                )
                .unwrap(),
            );
        }
        let buf_size = self.up_sampler.as_ref().unwrap().output_frames_next();
        if self.buffer[0].len() != buf_size {
            self.buffer[0] = vec![0.; buf_size];
            self.buffer[1] = vec![0.; buf_size];
        }
        // // 入力をup sample

        let mut buf1 = unsafe {
            std::slice::from_raw_parts_mut(
                self.buffer[0].as_ptr().cast_mut(),
                buf_size,
            )
        };
        let mut buf2 = unsafe {
            std::slice::from_raw_parts_mut(
                self.buffer[1].as_ptr().cast_mut(),
                buf_size,
            )
        };
        let _ = self.up_sampler.as_mut().unwrap().process_into_buffer(
            buffer.as_slice(),
            &mut [&mut buf1, &mut buf2],
            None,
        );
        self.composite.process_to_buffer(
            self.buffer[0].as_slice(),
            self.buffer[1].as_slice(),
            buf1,
        );
        self.fm_modulator
            .modulate_to_buffer(self.buffer[0].as_slice(), buf2);
        self.fm_demodulator
            .demodulate_to_buffer(self.buffer[1].as_slice(), buf1);
        self.restore
            .process_to_buffer(self.buffer[0].as_slice(), buf1, buf2);
        // // down sample
        let factor = buf_size as f64 / buffer.samples() as f64;
        // let samples = buffer.as_slice();
        buffer
            .as_slice()
            .iter_mut()
            .zip(self.buffer.iter())
            .for_each(|(s, buf)| {
                s.iter_mut()
                    .zip(utils::downsample_f32(buf, factor))
                    .for_each(|(s, buf)| {
                        *s = buf;
                    })
            });
        // utils::downsample_f32(samples[0], buf1, factor);
        // utils::downsample_f32(samples[1], buf2, factor);
        ProcessStatus::Normal
    }

    // This can be used for cleaning up special resources like socket connections whenever the
    // plugin is deactivated. Most plugins won't need to do anything here.
    fn deactivate(&mut self) {}
}

// impl ClapPlugin for Adpcm {
//     const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain";
//     const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
//     const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
//     const CLAP_SUPPORT_URL: Option<&'static str> = None;
//     const CLAP_FEATURES: &'static [ClapFeature] = &[
//         ClapFeature::AudioEffect,
//         ClapFeature::Stereo,
//         ClapFeature::Mono,
//         ClapFeature::Utility,
//     ];
// }

impl Vst3Plugin for FmRadio {
    const VST3_CLASS_ID: [u8; 16] = [
        0x71, 0xF4, 0xBF, 0xA6, 0x71, 0xBD, 0x42, 0xDD, 0xB7, 0xB6, 0xF4, 0xF6,
        0x79, 0xE2, 0x52, 0x81,
    ];
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

// nih_export_clap!(Adpcm);
nih_export_vst3!(FmRadio);
