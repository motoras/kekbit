use crate::api::Encodable;
use crate::api::Handler;
use crate::core::TickUnit;
use std::io::Result;
use std::io::Write;

pub struct TimestampHandler {
    tick: TickUnit,
}

impl Handler for TimestampHandler {
    #[inline]
    fn incoming(&mut self, w: &mut impl Write) -> Result<usize> {
        w.write(&self.tick.nix_time().to_le_bytes())
    }
}

pub struct SequenceHandler {
    seq: u64,
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

    #[test]
    fn check_ts() {
        // let s = "ABCDEF";
        // let ts = 1234567890;
        // let tse = TsEncodable {
        //     encodable: &s,
        //     timestamp: ts,
        // };
        // println!("{:?}", tse.timestamp);
    }
}
