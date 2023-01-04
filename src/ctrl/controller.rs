use crate::stack::{phl, Channel, ReadError};
use alloc::boxed::Box;
use futures::Stream;
use futures_async_stream::stream;

use super::{
    traits::{self, RxToken},
    Rssi,
};

/// Wireless M-Bus Transceiver Controller
pub struct Controller<Transceiver: traits::Transceiver> {
    transceiver: Transceiver,
    listening: bool,
}

pub struct Frame<Timestamp> {
    timestamp: Option<Timestamp>,
    rssi: Option<Rssi>,
    buffer: [u8; phl::MAX_FRAME_LENGTH],
    received: usize,
    channel: Option<Channel>,
    length: Option<usize>,
}

impl<Timestamp> const Default for Frame<Timestamp> {
    fn default() -> Self {
        Self {
            timestamp: None,
            rssi: None,
            buffer: [0; phl::MAX_FRAME_LENGTH],
            received: 0,
            channel: None,
            length: None,
        }
    }
}

impl<Timestamp> Frame<Timestamp> {
    pub fn bytes(&self) -> &[u8] {
        &self.buffer[0..self.length.unwrap()]
    }

    pub fn channel(&self) -> Channel {
        self.channel.unwrap()
    }
}

impl<Transceiver: traits::Transceiver> Controller<Transceiver> {
    /// Create a new controller
    pub const fn new(transceiver: Transceiver) -> Self {
        Self {
            transceiver,
            listening: false,
        }
    }

    /// Setup the transceiver and enter idle state.
    pub async fn init(&mut self) -> Result<(), Transceiver::Error> {
        self.listening = false;
        self.transceiver.init().await
    }

    /// Prepare bytes for transmission.
    /// All bytes for the transmission must be written before the transmission is started.
    pub async fn write(&mut self, buffer: &[u8]) -> Result<(), Transceiver::Error> {
        assert!(!self.listening);
        self.transceiver.write(buffer).await
    }

    /// Transmit pre-written bytes.
    /// The transmitter enters idle after the transmission completes.
    pub async fn transmit(&mut self) -> Result<(), Transceiver::Error> {
        assert!(!self.listening);
        self.transceiver.transmit().await
    }

    /// Start and run receiver.
    /// Note that the receiver is _not_ stopped when the stream is dropped, so idle() must be called manually after the stream is dropped.
    pub async fn receive<'a>(
        &'a mut self,
    ) -> Result<impl Stream<Item = Frame<Transceiver::Timestamp>> + 'a, Transceiver::Error> {
        assert!(!self.listening);

        // Start the receiver on the chip
        self.transceiver.listen().await?;
        self.listening = true;

        Ok(self.receive_stream())
    }

    #[stream(boxed_local, item = Frame<Transceiver::Timestamp>)]
    async fn receive_stream(&mut self) {
        loop {
            // Wait for frame to be detected
            let mut token = self
                .transceiver
                .receive(phl::DERIVE_FRAME_LENGTH_MIN)
                .await
                .unwrap();
            let mut frame = Frame::default();
            frame.timestamp = token.timestamp();

            // Frame was detected - read all frame bytes...
            loop {
                let received = self
                    .transceiver
                    .read(&mut token, &mut frame.buffer[frame.received..])
                    .await;

                if let Ok(received) = received {
                    // Things are progressing just fine - we are still receiving a frame

                    frame.received += received;

                    if frame.length.is_none() {
                        // Try and derive the frame length
                        match phl::derive_frame_length(&frame.buffer[..frame.received]) {
                            Ok((channel, length)) => {
                                self.transceiver.accept(&mut token, length).await.unwrap();
                                frame.channel = Some(channel);
                                frame.length = Some(length);
                                frame.rssi = Some(self.transceiver.get_rssi().await.unwrap());
                            }
                            Err(ReadError::NotEnoughBytes) => {
                                // We need more bytes to derive the frame length
                                continue;
                            }
                            Err(_) => {
                                // Invalid frame length - wait for a new frame to be received
                                break;
                            }
                        }
                    }

                    if let Some(frame_length) = frame.length && frame.received >= frame_length {
                            // Frame is fully received
                            yield frame;
                            break;
                    }
                } else {
                    // Error while reading - restart the receiver
                    self.transceiver.idle().await.unwrap();
                    self.transceiver.listen().await.unwrap();
                    break;
                }
            }
        }
    }

    // Stop the receiver.
    pub async fn idle(&mut self) -> Result<(), Transceiver::Error> {
        self.transceiver.idle().await?;
        self.listening = false;
        Ok(())
    }

    /// Release the transceiver
    pub fn release(self) -> Transceiver {
        self.transceiver
    }
}
