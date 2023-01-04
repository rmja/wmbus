use core::fmt::Debug;

#[cfg(test)]
use mockall::automock;

use super::Rssi;

#[cfg_attr(test, automock(type Timestamp = core::time::Duration; type RxToken = stubs::RxTokenStub; type Error = ();))]
pub trait Transceiver {
    type Timestamp: Copy;
    type RxToken: RxToken<Self::Timestamp>;
    type Error: Debug;

    /// Setup the transceiver and enter idle state.
    async fn init(&mut self) -> Result<(), Self::Error>;

    /// Prepare bytes for transmission.
    async fn write(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;

    /// Transmit already prepared bytes and return to idle state.
    async fn transmit(&mut self) -> Result<(), Self::Error>;

    /// Start the receiver.
    async fn listen(&mut self) -> Result<(), Self::Error>;

    /// Get the current rssi.
    async fn get_rssi(&mut self) -> Result<Rssi, Self::Error>;

    /// Try and receive a frame.
    /// The future will complete when `min_frame_length` frame bytes are received.
    /// The receiver will continue to receive the frame until either `accept` is invoked or `receive` are re-invoked.
    async fn receive(&mut self, min_frame_length: usize) -> Result<Self::RxToken, Self::Error>;

    /// Read bytes for the packet currently being received.
    async fn read(
        &mut self,
        token: &mut Self::RxToken,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Notify the receiver about the final frame length for the current receive.
    /// The receiver shoud re-start when this frame length has been received.
    async fn accept(
        &mut self,
        token: &mut Self::RxToken,
        frame_length: usize,
    ) -> Result<(), Self::Error>;

    /// Enter idle state.
    async fn idle(&mut self) -> Result<(), Self::Error>;
}

pub trait RxToken<Timestamp: Copy> {
    /// Get the start-of-frame timestamp
    fn timestamp(&self) -> Option<Timestamp>;
}

#[cfg(test)]
pub mod stubs {
    use super::RxToken;

    pub struct RxTokenStub {
        timestamp: core::time::Duration,
    }

    impl RxToken<core::time::Duration> for RxTokenStub {
        fn timestamp(&self) -> Option<core::time::Duration> {
            Some(self.timestamp)
        }
    }
}
