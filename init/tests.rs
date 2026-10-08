use super::*;
use std::{
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

fn fault_waiters(populated: Option<u32>) {
    let init = Arc::new(Init::new());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (winner_tx, winner_rx) = mpsc::channel();
    let winner_init = init.clone();
    let winner = thread::spawn(move || {
        let result = winner_init.get_or_init(|| {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            populated
        });
        winner_tx.send(result).unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let (result_tx, result_rx) = mpsc::channel();
    let waiters: Vec<_> = (0..4)
        .map(|_| {
            let init = init.clone();
            let tx = result_tx.clone();
            thread::spawn(move || {
                tx.send(init.get_or_init(|| panic!("a waiter must not initialize")))
                    .unwrap()
            })
        })
        .collect();
    let until = Instant::now() + Duration::from_secs(2);
    while init.waiting.load(Ordering::Relaxed) < 4 && Instant::now() < until {
        thread::yield_now();
    }
    let all_waiting = init.waiting.load(Ordering::Relaxed) == 4;
    release_tx.send(()).unwrap();
    let winner_result = winner_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let until = Instant::now() + Duration::from_millis(300);
    let mut results = Vec::new();
    while results.len() < 4 {
        match result_rx.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(result) => results.push(result),
            Err(_) => break,
        }
    }
    let failed_count = init.count.load(Ordering::Acquire);
    // Test-only cleanup of the old faulty loop. No table entries/syscalls are
    // accessed: this isolated gate owns synthetic state, never the global list.
    if results.len() < 4 {
        init.count.store(1, Ordering::Release);
    }
    winner.join().unwrap();
    for waiter in waiters {
        waiter.join().unwrap();
    }
    assert!(all_waiting, "all waiters must enter the actual gated path");
    assert!(!winner_result, "empty/failed initialization cannot succeed");
    assert_eq!(failed_count, 0, "failure must not publish entries");
    assert_eq!(
        results,
        vec![false; 4],
        "waiters must observe failure without an artificial successful publication"
    );
    assert!(!init.get_or_init(|| panic!("a terminal failure must not retry")));
}

#[test]
fn failed_initializer_finishes_every_waiter() {
    fault_waiters(None);
}

#[test]
fn empty_table_is_failed_and_finishes_every_waiter() {
    fault_waiters(Some(0));
}

#[test]
fn successful_initialization_runs_once_and_publishes_to_racing_readers() {
    let init = Arc::new(Init::new());
    let payload = Arc::new(core::sync::atomic::AtomicUsize::new(0));
    let calls = Arc::new(core::sync::atomic::AtomicUsize::new(0));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let init = init.clone();
            let payload = payload.clone();
            let calls = calls.clone();
            thread::spawn(move || {
                assert!(init.get_or_init(|| {
                    calls.fetch_add(1, Ordering::Relaxed);
                    payload.store(42, Ordering::Relaxed);
                    Some(3)
                }));
                assert_eq!(init.count.load(Ordering::Acquire), 3);
                assert_eq!(payload.load(Ordering::Relaxed), 42);
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn real_windows_table_is_nonempty_and_shared_by_racing_callers() {
    let hash = crate::sw3_hash_function_name("ZwQuerySystemTime");
    let threads: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(move || {
                // SAFETY: this known export resolves against this process's real ntdll;
                // no syscall, OS state mutation or fabricated pointer is involved.
                unsafe {
                    let number = crate::sw3_get_syscall_number(hash);
                    assert_ne!(number, u32::MAX);
                    // WoW64 invokes its FS gate, not a native sysenter gadget.
                    #[cfg(target_arch = "x86_64")]
                    assert!(!crate::sw3_get_syscall_address(hash).is_null());
                    let count = crate::sw3_debug_get_count();
                    assert!(count > number && count as usize <= crate::SW3_MAX_ENTRIES);
                    assert_eq!(crate::sw3_debug_get_hash(number as usize), hash);
                    assert_eq!(crate::sw3_debug_get_hash(usize::MAX), 0);
                    assert!(crate::sw3_debug_get_syscall_addr(usize::MAX).is_null());
                    number
                }
            })
        })
        .collect();
    let numbers: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert!(numbers.iter().all(|number| *number == numbers[0]));
}

#[test]
fn full_table_is_failed_and_finishes_every_waiter() {
    // A full bounded reverse export walk may have omitted earlier exports;
    // address sorting that truncated set would assign incorrect syscall IDs.
    fault_waiters(Some(crate::SW3_MAX_ENTRIES as u32));
}

#[test]
fn resolved_table_supports_a_read_only_real_syscall() {
    let mut timestamp: crate::LARGE_INTEGER = 0;
    // SAFETY: this read-only query receives a valid aligned writable i64; the
    // real native/WoW64 path resolves against this process's Windows table.
    let status = unsafe { crate::nt_query_system_time(&mut timestamp) };
    assert!(crate::NT_SUCCESS(status));
    assert!(timestamp > 0);
}
