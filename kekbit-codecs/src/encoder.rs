use kekbit_core::api::Writer;
use kekbit_core::shm::writer::ShmWriter;
use std::io::Writer

pub trait Encoder{
    fn write(&mut self, w: Writer);
}

pub trait EncWriter {
    
}

pub struct KekbitEncWriter {
    out: impl Writer,
}

impl EncWriter for KekbitEncWriter {
    fn write(&mut self, data: impl Serialize) {}
}


#[cfg(test)]
mod test {
    use super::*;
    use std::io::Writer
    use crate::api::{InvalidPosition, Reader, Writer};
    

}