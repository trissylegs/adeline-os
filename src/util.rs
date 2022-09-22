use core::{
    any::type_name,
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DebugHide<T>(pub T);

impl<T> DebugHide<T> {
    const fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T> From<T> for DebugHide<T> {
    fn from(t: T) -> Self {
        Self::new(t)
    }
}

impl<T> Debug for DebugHide<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(type_name::<T>()).finish_non_exhaustive()
    }
}

impl<T: Display> Display for DebugHide<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Deref for DebugHide<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for DebugHide<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
