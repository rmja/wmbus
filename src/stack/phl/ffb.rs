use alloc::vec::Vec;

use super::FrameFormat;
use super::is_valid_crc;
use super::Error;

const FIRST_BLOCK_DATA_LENGTH: usize = 1 + 1 + 2 + 6;
const SECOND_BLOCK_MAX_DATA_LENGTH: usize = 1 + 115;
const MIN_DATA_LENGTH: usize = FIRST_BLOCK_DATA_LENGTH + 1; // CI field must be present
const MIN_FRAME_LENGTH: usize = MIN_DATA_LENGTH + 2;

pub struct FFB;

impl const FrameFormat for FFB {
    const APL_MAX: usize = Self::FRAME_MAX - FIRST_BLOCK_DATA_LENGTH - 2 - 2;
    const FRAME_MAX: usize = 256;

    fn get_frame_length(buffer: &[u8]) -> Result<usize, Error> {
        if buffer.is_empty() {
            return Err(Error::Incomplete);
        }
    
        let frame_length = 1 + buffer[0] as usize;
        if frame_length < MIN_FRAME_LENGTH {
            return Err(Error::InvalidLength);
        }
    
        debug_assert!(frame_length <= Self::FRAME_MAX);
    
        Ok(frame_length)
    }
}

pub(crate) fn read(buffer: &[u8]) -> Result<Vec<u8>, Error> {
    let frame_length = FFB::get_frame_length(buffer)?;
    if buffer.len() < frame_length {
        return Err(Error::Incomplete);
    }

    let mut data = Vec::with_capacity(frame_length); // Too large

    for (index, block) in buffer
        .chunks(FIRST_BLOCK_DATA_LENGTH + SECOND_BLOCK_MAX_DATA_LENGTH + 2)
        .enumerate()
    {
        if !is_valid_crc(block) {
            return Err(Error::CrcBlock(index));
        }
        data.extend_from_slice(&block[..block.len() - 2]);
    }

    Ok(data)
}
