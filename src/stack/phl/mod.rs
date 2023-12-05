mod ffa;
mod ffb;

use bitvec::prelude::*;
use bytes::{BufMut, BytesMut};
use crc::{Crc, CRC_16_EN_13757};
use heapless::Vec;

use crate::modet::threeoutofsix::{self, ThreeOutOfSix};

pub use self::{ffa::FFA, ffb::FFB};

use super::{Layer, Mode, Packet, ReadError, WriteError};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_EN_13757);

pub const DERIVE_FRAME_LENGTH_MIN: usize = 3;
pub const APL_MAX: usize = FFA::APL_MAX;
pub const DATA_MAX: usize = FFA::DATA_MAX;
pub const FRAME_MAX: usize = FFA::FRAME_MAX;

pub struct Phl<A: Layer> {
    above: A,
}

#[derive(Clone)]
pub struct PhlFields;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    Incomplete,
    Syncword,
    ThreeOutOfSix(threeoutofsix::Error),
    InvalidLength,
    Crc(usize),
}

impl From<Error> for ReadError {
    fn from(value: Error) -> Self {
        match value {
            Error::Incomplete => ReadError::Incomplete,
            e => ReadError::Phl(e),
        }
    }
}

pub trait FrameFormat {
    const APL_MAX: usize;
    const DATA_MAX: usize;
    const FRAME_MAX: usize;

    fn get_frame_length(buffer: &[u8]) -> Result<usize, Error>;
    fn trim_crc(buffer: &[u8]) -> Result<Vec<u8, { Self::DATA_MAX }>, Error>;
}

#[derive(Debug, PartialEq)]
pub struct FrameMetadata {
    pub mode: Mode,
    pub frame_offset: usize,
    /// The total frame length including CRC's, but excluding 3oo6 encoding
    pub frame_length: usize,
}

impl FrameMetadata {
    pub fn read(buffer: &[u8]) -> Result<FrameMetadata, Error> {
        if buffer.len() < DERIVE_FRAME_LENGTH_MIN {
            return Err(Error::Incomplete);
        }

        if buffer[0] == 0x54 {
            Self::decode_modec(buffer)
        } else if buffer[1] == 0x44 {
            // This is very likely a ModeC FFB frame where we have synchronized on the last 16 bits of its syncword 543D_543D.
            // 0x44 is the SND-NR C-field within the frame

            // We can however not be sure about this because 0x44 can map to valid 3oo6 symbols.
            let first_is_3oo6 = (buffer[0] & 0xFC).count_ones() == 3;
            let second_is_3oo6 = ((buffer[0] & 0x03) | (buffer[1] & 0xF0)).count_ones() == 3;

            if first_is_3oo6 && second_is_3oo6 {
                // We try and receive more bytes so that we have what corresponds to the possible entire first block of a 3oo6 ModeT frame
                // If that block passes CRC then it is ModeT, otherwise we assume ModeC FFB

                // The first block is 12 bytes including its CRC - it is 3oo6 encoded so we actually need 18 bytes to proceed
                if let Some(result) = Self::try_decode_first_modet_block(buffer)? {
                    return Ok(result);
                }
            }

            // Invalid 3oo6 or invalid first block CRC
            // Assume ModeC FFB

            let frame_length = FFB::get_frame_length(buffer)?;
            Ok(FrameMetadata {
                mode: Mode::ModeCFFB,
                frame_offset: 0,
                frame_length,
            })
        } else {
            Self::decode_modet(buffer)
        }
    }

    fn decode_modec(buffer: &[u8]) -> Result<FrameMetadata, Error> {
        if buffer.len() < 2 {
            return Err(Error::Incomplete);
        }
        match buffer[1] {
            // Frame format A
            0xCD => {
                let frame_length = FFA::get_frame_length(&buffer[2..])?;
                Ok(FrameMetadata {
                    mode: Mode::ModeCFFA,
                    frame_offset: 2,
                    frame_length,
                })
            }
            // Frame format B
            0x3D => {
                let frame_length = FFB::get_frame_length(&buffer[2..])?;
                Ok(FrameMetadata {
                    mode: Mode::ModeCFFB,
                    frame_offset: 2,
                    frame_length,
                })
            }
            _ => Err(Error::Syncword),
        }
    }

    fn try_decode_first_modet_block(buffer: &[u8]) -> Result<Option<FrameMetadata>, Error> {
        if buffer.len() < (12 * 6) / 4 {
            return Err(Error::Incomplete);
        }

        let mut block = [0; 12];
        let bits = buffer.view_bits();
        if let Ok(decoded) = ThreeOutOfSix::decode(&mut block, &bits[..12 * 6]) {
            // It seems as if the first block was in fact 3oo6 encoded

            assert_eq!(12, decoded);

            if is_valid_crc(&block) {
                let frame_length = FFA::get_frame_length(buffer)?;
                return Ok(Some(FrameMetadata {
                    mode: Mode::ModeTMTO,
                    frame_offset: 0,
                    frame_length,
                }));
            }
        }

        Ok(None)
    }

