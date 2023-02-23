use crate::stack::phl;
use futures::Stream;
use futures_async_stream::stream;

use super::{
    traits::{self, RxToken},
    Frame,
};

/// Wireless M-Bus Transceiver Controller
pub struct Controller<Transceiver: traits::Transceiver> {
    transceiver: Transceiver,
    listening: bool,
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
    ) -> Result<impl Stream<Item = Frame> + 'a, Transceiver::Error> {
        assert!(!self.listening);

        // Start the receiver on the chip
        self.transceiver.listen().await?;
        self.listening = true;

        Ok(self.receive_stream())
    }

    #[stream(item = Frame)]
    async fn receive_stream(&mut self) {
        loop {
            // Wait for frame to be detected
            let mut token = self
                .transceiver
                .receive(phl::DERIVE_FRAME_LENGTH_MIN)
                .await
                .unwrap();
            let mut frame = Frame {
                timestamp: token.timestamp(),
                ..Default::default()
            };

            // Frame was detected - read all frame bytes...
            loop {
                let received = self
                    .transceiver
                    .read(&mut token, &mut frame.buffer[frame.received..])
                    .await;

                if let Ok(received) = received {
                    // Things are progressing just fine - we are still receiving a frame

                    frame.received += received;

                    if frame.len.is_none() {
                        // Try and derive the frame length
                        match phl::derive_frame_length(&frame.buffer[..frame.received]) {
                            Ok((mode, skip, length)) => {
                                let frame_len = skip + length;
                                self.transceiver
                                    .accept(&mut token, frame_len)
                                    .await
                                    .unwrap();
                                frame.mode = Some(mode);
                                frame.len = Some(frame_len);
                                frame.rssi = Some(self.transceiver.get_rssi().await.unwrap());
                            }
                            Err(phl::Error::Incomplete) => {
                                // We need more bytes to derive the frame length
                                continue;
                            }
                            Err(_) => {
                                // Invalid frame length - wait for a new frame to be received
                                break;
                            }
                        }
                    }

                    if let Some(frame_length) = frame.len && frame.received >= frame_length {
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
