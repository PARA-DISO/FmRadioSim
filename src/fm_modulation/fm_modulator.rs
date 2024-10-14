use std::f64::consts::TAU;
pub type SampleType = f32;
use crate::filter::{FilterInfo, Lpf};
pub struct FmModulator {
    integral: f64, // int_{0}^{t} x(\tau) d\tau ( 符号拡張)
    t: f64,        // 時刻t
    prev_sig: f32,
    // sample_rate: f64,
    sample_period: f64,
    carrier_freq: f64,
    // buffer: Vec<SampleType>,
    modulation_index: f64,
}
impl FmModulator {
    pub fn new() -> Self {
        Self {
            integral: 0.0,
            t: 0.0,
            prev_sig: 0.0,
            modulation_index: 1.,
            // sample_rate: (79_500_000f64 + 500_000f64) * 2.,
            sample_period: 1. / 79_500_000f64 * 3.,
            carrier_freq: 79_500_000f64,
            // buffer: Vec::new(),
        }
    }
    pub fn from(f: f64, sample_rate: f64) -> Self {
        Self {
            integral: 0.0,
            t: 0.0,
            prev_sig: 0.0,
            modulation_index: 1.,
            // sample_rate,
            sample_period: 1. / sample_rate,
            carrier_freq: f,
            // buffer: Vec::new(),
        }
    }
    pub fn process_to_buffer(&mut self, signal: &[f32], buffer: &mut [f32]) {
        for i in 0..signal.len() {
            self.integral += if i == 0 {
                self.prev_sig + signal[i]
            } else {
                signal[i - 1] + signal[i]
            } as f64;
            buffer[i] = (self.t
                + self.modulation_index * self.sample_period / 2.
                    * self.integral)
                .cos() as f32;
            self.t += TAU * self.carrier_freq * self.sample_period;
        }
        self.prev_sig = *(signal.last().unwrap());

        self.t = self.t.rem_euclid(TAU);
    }
}
// const TAPS: usize = 128;
// const CUT_OFF: f32 = 70_000f32;
pub struct FmDeModulator {
    t: f64, // 時刻t
    prev_sig: [f32; 2],
    // sample_rate: f64,
    sample_period: f64,
    carrier_freq: f64,
    result_filter: Lpf,
    input_filter: Lpf,
    filter_info: [FilterInfo; 3],
}
impl FmDeModulator {
    pub fn from(f: f64, sample_rate: f64, input_cut: f64) -> Self {
      println!("periodic: {}", (1. / sample_rate) as f32);
        Self {
            t: 0.0,
            prev_sig: [0.0; 2],
            // sample_rate,
            sample_period: dbg!(1. / sample_rate),
            carrier_freq: f,
            result_filter: Lpf::new(sample_rate as f32, f as f32, Lpf::Q),
            input_filter: Lpf::new(
                sample_rate as f32,
                f as f32,
                Lpf::Q,
            ),
            filter_info: [FilterInfo::default(); 3],
        }
    }
    pub fn process_to_buffer(&mut self, signal: &[f32], buffer: &mut [f32]) {
        let mut max:f32 = 0.;
        let mut s_max:f32 = 0.;
        let mut re_max = 0f32;
        let mut im_max = 0f32;
        let mut dre_max = 0f32;
        let mut dim_max = 0f32;
        for i in 0..signal.len() {
            // let s = self
            //     .input_filter
            //     .process_without_buffer(signal[i], &mut self.filter_info[0]);
            let s = signal[i];
            s_max = s_max.max(s.abs());
            // 複素変換
            let re = self.result_filter.process_without_buffer(
                ((s as f64) * (self.t).sin()) as f32,
                &mut self.filter_info[1],
            );
            let im = self.result_filter.process_without_buffer(
                -((s as f64) * (self.t).cos()) as f32,
                &mut self.filter_info[2],
            );
            // buffer[i] = im;
            re_max = re_max.max(re.abs());
            im_max = im_max.max(im.abs());
            // 微分
            let (d_re, d_im) = self.differential(re, im);
            dre_max = dre_max.max(d_re.abs());
            dim_max = dim_max.max(d_im.abs());
            // たすき掛け
            let a = d_re * im;
            let b = d_im * re;
            buffer[i] = a - b;
            max = max.max(buffer[i].abs());
            self.t += self.sample_period * TAU * self.carrier_freq;
        }
        println!("out_max: {max}");
        println!("input_max: {s_max}");
        println!("complex max: ({re_max},{im_max})");
        println!("complex d max: ({dre_max},{dim_max})");
        // self.t = self.t.rem_euclid(TAU);
    }
    #[inline(always)]
    fn differential(&mut self, r: f32, i: f32) -> (f32, f32) {
        let diff = {
            (
                (r - self.prev_sig[0]) / self.sample_period as f32,
                (i - self.prev_sig[1]) / self.sample_period as f32,
            )
        };
        self.prev_sig[0] = r;
        self.prev_sig[1] = i;
        diff
    }
}
