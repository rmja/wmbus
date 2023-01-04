pub mod apl;
pub mod dll;
pub mod ell;
pub mod phl;

use alloc::vec::Vec;

/// The Wireless M-Bus protocol stack
pub struct Stack<A: Layer> {
    phl: phl::Phl<dll::Dll<A>>,
}

/// Layer trait
pub trait Layer {
    fn read(&self, packet: &mut Packet, buffer: &[u8]) -> Result<(), ReadError>;
    fn write(&self, writer: &mut Vec<u8>, packet: &Packet);
}

/// A Wireless M-Bus packet
pub struct Packet {
    pub rssi: Option<Rssi>,
    pub channel: Channel,
    pub phl: Option<phl::PhlFields>,
    pub dll: Option<dll::DllFields>,
    pub ell: Option<ell::EllFields>,
    pub mbus_data: Vec<u8>,
}

pub type Rssi = i8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameFormat {
    FFA,
    FFB,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Channel {
    /// Mode C.
    ModeC(FrameFormat),
    /// Mode T. Frame is "three out of six" encoded and uses frame format A.
    ModeT,
}

impl Packet {
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

#[derive(Debug, PartialEq)]
pub enum ReadError {
    NotEnoughBytes,
    PhlInvalidSyncword,
    PhlInvalidThreeOutOfSix,
    PhlInvalidLength,
    PhlCrcError(usize),
    BcdConversionError,
    MBalCrcError,
    MBalControlError,
    MBalAddressError,
    MBalCommandError,
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
    pub fn read(&self, buffer: &[u8], channel: Channel) -> Result<Packet, ReadError> {
        let mut packet = Packet::new(channel);
        self.phl.read(&mut packet, buffer)?;
        Ok(packet)
    }

    /// Write a packet
    pub fn write(&self, writer: &mut Vec<u8>, packet: &Packet) {
        self.phl.write(writer, packet)
    }
}
