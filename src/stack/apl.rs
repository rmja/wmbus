use super::{Layer, Packet, ReadError, WriteError, Writer};

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

    fn write(&self, writer: &mut impl Writer, packet: &Packet) -> Result<(), WriteError> {
        writer.write(&packet.mbus_data)
    }
}
