/**
 * コンポジット信号を作成、復元するコード群
*/
use super::filter::{Emphasis, FilterInfo, Hpf, Lpf, Notch};
use std::f64::consts::TAU;

use super::filter::Deemphasis;
pub struct CompositeSignal {
    lpf: Lpf,
    sample_rate: f64,
    t: f64,
    filter_info: [FilterInfo; 4],
    emphasis: Emphasis,
}
impl CompositeSignal {
    const PILOT_FREQ: f64 = 19_000.;
    const CARRIER_FREQ: f64 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f64 = 15_000f64;
    pub const DEFAULT_SAMPLE_RATE: f64 =
        (Self::CARRIER_FREQ + Self::CUT_OFF_FREQ) * 3.;
    pub fn new(f: f64) -> Self {
        Self {
            lpf: Lpf::new(f, Self::CUT_OFF_FREQ, Lpf::Q),
            sample_rate: f,
            filter_info: [FilterInfo::default(); 4],
            t: 0.,
            emphasis: Emphasis::new(f, 50.),
        }
    }
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
    pub fn process(
        &mut self,
        l_channel: &[f64],
        r_channel: &[f64],
        buffer: &mut [f64],
    ) {
        for i in 0..l_channel.len() {
            // Low Pass
            let l = self
                .lpf
                .process_without_buffer(l_channel[i], &mut self.filter_info[0]);
            let r = self
                .lpf
                .process_without_buffer(r_channel[i], &mut self.filter_info[1]);
            // Pre-Emphasis
            let l = self
                .emphasis
                .process_without_buffer(l, &mut self.filter_info[2]);
            let r = self
                .emphasis
                .process_without_buffer(r, &mut self.filter_info[3]);
            // Convert to Composite Signal
            let a = l + r;
            let theta = TAU * Self::PILOT_FREQ * self.t;
            let cos = theta.cos();
            let double_sin = cos * theta.sin() * 2.;
            let b = (l - r) * double_sin;
            buffer[i] = (a + b + cos) / 3.;
            self.t += 1. / self.sample_rate;
        }
        // self.t = self.t.rem_euclid(1.);
    }
}
pub struct RestoreSignal {
    input_filter: Lpf,
    lpf16: Lpf,
    hpf: Hpf,
    notch: Notch,
    de_emphasis: Deemphasis,
    sample_rate: f64,
    t: f64,
    filter_info: [FilterInfo; 8],
    de_emphasis_info: [FilterInfo; 2],
}
impl RestoreSignal {
    const PILOT_FREQ: f64 = 19_000f64;
    const CARRIER_FREQ: f64 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f64 = 15_000f64;
    pub fn new(f: f64) -> Self {
        Self {
            input_filter: Lpf::new(
                f,
                Self::CARRIER_FREQ + Self::CUT_OFF_FREQ,
                Lpf::Q,
            ),
            lpf16: Lpf::new(f, 16_000f64, Lpf::Q),
            hpf: Hpf::new(f, Self::PILOT_FREQ, Hpf::Q),
            notch: Notch::new(f, Self::PILOT_FREQ, Notch::BW),
            de_emphasis: Deemphasis::new(f, 50.),
            sample_rate: f,
            t: 0.,
            filter_info: [FilterInfo::default(); 8],
            de_emphasis_info: [FilterInfo::default(); 2],
        }
    }
    pub fn process(
        &mut self,
        signal: &[f64],
        l_buffer: &mut [f64],
        r_buffer: &mut [f64],
    ) {
        // println!("Restore LPF-Coeff: {:?}",self.lpf16);
        // println!("Restore HPF-Coeff: {:?}",self.hpf);
        // println!("Restore NOTCH-Coeff: {:?}",self.notch);
        for i in 0..signal.len() {
            let sig = self
                .input_filter
                .process_without_buffer(signal[i], &mut self.filter_info[6]);
            let theta = TAU * Self::PILOT_FREQ * self.t;
            let cos = theta.cos();
            // 倍角公式によるキャリアの生成
            let sin = 2. * cos * (theta).sin();
            // PILOTの削除
            // let buffer = self.dc_pass.process_without_buffer(
            //     -self.bpf.process_without_buffer(signal[i],&mut self.bpf_info) * cos,
            //     // signal[i]* cos,
            //     &mut self.filter_info[0],
            // );
            // println!("{buffer}");
            // let remove_pilot = signal[i] + buffer * cos;
            let remove_pilot = self
                .notch
                .process_without_buffer(sig, &mut self.filter_info[0]);
            //  get L+R and L-R with LPF
            let a = self
                .lpf16
                .process_without_buffer(remove_pilot, &mut self.filter_info[1]); // L+R
                                                                                 // remove_pilot
            let b = self.lpf16.process_without_buffer(
                self.hpf.process_without_buffer(
                    remove_pilot,
                    &mut self.filter_info[3],
                ) * -2.
                    * sin,
                &mut self.filter_info[2],
            ); // L-R

            let l = self
                .lpf16
                .process_without_buffer((a + b) / 2., &mut self.filter_info[4]);
            let r = self
                .lpf16
                .process_without_buffer((a - b) / 2., &mut self.filter_info[5]);
            let l = self
                .de_emphasis
                .process_without_buffer(l, &mut self.de_emphasis_info[0]);
            let r = self
                .de_emphasis
                .process_without_buffer(r, &mut self.de_emphasis_info[1]);
            l_buffer[i] = l * 3.;
            r_buffer[i] = r * 3.;
            self.t += 1. / self.sample_rate;
        }
        // self.t = self.t.rem_euclid(1.);
        // println!("de-composite-testdata-abs-max: {} ({a_max} - {b_max})",a_max/b_max );
    }
}
