use core::{
    any::type_name,
    fmt::{Debug, Display, Formatter, self},
    ops::{Deref, DerefMut},
};

use crate::{console};

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

static INDENT: &'static str = "                                                                                ";

pub struct IndentPrint {
    depth: usize,
    newline: bool,
}
impl IndentPrint {
    pub(crate) fn new(depth: u8) -> Self {
        Self { depth: depth as usize, newline: true, }
    }
}

impl fmt::Write for IndentPrint {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut out = console::lock();

        let indent = &INDENT[0..self.depth];
        let mut rest = s;
        while rest.len() > 0 {
            match rest.split_once('\n') {
                Some((a, b)) => {
                    if self.newline {
                        writeln!(out, "{indent}{a}")?;
                    } else {
                        writeln!(out, "{a}")?;
                    }
                    self.newline = true;
                    rest = b;
                },

                None => {
                    if self.newline {
                        write!(out, "{indent}{rest}")?;
                    } else {
                        write!(out, "{rest}")?;
                    }
                    self.newline = false;
                    rest = "";
                }
            }
        }
        Ok(())
    }
}