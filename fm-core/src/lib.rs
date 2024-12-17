mod modulation_modules;
use libsoxr::{
    datatype::Datatype,
    spec::{QualityFlags, QualityRecipe, QualitySpec},
    Soxr,
};
use modulation_modules::*;
mod resampler;
use resampler::*;
mod utils;

use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Barrier, Mutex},
    thread,
    time::Instant,
};
use utils::{generate_pipline_buffer, ExecFlag, PipeLineBuffer, Shareable};
#[link(name = "freq_modulation")]
extern "C" {
    fn fm_modulate(
        output_signal: *mut f64,
        input_signal: *const f64,
        buf_len: u64,
        info: *mut modulator::Modulator,
    );
    fn convert_intermediate_freq(
        output_signal: *mut f64,
        input_signal: *const f64,
        // sample_period: f64,
        // fc: f64,
        // fi: f64,
        info: *mut modulator::CnvFiInfos,
        buf_len: usize,
    );
    fn fm_demodulate(
        output_signal: *mut f64,
        input_signal: *const f64,
        sample_period: f64,
        carrier_freq: f64,
        info: *mut modulator::DemodulationInfo,
        buf_len: u64,
    );
    fn upsample(dst: *mut f64, input: *const f64, info: *mut ResamplerInfo);
    fn downsample(dst: *mut f64, input: *const f64, info: *mut ResamplerInfo);
    fn filtering(
        output_signal: *mut f64,
        input_signal: *const f64,
        filter_coeff: *mut modulator::BandPassFilter,
        buf_len: u64,
    );
    fn filtering_with_resample(
        output_signal: *mut f64,
        input_signal: *const f64,
        filter_coeff: *mut modulator::BandPassFilter,
        buf_len: u64,
    );
    fn set_csr(flag: u32);
}

pub struct FmRadioSim {
    // basic parameters
    audio_sample_rate: usize,
    buffer_size: usize,
    // Simulation Modules
    composite: composite::CompositeSignal,
    restore: composite::RestoreSignal,
    modulator: Shareable<modulator::Modulator>,
    demodulator: Shareable<modulator::DeModulator>,
    freq_converter: Shareable<modulator::CvtIntermediateFreq>,

    bandpass_filter1: Shareable<modulator::BandPassFilter>,
    bandpass_filter2: Shareable<modulator::BandPassFilter>,

    // resampler
    upsampler: [Soxr; 2],
    downsampler: [Soxr; 2],
    upsampler_for_radio_waves: ResamplerInfo,
    downsampler_for_radio_waves: ResamplerInfo,
    // internal buffer
    // interleave/de-interleave
    tmp_buffer: [Vec<f64>; 2],      // audio sample rate
    audio_in_buffer: [Vec<f64>; 2], // 125kHz
    composite_signal: Vec<f64>,     // 125kHz
    up_sampled_signal: PipeLineBuffer, // 192MHz
    modulate_signal: PipeLineBuffer, // 192MHz
    intermediate_signal1: PipeLineBuffer, // 192MHz (inner 384MHz)
    intermediate_signal2: PipeLineBuffer, // 48MHz?
    intermediate_signal3: PipeLineBuffer, // 48MHz
    demodulate_signal: PipeLineBuffer, // 48MHz
    // demodulate_signal: Vec<f64>,
    post_down_sample: Vec<f64>, // 125kHz
    restored_signal_l: Vec<f64>,
    restored_signal_r: Vec<f64>, // 125kHz
    // Thread Pool (For management)
    read_state: bool,
    barrier: Arc<Barrier>,
    is_init: bool,
}
impl FmRadioSim {
    // define constants
    // pub const COMPOSITE_SAMPLE_RATE: usize = 125_000;
    pub const COMPOSITE_SAMPLE_RATE: usize = 192_000;
    // pub const FM_MODULATION_SAMPLE_RATE: usize = 192_000_000;
    pub const FM_MODULATION_SAMPLE_RATE: usize = 185_000_000;
    pub const INTERMEDIATE_FREQ: f64 = 10_700_000f64; // JISC6421:1994
                                                      // pub const INTERMEDIATE_FREQ: f64 = 79_500_000f64 - 440f64;
    pub const SIGNAL_MAX_FREQ: f64 = 53_000. * 2.; // x2 Composite freq max
    pub const RATIO_FS_INTER_FS: usize = 4;
    // fn set_fs(&mut self) {