    fn decode_modet(buffer: &[u8]) -> Result<FrameMetadata, Error> {
        if buffer.len() < 3 {
            return Err(Error::Incomplete);
        }
        let mut l_field = [0; 12];
        let bits = buffer.view_bits();
        let decoded =
            ThreeOutOfSix::decode(&mut l_field, &bits[..12]).map_err(Error::ThreeOutOfSix)?;
        assert_eq!(1, decoded);
        let frame_length = FFA::get_frame_length(&l_field)?;
        Ok(FrameMetadata {
            mode: Mode::ModeTMTO,
            frame_offset: 0,
            frame_length,
        })
    }
}

impl<A: Layer> Phl<A> {
    pub const fn new(above: A) -> Self {
        Self { above }
    }
}

impl<A: Layer> Layer for Phl<A> {
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError> {
        match packet.mode {
            Mode::ModeTMTO => {
                let mut symbols = (buffer.len() * 8) / 6;
                symbols &= !1; // The number of symbols must be even
                let mut decode_buf = [0; FFA::FRAME_MAX];
                let buffer_bits = buffer.view_bits::<Msb0>();
                let encoded = &buffer_bits[..6 * symbols];
                let decoded = ThreeOutOfSix::decode(&mut decode_buf, encoded)
                    .map_err(Error::ThreeOutOfSix)?;
                let payload = FFA::trim_crc(&decode_buf[..decoded])?;
                self.above.read(packet, &payload)
            }
            Mode::ModeCFFA => {
                let offset = buffer
                    .starts_with(&[0x54, 0xCD])
                    .then_some(2)
                    .unwrap_or_default();
                let payload = FFA::trim_crc(&buffer[offset..])?;
                self.above.read(packet, &payload)
            }
            Mode::ModeCFFB => {
                let offset = buffer
                    .starts_with(&[0x54, 0x3D])
                    .then_some(2)
                    .unwrap_or_default();
                let payload = FFB::trim_crc(&buffer[offset..])?;
                self.above.read(packet, &payload)
            }
        }
    }

    fn write<const N: usize>(
        &self,
        writer: &mut BytesMut,
        packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        let start = writer.len();
        writer.put_u8(0x00); // Dummy L field
        self.above.write(writer, packet)?;
        let len = writer.len() - start;

        // Write L field
        writer[start] = if len <= ffb::FIRST_BLOCK_DATA_LENGTH + ffb::SECOND_BLOCK_MAX_DATA_LENGTH {
            len + 2 - 1
        } else {
            len + 2 + 2 - 1
        } as u8;

        let data = &writer[start..];

        if len <= ffb::FIRST_BLOCK_DATA_LENGTH + ffb::SECOND_BLOCK_MAX_DATA_LENGTH {
            let mut digest = CRC.digest();
            digest.update(data);
            let crc = digest.finalize();
            writer.put_u16(crc);
        } else {
            // Move the optional block
            let first_len = ffb::FIRST_BLOCK_DATA_LENGTH + ffb::SECOND_BLOCK_MAX_DATA_LENGTH;
            writer.put_u16(0);
            let written = writer.len();
            writer.copy_within(start + first_len..written - 2, start + first_len + 2);

            let first_block = &mut writer[start..start + first_len + 2];
            let mut digest = CRC.digest();
            digest.update(&first_block[..first_len]);
            first_block[first_len..].copy_from_slice(&digest.finalize().to_be_bytes());

            let second_data = &writer[start + first_len + 2..];
            let mut digest = CRC.digest();
            digest.update(second_data);
            writer.put_u16(digest.finalize());
        }

        Ok(())
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
            FrameMetadata {
                mode: Mode::ModeCFFB,
                frame_offset: 2,
                frame_length: 1 + 0x4E
            },
            FrameMetadata::read(&[0x54, 0x3D, 0x4E]).unwrap()
        );
        assert_eq!(
            FrameMetadata {
                mode: Mode::ModeCFFB,
                frame_offset: 0,
                frame_length: 1 + 0x4E
            },
            // This is invalid 3oo6
            FrameMetadata::read(&[0x4E, 0x44, 0x00]).unwrap()
        );
        assert_eq!(
            Err(Error::Incomplete),
            // This is valid 3oo6
            FrameMetadata::read(&[0x4F, 0x44, 0x00])
        );
        assert_eq!(
            FrameMetadata {
                mode: Mode::ModeCFFB,
                frame_offset: 0,
                frame_length: 1 + 0x4E
            },
            FrameMetadata::read(&[
                0x4E, 0x44, 0x2D, 0x2C, 0x98, 0x27, 0x04, 0x67, 0x30, 0x04, 0x91, 0x53, 0x7A, 0xA6,
                0x10, 0x40, 0x25, 0x6D
            ])
            .unwrap()
        );
        assert_eq!(
            FrameMetadata {
                mode: Mode::ModeTMTO,
                frame_offset: 0,
                frame_length: 10 + 2 + 6 + 2
            },
            // 0x5a971c = 0b010110_101001_011100_011100, i.e. 0b0000_1111_0100_0100, i.e. 0x0F44?
            FrameMetadata::read(&[0x5a, 0x97, 0x1c]).unwrap()
        );
    }
}
