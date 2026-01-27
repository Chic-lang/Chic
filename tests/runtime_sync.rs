#![allow(unsafe_code)]

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
};
use std::thread;

use chic::runtime::{
    chic_rt_condvar_create, chic_rt_condvar_destroy, chic_rt_condvar_notify_one,
    chic_rt_condvar_wait, chic_rt_lock_create, chic_rt_lock_destroy, chic_rt_lock_enter,
    chic_rt_lock_exit, chic_rt_lock_is_held, chic_rt_lock_is_held_by_current_thread,
    chic_rt_mutex_create, chic_rt_mutex_destroy, chic_rt_mutex_lock, chic_rt_mutex_try_lock,
    chic_rt_mutex_unlock, chic_rt_once_complete, chic_rt_once_create, chic_rt_once_destroy,
    chic_rt_once_is_completed, chic_rt_once_try_begin, chic_rt_once_wait, chic_rt_rwlock_create,
    chic_rt_rwlock_destroy, chic_rt_rwlock_read_lock, chic_rt_rwlock_read_unlock,
    chic_rt_rwlock_try_write_lock, chic_rt_rwlock_write_unlock,
};

#[test]
fn runtime_mutex_coordinates_parallel_updates() {
    let mutex = unsafe { chic_rt_mutex_create() };
    assert_ne!(mutex, 0, "mutex handle should be non-zero");

    let counter = Arc::new(AtomicI32::new(0));

    let mut handles = Vec::new();
    for _ in 0..4 {
        let handle = mutex;
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1_000 {
                unsafe {
                    chic_rt_mutex_lock(handle);
                    let _ = chic_rt_mutex_try_lock(handle); // should fail while held
                    counter.fetch_add(1, Ordering::Relaxed);
                    chic_rt_mutex_unlock(handle);
                }
            }
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread");
    }

    unsafe {
        chic_rt_mutex_destroy(mutex);
    }

    assert_eq!(
        counter.load(Ordering::Relaxed),
        4_000,
        "all increments should be observed"
    );
}

#[test]
fn runtime_lock_serialises_workers() {
    let handle = unsafe { chic_rt_lock_create() };
    assert_ne!(handle, 0, "lock handle should be non-zero");

    let counter = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    for _ in 0..4 {
        let lock = handle;
        let thread_counter = Arc::clone(&counter);
        workers.push(thread::spawn(move || {
            for _ in 0..750 {
                unsafe {
                    chic_rt_lock_enter(lock);
                    assert!(
                        chic_rt_lock_is_held(lock),
                        "lock should report held while acquired"
                    );
                    assert!(
                        chic_rt_lock_is_held_by_current_thread(lock),
                        "lock should associate ownership with the current thread"
                    );
                    thread_counter.fetch_add(1, Ordering::Relaxed);
                    chic_rt_lock_exit(lock);
                }
            }
        }));
    }

    for worker in workers {
        worker.join().expect("lock worker thread");
    }

    unsafe {
        assert!(
            !chic_rt_lock_is_held(handle),
            "lock should not be held after workers finish"
        );
        chic_rt_lock_destroy(handle);
    }

    assert_eq!(counter.load(Ordering::Relaxed), 3_000);
}

#[test]
fn runtime_condvar_waits_for_notification() {
    let mutex = unsafe { chic_rt_mutex_create() };
    let condvar = unsafe { chic_rt_condvar_create() };

    let flag = Arc::new(AtomicBool::new(false));
    let signal_count = Arc::new(AtomicUsize::new(0));

    unsafe { chic_rt_mutex_lock(mutex) };

    let worker_flag = Arc::clone(&flag);
    let worker_signal = Arc::clone(&signal_count);
    let worker = thread::spawn(move || unsafe {
        chic_rt_mutex_lock(mutex);
        while !worker_flag.load(Ordering::Acquire) {
            chic_rt_condvar_wait(condvar, mutex);
        }
        worker_signal.fetch_add(1, Ordering::Relaxed);
        chic_rt_mutex_unlock(mutex);
    });

    thread::sleep(std::time::Duration::from_millis(10));
    flag.store(true, Ordering::Release);
    unsafe {
        chic_rt_condvar_notify_one(condvar);
        chic_rt_mutex_unlock(mutex);
    }

    worker.join().expect("worker thread joined");

    unsafe {
        chic_rt_condvar_destroy(condvar);
        chic_rt_mutex_destroy(mutex);
    }

    assert_eq!(signal_count.load(Ordering::Relaxed), 1);
}

#[test]
fn runtime_rwlock_supports_readers_and_single_writer() {
    let rwlock = unsafe { chic_rt_rwlock_create() };
    let value = Arc::new(AtomicI32::new(0));

    let mut readers = Vec::new();
    for _ in 0..3 {
        let handle = rwlock;
        let reader_value = Arc::clone(&value);
        readers.push(thread::spawn(move || unsafe {
            for _ in 0..500 {
                chic_rt_rwlock_read_lock(handle);
                let snapshot = reader_value.load(Ordering::Acquire);
                assert!(snapshot >= 0, "value should never be negative");
                chic_rt_rwlock_read_unlock(handle);
            }
        }));
    }

    let writer_value = Arc::clone(&value);
    let writer = thread::spawn(move || unsafe {
        for _ in 0..100 {
            while !chic_rt_rwlock_try_write_lock(rwlock) {
                std::hint::spin_loop();
            }
            let current = writer_value.load(Ordering::Relaxed);
            writer_value.store(current + 1, Ordering::Release);
            chic_rt_rwlock_write_unlock(rwlock);
        }
    });

    for reader in readers {
        reader.join().expect("reader thread");
    }
    writer.join().expect("writer thread");

    unsafe {
        chic_rt_rwlock_destroy(rwlock);
    }

    assert_eq!(value.load(Ordering::Relaxed), 100);
}

#[test]
fn runtime_once_allows_single_initialisation() {
    let once = unsafe { chic_rt_once_create() };
    assert_ne!(once, 0, "once handle should be non-zero");

    unsafe {
        assert!(
            chic_rt_once_try_begin(once),
            "first once_try_begin should succeed"
        );
        assert!(
            !chic_rt_once_try_begin(once),
            "second once_try_begin should fail once started"
        );
        assert!(
            !chic_rt_once_is_completed(once),
            "once should not be completed before complete is called"
        );
        chic_rt_once_complete(once);
        chic_rt_once_wait(once);
        assert!(chic_rt_once_is_completed(once));
    }

    unsafe {
        chic_rt_once_destroy(once);
    }
}
