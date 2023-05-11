use crate::stack::phl::FrameFormat;

pub mod threeoutofsix;

pub const SYNCWORD: [u8; 2] = [0x54, 0x3D];
pub const CHIPRATE: u32 = 100_000; // kcps
pub const THREE_OUT_OF_SIX_ENCODED_MAX: usize = (crate::stack::phl::FFA::FRAME_MAX * 6) / 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_max() {
        const FRAME_MAX: usize = 2 + 256 + 16 * 2;
        assert_eq!(290, FRAME_MAX);
        assert_eq!(435, THREE_OUT_OF_SIX_ENCODED_MAX);
    }
}
