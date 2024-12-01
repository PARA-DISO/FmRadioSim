use std::f64::consts::TAU;
// use iced::widget::shader::wgpu::naga::back::msl::sampler::Filter;

// pub type SampleType = f32;
use super::filter::{fast_filter, Bpf, FilterInfo, Lpf};

#[repr(C)]
#[derive(Default)]
pub struct CnvFiInfos {
    angle: [f64; 4],
    delta_angle: f64,
    prev_sig: [f64; 4],
    prev_cos: [f64; 4],
    next_cos: [f64; 4],
    stage: [f64; 16],
    filter_coeff: f64,
    // filter_coeff: Lpf,
    filter_info: [f64; 16],
}
impl CnvFiInfos {
    pub fn new(fs: f64, delta_angle: f64, cut_off: f64) -> Self {
        Self {
            filter_coeff: dbg!(fast_filter::get_lpf_coeff(
                dbg!(fs),
                dbg!(cut_off)
            )),
            // filter_coeff: Lpf::new(fs,cut_off,Lpf::Q),
            // angle: [-delta_angle, 0., delta_angle, 2. * delta_angle],
            angle: [0.,delta_angle,2.* delta_angle,3.* delta_angle],
            delta_angle,
            ..Default::default()
        }
    }
}
pub struct CvtIntermediateFreq {
    fc1: f64,
    fc2: f64,
    sample_periodic: f64,
    info: CnvFiInfos,
}

#[repr(C)]
#[derive(Default)]
pub struct DemodulationInfo {
    angle: [f64; 4],
    prev_sin: [f64; 4],
    prev_sig: [f64; 8],
    prev_internal: [f64; 8],
    filter_coeff: Lpf,
    // filter_info: [FilterInfo; 6],
    // filter_coeff: f64,
    filter_info: [f64; 16],
}
impl DemodulationInfo {
    pub fn new(fs: f64, fc: f64, cutoff: f64) -> Self {
        let delta_angle = dbg!(TAU * dbg!(fc) * (1. / fs));
        Self {
            angle: [0., delta_angle, 2. * delta_angle, 3. * delta_angle],
            filter_coeff: Lpf::new(fs, cutoff, Lpf::Q),
            // filter_coeff: fast_filter::get_lpf_coeff(fs, cutoff),
            ..Default::default()
        }
    }
}

impl CvtIntermediateFreq {
    pub fn new(fs: f64, fc1: f64, fc2: f64) -> Self {
        println!("fs: {}, fc: {}, fi: {}", fs, fc1, fc2);
        Self {
            fc1,
            fc2,
            sample_periodic: 1. / fs,
            info: CnvFiInfos::new(
                fs * 2.,
                1. / fs * TAU * (dbg!(fc1 - fc2)),
                fc2 * 2.,
            ),
        }
    }
    pub fn process(&mut self, input: &[f64], dst: &mut [f64]) {
        unsafe {
            crate::convert_intermediate_freq(
                dst.as_mut_ptr(),
                input.as_ptr(),
                self.sample_periodic,
                self.fc1,
                self.fc2,
                &raw mut self.info,
                input.len(),
            );
        }
    }
}

