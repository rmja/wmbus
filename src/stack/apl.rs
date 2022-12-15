use super::{Layer, Packet, ReadError};
use alloc::vec::Vec;

/// Application Layer
pub struct Apl;

impl Apl {
    pub const fn new() -> Self {
        Self
    }
}

impl Layer for Apl {
    fn read(&self, packet: &mut Packet, buffer: &[u8]) -> Result<(), ReadError> {
        packet.mbus_data = buffer.to_vec();
        Ok(())
    }

    fn write(&self, writer: &mut Vec<u8>, packet: &Packet) {
        writer.extend_from_slice(&packet.mbus_data);
    }
}
