use crate::use_shared_state::ProvidedStateInner;
use dioxus_core::{ScopeId, ScopeState};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::HashSet, sync::RwLock};

type ProvidedLock<T> = Arc<RwLock<ProvidedStateInner<T>>>;

pub struct SharedLockReadGuard<'a, T> {
    guard: RwLockReadGuard<'a, ProvidedStateInner<T>>,
}

impl<'a, T> Deref for SharedLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard.value
    }
}

pub struct SharedLockWriteGuard<'a, T> {
    guard: RwLockWriteGuard<'a, ProvidedStateInner<T>>,
}

impl<'a, T> Deref for SharedLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard.value
    }
}

impl<'a, T> DerefMut for SharedLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard.value
    }
}

// Tracks all the subscribers to a shared State
pub fn use_shared_lock<T: 'static>(cx: &ScopeState) -> Option<&UseSharedLock<T>> {
    let state_owner: &mut Option<UseSharedLockOwner<T>> = &mut *cx.use_hook(move || {
        let scope_id = cx.scope_id();
        let root = cx.consume_context::<ProvidedLock<T>>()?;

        root.write().unwrap().consumers.insert(scope_id);

        let state = UseSharedLock::new(root);
        let owner = UseSharedLockOwner { state, scope_id };
        Some(owner)
    });
    state_owner.as_mut().map(|s| {
        s.state.gen = s.state.inner.read().unwrap().gen;
        &s.state
    })
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
            .write()
            .unwrap()
            .consumers
            .remove(&self.scope_id);
    }
}

/// State that is shared between components through the context system
pub struct UseSharedLock<T> {
    inner: ProvidedLock<T>,
    gen: usize,
}

impl<T> UseSharedLock<T> {
    fn new(inner: ProvidedLock<T>) -> Self {
        let gen = inner.read().unwrap().gen;
        Self { inner, gen }
    }

    /// Read the shared value
    pub fn read(&self) -> SharedLockReadGuard<T> {
        match self.inner.read() {
            Ok(guard) => SharedLockReadGuard { guard },
            Err(message) => panic!(
                "Reading the shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Calling "write" will force the component to re-render
    pub fn write(&self) -> SharedLockWriteGuard<'_, T> {
        match self.inner.write() {
            Ok(mut guard) => {
                guard.notify_consumers();
                SharedLockWriteGuard { guard }
            }
            Err(message) => panic!(
                "Reading the shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Tries writing the value without forcing a re-render
    pub fn write_silent(&self) -> SharedLockWriteGuard<'_, T> {
        match self.inner.write() {
            Ok(guard) => SharedLockWriteGuard { guard },
            Err(message) => panic!(
                "Reading the shared state failed: {}\n({:?})",
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
            gen: self.gen,
        }
    }
}

impl<T> PartialEq for UseSharedLock<T> {
    fn eq(&self, other: &Self) -> bool {
        self.gen == other.gen
    }
}

pub fn use_shared_lock_provider<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) {
    cx.use_hook(|| {
        cx.provide_context(Arc::new(RwLock::new(ProvidedStateInner {
            value: f(),
            notify_any: cx.schedule_update_any(),
            consumers: HashSet::new(),
            gen: 0,
        })));
    });
}
