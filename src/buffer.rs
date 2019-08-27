use std::{
    alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout, LayoutErr},
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

#[derive(Debug)]
pub struct Buffer {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl Buffer {
    pub fn new(layout: Layout) -> Self {
        let ptr = if let Some(ptr) = NonNull::new(unsafe { alloc(layout) }) {
            ptr
        } else {
            // Abort the process
            handle_alloc_error(layout);
        };

        unsafe { ptr.as_ptr().write_bytes(0, layout.size()) };

        Self { ptr, layout }
    }

    pub fn from_size_align(size: usize, align: usize) -> Result<Self, LayoutErr> {
        Layout::from_size_align(size, align).map(Self::new)
    }

    pub fn resize(&mut self, new_size: usize) {
        let new_layout = Layout::from_size_align(new_size, self.layout.align()).unwrap();

        let new_ptr = unsafe { realloc(self.ptr.as_ptr(), self.layout, new_layout.size()) };

        let ptr = if let Some(ptr) = NonNull::new(new_ptr) {
            ptr
        } else {
            // Abort the process
            handle_alloc_error(new_layout);
        };

        if new_layout.size() > self.layout.size() {
            unsafe {
                ptr.as_ptr()
                    .offset(self.layout.size() as isize)
                    .write_bytes(0, new_layout.size() - self.layout.size())
            };
        }

        self.ptr = ptr;
        self.layout = new_layout;
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

impl std::ops::Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.layout.size()) }
    }
}

impl std::ops::DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.layout.size()) }
    }
}
