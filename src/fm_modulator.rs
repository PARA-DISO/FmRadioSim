use std::f64::consts::{PI, TAU};
pub type SampleType = f32;
use crate::filter::{FilterInfo, Lpf};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
pub struct FmModulator {
    integral: f64, // int_{0}^{t} x(\tau) d\tau ( 符号拡張)
    t: f64,        // 時刻t
    prev_sig: f32,
    sample_rate: f64,
    sample_period: f64,
    carrier_freq: f64,
    carrier_period: f64, // キャリア周期(1/f_c)
    buffer: Vec<SampleType>,
    modulation_index: f64,
    up_sampler: Option<FastFixedIn<f32>>,
}
impl FmModulator {
    pub fn new() -> Self {
        Self {
            integral: 0.0,
            t: 0.0,
            prev_sig: 0.0,
            modulation_index: 1.,
            sample_rate: (79_500_000f64 + 500_000f64) * 2.,
            sample_period: 1. / 79_500_000f64 * 3.,
            carrier_freq: 79_500_000f64,
            carrier_period: 1. / 79_500_000.,
            buffer: Vec::new(),
            up_sampler: None,
        }
    }
    pub fn from(f: f64, sample_rate: f64) -> Self {
        Self {
            integral: 0.0,
            t: 0.0,
            prev_sig: 0.0,
            modulation_index: 5.,
            sample_rate,
            sample_period: 1. / sample_rate,
            carrier_freq: f,
            carrier_period: 1. / f,
            buffer: Vec::new(),
            up_sampler: None,
        }
    }
    pub fn modulate(&mut self, signal: &[f32]) -> &[f32] {
        if self.buffer.len() != signal.len() {
            self.buffer = vec![0f32; signal.len()];
        }
        for i in 0..signal.len() {
            self.integral += if i == 0 {
                self.prev_sig + signal[i]
            } else {
                signal[i - 1] + signal[i]
            } as f64;
            self.buffer[i] = (self.t
                + self.modulation_index * self.sample_period / 2.
                    * self.integral)
                .cos() as f32;
            self.t += TAU * self.carrier_freq * self.sample_period;
        }
        self.prev_sig = *(signal.last().unwrap());

        self.t = self.t.rem_euclid(TAU);
        &self.buffer
    }
    pub fn get_buffer(&self) -> &[f32] {
        self.buffer.as_ref()
    }
}
// const TAPS: usize = 128;
const CUT_OFF: f32 = 70_000f32;
pub struct FmDeModulator {
    t: f64, // 時刻t
    prev_sig: [f32; 2],
    sample_rate: f64,
    sample_period: f64,
    carrier_freq: f64,
    carrier_period: f64, // キャリア周期(1/f_c)
    buffer: [Vec<f32>; 2],
    diff_buffer: [Vec<f32>; 2],
    sig_buffer: Vec<f32>,
    result_filter: Lpf,
    input_filter: Lpf,
    filter_info: [FilterInfo; 3],
}
impl FmDeModulator {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            prev_sig: [0.0; 2],
            sample_rate: 79_500_000f64 * 3.,
            sample_period: 1. / 79_500_000f64 * 3.,
            carrier_freq: 79_500_000f64,
            carrier_period: 1. / 79_500_000.,
            buffer: [Vec::new(), Vec::new()],
            diff_buffer: [Vec::new(), Vec::new()],
            sig_buffer: Vec::new(),
            result_filter: Lpf::new(79_500_000. * 3., CUT_OFF, Lpf::Q),
            input_filter: Lpf::new(79_500_000. * 3. as f32, (79_500_000 + 500_000) as f32, Lpf::Q),
            filter_info: [FilterInfo::default(),FilterInfo::default(), FilterInfo::default()],
        }
    }
    pub fn from(f: f64, sample_rate: f64, input_cut: f64) -> Self {
        Self {
            t: 0.0,
            prev_sig: [0.0; 2],
            sample_rate,
            sample_period: 1. / sample_rate,
            carrier_freq: f,
            carrier_period: 1. / f,
            buffer: [Vec::new(), Vec::new()],
            diff_buffer: [Vec::new(), Vec::new()],
            sig_buffer: Vec::new(),
            result_filter: Lpf::new(sample_rate as f32, CUT_OFF, Lpf::Q),
            input_filter: Lpf::new(sample_rate as f32, input_cut as f32, Lpf::Q),
            filter_info: [FilterInfo::default(),FilterInfo::default(), FilterInfo::default()],
        }
    }
    pub fn demodulate(&mut self, signal: &[f32]) {
        if self.buffer[0].len() != signal.len() {
            self.buffer = [vec![0f32; signal.len()], vec![0f32; signal.len()]];
            self.diff_buffer =
                [vec![0f32; signal.len()], vec![0f32; signal.len()]];
            self.sig_buffer = vec![0f32; signal.len()];
        }
        // let mut filter_info = [FilterInfo::default(),FilterInfo::default(), FilterInfo::default()];
        for i in 0..signal.len() {
            let s = self.input_filter.process_without_buffer(signal[i],&mut self.filter_info[0]);
            self.buffer[0][i] = self.result_filter.process_without_buffer(
                ((s as f64) * (self.t).sin()) as f32,
                &mut self.filter_info[1],
            );
            self.buffer[1][i] = self.result_filter.process_without_buffer(
                ((s as f64) * (self.t).cos()) as f32,
                &mut self.filter_info[2],
            );
            self.t += self.sample_period * TAU * self.carrier_freq;
        }
        // self.low_pass.process(&mut self.buffer[0]);
        // self.low_pass.process(&mut self.buffer[1]);

        self.differential();
        // self.differential(v1,v2);
        for i in 0..signal.len() {
            let (d_re, d_im) =
                { (self.diff_buffer[0][i], self.diff_buffer[1][i]) };
            let (s_re, s_im) = { (self.buffer[0][i], self.buffer[1][i]) };
            let a = d_re * s_im;
            let b = d_im * s_re;
            self.sig_buffer[i] = (a - b);
        }
        self.t = self.t.rem_euclid(TAU);
    }
    fn differential(&mut self) {
        let v1 = &self.buffer[0];
        let v2 = &self.buffer[1];
        for i in 0..v1.len() {
            // 実部
            self.diff_buffer[0][i] =
                (-if i == 0 { self.prev_sig[0] } else { v1[i - 1] } + v1[i])
                    / self.sample_period as f32;
            // 虚部
            self.diff_buffer[1][i] =
                (-if i == 0 { self.prev_sig[1] } else { v2[i - 1] } + v2[i])
                    / self.sample_period as f32;
        }

        self.prev_sig[0] = *v1.last().unwrap();
        self.prev_sig[1] = *v2.last().unwrap();
    }
    pub fn get_buffer(&self) -> &[f32] {
        self.sig_buffer.as_ref()
    }
}
