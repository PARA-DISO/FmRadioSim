/**
 * コンポジット信号を作成、復元するコード群
*/
use crate::Lpf;
use std::f32::consts::{PI, TAU};
struct CompositeSignal {
    lpf: Lpf,
    sample_rate: f32,
    buffer: [Vec<f32>; 2], // L,R or L+R, L-R
    out_buffer: Vec<f32>,
}
impl CompositeSignal {
    const PILOT_FREQ: f32 = 19_000f32;
    const CARRIER_FREQ: f32 = Self::PILOT_FREQ * 2.;
    const CUT_OFF_FREQ: f32 = 15_000f32;
    fn new(f: f32) -> Self {
        Self {
            lpf: Lpf::new(f, Self::CUT_OFF_FREQ, Lpf::Q),
            sample_rate: f,
            buffer: [Vec::new(), Vec::new()],
            out_buffer: Vec::new(),
        }
    }
    fn process(&mut self, l_channel: &[f32], r_channel: &[f32]) {
        if self.buffer[0].len() != l_channel.len() {
            self.buffer[0] = vec![0.0; l_channel.len()];
            self.buffer[1] = vec![0.0; l_channel.len()];
            self.out_buffer = vec![0.0; l_channel.len()];
        }
        // Low Cut
        self.lpf.process_with_buffer(&mut self.buffer[0], l_channel);
        self.lpf.process_with_buffer(&mut self.buffer[1], r_channel);
        let t = 1. / self.sample_rate;
        for i in 0..self.buffer[1].len() {
            let l = self.buffer[0][i];
            let r = self.buffer[1][i];
            let a = l + r;
            let b = (l - r) * (TAU * Self::CARRIER_FREQ * t).sin();
            self.out_buffer[i] = a + b + (TAU * Self::PILOT_FREQ * t).sin();
        }
    }
}
struct RestoredSignal {}
