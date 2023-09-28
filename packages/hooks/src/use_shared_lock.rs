use dioxus_core::{ScopeId, ScopeState};
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::HashSet, sync::RwLock};

struct ProvidedLock<T> {
    value: Arc<RwLock<T>>,
    notify_any: Arc<dyn Fn(ScopeId)>,
    consumers: Arc<RwLock<HashSet<ScopeId>>>,
    gen: usize,
}

impl<T> Clone for ProvidedLock<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            notify_any: self.notify_any.clone(),
            consumers: self.consumers.clone(),
            gen: self.gen,
        }
    }
}

impl<T> ProvidedLock<T> {
    pub(crate) fn notify_consumers(&mut self) {
        self.gen += 1;
        for consumer in self.consumers.write().unwrap().iter() {
            (self.notify_any)(*consumer);
        }
    }
}

// Tracks all the subscribers to a shared State
pub fn use_shared_lock<T: 'static>(cx: &ScopeState) -> Option<&UseSharedLock<T>> {
    let state_owner: &mut Option<UseSharedLockOwner<T>> = &mut *cx.use_hook(move || {
        let scope_id = cx.scope_id();
        let root = cx.consume_context::<ProvidedLock<T>>()?;

        root.consumers.write().unwrap().insert(scope_id);

        let state = UseSharedLock::new(root);
        let owner = UseSharedLockOwner { state, scope_id };
        Some(owner)
    });
    state_owner.as_mut().map(|s| &s.state)
}

/// This wrapper detects when the hook is dropped and will unsubscribe when the component is unmounted
struct UseSharedLockOwner<T> {
    state: UseSharedLock<T>,
    scope_id: ScopeId,
}

impl<T> Drop for UseSharedLockOwner<T> {
    fn drop(&mut self) {
        // we need to unsubscribe when our component is unmounted
        self.state
            .inner
            .consumers
            .write()
            .unwrap()
            .remove(&self.scope_id);
    }
}

/// State that is shared between components through the context system
pub struct UseSharedLock<T> {
    pub(crate) inner: ProvidedLock<T>,
}

impl<T> UseSharedLock<T> {
    fn new(inner: ProvidedLock<T>) -> Self {
        Self { inner }
    }

    /// Notify all consumers of the state that it has changed. (This is called automatically when you call "write")
    pub fn notify_consumers(&mut self) {
        self.inner.notify_consumers();
    }

    /// Read the shared value
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        match self.inner.value.read() {
            Ok(value) => value,
            Err(message) => panic!(
                "Reading the shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Calling "write" will force the component to re-render
    pub fn write(&mut self) -> RwLockWriteGuard<'_, T> {
        match self.inner.value.write() {
            Ok(value) => {
                self.notify_consumers();
                value
            }
            Err(message) => panic!(
                "Writing to shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Tries writing the value without forcing a re-render
    pub fn write_silent(&self) -> RwLockWriteGuard<'_, T> {
        match self.inner.value.write() {
            Ok(value) => value,
            Err(message) => panic!(
                "Writing to shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Take a reference to the inner value temporarily and produce a new value
    pub fn with<O>(&self, immutable_callback: impl FnOnce(&T) -> O) -> O {
        immutable_callback(&*self.read())
    }

    /// Take a mutable reference to the inner value temporarily and produce a new value
    pub fn with_mut<O>(&mut self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
        mutable_callback(&mut *self.write())
    }
}

impl<T> Clone for UseSharedLock<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> PartialEq for UseSharedLock<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.gen == other.inner.gen
    }
}

pub fn use_shared_lock_provider<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) {
    cx.use_hook(|| {
        cx.provide_context(ProvidedLock {
            value: Arc::new(RwLock::new(f())),
            notify_any: cx.schedule_update_any(),
            consumers: Arc::new(RwLock::new(HashSet::new())),
            gen: 0,
        });
    });
}
