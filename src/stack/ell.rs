use super::{Layer, Packet, ReadError, WriteError, Writer};
use crate::address::WMBusAddress;

pub struct Ell<A: Layer> {
    above: A,
}

#[derive(PartialEq)]
pub enum EllFields {
    Short {
        cc: u8,
        acc: u8,
    },
    Long {
        cc: u8,
        acc: u8,
        sn: u32,
        payload_crc: Option<u16>,
    },
    ShortDest {
        cc: u8,
        acc: u8,
        dest: WMBusAddress,
    },
    LongDest {
        cc: u8,
        acc: u8,
        dest: WMBusAddress,
        sn: u32,
        payload_crc: Option<u16>,
    },
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
            e => ReadError::Ell(e),
        }
    }
}

impl<A: Layer> Ell<A> {
    pub const fn new(above: A) -> Self {
        Self { above }
    }
}

impl EllFields {
    pub const fn ci(&self) -> u8 {
        match self {
            EllFields::Short { .. } => 0x8C,
            EllFields::Long { .. } => 0x8D,
            EllFields::ShortDest { .. } => 0x8E,
            EllFields::LongDest { .. } => 0x8F,
        }
    }
}

impl<A: Layer> Layer for Ell<A> {
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError> {
        let mut offset = 0;
        if !buffer.is_empty() {
            if let Some(header_length) = header_length(buffer[0]) {
                if buffer.len() < header_length {
                    return Err(Error::Incomplete)?;
                }
                packet.ell = match buffer[0] {
                    0x8C => Some(EllFields::Short {
                        cc: buffer[1],
                        acc: buffer[2],
                    }),
                    0x8D => Some(EllFields::Long {
                        cc: buffer[1],
                        acc: buffer[2],
                        sn: u32::from_le_bytes(buffer[3..7].try_into().unwrap()),
                        payload_crc: Some(u16::from_le_bytes(buffer[7..9].try_into().unwrap())),
                    }),
                    0x8E => Some(EllFields::ShortDest {
                        cc: buffer[1],
                        acc: buffer[2],
                        dest: WMBusAddress::from_bytes(buffer[3..11].try_into().unwrap())
                            .map_err(|_| Error::BcdConversion)?,
                    }),
                    0x8F => Some(EllFields::LongDest {
                        cc: buffer[1],
                        acc: buffer[2],
                        dest: WMBusAddress::from_bytes(buffer[3..11].try_into().unwrap())
                            .map_err(|_| Error::BcdConversion)?,
                        sn: u32::from_le_bytes(buffer[11..15].try_into().unwrap()),
                        payload_crc: Some(u16::from_le_bytes(buffer[15..17].try_into().unwrap())),
                    }),
                    _ => None,
                };

                offset = header_length;
            }
        }

        self.above.read(packet, &buffer[offset..])
    }

    fn write<const N: usize>(
        &self,
        _writer: &mut impl Writer,
        _packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        todo!()
    }
}

const fn header_length(ci: u8) -> Option<usize> {
    match ci {
        0x8C => Some(1 + 2),
        0x8D => Some(1 + 8),
        0x8E => Some(1 + 10),
        0x8F => Some(1 + 16),
        _ => None,
    }
}
