use crate::api::Encodable;
use crate::api::Handler;
use crate::core::TickUnit;
use std::io::Result;
use std::io::Write;

#[derive(Debug)]
pub struct TimestampHandler {
    tick: TickUnit,
}

impl Handler for TimestampHandler {
    #[inline]
    fn incoming(&mut self, w: &mut impl Write) -> Result<usize> {
        w.write(&self.tick.nix_time().to_le_bytes())
    }
}

impl TimestampHandler {
    #[inline]
    pub fn new(tick: TickUnit) -> Self {
        TimestampHandler { tick }
    }
}

#[derive(Default, Debug)]
pub struct SequenceHandler {
    seq: u64,
}

impl SequenceHandler {
    #[inline]
    pub fn new(seq: u64) -> SequenceHandler {
        SequenceHandler { seq }
    }
}

impl Handler for SequenceHandler {
    #[inline]
    fn incoming(&mut self, w: &mut impl Write) -> Result<usize> {
        self.seq += 1;
        w.write(&self.seq.to_le_bytes())
    }
}

pub struct ChainedHandler<H: Handler, D: Handler> {
    decorator: Box<D>,
    handler: Box<H>,
}

impl<H: Handler, D: Handler> ChainedHandler<H, D> {
    #[inline]
    pub fn link(handler: Box<H>, decorator: Box<D>) -> ChainedHandler<H, D> {
        ChainedHandler { decorator, handler }
    }
}

impl<H: Handler, D: Handler> Handler for ChainedHandler<H, D> {
    #[inline]
    fn handle(&mut self, d: &impl Encodable, w: &mut impl Write) -> Result<usize> {
        self.decorator
            .incoming(w)
            .and_then(|_| self.handler.handle(d, w))
            .and_then(|_| self.decorator.outgoing(w))
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
        let l1 = ChainedHandler::link(Box::new(h1), Box::new(h2));
        let h3 = IdHandler { id: 3 };
        let mut chain = ChainedHandler::link(Box::new(l1), Box::new(h3));
        let c = &mut std::io::Cursor::new(Vec::new());
        chain.handle(&"Doesn't matter".to_string(), c).unwrap();
        let expected = vec![3, 2, 1, 1, 2, 3];
        c.set_position(0);
        for i in 0..6 {
            let mut res = vec![0u8; 8];
            c.read_exact(&mut res).unwrap();
            let id = u64::from_le_bytes(res[..].try_into().unwrap());
            assert_eq!(id, expected[i]);
        }
    }

    struct IdHandler {
        id: i64,
    }

    impl Handler for IdHandler {
        fn incoming(&mut self, w: &mut impl Write) -> Result<usize> {
            w.write(&self.id.to_le_bytes()[..])
        }

        fn outgoing(&mut self, w: &mut impl Write) -> Result<usize> {
            w.write(&self.id.to_le_bytes()[..])
        }
    }
}
