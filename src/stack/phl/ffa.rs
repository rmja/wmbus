use heapless::Vec;

use super::is_valid_crc;
use super::Error;
use super::FrameFormat;

const FIRST_BLOCK_DATA_LENGTH: usize = 1 + 1 + 2 + 6;
const OTHER_BLOCK_MAX_DATA_LENGTH: usize = 16;
const MIN_DATA_LENGTH: usize = FIRST_BLOCK_DATA_LENGTH + 1; // CI field must be present
const MAX_DATA_LENGTH: usize = 256;
const MAX_BLOCK_COUNT: usize = 17; // 10 + (1 + 15) + 14 * 16 + 6 = 256

pub struct FFA;

impl FrameFormat for FFA {
    const APL_MAX: usize = MAX_DATA_LENGTH - FIRST_BLOCK_DATA_LENGTH;
    const DATA_MAX: usize = MAX_DATA_LENGTH;
    const FRAME_MAX: usize = MAX_DATA_LENGTH + 2 * MAX_BLOCK_COUNT;

    fn get_frame_length(buffer: &[u8]) -> Result<usize, Error> {
        if buffer.is_empty() {
            return Err(Error::Incomplete);
        }

        let data_length = 1 + buffer[0] as usize;
        get_frame_length_from_data_length(data_length)
    }

    fn read(buffer: &[u8]) -> Result<Vec<u8, { Self::DATA_MAX }>, Error> {
        let frame_length = Self::get_frame_length(buffer)?;
        if buffer.len() < frame_length {
            return Err(Error::Incomplete);
        }

        let (first_block, other_blocks) = buffer.split_at(FIRST_BLOCK_DATA_LENGTH + 2);

        // First block
        if !is_valid_crc(first_block) {
            return Err(Error::CrcBlock(0));
        }

        let mut data = Vec::from_slice(&first_block[..first_block.len() - 2]).unwrap();

        // Subsequent blocks
        for (index, block) in other_blocks
            .chunks(OTHER_BLOCK_MAX_DATA_LENGTH + 2)
            .enumerate()
        {
            if !is_valid_crc(block) {
                return Err(Error::CrcBlock(1 + index));
            }
            data.extend_from_slice(&block[..block.len() - 2]).unwrap();
        }

        Ok(data)
    }
}

const fn get_frame_length_from_data_length(data_length: usize) -> Result<usize, Error> {
    if data_length < MIN_DATA_LENGTH {
        return Err(Error::InvalidLength);
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

    debug_assert!(frame_length <= FFA::FRAME_MAX);

    Ok(frame_length)
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
