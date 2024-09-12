use std::{alloc::Layout, cell::Cell, mem::MaybeUninit, ptr::NonNull};

use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};

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
        unsafe {
            let _ = VirtualFree(self.start.as_ptr() as *mut _, 0, MEM_RELEASE);
        }
    }
}

impl VirtArena {
    fn new() -> Self {
        let start =
            unsafe { VirtualAlloc(None, super::VIRT_ALLOC_SIZE, MEM_RESERVE, PAGE_READWRITE) };

        let Some(start) = NonNull::new(start.cast()) else {
            panic!(
                "Failed to allocate virtual arena: {}",
                std::io::Error::last_os_error()
            );
        };

        Self {
            start,
            cursor: Cell::new(start),
        }
    }
}

impl super::VirtArenaRaw for VirtArena {
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
            let mut value = ptr.byte_add(off);
            let cursor: NonNull<u8> = value.byte_add(layout.size()).cast();

            if cursor.byte_offset_from(self.start) as usize > super::VIRT_ALLOC_SIZE {
                panic!("OOM");
            }

            VirtualAlloc(
                Some(ptr.as_ptr() as *const _),
                layout.size() + off,
                MEM_COMMIT,
                PAGE_READWRITE,
            );

            self.cursor.set(cursor);
            value.as_mut()
        }
    }
}
