use dioxus_core::{ScopeId, ScopeState};
use std::{collections::HashSet, rc::Rc, sync::RwLock};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use crate::use_shared_state::ProvidedStateInner;
type ProvidedLock<T> = Rc<RwLock<ProvidedStateInner<T>>>;

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
        let mut root = self.state.inner.write().unwrap();
        root.consumers.remove(&self.scope_id);
    }
}

/// State that is shared between components through the context system
pub struct UseSharedLock<T> {
    pub(crate) inner: ProvidedLock<T>,
    gen: usize,
}

impl<T> UseSharedLock<T> {
    fn new(inner: ProvidedLock<T>) -> Self {
        let gen = inner.read().unwrap().gen;
        Self { inner, gen }
    }

    /// Notify all consumers of the state that it has changed. (This is called automatically when you call "write")
    pub fn notify_consumers(&self) {
        self.inner.write().unwrap().notify_consumers();
    }

    /// Read the shared value
    pub fn read(&self) -> RwLockReadGuard<'_, ProvidedStateInner<T>> {
        match self.inner.read() {
            Ok(value) => value,
            Err(message) => panic!(
                "Reading the shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Calling "write" will force the component to re-render
    pub fn write(&self) -> RwLockWriteGuard<'_, ProvidedStateInner<T>> {
        match self.inner.write() {
            Ok(value) => {
                self.notify_consumers();
                value
            },
            Err(message) => panic!(
                "Writing to shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Tries writing the value without forcing a re-render
    pub fn write_silent(&self) -> RwLockWriteGuard<'_, ProvidedStateInner<T>> {
        match self.inner.write() {
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
    pub fn with_mut<O>(&self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
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
        let state: ProvidedLock<T> = Rc::new(RwLock::new(ProvidedStateInner {
            value: f(),
            notify_any: cx.schedule_update_any(),
            consumers: HashSet::new(),
            gen: 0,
        }));

        cx.provide_context(state);
    });
}
