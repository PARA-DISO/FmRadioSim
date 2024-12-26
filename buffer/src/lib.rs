pub struct FixedLenBuffer {
    buffer: Vec<f32>,
    index: usize,
    size: usize,
    capacity: usize,
    virtual_pos: usize,
    read_pos: usize,
}

impl FixedLenBuffer {
    pub fn new(data_size: usize, buffer_num: usize) -> Result<Self, String> {
        if buffer_num.is_power_of_two() {
            let buffer = vec![0.; data_size * buffer_num];
            Ok(Self {
                buffer,
                capacity: (buffer_num - 1),
                size: data_size,
                index: 0,
                virtual_pos: 0,
                read_pos: 0,
            })
        } else {
            Err("buffer_num must be power of two".to_string())
        }
    }

    pub fn enqueue(&mut self, data: &[f32]) -> bool {
        let size = self.size;
        let idx = self.index;

        let is_full = if self.capacity <= self.virtual_pos {
            self.read_pos = (self.read_pos + 1) & self.capacity;
            false
        } else {
            true
        };
        // let idx = (idx);
        // println!("write @ {idx}");
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.buffer.as_mut_ptr().add(size * idx),
                size,
            );
        }
        self.virtual_pos = self.capacity.min(self.virtual_pos + 1);
        self.index = (idx + 1) & self.capacity;
        is_full
    }

    pub fn set_len(&mut self, len: usize) {
        self.index = len & self.capacity;
        // self.read_len = pos & self.capacity;
        self.virtual_pos = len & self.capacity;
    }

    pub fn dequeue(&mut self, dst: &mut [f32]) -> bool {
        if self.virtual_pos == 0 {
            return false;
        }
        let size = self.size;
        let idx = self.read_pos;
        // println!("read @ {idx}");
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.buffer.as_mut_ptr().add(size * idx),
                dst.as_mut_ptr(),
                size,
            );
        }
        self.read_pos = (self.read_pos + 1) & self.capacity;
        self.virtual_pos -= 1;
        true
    }

    pub fn get_len(&self) -> usize {
        self.virtual_pos
    }
    pub fn is_empty(&self) -> bool {
        self.virtual_pos == 0
    }
}
