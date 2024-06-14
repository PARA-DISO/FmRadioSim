/**
 * コンポジット信号を作成、復元するコード群
*/
use crate::filter::{FilterInfo, Hpf, Lpf,Bpf};
use std::f32::consts::TAU;
pub struct CompositeSignal {
    lpf: Lpf,
    sample_rate: f32,
    buffer: Vec<f32>, // L,R or L+R, L-R
    t: f32,
    filter_info: [FilterInfo; 2],
}
impl CompositeSignal {
    const PILOT_FREQ: f32 = 19_000f32;
    const CARRIER_FREQ: f32 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f32 = 15_000f32;
    pub const DEFAULT_SAMPLE_RATE: f32 =
        (Self::CARRIER_FREQ + Self::CUT_OFF_FREQ) * 3.;
    pub fn new(f: f32) -> Self {
        Self {
            lpf: Lpf::new(f, Self::CUT_OFF_FREQ, Lpf::Q),
            sample_rate: f,
            buffer: Vec::new(),
            filter_info: [FilterInfo::default(), FilterInfo::default()],
            t: 0.,
        }
    }
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
    pub fn process_to_buffer(
        &mut self,
        l_channel: &[f32],
        r_channel: &[f32],
        buffer: &mut [f32],
    ) {
        // Low Pass
        for i in 0..l_channel.len() {
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
            buffer[i] = a + b + cos;
            self.t += 1. / self.sample_rate;
        }
        self.t = self.t.rem_euclid(1.);
    }
    pub fn process(&mut self, l_channel: &[f32], r_channel: &[f32]) {
        if self.buffer.len() != l_channel.len() {
            self.buffer = vec![0.0; l_channel.len()];
        }
        self.process_to_buffer(l_channel, r_channel, unsafe {
            let ptr = self.buffer.as_ptr();
            std::slice::from_raw_parts_mut(ptr.cast_mut(), l_channel.len())
        });
    }
    pub fn get_buffer(&self) -> &[f32] {
        self.buffer.as_slice()
    }
}
pub struct RestoredSignal {
    lpf: Lpf,
    lpf16: Lpf,
    hpf: Hpf,
    bpf: Bpf,
    sample_rate: f32,
    out_buffer: [Vec<f32>; 2],
    t: f32,
    filter_info: [FilterInfo; 6],
    bpf_info: [FilterInfo;2],
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
            bpf: Bpf::new(f,Self::PILOT_FREQ -1000f32,Self::PILOT_FREQ +1000f32,Bpf::Q),
            sample_rate: f,
            out_buffer: [Vec::new(), Vec::new()],
            t: 0.,
            filter_info: [
                FilterInfo::default(),
                FilterInfo::default(),
                FilterInfo::default(),
                FilterInfo::default(),
                FilterInfo::default(),
                FilterInfo::default(),
            ],
            bpf_info: [
              FilterInfo::default(),
              FilterInfo::default(),
            ]
        }
    }
    pub fn process_to_buffer(
        &mut self,
        signal: &[f32],
        l_buffer: &mut [f32],
        r_buffer: &mut [f32],
    ) {
        for i in 0..signal.len() {
            let theta = TAU * Self::PILOT_FREQ * self.t;
            let cos = theta.cos();
            // 倍角公式によるキャリアの生成
            let sin = 2. * cos * (theta).sin();
            // PILOTの削除
            let buffer = self.lpf16.process_without_buffer(
                -self.bpf.process_without_buffer(signal[i],&mut self.bpf_info) * cos,
                // signal[i]* cos,
                &mut self.filter_info[0],
            );
            // println!("{buffer}");
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

            l_buffer[i] = self.lpf16.process_without_buffer((a + b) / 2.,&mut self.filter_info[4]);
            r_buffer[i] =  self.lpf16.process_without_buffer((a - b) / 2.,&mut self.filter_info[5]);
            self.t += 1. / self.sample_rate;
        }
        // unreachable!();
        self.t = self.t.rem_euclid(1.);
    }
    pub fn process(&mut self, signal: &[f32]) {
        if self.out_buffer[0].len() != signal.len() {
            self.out_buffer =
                [vec![0.0; signal.len()], vec![0.0; signal.len()]];
        }
        let (l, r) = unsafe {
            let l_ptr = self.out_buffer[0].as_ptr();
            let r_ptr = self.out_buffer[1].as_ptr();
            (
                std::slice::from_raw_parts_mut(l_ptr.cast_mut(), signal.len()),
                std::slice::from_raw_parts_mut(r_ptr.cast_mut(), signal.len()),
            )
        };
        self.process_to_buffer(signal, l, r);
    }
    pub fn get_buffer(&self) -> &[Vec<f32>] {
        self.out_buffer.as_slice()
    }
}
