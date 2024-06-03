#[cfg(target_arch = "x86_64")]
use rustfft::{num_complex::Complex, FftPlannerAvx as FftPlanner};
#[cfg(not(target_arch = "x86_64"))]
use rustfft::{FftPlanner, num_complex::Complex};
pub type SampleType = f32;

// pub struct Fft {
//     planner: FftPlanner,
//     buffer: Vec<Complex<SampleType>>,
//     sample_rate:f64
// }
// impl Fft {
//   pub fn from(sample_rate:f64,buffer_size:usize) -> Self {
//     Self {
//       planner: FftPlanner,
//       buffer: Vec<Complex<SampleType>>,
//       sample_rate:f64
//     }
//   }
// }