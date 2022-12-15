pub mod ffa;
pub mod ffb;

use alloc::vec::Vec;
use crc::{Crc, CRC_16_EN_13757};

use super::{FrameFormat, Layer};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_EN_13757);

pub struct Phl<A: Layer> {
    above: A,
}

pub struct PhlFields;

impl<A: Layer> Phl<A> {
    pub const fn new(above: A) -> Self {
        Self { above }
    }
}

impl<A: Layer> Layer for Phl<A> {
    fn read(&self, packet: &mut super::Packet, buffer: &[u8]) -> Result<(), super::ReadError> {
        let payload = match packet.frame_format {
            FrameFormat::FFA => ffa::read(buffer)?,
            FrameFormat::FFB => ffb::read(buffer)?,
        };

        self.above.read(packet, &payload)
    }

    fn write(&self, _writer: &mut Vec<u8>, _packet: &super::Packet) {
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
