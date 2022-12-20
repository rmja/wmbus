use core::fmt::Debug;

#[cfg(test)]
use mockall::automock;

use super::Rssi;

#[cfg_attr(test, automock(type Timestamp = core::time::Duration; type Error = ();))]
pub trait Transceiver {
    type Timestamp;
    type Error: Debug;

    /// Setup the transceiver and enter idle state.
    async fn init(&mut self) -> Result<(), Self::Error>;

    /// Prepare bytes for transmission.
    async fn write(&mut self, buffer: &[u8]);

    /// Transmit already prepared bytes and return to idle state.
    async fn transmit(&mut self) -> Result<(), Self::Error>;

    /// Start receiver and try and receive a packet.
    /// The future will complete when a packet is detected.
    async fn receive(&mut self) -> (Self::Timestamp, Rssi);

    /// Read bytes for the packet currently being received.
    async fn read<'a>(
        &'a mut self,
        buffer: &mut [u8],
        frame_length: Option<usize>,
    ) -> Result<usize, Self::Error>;

    /// Enter idle state.
    async fn idle(&mut self);
}
