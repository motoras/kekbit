use crate::api::Encodable;
use crate::api::Reader;
use crate::api::WriteError;
use crate::api::Writer;
use crate::core::ReadResult;
use crate::core::TryIter;
use crossbeam_utils::Backoff;
use parking_lot::Mutex;
use std::iter::FusedIterator;
use std::iter::Iterator;
use std::sync::Arc;

/// A nonblocking iterator over messages in the channel, which tries multiple times to read
/// a message from a channel.
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

/// Writer which decorates another writer in order to make it available to multiple threads.
/// This writer is non-blocking but will try multiple times before it give up.
#[repr(transparent)]
pub struct RetryWriter<W: Writer> {
    mx_writer: Arc<Mutex<W>>,
}

impl<W: Writer> RetryWriter<W> {
    #[inline]
    pub fn new(mx_writer: Arc<Mutex<W>>) -> RetryWriter<W> {
        RetryWriter { mx_writer }
    }
}

impl<W: Writer> Writer for RetryWriter<W> {
    ///Tries to acquire the inner writer than writes the given message into channel.
    ///
    ///	# Errors
    ///
    /// Any error returned by the decorated writer will be passed on.
    /// WriteError::Wait will be returned if the inner writer cannot be acquired.
    #[inline]
    fn write<E: Encodable>(&mut self, data: &E) -> Result<u32, WriteError> {
        let backoff = Backoff::new();
        loop {
            let try_write = self.mx_writer.try_lock();
            match try_write {
                Some(mut writer) => {
                    return writer.write(data);
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
#[cfg(test)]
mod test {
    use super::*;
    use crate::api::*;
    use crate::core::*;
    use assert_matches::*;
    use tempdir::TempDir;
    #[test]
    fn retry_iter() {
        let metadata = Metadata::new(100, 1000, 10000, 1000, 1000, TickUnit::Millis);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let txt = "There are 10 kinds of people";
        let mut msgs = txt.split_whitespace();
        let mut writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
        let mut reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        let mut retry_iter: RetryIter<ShmReader> = reader.try_iter().into();
        assert_matches!(retry_iter.size_hint(), (0, None));
        assert_matches!(retry_iter.next(), Some(ReadResult::Nothing));
        for _i in 0..3 {
            let to_wr = msgs.next().unwrap().as_bytes();
            writer.write(&to_wr).unwrap();
        }
        for _i in 0..3 {
            assert_matches!(retry_iter.next(), Some(ReadResult::Record(_)));
        }
        assert_matches!(retry_iter.next(), Some(ReadResult::Nothing));
        std::mem::drop(writer);
        assert_matches!(retry_iter.next(), Some(ReadResult::Failed(ReadError::Closed)));
        assert_matches!(retry_iter.next(), None);
        assert_matches!(retry_iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn retry_write() {
        let metadata = Metadata::new(100, 1000, 10000, 1000, 1000, TickUnit::Millis);
        let test_tmp_dir = TempDir::new("kektest").unwrap();
        let writer = shm_writer(&test_tmp_dir.path(), &metadata, EncoderHandler::default()).unwrap();
        let arc_mx = Arc::new(Mutex::new(writer));
        let handles: Vec<std::thread::JoinHandle<()>> = (0..5)
            .map(|i| (i, arc_mx.clone()))
            .map(|(i, arc_mx)| {
                std::thread::spawn(move || {
                    let to_wr = format!("Hello {}", &i);
                    let mut retry_w = RetryWriter::new(arc_mx);
                    for _i in 0..3 {
                        loop {
                            match retry_w.write(&to_wr) {
                                Ok(_) => break,
                                Err(WriteError::Wait) => (),
                                Err(_) => panic!("Write failure"),
                            };
                        }
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let mut read_results = std::collections::HashMap::<&str, i32>::new();
        let mut shm_reader = shm_reader(&test_tmp_dir.path(), 1000).unwrap();
        let reader_iter = shm_reader.try_iter();
        for msg in reader_iter {
            match msg {
                ReadResult::Record(data) => {
                    *read_results.entry(std::str::from_utf8(data).unwrap()).or_default() += 1;
                }
                _ => break,
            }
        }
        assert_eq!(read_results.len(), 5);
        for (key, value) in read_results {
            dbg!(key, value);
            assert_eq!(value, 3);
        }
    }
}
