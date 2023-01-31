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
    fn write<const N: usize>(&self, writer: &mut impl Writer, packet: &Packet<N>) -> Result<(), WriteError>;
}

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

/// A Wireless M-Bus packet
pub struct Packet<const N: usize> {
    pub rssi: Option<Rssi>,
    pub channel: Channel,
    pub phl: Option<phl::PhlFields>,
    pub dll: Option<dll::DllFields>,
    pub ell: Option<ell::EllFields>,
    pub mbus_data: Vec<u8, N>,
}

pub type Rssi = i8;

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
pub enum Channel {
    /// Mode C FFA
    ModeCFFA,
    /// Mode C FFB
    ModeCFFB,
    /// Mode T FFA. Frame is "three out of six" encoded.
    ModeT,
}

impl<const N: usize> Packet<N> {
    pub const fn new(channel: Channel) -> Self {
        Self {
            rssi: None,
            channel,
            phl: None,
            dll: None,
            ell: None,
            mbus_data: Vec::new(),
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
    pub fn read<const N: usize>(&self, buffer: &[u8], channel: Channel) -> Result<Packet<N>, ReadError> {
        let mut packet = Packet::new(channel);
        self.phl.read(&mut packet, buffer)?;
        Ok(packet)
    }

    /// Write a packet
    pub fn write<const N: usize>(&self, writer: &mut impl Writer, packet: &Packet<N>) -> Result<(), WriteError> {
        self.phl.write(writer, packet)
    }
}
