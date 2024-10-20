use std::f64::consts::TAU;
// pub type SampleType = f32;
use crate::filter::{FilterInfo, Lpf};
pub struct FmModulator {
    integral: f64, // int_{0}^{t} x(\tau) d\tau ( 符号拡張)
    t: f64,        // 時刻t
    prev_sig: f64,
    sample_period: f64,
    carrier_freq: f64,
    modulation_index: f64,
}
impl FmModulator {
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
        Self {
            integral: 0.0,
            t: 0.0,
            prev_sig: 0.0,
            // modulation_index: 47./53.,
            modulation_index: 47. / 53.,
            // sample_rate,
            sample_period: 1. / sample_rate,
            carrier_freq: f,
            // buffer: Vec::new(),
        }
    }
    pub fn process_to_buffer(&mut self, signal: &[f64], buffer: &mut [f64]) {
        for i in 0..signal.len() {
            self.integral += if i == 0 {
                self.prev_sig + signal[i]
            } else {
                signal[i - 1] + signal[i]
            };
            buffer[i] = (self.t
                + self.modulation_index * self.sample_period / 2.
                    * self.integral)
                .cos();
            self.t += TAU * self.carrier_freq * self.sample_period;
        }
        self.prev_sig = *(signal.last().unwrap());

        self.t = self.t.rem_euclid(TAU);
    }
}
pub struct FmDeModulator {
    t: f64, // 時刻t
    prev_sig: [f64; 2],
    sample_period: f64,
    carrier_freq: f64,
    result_filter: Lpf,
    filter_info: [FilterInfo; 4],
}
impl FmDeModulator {
    pub fn from(f: f64, sample_rate: f64, cut_off: f64) -> Self {
        // println!("periodic: {}", (1. / sample_rate));
        // println!("carrier : cutoff = {}",f / cut_off);
        Self {
            t: 0.0,
            prev_sig: Default::default(),
            // sample_rate,
            sample_period: (1. / sample_rate),
            carrier_freq: f,
            result_filter: Lpf::new(sample_rate, cut_off, Lpf::Q),
            filter_info: Default::default(),
        }
    }
    pub fn process_to_buffer(&mut self, signal: &[f64], buffer: &mut [f64]) {
        for i in 0..signal.len() {
            // let s = self
            //     .input_filter
            //     .process_without_buffer(signal[i], &mut self.filter_info[0]);
            let s = signal[i];
            // 複素変換
            let re = self.result_filter.process_without_buffer(
                self.result_filter.process_without_buffer(
                    -((s) * (self.t).sin()),
                    &mut self.filter_info[0],
                ),
                &mut self.filter_info[1],
            );
            let im = self.result_filter.process_without_buffer(
                self.result_filter.process_without_buffer(
                    (s) * (self.t).cos(),
                    &mut self.filter_info[2],
                ),
                &mut self.filter_info[3],
            );

            // 微分
            let (d_re, d_im) = self.differential(re, im);
            // たすき掛け
            let a = d_re * im;
            let b = d_im * re;
            buffer[i] = a - b;
            self.t += TAU * self.carrier_freq * self.sample_period;
        }
        self.t = self.t.rem_euclid(TAU);
    }
    #[inline(always)]
    fn differential(&mut self, r: f64, i: f64) -> (f64, f64) {
        let delta_t = self.sample_period;
        let diff = {
            (
                (r - self.prev_sig[0]) / delta_t,
                (i - self.prev_sig[1]) / delta_t,
            )
        };
        self.prev_sig[0] = r;
        self.prev_sig[1] = i;
        diff
    }
}
