//! A thread whose first call into mimalloc reallocates a block another thread
//! allocated. Where mimalloc keeps a thread's theap in a pthread key or a TLS
//! slot (Android, Windows), such a thread has no theap yet when the call
//! arrives. The test harness allocates through the system heap, so the only
//! mimalloc calls a spawned thread makes are the ones written here.

use mimalloc::MiMalloc;
use std::alloc::{GlobalAlloc, Layout};

/// A block of `size` bytes filled with a pattern, allocated by this thread,
/// and the number of bytes mimalloc says it can hold.
fn block_on_this_thread(size: usize) -> (usize, usize) {
    let p = unsafe { MiMalloc.alloc(Layout::from_size_align(size, 1).unwrap()) };
    assert!(!p.is_null());
    for i in 0..size {
        unsafe { p.add(i).write(i as u8) };
    }
    let usable = unsafe { libmimalloc_sys::mi_usable_size(p as *const _) };
    assert!(usable >= size);
    (p as usize, usable)
}

/// Reallocates the block at `at` as the first heap call of a new thread,
/// checks the bytes it keeps and frees it there.
fn realloc_as_the_first_call_of_a_new_thread(at: usize, size: usize, new_size: usize) {
    std::thread::spawn(move || {
        let layout = Layout::from_size_align(size, 1).unwrap();
        let q = unsafe { MiMalloc.realloc(at as *mut u8, layout, new_size) };
        assert!(!q.is_null(), "realloc of {size} bytes to {new_size} answered null");
        for i in 0..size.min(new_size) {
            assert_eq!(unsafe { q.add(i).read() }, i as u8, "byte {i} survives the realloc");
        }
        unsafe { MiMalloc.dealloc(q, Layout::from_size_align(new_size, 1).unwrap()) };
    })
    .join()
    .expect("the thread runs to its end");
}

#[test]
fn a_new_thread_grows_a_block_in_place_as_its_first_heap_call() {
    for size in [24usize, 100, 1000, 3000] {
        let (at, usable) = block_on_this_thread(size);
        realloc_as_the_first_call_of_a_new_thread(at, size, usable);
    }
}

#[test]
fn a_new_thread_shrinks_a_block_in_place_as_its_first_heap_call() {
    for size in [64usize, 1000, 4000] {
        let (at, _) = block_on_this_thread(size);
        realloc_as_the_first_call_of_a_new_thread(at, size, size - size / 4);
    }
}

#[test]
fn a_new_thread_moves_a_block_as_its_first_heap_call() {
    for size in [24usize, 1000, 100_000] {
        let (at, usable) = block_on_this_thread(size);
        realloc_as_the_first_call_of_a_new_thread(at, size, usable * 2 + 1);
    }
}
