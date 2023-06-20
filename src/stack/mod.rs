pub mod apl;
pub mod dll;
pub mod ell;
pub mod phl;

use bytes::BytesMut;
use core::fmt::Debug;
use heapless::Vec;

pub const DEFAULT_APL_MAX: usize = phl::APL_MAX;

/// The Wireless M-Bus protocol stack
pub struct Stack<A: Layer> {
    pub phl: phl::Phl<dll::Dll<A>>,
}

/// Layer trait
pub trait Layer {
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError>;
    fn write<const N: usize>(
        &self,
        writer: &mut BytesMut,
        packet: &Packet<N>,
    ) -> Result<(), WriteError>;
}

impl<T: Layer> Layer for &T {
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError> {
        T::read(self, packet, buffer)
    }

    fn write<const N: usize>(
        &self,
        writer: &mut BytesMut,
        packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        T::write(self, writer, packet)
    }
}

/// A Wireless M-Bus packet
#[derive(Clone)]
pub struct Packet<const APL_MAX: usize = DEFAULT_APL_MAX> {
    pub frame_len: Option<usize>,
    pub rssi: Option<Rssi>,
    pub mode: Mode,
    pub phl: Option<phl::PhlFields>,
    pub dll: Option<dll::DllFields>,
    pub ell: Option<ell::EllFields>,
    pub apl: Vec<u8, APL_MAX>,
}

