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

    /// Start the receiver.
    async fn listen(&mut self);

    /// Try and receive a frame.
    /// The future will complete when `min_frame_length` frame bytes are received.
    /// The receiver will continue to receive the frame until either `accept` or `reject` are invoked.
    async fn receive(&mut self, min_frame_length: usize) -> Self::Timestamp;

    /// Get the current rssi.
    async fn get_rssi(&mut self) -> Rssi;

    /// Read bytes for the packet currently being received.
    async fn read<'a>(&'a mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Notify the receiver about the final frame length for the current receive.
    /// The receiver shoud re-start when this frame length has been received.
    async fn accept(&mut self, frame_length: usize);

    /// Notify the receiver that the currently receiving frame is invalid.
    /// The transceiver should discard all currently received frame bytes and re-start the receiver.
    async fn reject(&mut self);

    /// Enter idle state.
    async fn idle(&mut self);
}
