use crate::stack::ReadError;
use alloc::vec::Vec;

use super::is_valid_crc;

const FIRST_BLOCK_DATA_LENGTH: usize = 1 + 1 + 2 + 6;
const OTHER_BLOCK_MAX_DATA_LENGTH: usize = 16;
const MIN_DATA_LENGTH: usize = FIRST_BLOCK_DATA_LENGTH + 1; // CI field must be present
const MAX_DATA_LENGTH: usize = 256;
const MAX_BLOCK_COUNT: usize = 17; // 10 + (1 + 15) + 14 * 16 + 6 = 256
pub const MAX_FRAME_SIZE: usize = MAX_DATA_LENGTH + 2 * MAX_BLOCK_COUNT;

pub const fn get_frame_length(buffer: &[u8]) -> Result<usize, ReadError> {
    if buffer.is_empty() {
        return Err(ReadError::NotEnoughBytes);
    }

    let data_length = 1 + buffer[0] as usize;
    get_frame_length_from_data_length(data_length)
}

const fn get_frame_length_from_data_length(data_length: usize) -> Result<usize, ReadError> {
    if data_length < MIN_DATA_LENGTH {
        return Err(ReadError::PhlInvalidLength);
    }

    let other_data_length = data_length - FIRST_BLOCK_DATA_LENGTH;
    let full_block_count = other_data_length / OTHER_BLOCK_MAX_DATA_LENGTH;
    let last_block_data_length = other_data_length - full_block_count * OTHER_BLOCK_MAX_DATA_LENGTH;

    let last_block_frame_length = if last_block_data_length > 0 {
        last_block_data_length + 2
    } else {
        0
    };

    let frame_length = FIRST_BLOCK_DATA_LENGTH
        + 2
        + full_block_count * (OTHER_BLOCK_MAX_DATA_LENGTH + 2)
        + last_block_frame_length;

    debug_assert!(frame_length <= MAX_FRAME_SIZE);

    Ok(frame_length)
}

pub(crate) fn read(buffer: &[u8]) -> Result<Vec<u8>, ReadError> {
    let frame_length = get_frame_length(buffer)?;
    if buffer.len() < frame_length {
        return Err(ReadError::NotEnoughBytes);
    }

    let data_length = 1 + buffer[0] as usize;
    let mut data = Vec::with_capacity(data_length);

    let (first_block, other_blocks) = buffer.split_at(FIRST_BLOCK_DATA_LENGTH + 2);

    // First block
    if !is_valid_crc(first_block) {
        return Err(ReadError::PhlCrcError(0));
    }
    data.extend_from_slice(&first_block[..first_block.len() - 2]);

    // Subsequent blocks
    for (index, block) in other_blocks
        .chunks(OTHER_BLOCK_MAX_DATA_LENGTH + 2)
        .enumerate()
    {
        if !is_valid_crc(block) {
            return Err(ReadError::PhlCrcError(1 + index));
        }
        data.extend_from_slice(&block[..block.len() - 2]);
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_frame_length() {
        assert!(get_frame_length_from_data_length(0).is_err());
        assert!(get_frame_length_from_data_length(10).is_err());
        assert_eq!(
            Ok(10 + 2 + 1 + 2),
            get_frame_length_from_data_length(10 + 1)
        );
        assert_eq!(
            Ok(10 + 2 + 16 + 2),
            get_frame_length_from_data_length(10 + 16)
        );
        assert_eq!(
            Ok(10 + 2 + 16 + 2 + 1 + 2),
            get_frame_length_from_data_length(10 + 16 + 1)
        );
        assert_eq!(
            Ok(10 + 2 + 2 * (16 + 2)),
            get_frame_length_from_data_length(10 + 2 * 16)
        );
        assert_eq!(
            Ok(10 + 2 + 2 * (16 + 2) + 1 + 2),
            get_frame_length_from_data_length(10 + 2 * 16 + 1)
        );
        assert_eq!(
            Ok(10 + 2 + 3 * (16 + 2)),
            get_frame_length_from_data_length(10 + 3 * 16)
        );
        assert_eq!(
            Ok(10 + 2 + 15 * (16 + 2)),
            get_frame_length_from_data_length(10 + 15 * 16)
        );
        assert_eq!(
            Ok(10 + 2 + 15 * (16 + 2) + 1 + 2),
            get_frame_length_from_data_length(10 + 15 * 16 + 1)
        );
        assert_eq!(
            Ok(10 + 2 + 15 * (16 + 2) + 5 + 2),
            get_frame_length_from_data_length(10 + 15 * 16 + 5)
        );
    }
}
