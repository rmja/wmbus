use core::time::Duration;
use alloc::boxed::Box;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use super::{Rssi, TransceiverError};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Transceiver {
    /// Setup the transceiver and enter idle state.
    async fn init(&mut self) -> Result<(), TransceiverError>;

    /// Prepare bytes for transmission.
    async fn write(&mut self, buffer: &[u8]);

    /// Transmit already prepared bytes and return to idle state.
    async fn transmit(&mut self) -> Result<(), TransceiverError>;

    /// Start receiver and try and receive a packet.
    /// The future will complete when a packet is detected.
    async fn receive(&mut self) -> (Duration, Rssi);

    /// Read bytes for the packet currently being received.
    async fn read<'a>(
        &'a mut self,
        buffer: &mut [u8],
        frame_length: Option<usize>,
    ) -> Result<usize, TransceiverError>;

    /// Enter idle state.
    async fn idle(&mut self);
}
