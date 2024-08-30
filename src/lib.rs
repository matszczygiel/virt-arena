#[cfg(unix)]
mod unix;

use std::mem::MaybeUninit;

#[cfg(unix)]
type RawArena = unix::VirtArena;

/// A memory arena which leverages the virtual memory system
/// for allocating structures in a single contiguous memory region.
#[derive(Default)]
pub struct VirtArena(RawArena);

impl VirtArena {
    /// Allocates a struct `T` inside the arena and clears its memory to 0.
    ///
    /// # Safety
    /// Look into (`std::mem::zeroed()`)[std::mem::zeroed] for safety concerns.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn alloc_zeroed<T: Sized>(&self) -> &mut T {
        self.0.alloc_zeroed()
    }

    /// Allocates a struct `T` inside the arena and sets its
    /// content to the output of `fun`.   
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_with<T: Sized>(&self, fun: impl FnOnce() -> T) -> &mut T {
        self.0.alloc_with(fun)
    }

    /// Allocates memory for struct `T`.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_uninit<T: Sized>(&self) -> &mut MaybeUninit<T> {
        self.0.alloc_uninit()
    }

    /// Allocates a struct `T` inside the arena and moves `val` into the allocation.    
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T: Sized>(&self, val: T) -> &mut T {
        self.0.alloc_with(move || val)
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

const VIRT_ALLOC_SIZE: usize = 128 * (1 << 30); // 128 GiB is assumed to be enoght for every use case of this arena

trait VirtArenaRaw {
    fn bytes_used(&self) -> usize;
    fn reset(&mut self);

    #[allow(clippy::mut_from_ref)]
    fn alloc_uninit<T: Sized>(&self) -> &mut MaybeUninit<T>;

    #[allow(clippy::mut_from_ref)]
    unsafe fn alloc_zeroed<T: Sized>(&self) -> &mut T {
        let uninit = self.alloc_uninit();
        *uninit = MaybeUninit::zeroed();
        uninit.assume_init_mut()
    }

    #[allow(clippy::mut_from_ref)]
    fn alloc_with<T: Sized>(&self, fun: impl FnOnce() -> T) -> &mut T {
        let uninit = self.alloc_uninit();
        uninit.write(fun());
        unsafe { uninit.assume_init_mut() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena() {
        let mut arena = VirtArena::default();

        struct Test {
            thing: Option<u32>,
        }

        let test1 = unsafe { arena.alloc_zeroed::<Test>() };
        assert!(test1.thing.is_none());

        let test2 = unsafe { arena.alloc_zeroed::<Test>() };
        assert!(test2.thing.is_none());

        test1.thing = Some(345);

        arena.reset();

        let test1 = unsafe { arena.alloc_uninit::<Test>().assume_init_mut() };
        assert_eq!(test1.thing, Some(345));

        let test2 = unsafe { arena.alloc_uninit::<Test>().assume_init_mut() };
        assert!(test2.thing.is_none());
    }
}
