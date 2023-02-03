pub mod apl;
pub mod dll;
pub mod ell;
pub mod phl;

use core::fmt::Debug;
use heapless::Vec;

/// The Wireless M-Bus protocol stack
pub struct Stack<A: Layer> {
    phl: phl::Phl<dll::Dll<A>>,
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

/// A Wireless M-Bus packet
pub struct Packet<const N: usize = { phl::APL_MAX }> {
    pub rssi: Option<Rssi>,
    pub mode: Mode,
    pub phl: Option<phl::PhlFields>,
    pub dll: Option<dll::DllFields>,
    pub ell: Option<ell::EllFields>,
    pub apl: Vec<u8, N>,
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
pub enum ReadError {
    Incomplete,
    Capacity,
    Phl(phl::Error),
    Dll(dll::Error),
    Ell(ell::Error),
}

#[derive(Debug, PartialEq)]
pub enum WriteError {
    Capacity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn read<const N: usize>(&self, buffer: &[u8], mode: Mode) -> Result<Packet<N>, ReadError> {
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
