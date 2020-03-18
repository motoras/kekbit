use crate::api::Encodable;
use crate::api::Reader;
use crate::api::WriteError;
use crate::api::Writer;
use crate::core::ReadResult;
use crate::core::TryIter;
use crossbeam_utils::atomic::AtomicCell;
use crossbeam_utils::Backoff;
use std::iter::FusedIterator;
use std::iter::Iterator;

/// A nonblocking iterator over messages in the channel, which tries multiple times to read
/// a message if available.
#[repr(transparent)]
pub struct RetryIter<'a, R: Reader> {
    inner: TryIter<'a, R>,
}

impl<'a, R: Reader> From<TryIter<'a, R>> for RetryIter<'a, R> {
    fn from(try_iter: TryIter<'a, R>) -> RetryIter<'a, R> {
        RetryIter { inner: try_iter }
    }
}

impl<'a, R: Reader> Iterator for RetryIter<'a, R> {
    type Item = ReadResult<'a>;
    /// Tries multiple times to read a message from channel.
    ///
    /// # Errors
    ///
    /// If the ReadResult is a Failure all subsequent call will return None.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let backoff = Backoff::new();
        loop {
            let res = self.inner.next();
            match res {
                Some(ReadResult::Nothing) => {
                    if backoff.is_completed() {
                        return res;
                    } else {
                        backoff.snooze();
                    }
                }
                Some(_) => return res,
                None => return None,
            }
        }
    }
    ///Returns (0, None) if records may be still available in the channel or (0, Some(0)) if
    ///the channel is exhausted.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, R: Reader> FusedIterator for RetryIter<'a, R> {}

pub struct RetryWriter<W: Writer> {
    atm_writer: AtomicCell<Option<W>>,
}

impl<W: Writer> Writer for RetryWriter<W> {
    fn write<E: Encodable>(&mut self, data: &E) -> Result<u32, WriteError> {
        let backoff = Backoff::new();
        loop {
            let try_write = self.atm_writer.take();
            match try_write {
                Some(mut writer) => {
                    let res = writer.write(data);
                    self.atm_writer.store(Some(writer));
                    return res;
                }
                None => {
                    if backoff.is_completed() {
                        return Err(WriteError::Wait);
                    } else {
                        backoff.snooze();
                    }
                }
            }
        }
    }
}
