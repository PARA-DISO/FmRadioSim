use std::sync::{Arc, Condvar, Mutex};

pub type PipeLineBuffer = Arc<[Mutex<Vec<f64>>; 2]>;
pub fn generate_pipline_buffer(size: usize) -> PipeLineBuffer {
    Arc::new([Mutex::new(vec![0.; size]), Mutex::new(vec![0.; size])])
}
pub type Shareable<T> = Arc<Mutex<T>>;
#[macro_export]
macro_rules! sharable {
    ($v:expr) => {
        std::sync::Arc::new(std::sync::Mutex::new($v))
    };
}
pub type ExecFlag = Arc<(Mutex<bool>, Condvar)>;
#[macro_export]
macro_rules! exec_flag {
    () => {
        std::sync::Arc::new((Mutex::new(true), std::sync::Condvar::new()))
    };
}

pub mod float {
    pub const FLUSH_TO_ZERO: u32 = 1 << 15;
}
