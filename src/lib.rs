#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::{alloc::Layout, mem::MaybeUninit, ptr::NonNull};

#[cfg(unix)]
type RawArena = unix::VirtArena;
#[cfg(windows)]
type RawArena = windows::VirtArena;

/// A memory arena which leverages the virtual memory system
/// for allocating structures in a single contiguous memory region.
#[derive(Default)]
pub struct VirtArena(RawArena);

impl VirtArena {
    /// Allocates a memory for the given `layout`.
    pub fn alloc(&self, layout: Layout) -> NonNull<u8> {
        self.0.alloc(layout)
    }

    /// Allocates a struct `T` inside the arena and clears its memory to 0.
    ///
    /// # Safety
    /// Look into [std::mem::zeroed] for safety concerns.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn alloc_zeroed<T: Sized>(&self) -> &mut T {
        let layout = Layout::new::<T>();
        self.0.alloc_zeroed(layout).cast().as_mut()
    }

    /// Allocates a slice `[T]` inside the arena and clears its memory to 0.
    ///
    /// # Safety
    /// Look into [std::mem::zeroed] for safety concerns.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn alloc_slice_zeroed<T: Sized>(&self, count: usize) -> &mut [T] {
        let layout = Layout::array::<T>(count).expect("Failed to read the array layout");
        let ptr = self.0.alloc_zeroed(layout).cast();
        std::slice::from_raw_parts_mut(ptr.as_ptr(), count)
    }

    /// Allocates memory for struct `T`.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_uninit<T: Sized>(&self) -> &mut MaybeUninit<T> {
        let layout = Layout::new::<T>();
        unsafe { self.0.alloc(layout).cast().as_mut() }
    }

    /// Allocates memory for slice `[T]`.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice_uninit<T: Sized>(&self, count: usize) -> &mut [MaybeUninit<T>] {
        let layout = Layout::array::<T>(count).expect("Failed to read the array layout");
        let ptr = self.0.alloc(layout).cast();
        unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), count) }
    }

    /// Allocates a struct `T` inside the arena and sets its
    /// content to the output of `fun`.   
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_with<T: Sized>(&self, fun: impl FnOnce() -> T) -> &mut T {
        let uninit = self.alloc_uninit();
        uninit.write(fun());
        unsafe { uninit.assume_init_mut() }
    }

    /// Allocates a struct `T` inside the arena and moves `val` into the allocation.    
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_value<T: Sized>(&self, val: T) -> &mut T {
        self.alloc_with(move || val)
    }

    /// Returns the number of bytes currently allocated from the arena.
    pub fn bytes_used(&self) -> usize {
        self.0.bytes_used()
    }

    /// Restes the arena storage, Invalidating all the references allocated.
    /// This method does not run the destructors! Those need to be run manually.
    pub fn reset(&mut self) {
        self.0.reset()
    }
}

// We don't use any thread local storage so this should be fine
unsafe impl Send for VirtArena {}

const VIRT_ALLOC_SIZE: usize = 128 * (1 << 30); // 128 GiB is assumed to be enoght for every use case of this arena

trait VirtArenaRaw {
    fn bytes_used(&self) -> usize;
    fn reset(&mut self);

    fn alloc(&self, layout: Layout) -> NonNull<u8>;

    unsafe fn alloc_zeroed(&self, layout: Layout) -> NonNull<u8> {
        let ptr = self.alloc(layout);
        ptr.write_bytes(0, layout.size());
        ptr
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    #[test]
    fn arena() {
        let mut arena = VirtArena::default();

        struct Test {
            thing: Option<NonZeroU32>,
        }

        let test1 = unsafe { arena.alloc_zeroed::<Test>() };
        assert!(test1.thing.is_none());

        let test2 = unsafe { arena.alloc_zeroed::<Test>() };
        assert!(test2.thing.is_none());

        test1.thing = Some(NonZeroU32::new(345).unwrap());

        arena.reset();

        let test1 = unsafe { arena.alloc_uninit::<Test>().assume_init_mut() };
        assert_eq!(test1.thing.map(|v| v.get()), Some(345));

        let test2 = unsafe { arena.alloc_uninit::<Test>().assume_init_mut() };
        assert!(test2.thing.is_none());
    }
}
