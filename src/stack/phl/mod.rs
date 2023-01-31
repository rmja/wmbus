mod ffa;
mod ffb;

use bitvec::prelude::*;
use crc::{Crc, CRC_16_EN_13757};

use crate::modet::threeoutofsix::{self, ThreeOutOfSix};

pub use self::{ffa::FFA, ffb::FFB};

use super::{Channel, Layer, Packet, ReadError, WriteError, Writer};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_EN_13757);

pub const DERIVE_FRAME_LENGTH_MIN: usize = 3;
pub const MAX_FRAME_LENGTH: usize = FFA::FRAME_MAX;

pub struct Phl<A: Layer> {
    above: A,
}

pub struct PhlFields;

#[derive(Debug, PartialEq)]
pub enum Error {
    Incomplete,
    Syncword,
    ThreeOutOfSix(threeoutofsix::Error),
    InvalidLength,
    CrcBlock(usize),
}

impl From<Error> for ReadError {
    fn from(value: Error) -> Self {
        match value {
            Error::Incomplete => ReadError::Incomplete,
            e => ReadError::Phl(e),
        }
    }
}

#[const_trait]
pub trait FrameFormat {
    const APL_MAX: usize;
    const FRAME_MAX: usize;

    fn get_frame_length(buffer: &[u8]) -> Result<usize, Error>;
}

pub fn derive_frame_length(buffer: &[u8]) -> Result<(Channel, usize), Error> {
    if buffer.len() < DERIVE_FRAME_LENGTH_MIN {
        return Err(Error::Incomplete);
    }

    if buffer[0] == 0x54 {
        // This is Mode C

        match buffer[1] {
            // Frame format A
            0xCD => {
                let frame_length = 2 + FFA::get_frame_length(&buffer[2..])?;
                Ok((Channel::ModeCFFA, frame_length))
            }
            // Frame format B
            0x3D => {
                let frame_length = 2 + FFB::get_frame_length(&buffer[2..])?;
                Ok((Channel::ModeCFFB, frame_length))
            }
            _ => Err(Error::Syncword),
        }
    } else if buffer[1] == 0x44 {
        // This is very likely a ModeC FFB frame where we have synchronized on the last 16 bits of its syncword 543D_543D.
        // 0x44 is the SND-NR C-field within the frame

        // We can however not be sure about this because 0x44 can map to valid 3oo6 symbols.

        // We try and receive more bytes so that we have what corresponds to the possible entire first block of a 3oo6 ModeT frame
        // If that block passes CRC then it is ModeT, otherwise we assume ModeC FFB

        // The first block is 12 bytes including its CRC - it is 3oo6 encoded so we actually need 18 bytes to proceed
        if buffer.len() < (12 * 6) / 4 {
            return Err(Error::Incomplete);
        }

        let bits = buffer.view_bits();
        if let Ok(block) = ThreeOutOfSix::decode(&bits[..12 * 6]) {
            // It seems as if the first block was in fact 3oo6 encoded

            if is_valid_crc(&block) {
                let frame_length = FFA::get_frame_length(buffer)?;
                return Ok((Channel::ModeT, frame_length));
            }
        }

        // Invalid 3oo6 or invalid first block CRC
        // Assume ModeC FFB

        let frame_length = FFB::get_frame_length(buffer)?;
        Ok((Channel::ModeCFFB, frame_length))
    } else {
        let bits = buffer.view_bits();
        let buffer = ThreeOutOfSix::decode(&bits[..12]).map_err(Error::ThreeOutOfSix)?;
        assert_eq!(1, buffer.len());
        let frame_length = FFA::get_frame_length(&buffer)?;
        Ok((Channel::ModeT, frame_length))
    }
}

impl<A: Layer> Phl<A> {
    pub const fn new(above: A) -> Self {
        Self { above }
    }
}

impl<A: Layer> Layer for Phl<A> {
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError> {
        let payload = match packet.channel {
            Channel::ModeT => {
                let mut symbols = (buffer.len() * 8) / 6;
                symbols &= !1; // The number of symbols must be even
                let buffer_bits = buffer.view_bits::<Msb0>();
                let encoded = &buffer_bits[..6 * symbols];
                let decoded = ThreeOutOfSix::decode(encoded).map_err(Error::ThreeOutOfSix)?;
                ffa::read(&decoded)?
            }
            Channel::ModeCFFA => ffa::read(buffer)?,
            Channel::ModeCFFB => ffb::read(buffer)?,
        };

        self.above.read(packet, &payload)
    }

    fn write<const N: usize>(
        &self,
        _writer: &mut impl Writer,
        _packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        todo!()
    }
}

pub(crate) fn is_valid_crc(block: &[u8]) -> bool {
    let index = block.len() - 2;

    let mut digest = CRC.digest();
    digest.update(&block[0..index]);
    let actual = digest.finalize();

    let expected = u16::from_be_bytes(block[index..].try_into().unwrap());

    actual == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_derive_frame_length() {
        assert_eq!(
            (Channel::ModeCFFB, 2 + 1 + 0x4E),
            derive_frame_length(&[0x54, 0x3D, 0x4E]).unwrap()
        );
        assert_eq!(
            Err(Error::Incomplete),
            derive_frame_length(&[0x4E, 0x44, 0x2D])
        );
        assert_eq!(
            (Channel::ModeCFFB, 1 + 0x4E),
            derive_frame_length(&[
                0x4E, 0x44, 0x2D, 0x2C, 0x98, 0x27, 0x04, 0x67, 0x30, 0x04, 0x91, 0x53, 0x7A, 0xA6,
                0x10, 0x40, 0x25, 0x6D
            ])
            .unwrap()
        );
    }
}
