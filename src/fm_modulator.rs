// #[cfg(not(target_arch = "x86_64"))]
// use rustfft::{num_complex::Complex, FftPlanner};
// #[cfg(target_arch = "x86_64")]
// use rustfft::{num_complex::Complex, FftPlannerAvx as FftPlanner};
use sdr::fir::FIR;
use std::f64::consts::{PI, TAU};
pub type SampleType = f32;

pub struct FmModulator {
    integral: f64, // int_{0}^{t} x(\tau) d\tau ( 符号拡張)
    t: f64,        // 時刻t
    prev_sig: f32,
    sample_rate: f64,
    sample_period: f64,
    carrier_freq: f64,
    carrier_period: f64, // キャリア周期(1/f_c)
    buffer: Vec<SampleType>,
}
impl FmModulator {
    pub fn new() -> Self {
        Self {
            integral: 0.0,
            t: 0.0,
            prev_sig: 0.0,
            sample_rate: (79_500_000f64 + 500_000f64) * 2.,
            sample_period: 1. / 79_500_000f64 * 3.,
            carrier_freq: 79_500_000f64,
            carrier_period: 1. / 79_500_000.,
            buffer: Vec::new(),
        }
    }
    pub fn from(f: f64, sample_rate: f64) -> Self {
        Self {
            integral: 0.0,
            t: 0.0,
            prev_sig: 0.0,
            sample_rate,
            sample_period: 1. / sample_rate,
            carrier_freq: f,
            carrier_period: 1. / f,
            buffer: Vec::new(),
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
            self.buffer[i] = (TAU * self.carrier_freq * self.t
                +  self.sample_period / 2. * self.integral)
                .cos() as f32;
            self.t += self.sample_period;
        }
        self.prev_sig = *(signal.last().unwrap());
        &self.buffer
    }
    pub fn get_buffer(&self) -> &[f32] {
        // self.buffer.iter().map(|x| x.re).collect()
        self.buffer.as_ref()
    }
}
const TAPS: usize = 128;
const CUT_OFF: f64 = 15_000f64;
pub struct FmDeModulator {
    t: f64, // 時刻t
    prev_sig: [f32; 2],
    sample_rate: f64,
    sample_period: f64,
    carrier_freq: f64,
    carrier_period: f64, // キャリア周期(1/f_c)
    buffer: [Vec<f32>;2],
    diff_buffer: [Vec<f32>;2],
    sig_buffer: Vec<f32>,
    lpf: FIR<f32>,
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
            buffer: [Vec::new(),Vec::new()],
            diff_buffer: [Vec::new(),Vec::new()],
            sig_buffer: Vec::new(),
            lpf: FIR::lowpass(TAPS,0.1),
        }
    }
    pub fn from(f: f64, sample_rate: f64) -> Self {
        Self {
            t: 0.0,
            prev_sig: [0.0; 2],
            sample_rate,
            sample_period: 1. / sample_rate,
            carrier_freq: f,
            carrier_period: 1. / f,
            buffer: [Vec::new(),Vec::new()],
            diff_buffer: [Vec::new(),Vec::new()],
            sig_buffer: Vec::new(),
            lpf: FIR::lowpass(TAPS,dbg!(0.25)),
        }
    }
    pub fn demodulate(&mut self, signal: &[f32]) {
        if self.buffer[0].len() != signal.len() {
            self.buffer = [vec![0f32; signal.len()],vec![0f32; signal.len()]];
            self.diff_buffer = [vec![0f32; signal.len()],vec![0f32; signal.len()]];
            self.sig_buffer = vec![0f32; signal.len()];
        }
        for i in 0..signal.len() {
            let theta = self.t * TAU * self.carrier_freq;
            let s = signal[i];
            self.buffer[0][i] = ((s as f64) * theta.sin()) as f32;
            self.buffer[1][i] = ((s as f64) * theta.cos()) as f32;
            self.t += self.sample_period;
        }
        let v1 = self.lpf.process(&self.buffer[0]);
        let v2 = (self.lpf.process(&self.buffer[1]));
        // let v1 = self.buffer[0].clone();
        // let v2 = self.buffer[1].clone();
        self.differential(&v1,&v2);
        // self.differential(v1,v2);
        for i in 0..signal.len() {
            let (d_re, d_im) = {
                (self.diff_buffer[0][i], self.diff_buffer[1][i])
            };
            let (s_re, s_im) = {
                (v1[i],v2[i])
            };
            let a = d_re * s_im;
            let b = d_im * s_re;
            self.sig_buffer[i] = (a - b) / (self.sample_rate as f32 / 2.) as f32 - 1.0;
            self.sig_buffer[i] = (a - b)/ (self.sample_rate as f32 / 8.) -1.;
            //  / (self.sample_rate / 2.) as f32 - 1.0
        }
    }
    fn differential(&mut self, v1: &[f32], v2: &[f32]) {
        for i in 0..v1.len() {
            // 実部
            self.diff_buffer[0][i] = (-if i == 0 {
                self.prev_sig[0]
            } else {
                v1[i - 1]
            } + v1[i])
                / self.sample_period as f32;
          // 虚部
          self.diff_buffer[1][i] = (-if i == 0 {
                self.prev_sig[1]
            } else {
                v2[i - 1]
            } + v2[i])
                / self.sample_period as f32;
        }

        self.prev_sig[0] = *v1.last().unwrap();
        self.prev_sig[1] = *v2.last().unwrap();
    }
    pub fn get_buffer(&self) -> &[f32] {
        //   dbg!(&self.sig_buffer);
        //   if !self.sig_buffer.is_empty() {
        //   unreachable!();
        // }

        self.sig_buffer.as_ref()
    }
}
