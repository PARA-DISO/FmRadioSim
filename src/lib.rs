// use dasp_ring_buffer::Fixed as RingBuffer;
use buffer::FixedLenBuffer;
use fm_core::{sharable, FmRadioSim, Shareable};
use nih_plug::prelude::*;
// use parking_lot::Mutex;
use std::{
    hint,
    sync::atomic::{AtomicBool, Ordering},
};
use std::{
    hint::spin_loop,
    net::UdpSocket,
    sync::{mpsc, Arc, Barrier, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
// Debug
// use log::{LevelFilter,info};
// use libudprint::{init};

struct FmSim {
    socket: Arc<Mutex<Option<UdpSocket>>>,
    params: Arc<FmParams>,
    // fmradio: Shareable<FmRadioSim>,
    input_buffer: Shareable<[FixedLenBuffer; 2]>,
    output_buffer: Shareable<[FixedLenBuffer; 2]>,
    // input_buffer: Shareable<[VecDeque<f32>; 2]>,
    // output_buffer: Shareable<[VecDeque<f32>; 2]>,
    // tmp_buffer_l: Vec<f32>,
    // tmp_buffer_r: Vec<f32>,
    // buf_l: VecDeque<f32>,
    // buf_r: VecDeque<f32>,
    sample_rate: f32,
    buffer_size: usize,
    //
    input_signal_wait: Arc<AtomicBool>,
    output_signal_wait: Arc<AtomicBool>,
    // start_barrier: Arc<Condvar>,
    //
    is_init: bool,
    //
    msg_sender: Option<mpsc::Sender<usize>>,
    // msg_receiver: Option<mpsc::Receiver<usize>>,
    //
    handle: Option<JoinHandle<i32>>,
}

/// The [`Params`] derive macro gathers all of the information needed for the wrapper to know about
/// the plugin's parameters, persistent serializable fields, and nested parameter groups. You can
/// also easily implement [`Params`] by hand if you want to, for instance, have multiple instances
/// of a parameters struct for multiple identical oscillators/filters/envelopes.
#[derive(Params, Clone, Default)]
struct FmParams {
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

impl Default for FmSim {
    fn default() -> Self {
        Self {
            socket: Arc::new(Mutex::new(None)),
            params: Arc::new(FmParams::default()),
            sample_rate: 44100.,
            buffer_size: Self::DEFAULT_BUFFER_SIZE,
            // tmp_buffer_l: vec![0.; Self::DEFAULT_BUFFER_SIZE],
            // tmp_buffer_r: vec![0.; Self::DEFAULT_BUFFER_SIZE],
            // buf_l: VecDeque::from([0f32; 4096]),
            // buf_r: VecDeque::from([0f32; 4096]),
            // input_buffer: sharable!([
            //     VecDeque::<f32>::with_capacity(
            //         Self::DEFAULT_BUFFER_SIZE * Self::RING_BUFFER_SIZE
            //     ),
            //     VecDeque::<f32>::with_capacity(
            //         Self::DEFAULT_BUFFER_SIZE * Self::RING_BUFFER_SIZE
            //     )
            // ]),
            // output_buffer: sharable!([
            //     VecDeque::<f32>::with_capacity(
            //         Self::DEFAULT_BUFFER_SIZE * Self::RING_BUFFER_SIZE
            //     ),
            //     VecDeque::<f32>::with_capacity(
            //         Self::DEFAULT_BUFFER_SIZE * Self::RING_BUFFER_SIZE
            //     )
            // ]),
            input_buffer: sharable!([
                FixedLenBuffer::new(Self::DEFAULT_BUFFER_SIZE, Self::RING_BUFFER_SIZE).unwrap(),
                FixedLenBuffer::new(Self::DEFAULT_BUFFER_SIZE, Self::RING_BUFFER_SIZE).unwrap()
            ]),
            output_buffer: sharable!([
                FixedLenBuffer::new(Self::DEFAULT_BUFFER_SIZE, Self::RING_BUFFER_SIZE).unwrap(),
                FixedLenBuffer::new(Self::DEFAULT_BUFFER_SIZE, Self::RING_BUFFER_SIZE).unwrap()
            ]),
            // fmradio: sharable!(FmRadioSim::from(
            //     44100,
            //     Self::DEFAULT_BUFFER_SIZE,
            //     79_500_000f64,
            // )),
            //
            // input_signal: Arc::new((Mutex::new(false), Condvar::new())),
            // output_signal: Arc::new((Mutex::new(false), Condvar::new())),
            input_signal_wait: Arc::new(AtomicBool::new(false)),
            output_signal_wait: Arc::new(AtomicBool::new(false)),
            // start_barrier: Arc::new(Condvar::new()),
            is_init: false,
            msg_sender: None,
            // msg_receiver: None,
            handle: None,
        }
    }
}
impl FmSim {
    const DEFAULT_BUFFER_SIZE: usize = 700;
    const RING_BUFFER_SIZE: usize = 8;
    pub fn add_socket(&mut self, ip: impl AsRef<str>) {
        if self.socket.lock().unwrap().is_none() {
            let socket = UdpSocket::bind("127.0.0.1:12345").unwrap();
            socket.connect(ip.as_ref()).unwrap();
            self.socket = Arc::new(Mutex::new(Some(socket)));
        }
    }
    pub fn info(&self, msg: String) {
        if let Some(socket) = &*self.socket.lock().unwrap() {
            socket.send(msg.as_bytes()).unwrap();
        };
    }
}
unsafe impl Send for FmSim {}

impl Plugin for FmSim {
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
        if !self.is_init {
            self.is_init = true;
            self.msg_sender.as_ref().unwrap().send(1).unwrap();
            // self.info(String::from("wait @ main"));
            // self.start_barrier.wait();
        }
        let samples = buffer.as_slice();
        // let is_input_empty = self.input_buffer.lock().unwrap()[0].is_empty();
        // self.info(String::from("process init end"));
        // let input_is_empty = self.input_buffer.lock().unwrap()[1].is_empty();
        // 入力バッファへデータを追加
        samples
            .iter()
            .zip(self.input_buffer.lock().unwrap().iter_mut())
            .for_each(|(samples, buffer)| {
                buffer.enqueue(samples);
            });
        self.input_signal_wait.store(true, Ordering::Release);
        // if input_is_empty {
        //     self.info(String::from("sync input buffer @ main"));
        //     // self.input_signal_wait.wait();

        //     self.info(String::from("After input sync @ main"));
        // }

        if self.output_buffer.lock().unwrap()[1].is_empty() {
            self.info(String::from("output buffer is empty"));
            while !self.output_signal_wait.load(Ordering::Acquire) {
                hint::spin_loop();
            }
            self.output_signal_wait.store(false, Ordering::Release);
            // self.info(String::from(??"After output sync @ main"));
        }
        // 出力バッファからデータを取り出す
        samples
            .iter_mut()
            .zip(self.output_buffer.lock().unwrap().iter_mut())
            .for_each(|(samples, buffer)| {
                buffer.dequeue(samples);
            });
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
            // self.fmradio = sharable!(FmRadioSim::from(
            //     buffer_config.sample_rate as usize,
            //     700,
            //     79_500_000f64,
            // ));

            // self.sample_rate = buffer_config.sample_rate;
        }
        let (tx, rx) = mpsc::channel::<usize>();
        self.msg_sender = Some(tx);
        if let Ok(buf) = &mut self.output_buffer.lock() {
            buf[0].set_len(4);
            buf[1].set_len(4);
        }
        // self.re_init(buffer_config.sample_rate as f64, FmRadio::DEFAULT_BUF_SIZE);
        self.info(format!("Initialized: fs: {}", buffer_config.sample_rate));
        {
            let input_buffer = Arc::clone(&self.input_buffer);
            let output_buffer = Arc::clone(&self.output_buffer);
            let socket = Arc::clone(&self.socket);
            // let _start_barrier = Arc::clone(&self.start_barrier);
            let wait_input = Arc::clone(&self.input_signal_wait);
            let wait_output = Arc::clone(&self.output_signal_wait);
            //
            let buffer_size = self.buffer_size;
            let sample_rate = self.sample_rate as usize;
            let handle = std::thread::spawn(move || {
                let send_msg = |msg: &[u8]| {
                    socket.lock().unwrap().as_ref().unwrap().send(msg).unwrap();
                };
                let mut l_buffer = vec![0.; buffer_size];
                let mut r_buffer = vec![0.; buffer_size];
                let mut l_dst_buffer = vec![0.; buffer_size];
                let mut r_dst_buffer = vec![0.; buffer_size];
                // let (input_flag, input_sig) = &*input_signal;
                // let (output_flag, output_sig) = &*output_signal;

                if rx.recv().unwrap() == 0 {
                    return 0;
                }
                let mut fmradio =
                    FmRadioSim::from(sample_rate, Self::DEFAULT_BUFFER_SIZE, 79_500_000f64);
                fmradio.init_thread();
                send_msg(b"start processing thread");
                loop {
                    // while  {}
                    {
                        while !wait_input.load(Ordering::Acquire) {
                            hint::spin_loop();
                        }
                        wait_input.store(false, Ordering::Release);
                        let mut inputs = input_buffer.lock().unwrap();
                        // send_msg(format!("input buffer len: {:?}", inputs[1].get_len()).as_bytes());

                        inputs[0].dequeue(&mut l_buffer);
                        inputs[1].dequeue(&mut r_buffer);
                    }
                    // Note: [TEST] gain code
                    // [&l_buffer,&r_buffer].iter().zip([
                    //   &mut l_dst_buffer,&mut r_dst_buffer
                    // ].iter_mut()).for_each(
                    //     |(src, dst)| {
                    //       dst.iter_mut().zip(src.iter()).for_each(|(d,s)| {
                    //         *d = 0.9 * *s;
                    //       });
                    //     },
                    // );
                    // Note: FM SIM CODE
                    // let start = Instant::now();
                    fmradio.process(&l_buffer, &r_buffer, &mut l_dst_buffer, &mut r_dst_buffer);
                    // let end = start.elapsed();
                    {
                        let mut buffer = output_buffer.lock().unwrap();
                        // send_msg(
                        //     format!("output buffer len: {:?}", buffer[1].get_len()).as_bytes(),
                        // );
                        // let is_empty = buffer[1].is_empty();
                        buffer[0].enqueue(&l_dst_buffer);
                        if !buffer[1].enqueue(&r_dst_buffer) {
                            send_msg(b"output buffer is full");
                        };
                        wait_output.store(true, Ordering::Release);
                        // if is_empty {
                        //     send_msg(b"sync out-buffer @ processing thread");
                        //     // wait_output.wait();
                        //     send_msg(b"sync after out-buffer @ processing thread");
                        // } else {
                        //     send_msg(
                        //         format!("output buffer len: {:?}", buffer[1].get_len()).as_bytes(),
                        //     );
                        //     // wait_output.wait();
                        // }
                    }
                    // let _ = socket
                    // .lock()
                    // .unwrap()
                    // .as_ref()
                    // .unwrap()
                    // .send(format!("elapsed: {:?}",end).as_bytes());
                }
            });
            self.handle = Some(handle);
        }
        self.info("Initialized end".to_string());
        true
    }
    // This can be used for cleaning up special resources like socket connections whenever the
    // plugin is deactivated. Most plugins won't need to do anything here.
    fn deactivate(&mut self) {
        self.info("call deactivate".to_string());
        let _ = self.msg_sender.as_ref().unwrap().send(0);
        // if let Some(handle) = &self.handle {
        //     // handle.close();
        // }
        *self.socket.lock().unwrap() = None;
    }
}

impl ClapPlugin for FmSim {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for FmSim {
    const VST3_CLASS_ID: [u8; 16] = *b"GainMoistestPlug";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(FmSim);
nih_export_vst3!(FmSim);
