/**
 * コンポジット信号を作成、復元するコード群
*/
use crate::filter::{Bpf, FilterInfo, Hpf, Lpf};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
use std::f32::consts::{PI, TAU};
pub struct CompositeSignal {
    lpf: Lpf,
    sample_rate: f32,
    buffer: [Vec<f32>; 2], // L,R or L+R, L-R
    out_buffer: Vec<f32>,
    // up_sampler: Option<FastFixedIn<f32>>,
    t: f32,
    filter_info: [FilterInfo; 2],
}
impl CompositeSignal {
    const PILOT_FREQ: f32 = 19_000f32;
    const CARRIER_FREQ: f32 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f32 = 15_000f32;
    pub const DEFAULT_SAMPLE_RATE: f32 =
        (Self::CARRIER_FREQ + Self::CUT_OFF_FREQ) * 3.;
    pub fn new(f: f32, buffer_size: usize) -> Self {
        Self {
            lpf: Lpf::new(f, Self::CUT_OFF_FREQ, Lpf::Q),
            sample_rate:f,
            buffer: [Vec::new(), Vec::new()],
            out_buffer: Vec::new(),
            filter_info: [FilterInfo::default(),FilterInfo::default()],
            t: 0.,
        }
    }
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
    pub fn process(&mut self, l_channel: &[f32], r_channel: &[f32]) {
        if self.buffer[0].len() != l_channel.len() {
            self.buffer[0] = vec![0.0; l_channel.len()];
            self.buffer[1] = vec![0.0; l_channel.len()];
            self.out_buffer = vec![0.0; l_channel.len()];
        }
        // Low Pass
        // let mut filter_info = [FilterInfo::default(), FilterInfo::default()];
        for i in 0..self.buffer[1].len() {
            // let l = self.buffer[0][i];
            // let r = self.buffer[1][i];
            let l = self
                .lpf
                .process_without_buffer(l_channel[i], &mut self.filter_info[0]);
            let r = self
                .lpf
                .process_without_buffer(r_channel[i], &mut self.filter_info[1]);
            let a = l + r;
            let theta = TAU * Self::PILOT_FREQ * self.t;
            let cos = theta.cos();
            let double_sin = cos * theta.sin() * 2.;
            let b = (l - r) * double_sin;
            self.out_buffer[i] = a + b + cos;
            self.t += 1. / self.sample_rate;
        }
        self.t = self.t.rem_euclid(1.);
    }
    pub fn get_buffer(&self) -> &[f32] {
        self.out_buffer.as_slice()
    }
}
pub struct RestoredSignal {
    lpf: Lpf,
    lpf16: Lpf,
    hpf: Hpf,
    sample_rate: f32,
    buffer: [Vec<f32>; 3], // L,R or L+R, L-R
    out_buffer: [Vec<f32>; 2],
    t: f32,
    filter_info: [FilterInfo; 4],
}
impl RestoredSignal {
    const PILOT_FREQ: f32 = 19_000f32;
    const CARRIER_FREQ: f32 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f32 = 15_000f32;
    pub fn new(f: f32) -> Self {
        Self {
            lpf: Lpf::new(f, Self::PILOT_FREQ, Lpf::Q),
            lpf16: Lpf::new(f, 16_000f32, Lpf::Q),
            hpf: Hpf::new(f, Self::CARRIER_FREQ - Self::CUT_OFF_FREQ, Hpf::Q),
            sample_rate: f,
            buffer: [Vec::new(), Vec::new(), Vec::new()],
            out_buffer: [Vec::new(), Vec::new()],
            t: 0.,
            filter_info: [FilterInfo::default(),FilterInfo::default(), FilterInfo::default(), FilterInfo::default()],
        }
    }
    pub fn process(&mut self, signal: &[f32]) {
        if self.buffer[0].len() != signal.len() {
            self.buffer[0] = vec![0.0; signal.len()];
            // self.buffer[1] = vec![0.0; signal.len()];
            // self.buffer[2] = vec![0.0; signal.len()];
            self.out_buffer =
                [vec![0.0; signal.len()], vec![0.0; signal.len()]];
        }
        {
            let mut t = self.t;
            // let mut filter_info = [
            //     FilterInfo::default(),
            //     FilterInfo::default(),
            //     FilterInfo::default(),
            //     FilterInfo::default(),
            // ];

            for i in 0..signal.len() {
                let theta = (TAU * Self::PILOT_FREQ * self.t);
                let cos = theta.cos();
                // let sin = (TAU * Self::CARRIER_FREQ * self.t).sin();
                // 倍角公式によるキャリアの生成
                let sin = 2. * cos * (theta).sin();
                // PILOTの削除
                let buffer = self.lpf.process_without_buffer(
                    -signal[i] * cos,
                    &mut self.filter_info[0],
                );
                let remove_pilot = signal[i] + buffer * cos;
                //  get L+R and L-R with LPF
                let a = self
                    .lpf16
                    .process_without_buffer(remove_pilot, &mut self.filter_info[1]); // L+R
                let b = self.lpf16.process_without_buffer(
                    self.hpf.process_without_buffer(
                        remove_pilot,
                        &mut self.filter_info[3],
                    ) * 2.
                        * sin,
                    &mut self.filter_info[2],
                ); // L-R

                self.out_buffer[0][i] = (a + b) / 2.;
                self.out_buffer[1][i] = (a - b) / 2.;
                self.t += 1. / self.sample_rate;
            }
        }
        self.t = self.t.rem_euclid(1.);
    }
    pub fn get_buffer(&self) -> &[Vec<f32>] {
        self.out_buffer.as_slice()
    }
}
