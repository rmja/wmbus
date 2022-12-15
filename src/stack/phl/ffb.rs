use crate::stack::ReadError;
use alloc::vec::Vec;

use super::is_valid_crc;

const FIRST_BLOCK_DATA_LENGTH: usize = 1 + 1 + 2 + 6;
const SECOND_BLOCK_MAX_DATA_LENGTH: usize = 1 + 115;
const MIN_DATA_LENGTH: usize = FIRST_BLOCK_DATA_LENGTH + 1;
const MIN_FRAME_LENGTH: usize = MIN_DATA_LENGTH + 2;
pub const MAX_FRAME_LENGTH: usize = 256;

pub const fn get_frame_length(buffer: &[u8]) -> Result<usize, ReadError> {
    if buffer.len() == 0 {
        return Err(ReadError::NotEnoughBytes);
    }

    let frame_length = 1 + buffer[0] as usize;
    if frame_length < MIN_FRAME_LENGTH {
        return Err(ReadError::PhlInvalidLength);
    }

    debug_assert!(frame_length <= MAX_FRAME_LENGTH);

    Ok(frame_length)
}

pub(crate) fn read(buffer: &[u8]) -> Result<Vec<u8>, ReadError> {
    let frame_length = get_frame_length(buffer)?;
    if buffer.len() < frame_length {
        return Err(ReadError::NotEnoughBytes);
    }

    let mut data = Vec::with_capacity(frame_length); // Too large

    for (index, block) in buffer
        .chunks(FIRST_BLOCK_DATA_LENGTH + SECOND_BLOCK_MAX_DATA_LENGTH + 2)
        .enumerate()
    {
        if !is_valid_crc(block) {
            return Err(ReadError::PhlCrcError(index));
        }
        data.extend_from_slice(&block[..block.len() - 2]);
    }

    Ok(data)
}