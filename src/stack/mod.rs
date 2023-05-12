pub mod apl;
pub mod dll;
pub mod ell;
pub mod phl;

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
        writer: &mut impl Writer,
        packet: &Packet<N>,
    ) -> Result<(), WriteError>;
}

impl<T: Layer> Layer for &T {
    fn read<const N: usize>(&self, packet: &mut Packet<N>, buffer: &[u8]) -> Result<(), ReadError> {
        T::read(self, packet, buffer)
    }

    fn write<const N: usize>(
        &self,
        writer: &mut impl Writer,
        packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        T::write(self, writer, packet)
    }
}

/// A Wireless M-Bus packet
#[derive(Clone)]
pub struct Packet<const APL_MAX: usize = DEFAULT_APL_MAX> {
    pub rssi: Option<Rssi>,
    pub mode: Mode,
    pub phl: Option<phl::PhlFields>,
    pub dll: Option<dll::DllFields>,
    pub ell: Option<ell::EllFields>,
    pub apl: Vec<u8, APL_MAX>,
}

pub type Rssi = i16;

pub trait Writer {
    fn write(&mut self, buf: &[u8]) -> Result<(), WriteError>;
}

impl<const N: usize> Writer for Vec<u8, N> {
    fn write(&mut self, buf: &[u8]) -> Result<(), WriteError> {
        self.extend_from_slice(buf)
            .map_err(|_| WriteError::Capacity)
    }
}

#[cfg(feature = "alloc")]
impl Writer for alloc::vec::Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<(), WriteError> {
        self.extend_from_slice(buf);
        Ok(())
    }
}

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
pub enum WriteError {
    Capacity,
}

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
        self.phl.read(&mut packet, buffer)?;
        Ok(packet)
    }

    /// Write a packet
    pub fn write<const N: usize>(
        &self,
        writer: &mut impl Writer,
        packet: &Packet<N>,
    ) -> Result<(), WriteError> {
        self.phl.write(writer, packet)
    }
}

#[cfg(test)]
mod tests {
    use crate::stack::phl::FrameMetadata;

    use super::*;

    #[test]
    fn can_read() {
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
}
