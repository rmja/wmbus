pub mod ffa;
pub mod ffb;

use alloc::vec::Vec;
use crc::{Crc, CRC_16_EN_13757};

use super::{Channel, FrameFormat, Layer, Packet, ReadError};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_EN_13757);

pub const MAX_FRAME_LENGTH: usize = ffa::MAX_FRAME_SIZE;

pub struct Phl<A: Layer> {
    above: A,
}

pub struct PhlFields;

pub fn get_frame_length(buffer: &[u8]) -> Result<(Channel, usize), ReadError> {
    if buffer.len() < 3 {
        return Err(ReadError::NotEnoughBytes);
    }

    if buffer[0] == 0x54 {
        // This is Mode C

        match buffer[1] {
            // Frame format A
            0xCD => {
                let frame_length = ffa::get_frame_length(buffer)?;
                Ok((Channel::ModeC(FrameFormat::FFA), frame_length))
            }
            // Frame format B
            0x3D => {
                let frame_length = ffb::get_frame_length(buffer)?;
                Ok((Channel::ModeC(FrameFormat::FFB), frame_length))
            }
            _ => Err(ReadError::PhlInvalidSyncword),
        }
    } else {
        // TODO: Three out of six decode
        todo!()
        // let frame_length = ffa::get_frame_length(buffer)?;
        // Ok((Channel::ModeT, frame_length))
    }
}

impl<A: Layer> Phl<A> {
    pub const fn new(above: A) -> Self {
        Self { above }
    }
}

impl<A: Layer> Layer for Phl<A> {
    fn read(&self, packet: &mut Packet, buffer: &[u8]) -> Result<(), ReadError> {
        let payload = match packet.channel {
            Channel::ModeT => ffa::read(buffer)?,
            Channel::ModeC(FrameFormat::FFA) => ffa::read(buffer)?,
            Channel::ModeC(FrameFormat::FFB) => ffb::read(buffer)?,
        };

        self.above.read(packet, &payload)
    }

    fn write(&self, _writer: &mut Vec<u8>, _packet: &Packet) {
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
