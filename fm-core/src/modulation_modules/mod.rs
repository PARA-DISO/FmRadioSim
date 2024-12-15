pub mod composite;
pub mod filter;
pub mod modulator;
#[inline]
pub fn get_8x_sample_rate(fs1: usize, fs2: usize) -> usize {
    let tmp = (fs1 as f64 / fs2 as f64).ceil() as usize;
    ((tmp & 0xffff_ffff_ffff_fff0) + if tmp & 0b1111 != 0 { 16 } else { 0 }) * fs2
}
