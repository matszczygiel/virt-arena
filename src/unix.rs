use std::alloc::Layout;
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

use libc::*;

pub struct VirtArena {
    start: NonNull<u8>,
    cursor: Cell<NonNull<u8>>,
}

impl Default for VirtArena {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VirtArena {
    fn drop(&mut self) {
        unsafe { munmap(self.start.as_ptr().cast(), super::VIRT_ALLOC_SIZE) };
    }
}

impl VirtArena {
    fn new() -> Self {
        let start = unsafe {
            mmap(
                std::ptr::null_mut(),
                super::VIRT_ALLOC_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_ANONYMOUS | MAP_PRIVATE | MAP_NORESERVE,
                -1,
                0,
            )
        };

        if start == MAP_FAILED {
            panic!(
                "Failed to allocate virtual arena: {}",
                std::io::Error::last_os_error()
            );
        }
        let start = NonNull::new(start.cast()).expect("mmaped pointer should never be NULL");

        Self {
            start,
            cursor: Cell::new(start),
        }
    }
}

impl crate::VirtArenaRaw for VirtArena {
    fn bytes_used(&self) -> usize {
        unsafe { self.cursor.get().byte_offset_from(self.start) as usize }
    }

    fn reset(&mut self) {
        self.cursor.set(self.start);
    }

    fn alloc_uninit<T: Sized>(&self) -> &mut MaybeUninit<T> {
        let layout = Layout::new::<MaybeUninit<T>>();

        let ptr: NonNull<MaybeUninit<T>> = self.cursor.get().cast();

        let off = ptr.align_offset(layout.align());

        unsafe {
            let mut ptr = ptr.byte_add(off);
            let cursor: NonNull<u8> = ptr.byte_add(layout.size()).cast();

            if cursor.byte_offset_from(self.start) as usize > super::VIRT_ALLOC_SIZE {
                panic!("OOM");
            }
            self.cursor.set(cursor);

            ptr.as_mut()
        }
    }
}
