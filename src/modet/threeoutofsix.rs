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
    /// The provided buffer is not sufficiently largo to include the result
    Capacity,
    /// The input length is invalid
    InputLength,
    /// The decode of a symbol failed
    Symbol(usize),
}

impl ThreeOutOfSix {
    /// 3oo6 encode into the provided buffer and returns the number of bits encoded
    pub fn encode(buffer: &mut BitSlice<u8, Msb0>, source: &[u8]) -> Result<usize, Error> {
        if buffer.len() < source.len() * 2 * 6 {
            return Err(Error::Capacity);
        }

        let mut written = 0;
        for byte in source {
            for nibble in [byte >> 4, byte & 0x0F] {
                let symbol = ENCODE_TABLE[nibble as usize];
                buffer.set(written, symbol & 0x20 != 0);
                written += 1;
                buffer.set(written, symbol & 0x10 != 0);
                written += 1;
                buffer.set(written, symbol & 0x08 != 0);
                written += 1;
                buffer.set(written, symbol & 0x04 != 0);
                written += 1;
                buffer.set(written, symbol & 0x02 != 0);
                written += 1;
                buffer.set(written, symbol & 0x01 != 0);
                written += 1;
            }
        }

        Ok(written)
    }

    pub fn decode<T: BitStore>(
        buffer: &mut [u8],
        input: &BitSlice<T, Msb0>,
    ) -> Result<usize, Error> {
        let symbols = input.chunks_exact(6);
        if !symbols.remainder().is_empty() || symbols.len() & 1 != 0 {
            return Err(Error::InputLength);
        }

        let mut written = 0;
        let mut carry = None;

        for (index, symbol) in symbols.enumerate() {
            let table_index = symbol.load_be::<usize>();
            let value = DECODE_TABLE[table_index];
            if value == -1 {
                return Err(Error::Symbol(index));
            }
            let value = value as u8;
            if let Some(previous) = carry.take() {
                buffer[written] = (previous << 4) | value;
                written += 1;
            } else {
                carry = Some(value);
            }
        }

        Ok(written)
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
        let mut buffer = bitarr![u8, Msb0; 0; 800];
        let encoded_bits = ThreeOutOfSix::encode(&mut buffer, &data).unwrap();
        let encoded_expected: Vec<u8> = vec![
            0x3a, 0x97, 0x1c, 0x6a, 0xc6, 0x56, 0x39, 0x33, 0x8d, 0x71, 0x92, 0xd6, 0x65, 0x66,
            0x8e, 0x8f, 0x1d, 0x34, 0x98, 0xe5, 0x9a, 0x96, 0x93, 0x63, 0x34, 0xd5, 0x9a, 0xd1,
            0x63, 0x56, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96,
            0x65, 0x99, 0x8b, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65,
            0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0xa6,
            0x9a, 0x69, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0x59, 0x65, 0x96, 0xa6, 0x9a, 0x69,
        ];
        let expected: BitVec<u8, Msb0> = BitVec::from_vec(encoded_expected);

        assert_eq_hex!(expected, buffer[..encoded_bits]);
    }

    #[test]
    pub fn can_encode_correctly_terminates() {
        let mut buffer = bitarr![u8, Msb0; 0; 12];
        let data: [u8; 1] = [0x12];
        let encoded = ThreeOutOfSix::encode(&mut buffer, &data).unwrap();

        assert_eq!(12, encoded);
        assert_eq!(
            bitvec![u8, Msb0; 0, 0, 1, 1, 0, 1, 0, 0, 1, 1, 1, 0],
            &buffer[..encoded]
        );
    }

    #[test]
    pub fn can_decode() {
        let data = vec![
            0x2F, 0x44, 0x68, 0x50, 0x27, 0x21, 0x45, 0x30, 0x50, 0x62, 0xBD, 0xCC, 0xA2, 0x06,
            0x9F, 0x1B, 0x11, 0x06, 0xC0, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x55, 0xA3, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF,
        ];
        let mut encode_buf = bitarr![u8, Msb0; 0; 800];
        let encoded = ThreeOutOfSix::encode(&mut encode_buf, &data).unwrap();
        let mut decode_buf = [0; 100];
        let decoded = ThreeOutOfSix::decode(&mut decode_buf, &encode_buf[..encoded]).unwrap();
        assert_eq!(data, decode_buf[..decoded]);
    }
}
