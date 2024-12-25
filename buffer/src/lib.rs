pub struct FixedLenBuffer {
    buffer: Vec<f32>,
    index: isize,
    size: isize,
    capacity: isize,
    virtual_pos: isize,
    read_pos: isize,
}

impl FixedLenBuffer {
    pub fn new(data_size: usize, buffer_num: usize) -> Result<Self, String> {
        if buffer_num.is_power_of_two() {
            let buffer = vec![0.; data_size * buffer_num];
            Ok(Self {
                buffer,
                capacity: (buffer_num - 1) as isize,
                size: data_size as isize,
                index: 0,
                virtual_pos: 0,
                read_pos: 0,
            })
        } else {
            Err("buffer_num must be power of two".to_string())
        }
    }

    pub fn enqueue(&mut self, data: &[f32]) {
        let size = self.size;
        let idx = self.index;
        if self.capacity <= self.virtual_pos {
            self.read_pos = (self.read_pos + 1) & self.capacity;
        }
        // let idx = (idx);
        // println!("write @ {idx}");
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.buffer.as_mut_ptr().add((size * idx) as usize),
                size as usize,
            );
        }
        self.virtual_pos = self.capacity.min(self.virtual_pos + 1);
        self.index = (idx + 1) & self.capacity;
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.index = pos as isize & self.capacity;
        // self.read_pos = pos as isize & self.capacity;
        self.virtual_pos = pos as isize & self.capacity;
    }

    pub fn dequeue(&mut self, dst: &mut [f32]) -> bool {
        if self.virtual_pos == -1 {
            return false;
        }
        let size = self.size;
        let idx = self.read_pos;
        // println!("read @ {idx}");
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.buffer.as_mut_ptr().add((size * idx) as usize),
                dst.as_mut_ptr(),
                size as usize,
            );
        }
        self.read_pos = (self.read_pos + 1) & self.capacity;
        self.virtual_pos -= 1;
        true
    }

    pub fn get_pos(&self) -> isize {
        self.virtual_pos
    }
    pub fn is_empty(&self) -> bool {
        self.virtual_pos == -1
    }
}
