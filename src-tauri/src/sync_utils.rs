use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub fn lock_mutex_recover<'a, T>(lock: &'a Mutex<T>, name: &str) -> MutexGuard<'a, T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::warn!("{name} mutex poisoned, recovering inner state");
            e.into_inner()
        }
    }
}

pub fn read_lock_recover<'a, T>(lock: &'a RwLock<T>, name: &str) -> RwLockReadGuard<'a, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(e) => {
            log::warn!("{name} read lock poisoned, recovering inner state");
            e.into_inner()
        }
    }
}

pub fn write_lock_recover<'a, T>(lock: &'a RwLock<T>, name: &str) -> RwLockWriteGuard<'a, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(e) => {
            log::warn!("{name} write lock poisoned, recovering inner state");
            e.into_inner()
        }
    }
}
