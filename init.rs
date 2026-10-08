//! Single-writer publication gate shared by the native table and fault tests.
use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

#[repr(C)]
pub(super) struct Init {
    pub(super) count: AtomicU32,
    state: AtomicU8,
    #[cfg(test)]
    waiting: core::sync::atomic::AtomicUsize,
}

impl Init {
    pub(super) const fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
            state: AtomicU8::new(UNINITIALIZED),
            #[cfg(test)]
            waiting: core::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Publish a nonempty table or a permanent failure. A failed native walk
    /// must finish every waiter; retrying cannot repair waiters on the old gate.
    /// A full reverse export walk may be truncated and cannot define valid IDs.
    pub(super) fn get_or_init(&self, populate: impl FnOnce() -> Option<u32>) -> bool {
        match self.state.load(Ordering::Acquire) {
            READY => return true,
            FAILED => return false,
            _ => {}
        }
        if self
            .state
            .compare_exchange(
                UNINITIALIZED,
                INITIALIZING,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            match populate() {
                Some(count) if count > 0 && (count as usize) < crate::SW3_MAX_ENTRIES => {
                    // Debug readers also acquire count directly. The mutable
                    // table borrow has ended before either publication store.
                    self.count.store(count, Ordering::Release);
                    self.state.store(READY, Ordering::Release);
                    return true;
                }
                _ => {
                    self.state.store(FAILED, Ordering::Release);
                    return false;
                }
            }
        }
        #[cfg(test)]
        self.waiting.fetch_add(1, Ordering::Relaxed);
        loop {
            match self.state.load(Ordering::Acquire) {
                READY => return true,
                FAILED => return false,
                _ => core::hint::spin_loop(),
            }
        }
    }
}

#[cfg(test)]
mod tests;