    // }
    //
    pub fn from(
        audio_fs: usize,
        buffer_size: usize,
        carrier_freq: f64,
    ) -> Self {
        // calc basic params
        let fm_sample_rate = get_8x_sample_rate(
            Self::FM_MODULATION_SAMPLE_RATE,
            Self::COMPOSITE_SAMPLE_RATE,
        );
        let intermediate_fs = fm_sample_rate / Self::RATIO_FS_INTER_FS;
        // generate soxr
        let upsampler = [
            generate_resampler(
                audio_fs as f64,
                Self::COMPOSITE_SAMPLE_RATE as f64,
            )
            .unwrap(),
            generate_resampler(
                audio_fs as f64,
                Self::COMPOSITE_SAMPLE_RATE as f64,
            )
            .unwrap(),
        ];
        let downsampler = [
            generate_resampler(
                Self::COMPOSITE_SAMPLE_RATE as f64,
                audio_fs as f64,
            )
            .unwrap(),
            generate_resampler(
                Self::COMPOSITE_SAMPLE_RATE as f64,
                audio_fs as f64,
            )
            .unwrap(),
        ];
        // calculate buffer size
        let composite_buffer_size =
            get_buffer_size(audio_fs, Self::COMPOSITE_SAMPLE_RATE, buffer_size);
        let modulated_buffer_size = get_buffer_size(
            Self::COMPOSITE_SAMPLE_RATE,
            fm_sample_rate,
            composite_buffer_size,
        );
        let intermediate_buffer_size =
            modulated_buffer_size / Self::RATIO_FS_INTER_FS;
        // MHz order resampler init
        let upsampler_for_radio_waves = ResamplerInfo::new_upsample_info(
            Self::COMPOSITE_SAMPLE_RATE,
            fm_sample_rate,
            composite_buffer_size,
        );
        let downsampler_for_radio_waves = ResamplerInfo::new_downsample_info(
            intermediate_fs,
            Self::COMPOSITE_SAMPLE_RATE,
            intermediate_buffer_size,
        );
        Self {
            audio_sample_rate: audio_fs,
            buffer_size,
            //
            composite: composite::CompositeSignal::new(
                Self::COMPOSITE_SAMPLE_RATE as f64,
            ),
            restore: composite::RestoreSignal::new(
                Self::COMPOSITE_SAMPLE_RATE as f64,
            ),
            modulator: sharable!(modulator::Modulator::from(
                carrier_freq,
                fm_sample_rate as f64,
            )),
            freq_converter: sharable!(modulator::CvtIntermediateFreq::new(
                fm_sample_rate as f64,
                carrier_freq,
                Self::INTERMEDIATE_FREQ,
            )),
            bandpass_filter1: sharable!(modulator::BandPassFilter::new(
                fm_sample_rate as f64,
                Self::INTERMEDIATE_FREQ,
            )),
            bandpass_filter2: sharable!(modulator::BandPassFilter::new(
                fm_sample_rate as f64,
                Self::INTERMEDIATE_FREQ,
            )),
            demodulator: sharable!(modulator::DeModulator::from(
                Self::INTERMEDIATE_FREQ,
                intermediate_fs as f64,
                // 880.
                Self::SIGNAL_MAX_FREQ,
            )),
            // resampler
            upsampler,
            downsampler,
            upsampler_for_radio_waves,
            downsampler_for_radio_waves,
            // buffer
            tmp_buffer: [vec![0.; buffer_size], vec![0.; buffer_size]],
            audio_in_buffer: [
                vec![0.; composite_buffer_size],
                vec![0.; composite_buffer_size],
            ],
            composite_signal: vec![0.; composite_buffer_size],
            up_sampled_signal: generate_pipline_buffer(modulated_buffer_size),
            modulate_signal: generate_pipline_buffer(modulated_buffer_size),
            intermediate_signal1: generate_pipline_buffer(
                modulated_buffer_size,
            ),
            intermediate_signal2: generate_pipline_buffer(
                modulated_buffer_size,
            ),
            intermediate_signal3: generate_pipline_buffer(
                intermediate_buffer_size,
            ),
            // demodulate_signal: generate_pipline_buffer(
            //     intermediate_buffer_size,
            // ),
            demodulate_signal: generate_pipline_buffer(
                intermediate_buffer_size,
            ),
            post_down_sample: vec![0.; composite_buffer_size],
            restored_signal_l: vec![0.; composite_buffer_size],
            restored_signal_r: vec![0.; composite_buffer_size],
            //
            read_state: false,
            barrier: Arc::new(Barrier::new(6)),
            // barrier: Arc::new(Barrier::new(3)),
            is_init: false,
        }
    }
    pub fn get_intermediate(&self) -> &[f64] {
        if self.read_state {
            let array = (self.intermediate_signal3[0]).lock().unwrap();
            unsafe { std::slice::from_raw_parts(array.as_ptr(), array.len()) }
        } else {
            let array = (self.intermediate_signal3[1]).lock().unwrap();
            unsafe { std::slice::from_raw_parts(array.as_ptr(), array.len()) }
        }
    }
    pub fn get_modulate(&self) -> &[f64] {
        if self.read_state {
            let array = (self.modulate_signal[0]).lock().unwrap();
            unsafe { std::slice::from_raw_parts(array.as_ptr(), array.len()) }
        } else {
            let array = (self.modulate_signal[1]).lock().unwrap();
            unsafe { std::slice::from_raw_parts(array.as_ptr(), array.len()) }
        }
    }
    pub fn get_demodulate(&self) -> &[f64] {
        unsafe {
            let tmp = self.demodulate_signal[self.read_state as usize]
                .lock()
                .unwrap();
            std::slice::from_raw_parts(tmp.as_ptr(), tmp.len())
        }
    }
    pub fn get_composite(&self) -> &[f64] {
        &self.composite_signal
    }
    pub fn get_down_sampling(&self) -> &[f64] {
        &self.post_down_sample
    }
    pub fn init_thread(&mut self) {
        if self.is_init {
            println!("threads is already init.");
            return;
        }
        self.is_init = true;
        println!("initialize Threads.");
        // Modules
        let listener0 = Arc::clone(&self.barrier);
        let listener1 = Arc::clone(&self.barrier);
        let listener2 = Arc::clone(&self.barrier);
        let listener3 = Arc::clone(&self.barrier);
        let listener4 = Arc::clone(&self.barrier);
        // Modulation Process
        {
            let modulator = Arc::clone(&self.modulator);
            let up_sample_signal = Arc::clone(&self.up_sampled_signal);
            let modulate_signal = Arc::clone(&self.modulate_signal);
            let _ = thread::spawn(move || {
                unsafe {
                    set_csr(crate::utils::float::FLUSH_TO_ZERO);
                }
                let mut state = false;
                let up_sample_signal = Arc::clone(&up_sample_signal);
                let modulate_signal = Arc::clone(&modulate_signal);

                loop {
                    listener0.wait();
                    let start = Instant::now();
                    modulator.lock().unwrap().process(
                        &up_sample_signal[(!state) as usize].lock().unwrap(),
                        &mut modulate_signal[state as usize].lock().unwrap(),
                    );
                    // println!("hoge");
                    let end = start.elapsed();
                    listener0.wait();
                    state ^= true;

                    println!("Modulate: {:?}", end);
                }
            });
        }
        // Convert-fi Process
        {
            let intermediate_signal = Arc::clone(&self.intermediate_signal1);
            let modulate_signal = Arc::clone(&self.modulate_signal);
            let freq_converter = Arc::clone(&self.freq_converter);
            let _ = thread::spawn(move || {
                unsafe {
                    set_csr(crate::utils::float::FLUSH_TO_ZERO);
                }
                let mut state = false;
                let intermediate_signal = Arc::clone(&intermediate_signal);
                let modulate_signal = Arc::clone(&modulate_signal);

                loop {
                    listener1.wait();
                    let start = Instant::now();
                    freq_converter.lock().unwrap().process(
                        &modulate_signal[(!state) as usize].lock().unwrap(),
                        &mut intermediate_signal[state as usize]
                            .lock()
                            .unwrap(),
                    );
                    let end = start.elapsed();
                    listener1.wait();
                    state ^= true;

                    println!("Cvt-Freq: {:?}", end);
                    // println!("fuga");
                }
            });
        }
        // BPF
        {
            let bandpass_filter = Arc::clone(&self.bandpass_filter1);
            let intermediate_signal_in = Arc::clone(&self.intermediate_signal1);
            let intermediate_signal_out =
                Arc::clone(&self.intermediate_signal2);
            let _ = thread::spawn(move || {
                unsafe {
                    set_csr(crate::utils::float::FLUSH_TO_ZERO);
                }
                let mut state = false;
                let intermediate_signal = Arc::clone(&intermediate_signal_in);
                let intermediate_signal_out =
                    Arc::clone(&intermediate_signal_out);

                loop {
                    listener2.wait();
                    let start = Instant::now();
                    bandpass_filter.lock().unwrap().process_no_resample(
                        &intermediate_signal[(!state) as usize].lock().unwrap(),
                        &mut intermediate_signal_out[state as usize]
                            .lock()
                            .unwrap(),
                    );
                    let end = start.elapsed();
                    listener2.wait();
                    state ^= true;

                    println!("BPF1: {:?}", end);
                }
            });
        }
        {
            let bandpass_filter = Arc::clone(&self.bandpass_filter2);
            let intermediate_signal_in = Arc::clone(&self.intermediate_signal2);
            let intermediate_signal_out =
                Arc::clone(&self.intermediate_signal3);
            let _ = thread::spawn(move || {
                unsafe {
                    set_csr(crate::utils::float::FLUSH_TO_ZERO);
                }
                let mut state = false;
                let intermediate_signal = Arc::clone(&intermediate_signal_in);
                let intermediate_signal_out =
                    Arc::clone(&intermediate_signal_out);

                loop {
                    listener3.wait();
                    let start = Instant::now();
                    bandpass_filter.lock().unwrap().process(
                        &intermediate_signal[(!state) as usize].lock().unwrap(),
                        &mut intermediate_signal_out[state as usize]
                            .lock()
                            .unwrap(),
                    );
                    let end = start.elapsed();
                    listener3.wait();
                    state ^= true;

                    println!("BPF2 : {:?}", end);
                }
            });
        }
        {
            let demodulation = Arc::clone(&self.demodulator);
            let intermediate_signal = Arc::clone(&self.intermediate_signal3);
            let demodulate_signal = Arc::clone(&self.demodulate_signal);
            let _ = thread::spawn(move || {
                unsafe {
                    set_csr(crate::utils::float::FLUSH_TO_ZERO);
                }
                let mut state = false;
                let intermediate_signal = Arc::clone(&intermediate_signal);
                let demodulate_signal = Arc::clone(&demodulate_signal);

                loop {
                    listener4.wait();
                    let start = Instant::now();
                    demodulation.lock().unwrap().process(
                        &intermediate_signal[(!state) as usize].lock().unwrap(),
                        &mut demodulate_signal[state as usize].lock().unwrap(),
                    );
                    let end = start.elapsed();
                    listener4.wait();
                    state ^= true;
                    
                    println!("De-Modulate: {:?}", end);
                }
            });
        }
    }
    pub fn process(
        &mut self,
        input_l: &[f32],
        input_r: &[f32],
        dst_l: &mut [f32],
        dst_r: &mut [f32],
    ) {
        self.barrier.wait();
        // cvar_3.notify_one();
        // de-interleave
        for (i, lr) in input_l.iter().zip(input_r).enumerate() {
            unsafe {
                *self.tmp_buffer[0].get_unchecked_mut(i) = *lr.0 as f64;
                *self.tmp_buffer[1].get_unchecked_mut(i) = *lr.1 as f64;
            }
        }
        // up sample
        let _ = self.upsampler[0].process::<f64, f64>(
            Some(&self.tmp_buffer[0]),
            &mut self.audio_in_buffer[0],
        );
        let _ = self.upsampler[1].process::<f64, f64>(
            Some(&self.tmp_buffer[1]),
            &mut self.audio_in_buffer[1],
        );
        // composite
        self.composite.process(
            &self.audio_in_buffer[0],
            &self.audio_in_buffer[1],
            &mut self.composite_signal,
        );

        //
        unsafe {
            upsample(
                self.up_sampled_signal[(self.read_state) as usize]
                    .lock()
                    .unwrap()
                    .deref_mut()
                    .as_mut_slice()
                    .as_mut_ptr(),
                self.composite_signal.as_ptr(),
                // self.audio_in_buffer[0].as_ptr(),
                &raw mut self.upsampler_for_radio_waves,
            );
        }

        // self.demodulator.process(
        //     // &intermediate_signal_out,
        //     &self.intermediate_signal3[self.read_state as usize]
        //         .lock()
        //         .unwrap(),
        //     &mut self.demodulate_signal,
        // );
        // println!("check point2");
        //
        unsafe {
            downsample(
                self.post_down_sample.as_mut_ptr(),
                self.demodulate_signal[(!self.read_state) as usize]
                    .lock()
                    .unwrap()
                    .as_ptr(),
                &raw mut self.downsampler_for_radio_waves,
            );
        }
        self.restore.process(
            &self.post_down_sample,
            // &self.composite_signal
            &mut self.restored_signal_l,
            &mut self.restored_signal_r,
        );
        // down sample
        let _ = self.downsampler[0].process::<f64, f64>(
            Some(&self.restored_signal_l),
            &mut self.tmp_buffer[0],
        );
        let _ = self.downsampler[1].process::<f64, f64>(
            Some(&self.restored_signal_r),
            &mut self.tmp_buffer[1],
        );
        // interleave
        for (i, lr) in dst_l.iter_mut().zip(dst_r.iter_mut()).enumerate() {
            unsafe {
                *lr.0 = *self.tmp_buffer[0].get_unchecked(i) as f32;
                *lr.1 = *self.tmp_buffer[1].get_unchecked(i) as f32;
            }
        }
        // println!("check point3");
        self.read_state ^= true;
        self.barrier.wait();
    }
    pub fn process_serial(
        &mut self,
        input_l: &[f32],
        input_r: &[f32],
        dst_l: &mut [f32],
        dst_r: &mut [f32],
    ) {
        // self.barrier.wait();
        // cvar_3.notify_one();
        // de-interleave
        for (i, lr) in input_l.iter().zip(input_r).enumerate() {
            unsafe {
                *self.tmp_buffer[0].get_unchecked_mut(i) = *lr.0 as f64;
                *self.tmp_buffer[1].get_unchecked_mut(i) = *lr.1 as f64;
            }
        }
        let timer_start = Instant::now();
        // up sample
        let _ = self.upsampler[0].process::<f64, f64>(
            Some(&self.tmp_buffer[0]),
            &mut self.audio_in_buffer[0],
            // &mut self.composite_signal
        );
        let _ = self.upsampler[1].process::<f64, f64>(
            Some(&self.tmp_buffer[1]),
            &mut self.audio_in_buffer[1],
        );
        let lap0 = timer_start.elapsed();
        // composite
        self.composite.process(
            &self.audio_in_buffer[0],
            &self.audio_in_buffer[1],
            &mut self.composite_signal,
        );
        let lap1 = timer_start.elapsed();
        unsafe {
            upsample(
                self.up_sampled_signal[0]
                    .lock()
                    .unwrap()
                    .deref_mut()
                    .as_mut_slice()
                    .as_mut_ptr(),
                self.composite_signal.as_ptr(),
                // self.audio_in_buffer[0].as_ptr(),
                &raw mut self.upsampler_for_radio_waves,
            );
        }
        // println!("check point1");
        //
        let lap2 = timer_start.elapsed();
        self.modulator.lock().unwrap().process(
            &self.up_sampled_signal[0].lock().unwrap(),
            &mut self.modulate_signal[0].lock().unwrap(),
        );
        let lap3 = timer_start.elapsed();
        // super heterodyne
        self.freq_converter.lock().unwrap().process(
            &self.modulate_signal[0].lock().unwrap(),
            &mut self.intermediate_signal1[0].lock().unwrap(),
        );
        let lap4 = timer_start.elapsed();
        self.bandpass_filter1.lock().unwrap().process_no_resample(
            &self.intermediate_signal1[0].lock().unwrap(),
            &mut self.intermediate_signal2[0].lock().unwrap(),
        );
        let lap5 = timer_start.elapsed();
        self.bandpass_filter2.lock().unwrap().process(
            &self.intermediate_signal2[0].lock().unwrap(),
            &mut self.intermediate_signal3[0].lock().unwrap(),
        );
        // de-modulate
        let lap6 = timer_start.elapsed();
        self.demodulator.lock().unwrap().process(
            &self.intermediate_signal3[0].lock().unwrap(),
            &mut self.demodulate_signal[0].lock().unwrap(),
        );
        // println!("check point2");
        //
        let lap7 = timer_start.elapsed();
        unsafe {
            downsample(
                self.post_down_sample.as_mut_ptr(),
                self.demodulate_signal[0].lock().unwrap().as_ptr(),
                &raw mut self.downsampler_for_radio_waves,
            );
        }
        let lap8 = timer_start.elapsed();
        self.restore.process(
            &self.post_down_sample,
            &mut self.restored_signal_l,
            &mut self.restored_signal_r,
        );
        let lap9 = timer_start.elapsed();
        // down sample
        let _ = self.downsampler[0].process::<f64, f64>(
            Some(&self.restored_signal_l),
            &mut self.tmp_buffer[0],
        );
        let _ = self.downsampler[1].process::<f64, f64>(
            Some(&self.restored_signal_r),
            &mut self.tmp_buffer[1],
        );
        let lap10 = timer_start.elapsed();
        // interleave
        for (i, lr) in dst_l.iter_mut().zip(dst_r.iter_mut()).enumerate() {
            unsafe {
                *lr.0 = *self.tmp_buffer[0].get_unchecked(i) as f32;
                *lr.1 = *self.tmp_buffer[1].get_unchecked(i) as f32;
            }
        }
        println!(
            "===============================
up-sampling    : {:?}
composite      : {:?}
up-sampling    : {:?}
modulate       : {:?}
cvt-freq       : {:?}
bandpass-filter1: {:?}
bandpass-filter2: {:?}
demodulate     : {:?}
down-sampling  : {:?}
restore        : {:?}
down-sampling  : {:?}",
            lap0,
            lap1 - lap0,
            lap2 - lap1,
            lap3 - lap2,
            lap4 - lap3,
            lap5 - lap4,
            lap6 - lap5,
            lap7 - lap6,
            lap8 - lap7,
            lap9 - lap8,
            lap10 - lap9,
        );
    }
}
fn generate_resampler(f1: f64, f2: f64) -> libsoxr::Result<Soxr> {
    Soxr::create(
        f1,
        f2,
        1,
        Some(&libsoxr::IOSpec::new(
            Datatype::Float64I,
            Datatype::Float64I,
        )),
        Some(&QualitySpec::new(
            &QualityRecipe::Quick,
            QualityFlags::ROLLOFF_NONE,
        )),
        None,
    )
}

#[inline]
fn get_buffer_size(s1: usize, s2: usize, base_size: usize) -> usize {
    (s2 as f64 / s1 as f64 * base_size as f64 + 0.5).floor() as usize
}
