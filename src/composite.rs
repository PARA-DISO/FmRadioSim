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
    up_sampler: Option<FastFixedIn<f32>>,
    t: f32,
}
impl CompositeSignal {
    const PILOT_FREQ: f32 = 19_000f32;
    const CARRIER_FREQ: f32 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f32 = 15_000f32;
    const DEFAULT_SAMPLE_RATE: f32 = (Self::CARRIER_FREQ + Self::CUT_OFF_FREQ) * 3.;
    pub fn new(f: f32, buffer_size: usize) -> Self {
        Self {
            lpf: Lpf::new(f, Self::CUT_OFF_FREQ, Lpf::Q),
            sample_rate: if f < Self::DEFAULT_SAMPLE_RATE {
              Self::DEFAULT_SAMPLE_RATE
            } else {
              f
            },
            buffer: [Vec::new(), Vec::new()],
            out_buffer: Vec::new(),
            up_sampler: if f < Self::DEFAULT_SAMPLE_RATE {
                Some(
                    FastFixedIn::new(
                        (Self::DEFAULT_SAMPLE_RATE / f) as f64,
                        (Self::DEFAULT_SAMPLE_RATE / f) as f64,
                        PolynomialDegree::Linear,
                        buffer_size,
                        2,
                    )
                    .unwrap(),
                )
            } else {
                None
            },
            t: 0.,
        }
    }
    pub fn  sample_rate(&self) ->f32 {
      self.sample_rate
    }
    pub fn process(&mut self, l_channel: &[f32], r_channel: &[f32]) {
        if self.up_sampler.is_none() {
            if self.buffer[0].len() != l_channel.len() {
                self.buffer[0] = vec![0.0; l_channel.len()];
                self.buffer[1] = vec![0.0; l_channel.len()];
                self.out_buffer = vec![0.0; l_channel.len()];
            }
            self.lpf.process_with_buffer(&mut self.buffer[0], l_channel);
            self.lpf.process_with_buffer(&mut self.buffer[1], r_channel);
        } else {
            let up_sampler = self.up_sampler.as_mut().unwrap();
            if self.buffer[0].len() != up_sampler.output_frames_next() {
                self.buffer[0] = vec![0.0; up_sampler.output_frames_next()];
                self.buffer[1] = vec![0.0; up_sampler.output_frames_next()];
                self.out_buffer = vec![0.0; up_sampler.output_frames_next()];
            }
            let _ = up_sampler.process_into_buffer(&[l_channel, r_channel], &mut self.buffer, None);
            self.lpf.process(&mut self.buffer[0]);
            self.lpf.process(&mut self.buffer[1]);
        }

        // Low Pass

        for i in 0..self.buffer[1].len() {
            let l = self.buffer[0][i];
            let r = self.buffer[1][i];
            let a = l + r;
            // let b = (l - r) * (TAU * Self::CARRIER_FREQ * self.t).sin();
            // self.out_buffer[i] = a + b + (TAU * Self::PILOT_FREQ * self.t).sin();
            let b = (l - r) * (TAU * Self::CARRIER_FREQ * self.t).sin();
            self.out_buffer[i] = a + b + (TAU * Self::PILOT_FREQ * self.t).cos();
            self.t += 1. / self.sample_rate;
        }
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
}
impl RestoredSignal {
    const PILOT_FREQ: f32 = 19_000f32;
    const CARRIER_FREQ: f32 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f32 = 15_000f32;
    pub fn new(f: f32) -> Self {
        Self {
            lpf: Lpf::new(f, Self::PILOT_FREQ, Lpf::Q),
            lpf16: Lpf::new(f, 16_000f32, Lpf::Q),
            hpf: Hpf::new(f, Self::PILOT_FREQ, Hpf::Q),
            sample_rate: f,
            buffer: [Vec::new(), Vec::new(), Vec::new()],
            out_buffer: [Vec::new(), Vec::new()],
            t: 0.,
        }
    }
    pub fn process(&mut self, signal: &[f32]) {
        if self.buffer[0].len() != signal.len() {
            self.buffer[0] = vec![0.0; signal.len()];
            // self.buffer[1] = vec![0.0; signal.len()];
            // self.buffer[2] = vec![0.0; signal.len()];
            self.out_buffer = [vec![0.0; signal.len()], vec![0.0; signal.len()]];
        }
        {
            let mut t = self.t;
            let mut filter_info = FilterInfo::default();
            for i in 0..signal.len() {
                let cos = (TAU * Self::PILOT_FREQ * t).cos();
                // let sin_39 = -(TAU * Self::CARRIER_FREQ * self.t).sin();
                // PILOTの削除
                (self.buffer[0][i], filter_info) = self
                    .lpf
                    .process_without_buffer(-signal[i] * cos, filter_info);
                t += 1. / self.sample_rate;
            }
        }
        // self.lpf.process(&mut self.buffer[0]);

        // let mut t = self.t;
        {
            for i in 0..signal.len() {
                let cos_19 = (TAU * Self::PILOT_FREQ * self.t).cos();
                let sin_38 = (TAU * Self::CARRIER_FREQ * self.t).sin();
                // PILOTの削除
                let remove_plot = (signal[i] + (self.buffer[0][i]) * cos_19);
                // let (filtered_sig, tmp) = self.hpf.process_without_buffer(remove_plot, filter_info);

                self.out_buffer[0][i] = remove_plot; // L+R
                self.out_buffer[1][i] = remove_plot * 2. * sin_38; // L-R
                self.t += 1. / self.sample_rate;
                // filter_info = tmp;
            }
        }
        self.lpf16.process(&mut self.out_buffer[0]);
        self.lpf16.process(&mut self.out_buffer[1]);
        for i in 0..signal.len() {
            // LR取り出し
            let a = self.out_buffer[0][i];
            let b = self.out_buffer[1][i];

            self.out_buffer[0][i] = a + b;
            self.out_buffer[1][i] = a - b;
        }
    //     self.lpf16.process(&mut self.out_buffer[0]);
    //     self.lpf16.process(&mut self.out_buffer[1]);
    }

    pub fn get_buffer(&self) -> &[Vec<f32>] {
        self.out_buffer.as_slice()
    }
}
