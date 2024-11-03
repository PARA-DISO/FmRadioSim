use std::f64::consts::{FRAC_1_SQRT_2, TAU};
pub type FilterInfo = [f64; 4];
#[repr(C)]
#[derive(Debug,Default)]
pub struct Lpf {
    c0: f64,
    c1: f64,
    c2: f64,
    c3: f64,
    c4: f64,
}
impl Lpf {
    pub const Q: f64 = FRAC_1_SQRT_2;
    pub fn new(sample_rate: f64, cutoff: f64, q: f64) -> Self {
        let omega = TAU * (cutoff) / (sample_rate);
        let alpha = (omega).sin() / (2f64 * q);
        let a0 = 1f64 + (alpha);
        let a1 = -2f64 * omega.cos();
        let a2 = 1f64 - alpha;
        let b0 = (1f64 - omega.cos()) / 2f64;
        let b1 = 1f64 - omega.cos();
        let b2 = (1f64 - omega.cos()) / 2f64;
        Self {
            c0: (b0 / a0),
            c1: (b1 / a0),
            c2: (b2 / a0),
            c3: (a1 / a0),
            c4: (a2 / a0),
        }
    }
    pub fn process(&self, signal: &mut [f64]) {
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

    pub fn process_with_buffer(&self, buffer: &mut [f64], signal: &[f64]) {
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
        signal: f64,
        info: &mut FilterInfo,
    ) -> f64 {
        let [in1, in2, out1, out2] = info;
        let buf = self.c0 * signal + self.c1 * *in1 + self.c2 * *in2
            - self.c3 * *out1
            - self.c4 * *out2;
        *info = [signal, *in1, buf, *out1];
        buf
    }
}

pub struct Hpf {
    c0: f64,
    c1: f64,
    c2: f64,
    c3: f64,
    c4: f64,
}

impl Hpf {
    pub const Q: f64 = FRAC_1_SQRT_2;
    pub fn new(sample_rate: f64, cutoff: f64, q: f64) -> Self {
        let omega = TAU * (cutoff / sample_rate);
        let alpha = omega.sin() / (2f64 * q);
        let a0 = 1f64 + alpha;
        let a1 = -2f64 * omega.cos();
        let a2 = 1f64 - alpha;
        let b0 = (1f64 + omega.cos()) / 2f64;
        let b1 = -(1f64 + omega.cos());
        let b2 = (1f64 + omega.cos()) / 2f64;
        Self {
            c0: b0 / a0,
            c1: b1 / a0,
            c2: b2 / a0,
            c3: a1 / a0,
            c4: a2 / a0,
        }
    }
    pub fn process(&self, signal: &mut [f64]) {
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
    pub fn process_with_buffer(&self, buffer: &mut [f64], signal: &[f64]) {
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
        signal: f64,
        info: &mut FilterInfo,
    ) -> f64 {
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
    pub const Q: f64 = FRAC_1_SQRT_2;
    pub fn new(sample_rate: f64, low_cut: f64, high_cut: f64, q: f64) -> Bpf {
        Bpf {
            high_pass: Hpf::new(sample_rate, low_cut, q),
            low_pass: Lpf::new(sample_rate, high_cut, q),
        }
    }
    // pub fn process(&mut self, signal: &mut [f64]) {
    //     for i in 0..signal.len() {
    //         signal[i] = self.process_without_buffer(signal[i]);
    //     }
    // }
    pub fn process_without_buffer(
        &mut self,
        signal: f64,
        filter_info: &mut [FilterInfo; 2],
    ) -> f64 {
        let buf = self
            .high_pass
            .process_without_buffer(signal, &mut filter_info[0]);
        self.low_pass
            .process_without_buffer(buf, &mut filter_info[1])
    }
}

pub struct Notch {
    c0: f64,
    c1: f64,
    c2: f64,
    c3: f64,
    c4: f64,
}

impl Notch {
    pub const BW: f64 = 0.3;
    pub fn new(sample_rate: f64, cutoff: f64, bw: f64) -> Self {
        let omega = TAU * cutoff / sample_rate;
        let alpha = omega.sin()
            * (std::f64::consts::LN_2 / 2f64 * bw * omega / omega.sin()).sinh();
        let a0 = 1f64 + alpha;
        let a1 = -2f64 * omega.cos();
        let a2 = 1f64 - alpha;
        let b0 = 1f64;
        let b1 = -2f64 * omega.cos();
        let b2 = 1f64;
        Self {
            c0: b0 / a0,
            c1: b1 / a0,
            c2: b2 / a0,
            c3: a1 / a0,
            c4: a2 / a0,
        }
    }
    pub fn process(&self, signal: &mut [f64]) {
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
    pub fn process_with_buffer(&self, buffer: &mut [f64], signal: &[f64]) {
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
        signal: f64,
        info: &mut FilterInfo,
    ) -> f64 {
        let [in1, in2, out1, out2] = info;
        let buf = self.c0 * signal + self.c1 * *in1 + self.c2 * *in2
            - self.c3 * *out1
            - self.c4 * *out2;
        *info = [signal, *in1, buf, *out1];
        buf
    }
}
pub struct Emphasis {
    a0: f64,
    a1: f64,
    b0: f64,
}
impl Emphasis {
    pub fn new(sample_rate: f64, tau: f64) -> Self {
        let sample_rate = sample_rate / 1000.;
        let coeff = sample_rate / (sample_rate + 2. * tau);
        let coeff_rev = 1. / coeff;
        Self {
            a0: coeff_rev,
            a1: -coeff_rev * (2. * tau - sample_rate)
                / (2. * tau + sample_rate),
            b0: 1.,
        }
    }
    pub fn process_without_buffer(
        &self,
        signal: f64,
        info: &mut FilterInfo,
    ) -> f64 {
        let [in1, _, out1, _] = info;
        let buf = self.a0 * signal + self.a1 * *in1 - self.b0 * *out1;
        *info = [signal, *in1, buf, *out1];
        buf
    }
}
pub struct Deemphasis {
    a0: f64,
    a1: f64,
    b0: f64,
}
impl Deemphasis {
    pub fn new(sample_rate: f64, tau: f64) -> Self {
        let sample_rate = sample_rate / 1000.;
        let coeff = sample_rate / (sample_rate + 2. * tau);
        Self {
            a0: coeff,
            a1: coeff,
            b0: (2. * tau - sample_rate) / (2. * tau + sample_rate),
        }
    }
    pub fn process_without_buffer(
        &self,
        signal: f64,
        info: &mut FilterInfo,
    ) -> f64 {
        let [in1, _, out1, _] = info;
        let buf = self.a0 * signal + self.a1 * *in1 + self.b0 * *out1;
        *info = [signal, *in1, buf, *out1];
        buf
    }
}
