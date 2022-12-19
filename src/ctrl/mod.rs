pub mod traits;
mod controller;

pub type Rssi = i8;

pub use controller::Controller;

#[derive(Debug)]
pub enum TransceiverError {
    /// The transceiver was not found to be present
    NotPresent,
    Timeout,
}
