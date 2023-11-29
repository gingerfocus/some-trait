use core::fmt::Debug;

use alloc::boxed::Box;

/// Repersents an iterator that can return None with the expectation that it
/// will return Some in the future.
///
/// This is repersented through results enum. When using this trait there is an
/// expectation that the none variant is not signifigent.
/// repersents nothing and [`Eof`] repersent the true end the stream.
///
/// The goal is the make iterators that dont have to return and enum with some
/// garbage none type.
pub trait FallibleIterator {
    type SomeItem;
    type Error;

    /// Required implementation
    fn some_next(&mut self) -> Result<Option<Self::SomeItem>, Self::Error>;

    /// Polls this iterator until it returns a value. Returns Nothing when if
    /// finds an errors. This will continue polling the iterator even if it
    /// returns None.
    fn seek_next(&mut self) -> Option<Self::SomeItem> {
        loop {
            match self.some_next() {
                Some(Ok(t)) => return Some(t),
                Some(Err(_)) => return None,
                None => {}
            }
        }
    }

    /// Converts this Iterator into a [`Iterator`] that will skip none values
    /// and end when any error is returned.
    fn fuse_err(self) -> FuseIterAlways<Self>
    where
        Self: Sized + 'static,
    {
        FuseIterAlways { inner: self }
    }

    /// Converts this [`Iterator`] into a [`FuseIter`] that will skip none
    /// values. It ends when the specified error is returned and all other
    /// errors are skipped.
    fn fuse_err_on(self, error: <Self as FallibleIterator>::Error) -> FuseIterVariant<Self>
    where
        Self: Sized + 'static,
        Self::Error: PartialEq + 'static,
    {
        FuseIterVariant {
            inner: self,
            variant: error,
        }
    }

    /// Cuts the iterator off when the closure returns ture. Skips none values
    fn fuse_err_when<F>(self, f: F) -> FuseIterClosure<Self>
    where
        Self: Sized + 'static,
        F: Fn(Self::Error) -> bool + 'static,
    {
        FuseIterClosure {
            inner: self,
            cond: Box::new(f),
        }
    }
}

/// A wrapper type around [`FallibleIteratorExt`] that skips the None values
#[derive(Debug)]
pub struct FuseIterAlways<I>
where
    I: FallibleIterator,
{
    inner: I,
}

impl<I> Iterator for FuseIterAlways<I>
where
    I: FallibleIterator,
{
    type Item = I::SomeItem;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.seek_next()
    }
}

/// A wrapper type around [`FallibleIteratorExt`] that skips the None values
#[derive(Debug, Hash)]
pub struct FuseIterVariant<I>
where
    I: FallibleIterator,
    I::Error: PartialEq,
{
    inner: I,
    variant: I::Error,
}

impl<I> Iterator for FuseIterVariant<I>
where
    I: FallibleIterator,
    I::Error: PartialEq,
{
    type Item = I::SomeItem;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.some_next() {
                Some(Ok(t)) => return Some(t),
                Some(Err(e)) if e == self.variant => return None,
                Some(Err(_)) | None => {}
            }
        }
    }
}

/// A wrapper type around [`FallibleIteratorExt`] that skips the None values
pub struct FuseIterClosure<I>
where
    I: FallibleIterator,
{
    inner: I,
    cond: Box<dyn Fn(I::Error) -> bool>,
}

impl<I> Iterator for FuseIterClosure<I>
where
    I: FallibleIterator,
{
    type Item = I::SomeItem;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.some_next() {
                Some(Ok(t)) => return Some(t),
                Some(Err(e)) => {
                    if !(self.cond)(e) {
                        return None;
                    }
                }
                None => {}
            }
        }
    }
}

impl<I, T, E> FallibleIterator for I where I: Iterator<Item = Result<T, E>> {}

use core::ops::{Deref, DerefMut};

pub struct W<T>(T);

#[rustfmt::skip]
impl<T> DerefMut for W<T> {
    #[inline] fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

#[rustfmt::skip]
impl<T> Deref for W<T> {
    type Target = T;
    #[inline] fn deref(&self) -> &Self::Target { &self.0 }
}

impl<FI> IntoIterator for W<FI>
where
    FI: FallibleIterator,
{
    type Item = Result<FI::SomeItem, FI::Error>;
    type IntoIter = IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { inner: self }
    }
}

pub struct IntoIter<FI> {
    inner: FI,
}

#[cfg(test)]
mod test {
    use super::*;

    struct A;

    impl Iterator for A {
        type Item = Result<(), ()>;

        fn next(&mut self) -> Option<Self::Item> {
            Some(Ok(()))
        }
    }

    #[test]
    fn name() {
        let a = A;
        for _ in a.fuse_err() {}
    }
}
