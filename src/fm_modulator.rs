#[cfg(not(target_arch = "x86_64"))]
use rustfft::{num_complex::Complex, FftPlanner};
#[cfg(target_arch = "x86_64")]
use rustfft::{num_complex::Complex, FftPlannerAvx as FftPlanner};

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
            sample_rate: 79_500_000f64 * 3.,
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
                + self.sample_period / 2. * self.integral)
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
pub struct FmDeModulator {
    t: f64, // 時刻t
    prev_sig: [Complex<f64>; 2],
    sample_rate: f64,
    sample_period: f64,
    carrier_freq: f64,
    carrier_period: f64, // キャリア周期(1/f_c)
    buffer: Vec<Complex<f64>>,
    diff_buffer: Vec<Complex<f64>>,
    sig_buffer: Vec<f32>,
}
impl FmDeModulator {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            prev_sig: [Complex::from(0.0); 2],
            sample_rate: 79_500_000f64 * 3.,
            sample_period: 1. / 79_500_000f64 * 3.,
            carrier_freq: 79_500_000f64,
            carrier_period: 1. / 79_500_000.,
            buffer: Vec::new(),
            diff_buffer: Vec::new(),
            sig_buffer: Vec::new(),
        }
    }
    pub fn from(f: f64, sample_rate: f64) -> Self {
        Self {
            t: 0.0,
            prev_sig: [Complex::from(0.0); 2],
            sample_rate,
            sample_period: 1. / sample_rate,
            carrier_freq: f,
            carrier_period: 1. / f,
            buffer: Vec::new(),
            diff_buffer: Vec::new(),
            sig_buffer: Vec::new(),
        }
    }
    pub fn demodulate(&mut self, signal: &[f32]) {
        if self.buffer.len() != signal.len() {
            self.buffer = vec![Complex::from(0f64); signal.len()];
            self.diff_buffer = vec![Complex::from(0f64); signal.len()];
            self.sig_buffer = vec![0f32; signal.len()];
        }
        for i in 0..signal.len() {
            let theta = self.t * TAU * self.carrier_freq;
            let s = signal[i];
            self.buffer[i] = Complex::new((s as f64) * theta.sin(), (s as f64) * theta.cos());
            self.t += self.sample_period;
        }
        self.differential();
        for i in 0..signal.len() {
            let (d_re, d_im) = {
                let t = self.diff_buffer[i];
                (t.re, t.im)
            };
            let (s_re, s_im) = {
                let t = self.buffer[i];
                (t.re, t.im)
            };
            let a = d_re * s_im;
            let b = d_im * s_re;
            self.sig_buffer[i] = (a - b) as f32 / (self.sample_rate / 2.) as f32 - 1.0;
        }
    }
    fn differential(&mut self) {
        for i in 0..self.buffer.len() {
            self.diff_buffer[i] = (-if i == 0 {
                self.prev_sig[0]
            } else {
                self.buffer[i - 1]
            } + self.buffer[i])
                / self.sample_period;
        }
        self.prev_sig[0] = *self.buffer.last().unwrap();
    }
    pub fn get_buffer(&self) -> &[f32] {
        //   dbg!(&self.sig_buffer);
        //   if !self.sig_buffer.is_empty() {
        //   unreachable!();
        // }

        self.sig_buffer.as_ref()
    }
}
