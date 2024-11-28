#[repr(C)]
pub struct ResamplerInfo {
    prev: f64,
    multiplier: usize,
    input_len: usize,
}

impl ResamplerInfo {
    pub fn new_upsample_info(
        src_fs: usize,
        dst_fs: usize,
        input_size: usize,
    ) -> Self {
        Self {
            prev: 0.0,
            multiplier: dst_fs / src_fs,
            input_len: input_size,
        }
    }
    pub fn new_downsample_info(
        src_fs: usize,
        dst_fs: usize,
        input_size: usize,
    ) -> Self {
        Self {
            prev: 0.0,
            multiplier: src_fs / dst_fs,
            input_len: input_size,
        }
    }
}
