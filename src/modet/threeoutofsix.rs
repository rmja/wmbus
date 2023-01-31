use alloc::vec::Vec;
use bitvec::{field::BitField, prelude::*};

pub struct ThreeOutOfSix;

// Table 10 in EN13757-4
#[rustfmt::skip]
const ENCODE_TABLE: [u8; 0x10] = [
    22, 13, 14, 11, 28, 25, 26, 19, 44, 37, 38, 35, 52, 49, 50, 41,
];
#[rustfmt::skip]
const DECODE_TABLE: [i8; 0x40] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,  3, -1,  1,  2, -1,
    -1, -1, -1,  7, -1, -1,  0, -1, -1,  5,  6, -1,  4, -1, -1, -1,
    -1, -1, -1, 11, -1,  9, 10, -1, -1, 15, -1, -1,  8, -1, -1, -1,
    -1, 13, 14, -1, 12, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidInputLength,
    InvalidSymbol,
    Write,
}

impl ThreeOutOfSix {
    pub fn encode(source: &[u8]) -> BitVec<u8, Msb0> {
        let mut encoded = BitVec::with_capacity(source.len() * 2 * 6);

        for byte in source {
            for nibble in [byte >> 4, byte & 0x0F] {
                let symbol = ENCODE_TABLE[nibble as usize];
                encoded.push(symbol & 0x20 != 0);
                encoded.push(symbol & 0x10 != 0);
                encoded.push(symbol & 0x08 != 0);
                encoded.push(symbol & 0x04 != 0);
                encoded.push(symbol & 0x02 != 0);
                encoded.push(symbol & 0x01 != 0);
            }
        }

        encoded
    }

    pub fn decode<T: BitStore>(input: &BitSlice<T, Msb0>) -> Result<Vec<u8>, Error> {
        let symbols = input.chunks_exact(6);
        if !symbols.remainder().is_empty() || symbols.len() & 1 != 0 {
            return Err(Error::InvalidInputLength);
        }

        let mut decoded = Vec::with_capacity(symbols.len() * 2);
        let mut carry = None;

        for symbol in symbols {
            let index = symbol.load_be::<usize>();
            let value = DECODE_TABLE[index];
            if value == -1 {
                return Err(Error::InvalidSymbol);
            }
            let value = value as u8;
            if let Some(previous) = carry.take() {
                decoded.push((previous << 4) | value);
            } else {
                carry = Some(value);
            }
        }

        Ok(decoded)
    }
}

#[cfg(test)]
pub mod tests {
    use assert_hex::assert_eq_hex;

    use super::*;

    #[test]
    pub fn can_encode_example() {
        let data = vec![
            0x2F, 0x44, 0x68, 0x50, 0x27, 0x21, 0x45, 0x30, 0x50, 0x62, 0xBD, 0xCC, 0xA2, 0x06,
            0x9F, 0x1B, 0x11, 0x06, 0xC0, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x55, 0xA3, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF,
        ];
        let encoded = ThreeOutOfSix::encode(&data);
        let encoded_expected: Vec<u8> = vec![
            0x3a, 0x97, 0x1c, 0x6a, 0xc6, 0x56, 0x39, 0x33, 0x8d, 0x71, 0x92, 0xd6, 0x65, 0x66,
            0x8e, 0x8f, 0x1d, 0x34, 0x98, 0xe5, 0x9a, 0x96, 0x93, 0x63, 0x34, 0xd5, 0x9a, 0xd1,
            0x63, 0x56, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96,
            0x65, 0x99, 0x8b, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65,
            0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0xa6,
            0x9a, 0x69, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0xa6, 0x9a, 0x69,
        ];
        let expected: BitVec<u8, Msb0> = BitVec::from_vec(encoded_expected);

        assert_eq_hex!(expected, encoded);
    }

    #[test]
    pub fn can_encode_correctly_terminates() {
        let data: [u8; 1] = [0x12];
        let encoded = ThreeOutOfSix::encode(&data);

        assert_eq!(bitvec![0, 0, 1, 1, 0, 1, 0, 0, 1, 1, 1, 0], encoded);
    }

    #[test]
    pub fn can_decode() {
        let data = vec![
            0x2F, 0x44, 0x68, 0x50, 0x27, 0x21, 0x45, 0x30, 0x50, 0x62, 0xBD, 0xCC, 0xA2, 0x06,
            0x9F, 0x1B, 0x11, 0x06, 0xC0, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x55, 0xA3, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF,
        ];
        let encoded = ThreeOutOfSix::encode(&data);
        let decoded = ThreeOutOfSix::decode(&encoded);
        assert_eq!(data, decoded.unwrap());
    }
}
