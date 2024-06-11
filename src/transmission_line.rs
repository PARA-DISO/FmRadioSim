// use rand::prelude::*;
use rand_distr::{Distribution, StandardNormal};
pub struct TransmissionLine {
    signal_coeff: f32,
    noise_coeff: f32,
    rng: sfmt::ThreadRng,
    normal: rand_distr::StandardNormal,
}
impl TransmissionLine {
    pub fn new() {
      
    }
    pub fn from_snr(noise_gain: f32) -> Self {
        let noise_coeff = dbg!(10f32.powf(noise_gain / 20f32));
        let signal_coeff = 1. - noise_coeff;
        println!("SNR: {}", (signal_coeff / noise_coeff).log10() * 20f32);
        Self {
            signal_coeff,
            noise_coeff,
            rng: sfmt::thread_rng(),
            normal: rand_distr::StandardNormal,
        }
    }
    pub fn process(&mut self, buffer: &mut [f32]) {
        buffer.iter_mut().for_each(|x| {
            let s = *x * self.signal_coeff;
            //  let n:f32 =self.normal.sample(&mut  self.rng) * self.noise_coeff;
            let n = (<StandardNormal as Distribution<f32>>::sample::<
                sfmt::ThreadRng,
            >(&self.normal, &mut self.rng)
                * 2.
                - 1.)
                * self.noise_coeff;
            *x = s + n;
        });
    }
    pub fn process_to_buf(&mut self, dst: &mut [f32], input: &[f32]) {
        input.iter().zip(dst.iter_mut()).for_each(|(x, d)| {
            let s = *x * self.signal_coeff;
            //  let n:f32 =self.normal.sample(&mut  self.rng) * self.noise_coeff;
            let n = <StandardNormal as Distribution<f32>>::sample::<
                sfmt::ThreadRng,
            >(&self.normal, &mut self.rng)
                * self.noise_coeff;
            *d = s + n;
        });
    }
}
