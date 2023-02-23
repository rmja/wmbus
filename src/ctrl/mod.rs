mod controller;
pub mod traits;

pub use controller::Controller;
use embassy_time::Instant;

use crate::stack::{phl, Layer, Mode, Packet, ReadError, Rssi, Stack};

pub struct Frame {
    pub timestamp: Instant,
    pub rssi: Option<Rssi>,
    buffer: [u8; phl::FRAME_MAX],
    received: usize,
    mode: Option<Mode>,
    len: Option<usize>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            rssi: None,
            buffer: [0; phl::FRAME_MAX],
            received: 0,
            mode: None,
            len: None,
        }
    }
}

#[allow(clippy::len_without_is_empty)]
impl Frame {
    pub fn len(&self) -> usize {
        self.len.unwrap()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buffer[0..self.len.unwrap()]
    }

    pub fn mode(&self) -> Mode {
        self.mode.unwrap()
    }
}

impl<A: Layer> Stack<A> {
    pub fn read_from_frame(&self, frame: &Frame) -> Result<Packet, ReadError> {
        let mut packet = self.read(frame.bytes(), frame.mode())?;
        packet.rssi = frame.rssi;
        Ok(packet)
    }
}
