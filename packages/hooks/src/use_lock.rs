use dioxus_core::ScopeState;
use std::sync::Arc;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub fn use_lock<T: 'static>(cx: &ScopeState, initialize_rwlock: impl FnOnce() -> T) -> &UseLock<T> {
    cx.use_hook(|| UseLock {
        update: cx.schedule_update(),
        value: Arc::new(RwLock::new(initialize_rwlock())),
        gen: 0,
    })
}

pub struct UseLock<T> {
    update: Arc<dyn Fn()>,
    value: Arc<RwLock<T>>,
    gen: usize,
}

impl<T> Clone for UseLock<T> {
    fn clone(&self) -> Self {
        Self {
            update: self.update.clone(),
            value: self.value.clone(),
            gen: self.gen,
        }
    }
}

impl<T> UseLock<T> {
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.value.read().unwrap()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.needs_update();
        self.value.write().unwrap()
    }

    pub fn set(&self, new: T) {
        *self.value.write().unwrap() = new;
        self.needs_update();
    }

    pub fn write_silent(&self) -> RwLockWriteGuard<'_, T> {
        self.value.write().unwrap()
    }

    pub fn with<O>(&self, immutable_callback: impl FnOnce(&T) -> O) -> O {
        immutable_callback(&*self.read())
    }

    pub fn with_mut<O>(&self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
        mutable_callback(&mut *self.write())
    }

    pub fn needs_update(&self) {
        (self.update)();
    }
}

impl<T> PartialEq for UseLock<T> {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.value, &other.value) {
            self.gen == other.gen
        } else {
            false
        }
    }
}
