use crate::api::Encodable;
use crate::api::Handler;
use crate::core::TickUnit;
use std::io::Result;
use std::io::Write;

/// Handler which adds a timestamp to a record using a given tick unit.
/// This is probably the most used decorator.
#[derive(Debug)]
#[repr(transparent)]
pub struct TimestampHandler {
    ///the tick unit used for timestamps
    tick: TickUnit,
}

impl Handler for TimestampHandler {
    ///Writes a time stamp into a channel before a record.
    #[inline]
    fn incoming(&mut self, _data: &impl Encodable, w: &mut impl Write) -> Result<usize> {
        w.write(&self.tick.nix_time().to_le_bytes())
    }
}

impl TimestampHandler {
    ///Creates a new TimestampHandler which will provide time stamps using the given tick unit
    #[inline]
    pub fn new(tick: TickUnit) -> Self {
        TimestampHandler { tick }
    }
}

///Handler which adds a sequence id to a record.
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct SequenceHandler {
    seq: u64,
}

impl SequenceHandler {
    ///Creates a new SequenceHandler which will start from the given number.
    ///
    /// # Arguments
    ///
    /// * `seq` - Starting number of the sequence
    ///
    #[inline]
    pub fn new(seq: u64) -> SequenceHandler {
        SequenceHandler { seq }
    }
}

impl Handler for SequenceHandler {
    ///Writes a sequence number into a channel before a record.
    #[inline]
    fn incoming(&mut self, _data: &impl Encodable, w: &mut impl Write) -> Result<usize> {
        self.seq += 1;
        w.write(&self.seq.to_le_bytes())
    }
}
/// A handler which chains two handlers.
/// Chaining mulltiple such handlers will generate a complex chain of handlers
/// used to preproces/write/postprocess a record.
pub struct ChainedHandler<H: Handler, D: Handler> {
    decorator: Box<D>,
    handler: Box<H>,
}

impl<H: Handler, D: Handler> ChainedHandler<H, D> {
    /// Links to handlers together
    /// Return a handler which first will compose the given handlers
    ///
    /// # Arguments
    ///
    /// * `handler` - The bottom handler the one will be wrapped
    /// * `decorator` - The top handler the one will decorate the bottom one
    ///
    #[inline]
    pub fn link(handler: H, decorator: D) -> ChainedHandler<H, D> {
        ChainedHandler {
            decorator: Box::new(decorator),
            handler: Box::new(handler),
        }
    }
}

impl<H: Handler, D: Handler> Handler for ChainedHandler<H, D> {
    #[inline]
    fn handle(&mut self, d: &impl Encodable, w: &mut impl Write) -> Result<usize> {
        self.decorator
            .incoming(d, w)
            .and_then(|_| self.handler.handle(d, w))
            .and_then(|_| self.decorator.outgoing(d, w))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;
    use std::io::Read;

    #[test]
    fn test_ts_handler() {
        let tick = TickUnit::Nanos;
        let mut ts_handler = TimestampHandler::new(tick);
        let before = tick.nix_time();
        let c = &mut std::io::Cursor::new(Vec::new());
        ts_handler.handle(&"Doesn't matter".to_string(), c).unwrap();
        ts_handler.handle(&"Doesn't matter".to_string(), c).unwrap();
        ts_handler.handle(&"Doesn't matter".to_string(), c).unwrap();
        let after = tick.nix_time();
        c.set_position(0);
        for _i in 0..3 {
            let mut res = vec![0u8; 8];
            c.read_exact(&mut res).unwrap();
            let ts = u64::from_le_bytes(res[..].try_into().unwrap());
            assert!(ts > before);
            assert!(ts < after);
        }
    }

    #[test]
    fn test_seq_handler() {
        let mut seq_handler = SequenceHandler::new(47);
        assert_eq!(seq_handler.seq, 47);
        let expected = vec![48, 49, 50];
        let c = &mut std::io::Cursor::new(Vec::new());
        seq_handler.handle(&"Doesn't matter".to_string(), c).unwrap();
        seq_handler.handle(&"Doesn't matter".to_string(), c).unwrap();
        seq_handler.handle(&"Doesn't matter".to_string(), c).unwrap();
        c.set_position(0);
        for i in 0..3 {
            let mut res = vec![0u8; 8];
            c.read_exact(&mut res).unwrap();
            let id = u64::from_le_bytes(res[..].try_into().unwrap());
            assert_eq!(id, expected[i]);
        }
        assert_eq!(seq_handler.seq, 50);
        let seq_handler_def = SequenceHandler::default();
        assert_eq!(seq_handler_def.seq, 0);
    }

    #[test]
    fn test_chain() {
        let h1 = IdHandler { id: 1 };
        let h2 = IdHandler { id: 2 };
        let l1 = ChainedHandler::link(h1, h2);
        let h3 = IdHandler { id: 3 };
        let l2 = ChainedHandler::link(l1, h3);
        let h4 = InHandler::default();
        let l3 = ChainedHandler::link(l2, h4);
        let h5 = OutHandler::default();
        let mut chain = ChainedHandler::link(l3, h5);
        let c = &mut std::io::Cursor::new(Vec::new());
        chain.handle(&"Doesn't matter".to_string(), c).unwrap();
        let expected = vec![-1, 3, 2, 1, 1, 2, 3, -1];
        c.set_position(0);
        for i in 0..8 {
            let mut res = vec![0u8; 8];
            c.read_exact(&mut res).unwrap();
            let id = i64::from_le_bytes(res[..].try_into().unwrap());
            assert_eq!(id, expected[i]);
        }
    }

    struct IdHandler {
        id: u64,
    }

    impl Handler for IdHandler {
        fn incoming(&mut self, _data: &impl Encodable, w: &mut impl Write) -> Result<usize> {
            w.write(&self.id.to_le_bytes()[..])
        }

        fn outgoing(&mut self, _data: &impl Encodable, w: &mut impl Write) -> Result<usize> {
            w.write(&self.id.to_le_bytes()[..])
        }
    }

    #[derive(Default)]
    struct InHandler {}

    impl Handler for InHandler {
        fn incoming(&mut self, _data: &impl Encodable, w: &mut impl Write) -> Result<usize> {
            w.write(&(-1i64).to_le_bytes()[..])
        }
    }

    #[derive(Default)]
    struct OutHandler {}

    impl Handler for OutHandler {
        fn outgoing(&mut self, _data: &impl Encodable, w: &mut impl Write) -> Result<usize> {
            w.write(&(-1i64).to_le_bytes()[..])
        }
    }
}
