use super::{Layer, Packet, ReadError, WriteError, Writer};
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
        packet.mbus_data = Vec::from_slice(buffer).map_err(|_| ReadError::Capacity)?;
        Ok(())
    }

    fn write<const N: usize>(&self, writer: &mut impl Writer, packet: &Packet<N>) -> Result<(), WriteError> {
        writer.write(&packet.mbus_data)
    }
}
