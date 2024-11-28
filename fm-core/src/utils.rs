use std::sync::Mutex;

pub type PipeLineBuffer = [Mutex<Vec<f64>>;2];
pub fn generate_pipline_buffer(size: usize)-> PipeLineBuffer {
  return [
    Mutex::new(vec![0.;size]),
    Mutex::new(vec![0.;size])
  ]
}