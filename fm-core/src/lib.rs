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
        sample_period: f64,
        fc: f64,
        fi: f64,
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
}

pub struct FmRadioSim {
    // basic parameters
    audio_sample_rate: usize,
    buffer_size: usize,
    // Simulation Modules
    composite: composite::CompositeSignal,
    restore: composite::RestoreSignal,
    modulator: Shareable<modulator::Modulator>,
    demodulator: modulator::DeModulator,
    freq_converter: Shareable<modulator::CvtIntermediateFreq>,
    bandpass_filter: Shareable<modulator::BandPassFilter>,
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
    intermediate_signal: PipeLineBuffer, // 192MHz (inner 384MHz)
    intermediate_signal_out: PipeLineBuffer, // 48MHz
    // demodulate_signal: PipeLineBuffer, // 48MHz
    demodulate_signal: Vec<f64>,
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
    pub const FM_MODULATION_SAMPLE_RATE: usize = 192_000_000;
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
            bandpass_filter: sharable!(modulator::BandPassFilter::new(
                fm_sample_rate as f64,
                Self::INTERMEDIATE_FREQ,
            )),
            demodulator: modulator::DeModulator::from(
                Self::INTERMEDIATE_FREQ,
                intermediate_fs as f64,
                // 880.
                Self::SIGNAL_MAX_FREQ,

            ),
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
            intermediate_signal: generate_pipline_buffer(modulated_buffer_size),
            intermediate_signal_out: generate_pipline_buffer(
                intermediate_buffer_size,
            ),
            // demodulate_signal: generate_pipline_buffer(
            //     intermediate_buffer_size,
            // ),
            demodulate_signal: vec![0.; dbg!(intermediate_buffer_size)],
            post_down_sample: vec![0.; composite_buffer_size],
            restored_signal_l: vec![0.; composite_buffer_size],
            restored_signal_r: vec![0.; composite_buffer_size],
            //
            read_state: true,
            barrier: Arc::new(Barrier::new(4)),
            // barrier: Arc::new(Barrier::new(3)),
            is_init: false,
        }
    }
    pub fn get_intermediate(&self) -> &[f64] {
        if self.read_state {
            let array = (self.intermediate_signal_out[0]).lock().unwrap();
            unsafe { std::slice::from_raw_parts(array.as_ptr(), array.len()) }
        } else {
            let array = (self.intermediate_signal_out[1]).lock().unwrap();
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
      &self.demodulate_signal
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
        // let modulate_counter = Arc::new(Mutex::new(0));
        // Buffer
        let up_sample_signal_outer1 = Arc::clone(&self.up_sampled_signal);
        let modulate_signal_outer1 = Arc::clone(&self.modulate_signal);
        let intermediate_signal_outer1 = Arc::clone(&self.intermediate_signal);
        let modulate_signal_outer2 = Arc::clone(&self.modulate_signal);
        // let intermediate_signal_outer2 = Arc::clone(&self.intermediate_signal);
        // let intermediate_signal_out1 =
        //     Arc::clone(&self.intermediate_signal_out);
        // Modules
        let modulator = Arc::clone(&self.modulator);
        let freq_converter = Arc::clone(&self.freq_converter);
        let bandpass_filter = Arc::clone(&self.bandpass_filter);
        let listener0 = Arc::clone(&self.barrier);
        let listener1 = Arc::clone(&self.barrier);
        let listener2 = Arc::clone(&self.barrier);
        // Modulation Process
        {let _ = thread::spawn(move || {
            let mut counter = 0;
            let up_sample_signal = Arc::clone(&up_sample_signal_outer1);
            let modulate_signal = Arc::clone(&modulate_signal_outer1);

            loop {
                listener0.wait();
                let (read_buffer, mut write_buffer) = if counter & 1 == 0 {
                  println!("modulation| read: 1 / write: 0");
                    (
                        up_sample_signal[1].lock().unwrap(),
                        modulate_signal[0].lock().unwrap(),
                    )
                } else {
                  println!("modulation| read: 0 / write: 1");
                    (
                        up_sample_signal[0].lock().unwrap(),
                        modulate_signal[1].lock().unwrap(),
                    )
                };
                modulator
                    .lock()
                    .unwrap()
                    .process(&read_buffer, &mut write_buffer);
                // println!("hoge");
                counter += 1;
            }
        });}
        // Convert-fi Process
        {let _ = thread::spawn(move || {
            let mut counter = 0;
            let intermediate_signal = Arc::clone(&intermediate_signal_outer1);
            let modulate_signal = Arc::clone(&modulate_signal_outer2);

            loop {
                listener1.wait();
                let (read_buffer, mut write_buffer) = if counter & 1 == 0 {
                  println!("cvt-freq | read: 1 / write: 0");
                    (
                        modulate_signal[1].lock().unwrap(),
                        intermediate_signal[0].lock().unwrap(),
                    )
                } else {
                  println!("cvt-freq | read: 0 / write: 1");
                    (
                        modulate_signal[0].lock().unwrap(),
                        intermediate_signal[1].lock().unwrap(),
                    )
                };
                freq_converter
                    .lock()
                    .unwrap()
                    .process(&read_buffer, &mut write_buffer);
                counter += 1;
                // println!("fuga");
            }
        });}
        // // BPF
        // Note:個々がバグの原因。
        // let intermediate_signal = if self.read_state {
        //   &self.intermediate_signal[1].lock().unwrap()
        // } else {
        //   &self.intermediate_signal[0].lock().unwrap()
        // };
        {let intermediate_signal_outer2 = Arc::clone(&self.intermediate_signal);
        let intermediate_signal_out_outer1 = Arc::clone(&self.intermediate_signal_out);
        let _ = thread::spawn(move || {
            let mut counter = 0;
            let intermediate_signal = Arc::clone(&intermediate_signal_outer2);
            let intermediate_signal_out = Arc::clone(&intermediate_signal_out_outer1);

            loop {
                listener2.wait();
                // let start = Instant::now();
                let (read_buffer, mut write_buffer) = if counter & 1 == 0 {
                  println!("bpf | read: 1 / write: 0");
                    (
                        intermediate_signal[1].lock().unwrap(),
                        intermediate_signal_out[0].lock().unwrap(),
                    )
                } else {
                  println!("bpf | read: 0 / write: 1");
                    (
                        intermediate_signal[0].lock().unwrap(),
                        intermediate_signal_out[1].lock().unwrap(),
                    )
                };
                bandpass_filter
                    .lock()
                    .unwrap()
                    .process(&read_buffer, &mut write_buffer);
                counter += 1;
                // println!("{:?}",start.elapsed());
            }
        });}
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
              self.up_sampled_signal[(!self.read_state) as usize].lock().unwrap().deref_mut().as_mut_slice().as_mut_ptr(),
                self.composite_signal.as_ptr(),
                // self.audio_in_buffer[0].as_ptr(),
                &raw mut self.upsampler_for_radio_waves,
            );
        }

        self.demodulator.process(
            // &intermediate_signal_out,
            &self.intermediate_signal_out[dbg!(self.read_state as usize)].lock().unwrap(),
            &mut self.demodulate_signal,
        );
        // println!("check point2");
        //
        unsafe {
            downsample(
                self.post_down_sample.as_mut_ptr(),
                self.demodulate_signal.as_ptr(),
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
            &mut self.intermediate_signal[0].lock().unwrap(),
        );
        let lap4 = timer_start.elapsed();
        self.bandpass_filter.lock().unwrap().process(
            &self.intermediate_signal[0].lock().unwrap(),
            &mut self.intermediate_signal_out[1].lock().unwrap(),
        );
        // de-modulate
        let lap5 = timer_start.elapsed();
        self.demodulator.process(
            &self.intermediate_signal_out[1].lock().unwrap(),
            &mut self.demodulate_signal,
        );
        // println!("check point2");
        //
        let lap6 = timer_start.elapsed();
        unsafe {
            downsample(
                self.post_down_sample.as_mut_ptr(),
                self.demodulate_signal.as_ptr(),
                &raw mut self.downsampler_for_radio_waves,
            );
        }
        let lap7 = timer_start.elapsed();
        self.restore.process(
            &self.post_down_sample,
            &mut self.restored_signal_l,
            &mut self.restored_signal_r,
        );
        let lap8 = timer_start.elapsed();
        // down sample
        let _ = self.downsampler[0].process::<f64, f64>(
            Some(&self.restored_signal_l),
            &mut self.tmp_buffer[0],
        );
        let _ = self.downsampler[1].process::<f64, f64>(
            Some(&self.restored_signal_r),
            &mut self.tmp_buffer[1],
        );
        let lap9 = timer_start.elapsed();
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
bandpass-filter: {:?}
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