pub type Rssi = i16;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ReadError {
    Incomplete,
    Capacity,
    Phl(phl::Error),
    Dll(dll::Error),
    Ell(ell::Error),
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WriteError {}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Mode {
    /// Mode C FFA
    ModeCFFA,
    /// Mode C FFB
    ModeCFFB,
    /// Mode T meter-to-other
    /// Uses frame format A and frame is "three out of six" encoded.
    ModeTMTO,
}

impl<const N: usize> Packet<N> {
    /// Create a new empty packet
    pub const fn new(mode: Mode) -> Self {
        Self {
            frame_len: None,
            rssi: None,
            mode,
            phl: None,
            dll: None,
            ell: None,
            apl: Vec::new(),
        }
    }

    /// Create a new packet with a given payload
    pub fn with_apl(mode: Mode, apl: [u8; N]) -> Self {
        Self {
            frame_len: None,
            rssi: None,
            mode,
            phl: None,
            dll: None,
            ell: None,
            apl: Vec::from_slice(&apl).unwrap(),
        }
    }
}

impl Stack<ell::Ell<apl::Apl>> {
    /// Create a new Wireless M-Bus stack
    pub fn new() -> Self {
        Self {
            phl: phl::Phl::new(dll::Dll::new(ell::Ell::new(apl::Apl::new()))),
        }
    }
}

impl Default for Stack<ell::Ell<apl::Apl>> {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack<apl::Apl> {
    /// Create a new Wireless M-Bus stack without extended link layer
    pub fn without_ell() -> Self {
        Self {
            phl: phl::Phl::new(dll::Dll::new(apl::Apl::new())),
        }
    }
}

impl<A: Layer> Stack<A> {
    /// Read a packet from a byte buffer
    pub fn read(&self, buffer: &[u8], mode: Mode) -> Result<Packet, ReadError> {
        let mut packet = Packet::new(mode);
        packet.frame_len = Some(buffer.len());
        self.phl.read(&mut packet, buffer)?;
        Ok(packet)
    }

    /// Write a packet
    pub fn write<const N: usize>(
        &self,
        writer: &mut BytesMut,
        packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        self.phl.write(writer, packet)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        stack::{dll::DllFields, phl::FrameMetadata},
        DeviceType, ManufacturerCode, WMBusAddress,
    };

    use super::*;

    #[test]
    fn can_read_modecffb() {
        let stack = Stack::default();

        let frame = &[
            0x54, 0x3d, 0x23, 0x44, 0x2d, 0x2c, 0x33, 0x66, 0x00, 0x00, 0x17, 0x16, 0x8d, 0x20,
            0x86, 0x41, 0xce, 0x05, 0x26, 0x74, 0x7b, 0x1f, 0x09, 0x61, 0x17, 0x8c, 0xba, 0xf9,
            0xa8, 0x8e, 0x58, 0x71, 0x45, 0x72, 0xed, 0x55, 0xe8, 0xd4,
        ];
        let metadata = FrameMetadata::read(frame).unwrap();
        assert_eq!(Mode::ModeCFFB, metadata.mode);
        assert_eq!(2, metadata.frame_offset);
        assert_eq!(frame.len() - 2, metadata.frame_length);
        stack.read(frame, metadata.mode).unwrap();
        stack
            .read(&frame[metadata.frame_offset..], metadata.mode)
            .unwrap();
    }

    #[test]
    fn can_read_modetmto() {
        let stack = Stack::default();

        let frame = &[
            0x5a, 0x97, 0x1c, 0x3b, 0x13, 0xb4, 0x4e, 0xc6, 0x5a, 0x2d, 0xc3, 0x4e, 0x58, 0xd2,
            0xce, 0x6a, 0x9d, 0x29, 0x99, 0x65, 0x96, 0x58, 0xd5, 0x8e, 0x58, 0xb5, 0x9c, 0x4d,
            0xa4, 0xec,
        ];
        let metadata = FrameMetadata::read(frame).unwrap();
        assert_eq!(Mode::ModeTMTO, metadata.mode);
        assert_eq!(0, metadata.frame_offset);
        assert_eq!(20, metadata.frame_length);
        stack.read(frame, metadata.mode).unwrap();
        stack
            .read(&frame[metadata.frame_offset..], metadata.mode)
            .unwrap();
    }

    #[test]
    fn can_write_modecffb_two_blocks() {
        let stack = Stack::without_ell();

        let mut packet: Packet = Packet::new(Mode::ModeCFFB);
        packet.dll = Some(DllFields {
            control: 0x44,
            address: WMBusAddress::new(ManufacturerCode::KAM, 12345678, 0x01, DeviceType::Repeater),
        });
        packet
            .apl
            .extend_from_slice(&[0xa0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
            .unwrap();

        let mut writer = BytesMut::new();
        stack.write(&mut writer, &packet).unwrap();

        assert_eq!(
            &[
                0x15, 0x44, 0x2d, 0x2c, 0x78, 0x56, 0x34, 0x12, 0x01, 0x32, 0xa0, 0x00, 0x01, 0x02,
                0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0xaf, 0x95,
            ],
            writer.to_vec().as_slice()
        );

        stack.read(&writer, Mode::ModeCFFB).unwrap();
    }

    #[test]
    fn can_write_modecffb_three_blocks() {
        let stack = Stack::without_ell();

        let mut packet: Packet = Packet::new(Mode::ModeCFFB);
        packet.dll = Some(DllFields {
            control: 0x44,
            address: WMBusAddress::new(ManufacturerCode::KAM, 12345678, 0x01, DeviceType::Repeater),
        });
        packet
            .apl
            .extend_from_slice(&[
                0xa0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a,
                0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
                0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
                0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44,
                0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52,
                0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60,
                0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e,
                0x6f, 0x70, 0x71, 0x72, 0x73,
            ])
            .unwrap();

        let mut writer = BytesMut::new();
        stack.write(&mut writer, &packet).unwrap();

        println!("PACKET: {:02x?}", writer.to_vec());

        assert_eq!(
            &[
                0x82, 0x44, 0x2d, 0x2c, 0x78, 0x56, 0x34, 0x12, 0x01, 0x32, 0xa0, 0x00, 0x01, 0x02,
                0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
                0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
                0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c,
                0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a,
                0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
                0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56,
                0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60, 0x61, 0x62, 0x63, 0x64,
                0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72,
                0xbe, 0x64, 0x73, 0x0a, 0x02
            ],
            writer.to_vec().as_slice()
        );

        stack.read(&writer, Mode::ModeCFFB).unwrap();
    }
}
