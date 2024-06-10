use std::f32::consts::{FRAC_1_SQRT_2, TAU};
pub type FilterInfo = [f32; 4];
pub struct Lpf {
    c0: f32,
    c1: f32,
    c2: f32,
    c3: f32,
    c4: f32,
}
impl Lpf {
    pub const Q: f32 = FRAC_1_SQRT_2;
    pub fn new(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let omega = TAU * cutoff / sample_rate;
        let alpha = omega.sin() / (2f32 * q);
        let a0 = 1f32 + alpha;
        let a1 = -2f32 * omega.cos();
        let a2 = 1f32 - alpha;
        let b0 = (1f32 - omega.cos()) / 2f32;
        let b1 = 1f32 - omega.cos();
        let b2 = (1f32 - omega.cos()) / 2f32;
        Self {
            c0: b0 / a0,
            c1: b1 / a0,
            c2: b2 / a0,
            c3: a1 / a0,
            c4: a2 / a0,

        }
    }
    pub fn process(&self, signal: &mut [f32]) {
        let mut i1 = 0.;
        let mut i2 = 0.;
        let mut o1 = 0.;
        let mut o2 = 0.;
        for i in 0..signal.len() {
            let x = signal[i];
            signal[i] = self.c0 * x + self.c1 * i1 + self.c2 * i2
                - self.c3 * o1
                - self.c4 * o2;
            i2 = i1;
            i1 = x;
            o2 = o1;
            o1 = signal[i];
        }
    }

    pub fn process_with_buffer(&self, buffer: &mut [f32], signal: &[f32]) {
        let mut i1 = 0.;
        let mut i2 = 0.;
        let mut o1 = 0.;
        let mut o2 = 0.;
        for i in 0..signal.len() {
            let x = signal[i];
            buffer[i] = self.c0 * x + self.c1 * i1 + self.c2 * i2
                - self.c3 * o1
                - self.c4 * o2;
            i2 = i1;
            i1 = x;
            o2 = o1;
            o1 = buffer[i];
        }
    }
    pub fn process_without_buffer(
        &self,
        signal: f32,
        info: &mut FilterInfo,
    ) -> f32 {
        let [in1, in2, out1, out2] = info;
        let buf = self.c0 * signal + self.c1 * *in1 + self.c2 * *in2
            - self.c3 * *out1
            - self.c4 * *out2;
        *info = [signal, *in1, buf, *out1];
        buf
    }
}

pub struct Hpf {
    c0: f32,
    c1: f32,
    c2: f32,
    c3: f32,
    c4: f32,
}

impl Hpf {
    pub const Q: f32 = FRAC_1_SQRT_2;
    pub fn new(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let omega = TAU * cutoff / sample_rate;
        let alpha = omega.sin() / (2f32 * q);
        let a0 = 1f32 + alpha;
        let a1 = -2f32 * omega.cos();
        let a2 = 1f32 - alpha;
        let b0 = (1f32 + omega.cos()) / 2f32;
        let b1 = -(1f32 + omega.cos());
        let b2 = (1f32 + omega.cos()) / 2f32;
        Self {
            c0: b0 / a0,
            c1: b1 / a0,
            c2: b2 / a0,
            c3: a1 / a0,
            c4: a2 / a0,
        }
    }
    pub fn process(&self, signal: &mut [f32]) {
        let mut i1 = 0.;
        let mut i2 = 0.;
        let mut o1 = 0.;
        let mut o2 = 0.;
        for i in 0..signal.len() {
            let x = signal[i];
            signal[i] = self.c0 * x + self.c1 * i1 + self.c2 * i2
                - self.c3 * o1
                - self.c4 * o2;
            i2 = i1;
            i1 = x;
            o2 = o1;
            o1 = signal[i];
        }
    }
    pub fn process_with_buffer(&self, buffer: &mut [f32], signal: &[f32]) {
        let mut i1 = 0.;
        let mut i2 = 0.;
        let mut o1 = 0.;
        let mut o2 = 0.;
        for i in 0..signal.len() {
            let x = signal[i];
            buffer[i] = self.c0 * x + self.c1 * i1 + self.c2 * i2
                - self.c3 * o1
                - self.c4 * o2;
            i2 = i1;
            i1 = x;
            o2 = o1;
            o1 = buffer[i];
        }
    }
    pub fn process_without_buffer(
        &mut self,
        signal: f32,
        info: &mut FilterInfo,
    ) -> f32 {
        let [in1, in2, out1, out2] = info;
        let buf = self.c0 * signal + self.c1 * *in1 + self.c2 * *in2
            - self.c3 * *out1
            - self.c4 * *out2;
        *info = [signal, *in1, buf, *out1];
        buf
    }
}
pub struct Bpf {
    high_pass: Hpf,
    low_pass: Lpf,
}
impl Bpf {
    pub const Q: f32 = FRAC_1_SQRT_2;
    pub fn new(sample_rate: f32, low_cut: f32, high_cut: f32, q: f32) -> Bpf {
        Bpf {
            high_pass: Hpf::new(sample_rate, low_cut, q),
            low_pass: Lpf::new(sample_rate, high_cut, q),
        }
    }
    // pub fn process(&mut self, signal: &mut [f32]) {
    //     for i in 0..signal.len() {
    //         signal[i] = self.process_without_buffer(signal[i]);
    //     }
    // }
    pub fn process_without_buffer(
        &mut self,
        signal: f32,
        filter_info: &mut [FilterInfo; 2],
    ) -> f32 {
        let buf = self
            .high_pass
            .process_without_buffer(signal, &mut filter_info[0]);
        self.low_pass
            .process_without_buffer(buf, &mut filter_info[1])
    }
}
