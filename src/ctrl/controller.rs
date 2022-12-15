use core::time::Duration;

use async_stream::stream;
use futures::Stream;
use crate::stack::{phl, Channel, ReadError};

use super::{adapters::{Transceiver, TransceiverError}, Rssi};

/// Wireless M-Bus Transceiver Controller
pub struct Controller<T: Transceiver> {
    transceiver: T,
    receiving: bool,
}

pub struct Frame {
    pub timestamp: Duration,
    pub rssi: Rssi,
    buffer: [u8; phl::MAX_FRAME_LENGTH],
    received: usize,
    channel: Option<Channel>,
    length: Option<usize>,
}

impl Frame {
    const fn new(timestamp: Duration, rssi: Rssi) -> Self {
        Self {
            timestamp,
            rssi,
            buffer: [0; phl::MAX_FRAME_LENGTH],
            received: 0,
            channel: None,
            length: None,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buffer[0..self.length.unwrap()]
    }
}

impl<T: Transceiver> Controller<T> {
    /// Create a new controller
    pub const fn new(transceiver: T) -> Self {
        Self {
            transceiver,
            receiving: false,
        }
    }
    /// Prepare bytes for transmission.
    /// All bytes for the transmission must be written before the transmission is started.
    pub async fn write(&mut self, buffer: &[u8]) {
        assert!(!self.receiving);
        self.transceiver.write(buffer).await;
    }

    /// Transmit pre-written bytes.
    /// The transmitter enters idle after the transmission completes.
    pub async fn transmit(&mut self) -> Result<(), TransceiverError> {
        assert!(!self.receiving);
        self.transceiver.transmit().await
    }

    /// Start and run receiver.
    /// Note that the receiver is _not_ stopped when the stream is dropped, so idle() must be called manually after the stream is dropped.
    pub async fn receive<'a>(&'a mut self) -> impl Stream<Item = Frame> + 'a {
        assert!(!self.receiving);
        self.receiving = true;

        stream! {
            loop {
                let (timestamp, rssi) = self.transceiver.receive().await;
                let mut frame = Frame::new(timestamp, rssi);

                // Frame was detected - read all frame bytes...
                loop {
                    let mut buffer = &mut frame.buffer[frame.received..];
                    let received = self.transceiver.read(&mut buffer, frame.length).await;

                    if let Ok(received) = received {
                        frame.received += received;

                        if let Some(framelen) = frame.length {
                            if frame.received >= framelen {
                                yield frame;
                                break;
                            }
                        }
                        else {
                            // Try and derive the frame length
                            match phl::get_frame_length(&frame.buffer[..frame.received]) {
                                Ok((channel, length)) =>  {
                                    frame.channel = Some(channel);
                                    frame.length = Some(length);
                                },
                                Err(ReadError::NotEnoughBytes) => {
                                    continue;
                                }
                                Err(_) => {
                                    // Invalid frame length - restart receive
                                    self.transceiver.idle().await;
                                    break;
                                }
                            }
                        }
                    }
                    else {
                        // Error during read - restart receive
                        self.transceiver.idle().await;
                        break;
                    }
                }
            }
        }
    }

    // Stop the receiver.
    pub async fn idle(&mut self) {
        self.transceiver.idle().await;
        self.receiving = false;
    }

    /// Release the transceiver
    pub fn release(self) -> T {
        self.transceiver
    }
}
