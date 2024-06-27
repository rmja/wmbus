use bytes::{BufMut, BytesMut};

use crate::address::WMBusAddress;

use super::{Layer, Packet, ReadError, WriteError};

const HEADER_LENGTH: usize = 10;

/// Data-Link Layer
pub struct Dll<A: Layer> {
    above: A,
}

#[derive(Clone)]
pub struct DllFields {
    pub control: u8,
    pub address: WMBusAddress,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError> {
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

    fn write<const N: usize>(
        &self,
        writer: &mut BytesMut,
        packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        let fields = packet.dll.as_ref().unwrap();
        writer.put_u8(fields.control);
        writer.put_slice(&fields.address.get_bytes());
        self.above.write(writer, packet)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        stack::{apl::Apl, Mode, Packet},
        DeviceType, ManufacturerCode,
    };

    use super::*;

    #[test]
    fn can_read_hyd_default() {
        // Given
        let mut packet: Packet = Packet::new(Mode::ModeTMTO);
        let dll = Dll::new(Apl::new());
        let buffer: [u8; 10] = [0x00, 0x00, 0x24, 0x23, 0x14, 0x89, 0x81, 0x44, 0x20, 0x04];

        // When
        dll.read(&mut packet, &buffer).unwrap();

        // Then
        assert_eq!(
            WMBusAddress::new(ManufacturerCode::HYD, 44818914, 0x20, DeviceType::Heat),
            packet.dll.unwrap().address
        );
    }

    #[test]
    fn can_read_hyd_reversed() {
        // Given
        let mut packet: Packet = Packet::new(Mode::ModeTMTO);
        let dll = Dll::new(Apl::new());
        let buffer: [u8; 10] = [0x00, 0x00, 0x24, 0x23, 0x85, 0x07, 0x47, 0x35, 0x04, 0x09];

        // When
        dll.read(&mut packet, &buffer).unwrap();

        // Then
        assert_eq!(
            WMBusAddress::new(ManufacturerCode::HYD, 09043547, 0x85, DeviceType::Water),
            packet.dll.unwrap().address
        );
    }
}