pub struct Modulator {
    integral: f64, // int_{0}^{t} x(\tau) d\tau ( 符号拡張)
    // t: f64,        // 時刻t
    t: [f64; 4], // 時刻t
    prev_sig: f64,
    sample_period: f64,
    carrier_freq: f64,
    modulation_index: f64,
}
impl Modulator {
    // pub fn new() -> Self {
    //     Self {
    //         integral: 0.0,
    //         t: 0.0,
    //         prev_sig: 0.0,
    //         modulation_index: 1.,
    //         // sample_rate: (79_500_000f64 + 500_000f64) * 2.,
    //         sample_period: 1. / 79_500_000f64 * 3.,
    //         carrier_freq: 79_500_000f64,
    //         // buffer: Vec::new(),
    //     }
    // }
    pub fn from(f: f64, sample_rate: f64) -> Self {
        let sample_period = 1. / sample_rate;
        Self {
            integral: 0.0,
            // t: 0.0,
            t: [
                0.0,
                TAU * f * sample_period,
                TAU * f * sample_period * 2.,
                TAU * f * sample_period * 3.,
            ],
            prev_sig: 0.0,
            // modulation_index: 47./53.,
            modulation_index: 47. / 53.,
            // sample_rate,
            sample_period,
            carrier_freq: f,
        }
    }
    pub fn process(&mut self, signal: &[f64], buffer: &mut [f64]) {
        // for i in 0..signal.len() {
        //     self.integral += self.prev_sig + signal[i];
        //     self.prev_sig = signal[i];
        //     buffer[i] = (self.t[0]
        //         + self.modulation_index * self.sample_period / 2.
        //             * self.integral)
        //         .cos();
        //     self.t[0] += TAU * self.carrier_freq * self.sample_period;
        // }
        // self.prev_sig = *(signal.last().unwrap());

        // self.t[0] = self.t[0].rem_euclid(TAU);
        unsafe {
            crate::fm_modulate(
                buffer.as_mut_ptr(),
                signal.as_ptr(),
                &raw mut self.prev_sig,
                &raw mut self.integral,
                self.sample_period,
                self.t.as_mut_ptr(),
                self.modulation_index,
                self.carrier_freq,
                buffer.len() as u64,
            )
        };
    }
}
pub struct DeModulator {
    // t: f64, // 時刻t
    // prev_sig: [f64; 2],
    info: DemodulationInfo,
    sample_period: f64,
    carrier_freq: f64,
    // result_filter: Lpf,
    // filter_info: [FilterInfo; 4],
}
impl DeModulator {
    pub fn from(f: f64, sample_rate: f64, cut_off: f64) -> Self {
        // println!("periodic: {}", (1. / sample_rate));
        // println!("carrier : cutoff = {}",f / cut_off);
        println!("fs/2fc = {}", sample_rate / f);
        Self {
            // t: 0.0,
            // prev_sig: Default::default(),
            info: DemodulationInfo::new(sample_rate, f, cut_off),
            // sample_rate,
            sample_period: (1. / sample_rate),
            carrier_freq: f,
            // result_filter: Lpf::new(sample_rate, cut_off, Lpf::Q),
            // filter_info: Default::default(),
        }
    }
    pub fn process(&mut self, signal: &[f64], buffer: &mut [f64]) {
        unsafe {
            crate::fm_demodulate(
                buffer.as_mut_ptr(),
                signal.as_ptr(),
                self.sample_period,
                self.carrier_freq,
                &raw mut self.info,
                buffer.len() as u64,
            );
        }
    }
}

#[derive(Default)]
#[repr(C)]
pub struct BandPassFilter {
    prev_sig: [f64; 2],
    prev_prev_sig: [f64; 2],
    prev_out: [f64; 2],
    prev_prev_out: [f64; 2],
    stage: [f64; 4],
    filter_coeff: Bpf,
}
impl BandPassFilter {
    const BAND_WIDTH: f64 = 0.2; // +- 124kHz when fc = 10.7MHz
    pub fn new(fs: f64, cutoff: f64) -> Self {
        Self {
            filter_coeff: Bpf::new(fs, cutoff, Self::BAND_WIDTH),
            ..Default::default()
        }
    }
    pub fn process(&mut self, input: &[f64], dst: &mut [f64]) {
        unsafe {
            crate::filtering(
                dst.as_mut_ptr(),
                input.as_ptr(),
                self as *mut Self,
                input.len() as u64,
            )
        }
    }
}

// mod fm_sys {
//   use std::ffi::c_void;
//   extern "C" {
//       pub fn fm_modulate(
//           output_signal: *mut f64,
//           input_signal: *const f64,
//           prev_sig: *mut f64,
//           sum: *mut f64,
//           sample_periodic: f64,
//           angle: *mut f64,
//           modulate_index: f64,
//           fc: f64,
//           buf_len: u64,
//       );
//       pub fn fm_demodulate(
//           output_signal: *mut f64,
//           input_signal: *const f64,
//           sample_period: f64,
//           carrier_freq: f64,
//           info: *mut crate::fm_modulator::DemodulationInfo,
//           buf_len: u64,
//       );
//       pub fn convert_intermediate_freq(
//           output_signal: *mut f64,
//           input_signal: *const f64,
//           sample_period: f64,
//           fc: f64,
//           fi: f64,
//           info: *mut crate::fm_modulator::CnvFiInfos,
//           buf_len: usize,
//       );
//       pub fn filtering(
//         output_signal: *mut f64,
//         input_signal: *const f64,
//         filter_info: *mut crate::fm_modulator::Filtering,
//         buf_len: u64,
//       );
//   }
// }
