use crate::address::WMBusAddress;

use super::{Layer, ReadError, WriteError, Writer};

const HEADER_LENGTH: usize = 10;

pub struct Dll<A: Layer> {
    above: A,
}

pub struct DllFields {
    pub control: u8,
    pub address: WMBusAddress,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Incomplete,
    BcdConversion,
}

impl From<Error> for ReadError {
    fn from(value: Error) -> Self {
        match value {
            Error::Incomplete => ReadError::Incomplete,
            e => ReadError::Dll(e),
        }
    }
}

impl<A: Layer> Dll<A> {
    pub const fn new(above: A) -> Self {
        Self { above }
    }
}

impl<A: Layer> Layer for Dll<A> {
    fn read(&self, packet: &mut super::Packet, buffer: &[u8]) -> Result<(), ReadError> {
        if buffer.len() < HEADER_LENGTH {
            return Err(Error::Incomplete)?;
        }

        packet.dll = Some(DllFields {
            control: buffer[1],
            address: WMBusAddress::from_bytes(buffer[2..10].try_into().unwrap())
                .map_err(|_| Error::BcdConversion)?,
        });

        self.above.read(packet, &buffer[HEADER_LENGTH..])
    }

    fn write(&self, _writer: &mut impl Writer, _packet: &super::Packet) -> Result<(), WriteError> {
        todo!()
    }
}
