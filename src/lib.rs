// use dasp_ring_buffer::Fixed as RingBuffer;
use fm_core::FmRadioSim;
use nih_plug::prelude::*;
// use parking_lot::Mutex;
use std::{collections::VecDeque, net::UdpSocket, sync::Arc, time::Duration};
// Debug
// use log::{LevelFilter,info};
// use libudprint::{init};

struct Gain {
    socket: Option<UdpSocket>,
    params: Arc<GainParams>,
    fmradio: FmRadioSim,
    tmp_buffer_l: Vec<f32>,
    tmp_buffer_r: Vec<f32>,
    buf_l: VecDeque<f32>,
    buf_r: VecDeque<f32>,
    sample_rate: f32,
    buffer_size: usize,
}

/// The [`Params`] derive macro gathers all of the information needed for the wrapper to know about
/// the plugin's parameters, persistent serializable fields, and nested parameter groups. You can
/// also easily implement [`Params`] by hand if you want to, for instance, have multiple instances
/// of a parameters struct for multiple identical oscillators/filters/envelopes.
#[derive(Params, Clone, Default)]
struct GainParams {
    // The parameter's ID is used to identify the parameter in the wrapped plugin API. As long as
    // these IDs remain constant, you can rename and reorder these fields as you wish. The
    // parameters are exposed to the host in the same order they were defined. In this case, this
    // gain parameter is stored as linear gain while the values are displayed in decibels.
    // #[id = "gain"]
    // pub gain: FloatParam,

    // /// This field isn't used in this example, but anything written to the vector would be restored
    // /// together with a preset/state file saved for this plugin. This can be useful for storing
    // /// things like sample data.
    // #[persist = "industry_secrets"]
    // pub random_data: Mutex<Vec<f32>>,

    // /// You can also nest parameter structs. These will appear as a separate nested group if your
    // /// DAW displays parameters in a tree structure.
    // #[nested(group = "Subparameters")]
    // pub sub_params: SubParams,

    // /// Nested parameters also support some advanced functionality for reusing the same parameter
    // /// struct multiple times.
    // #[nested(array, group = "Array Parameters")]
    // pub array_params: [ArrayParams; 3],
}

#[derive(Params)]
struct SubParams {
    #[id = "thing"]
    pub nested_parameter: FloatParam,
}

#[derive(Params)]
struct ArrayParams {
    /// This parameter's ID will get a `_1`, `_2`, and a `_3` suffix because of how it's used in
    /// `array_params` above.
    #[id = "noope"]
    pub nope: FloatParam,
}

impl Default for Gain {
    fn default() -> Self {
        Self {
            socket: None,
            params: Arc::new(GainParams::default()),
            sample_rate: 44100.,
            buffer_size: Self::DEFAULT_BUFFER_SIZE,
            tmp_buffer_l: vec![0.; Self::DEFAULT_BUFFER_SIZE],
            tmp_buffer_r: vec![0.; Self::DEFAULT_BUFFER_SIZE],
            buf_l: VecDeque::from([0f32; 4096]),
            buf_r: VecDeque::from([0f32; 4096]),
            fmradio: FmRadioSim::from(
                44100,
                Self::DEFAULT_BUFFER_SIZE,
                79_500_000f64,
            ),
        }
    }
}
impl Gain {
    const DEFAULT_BUFFER_SIZE: usize = 700;
    const RING_BUFFER_SIZE: usize = 4;
    pub fn add_socket(&mut self, ip: impl AsRef<str>) {
        if self.socket.is_none() {
            let socket = UdpSocket::bind("127.0.0.1:12345").unwrap();
            socket.connect(ip.as_ref()).unwrap();
            self.socket = Some(socket);
        }
    }
    pub fn info(&self, msg: String) {
        if let Some(socket) = &self.socket {
            socket.send(msg.as_bytes()).unwrap();
        };
    }
}
unsafe impl Send for Gain {}
// impl Default for GainParams {
//     fn default() -> Self {
//         Self {}
//     }
// }

impl Plugin for Gain {
    const NAME: &'static str = "Gain";
    const VENDOR: &'static str = "Moist Plugins GmbH";
    // You can use `env!("CARGO_PKG_HOMEPAGE")` to reference the homepage field from the
    // `Cargo.toml` file here
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";

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

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    // This plugin doesn't need any special initialization, but if you need to do anything expensive
    // then this would be the place. State is kept around when the host reconfigures the
    // plugin. If we do need special initialization, we could implement the `initialize()` and/or
    // `reset()` methods

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let buffer_size = buffer.samples();
        // if buffer_size != self.buffer_size {
        //     self.buffer_size = buffer_size;
        //     self.fmradio = FmRadioSim::from(
        //         self.sample_rate as usize,
        //         buffer_size,
        //         79_500_000f64,
        //     );
        //     self.fmradio.init_thread();
        //     self.buf_l = vec![0.; buffer_size];
        //     self.buf_r = vec![0.; buffer_size];
        // }
        let samples = buffer.as_slice();
        // std::thread::sleep(Duration::from_secs(1));
        // self.info(format!("Sample Size: {buffer_size}"));
        // self.info("Start Processing".into());
        self.fmradio.process(
            samples[0],
            samples[1],
            &mut self.tmp_buffer_l,
            &mut self.tmp_buffer_r,
        );
        buffer
            .iter_samples()
            .zip([&mut self.buf_l, &mut self.buf_r])
            .for_each(|(mut dst, buf)| {
                dst.iter_mut().for_each(|d| {
                    *d = buf.pop_front().unwrap();
                });
            });
        self.tmp_buffer_l
            .iter()
            .zip(self.tmp_buffer_r.iter())
            .for_each(|(l, r)| {
                self.buf_l.push_back(*l);
                self.buf_r.push_back(*r);
            });
        // self.info(format!("Sample Size: {buffer_size}"));
        ProcessStatus::Normal
    }
    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        /* Add input/output Buffer */
        self.add_socket("127.0.0.1:54635");
        // simple_logging::log_to_file("./fmradio.log", log::LevelFilter::Info).unwrap();
        // init("127.0.0.1:54635", LevelFilter::Info).unwrap();

        if self.sample_rate != buffer_config.sample_rate {
            self.fmradio = FmRadioSim::from(
                buffer_config.sample_rate as usize,
                700,
                79_500_000f64,
            );
            
            self.sample_rate = buffer_config.sample_rate;
        }
        self.fmradio.init_thread();
        // self.re_init(buffer_config.sample_rate as f64, FmRadio::DEFAULT_BUF_SIZE);
        self.info(format!("Initialized: fs: {}", buffer_config.sample_rate));
        true
    }
    // This can be used for cleaning up special resources like socket connections whenever the
    // plugin is deactivated. Most plugins won't need to do anything here.
    fn deactivate(&mut self) {}
}

impl ClapPlugin for Gain {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Gain {
    const VST3_CLASS_ID: [u8; 16] = *b"GainMoistestPlug";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(Gain);
nih_export_vst3!(Gain);
