use super::{Layer, Packet, ReadError, WriteError};
use bytes::{BufMut, BytesMut};
use heapless::Vec;

/// Application Layer
pub struct Apl;

impl Apl {
    pub const fn new() -> Self {
        Self
    }
}

impl Layer for Apl {
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError> {
        packet.apl = Vec::from_slice(buffer).map_err(|_| ReadError::Capacity)?;
        Ok(())
    }

    fn write<const N: usize>(
        &self,
        writer: &mut BytesMut,
        packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        writer.put_slice(&packet.apl);
        Ok(())
    }
}
