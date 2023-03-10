use assert_hex::assert_eq_hex;
use bitvec::prelude::*;
use wmbus::{
    modet::threeoutofsix::ThreeOutOfSix,
    stack::{
        phl::{FrameFormat, FFA, FFB},
        Mode, Stack,
    },
    DeviceType, ManufacturerCode,
};

#[test]
fn can_read_modec_ffa() {
    // Given
    let stack = Stack::new();
    #[rustfmt::skip]
    let frame = &[
        0x4E, 0x44, 0x2D, 0x2C, 0x98, 0x27, 0x04, 0x67, 0x30, 0x04, 0x91, 0x53,
        0x7A, 0xA6, 0x10, 0x40, 0x25, 0x6D, 0x3C, 0xA0, 0xF7, 0x2F, 0xF1, 0xEF, 0x06, 0x80, 0x6C, 0x50, 0xA1, 0x04,
        0x21, 0xCB, 0xD1, 0x32, 0xE3, 0xB1, 0xD0, 0x11, 0x6A, 0x05, 0x57, 0x69, 0x6E, 0x0E, 0x37, 0xC2, 0xE9, 0xF0,
        0x86, 0x36, 0xFE, 0x31, 0xF6, 0x8E, 0x6B, 0x4D, 0xEE, 0x5E, 0x38, 0x53, 0x16, 0xC2, 0x16, 0xA9, 0x6E, 0x27,
        0x7D, 0x48, 0xB1, 0x45, 0x92, 0x72, 0x38, 0x61, 0x46, 0xF7, 0x8C, 0x77, 0x66, 0xD5, 0x19, 0xFC, 0x44, 0x49,
        0x99, 0x3A, 0xDA, 0x5A, 0xAD, 0x95, 0xA5,
    ];

    // When
    let packet = stack.read(frame, Mode::ModeCFFA).unwrap();

    // Then
    assert_eq!(frame.len(), FFA::get_frame_length(frame).unwrap());

    let dll = packet.dll.unwrap();
    assert_eq!(
        ManufacturerCode::KAM,
        dll.address.manufacturer_code().unwrap()
    );
    assert_eq!(67042798, dll.address.serial_number());
    assert_eq_hex!(0x30, dll.address.version());
    assert_eq!(DeviceType::Heat, dll.address.device_type().unwrap());

    assert!(packet.ell.is_none());

    let apl = packet.apl;
    assert_eq!(69, apl.len());
    assert_eq_hex!(0x7A, apl[0]);
    assert_eq_hex!(0xAD, *apl.last().unwrap());
}

#[test]
fn can_read_modec_ffb() {
    // Given
    let stack = Stack::new();
    #[rustfmt::skip]
    let frame = &[
        0x13, 0x44, 0x2D, 0x2C, 0x78, 0x56, 0x34, 0x12, 0x01, 0x32,
        0xA0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0xC3, 0xC0,
    ];

    // When
    let packet = stack.read(frame, Mode::ModeCFFB).unwrap();

    // Then
    assert_eq!(frame.len(), FFB::get_frame_length(frame).unwrap());

    let dll = packet.dll.unwrap();
    assert_eq!(
        ManufacturerCode::KAM,
        dll.address.manufacturer_code().unwrap()
    );
    assert_eq!(12345678, dll.address.serial_number());
    assert_eq_hex!(0x01, dll.address.version());
    assert_eq!(DeviceType::Repeater, dll.address.device_type().unwrap());

    assert!(packet.ell.is_none());

    let apl = packet.apl;
    assert_eq!(8, apl.len());
    assert_eq_hex!(0xA0, apl[0]);
    assert_eq_hex!(0x06, *apl.last().unwrap());
}

#[test]
fn can_read_modec_ffb_presync() {
    // Given
    let stack = Stack::new();
    #[rustfmt::skip]
    let frame = &[
        0x54, 0x3D,
        0x13, 0x44, 0x2D, 0x2C, 0x78, 0x56, 0x34, 0x12, 0x01, 0x32,
        0xA0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0xC3, 0xC0,
    ];

    // When
    let packet = stack.read(frame, Mode::ModeCFFB).unwrap();

    // Then
    let dll = packet.dll.unwrap();
    assert_eq!(
        ManufacturerCode::KAM,
        dll.address.manufacturer_code().unwrap()
    );
    assert_eq!(12345678, dll.address.serial_number());
    assert_eq_hex!(0x01, dll.address.version());
    assert_eq!(DeviceType::Repeater, dll.address.device_type().unwrap());

    assert!(packet.ell.is_none());

    let apl = packet.apl;
    assert_eq!(8, apl.len());
    assert_eq_hex!(0xA0, apl[0]);
    assert_eq_hex!(0x06, *apl.last().unwrap());
}

#[test]
fn can_read_modet() {
    // Given
    let stack = Stack::new();
    #[rustfmt::skip]
    let frame = &[
        0x4E, 0x44, 0x2D, 0x2C, 0x98, 0x27, 0x04, 0x67, 0x30, 0x04, 0x91, 0x53,
        0x7A, 0xA6, 0x10, 0x40, 0x25, 0x6D, 0x3C, 0xA0, 0xF7, 0x2F, 0xF1, 0xEF, 0x06, 0x80, 0x6C, 0x50, 0xA1, 0x04,
        0x21, 0xCB, 0xD1, 0x32, 0xE3, 0xB1, 0xD0, 0x11, 0x6A, 0x05, 0x57, 0x69, 0x6E, 0x0E, 0x37, 0xC2, 0xE9, 0xF0,
        0x86, 0x36, 0xFE, 0x31, 0xF6, 0x8E, 0x6B, 0x4D, 0xEE, 0x5E, 0x38, 0x53, 0x16, 0xC2, 0x16, 0xA9, 0x6E, 0x27,
        0x7D, 0x48, 0xB1, 0x45, 0x92, 0x72, 0x38, 0x61, 0x46, 0xF7, 0x8C, 0x77, 0x66, 0xD5, 0x19, 0xFC, 0x44, 0x49,
        0x99, 0x3A, 0xDA, 0x5A, 0xAD, 0x95, 0xA5,
    ];
    let mut encode_buf = bitarr![u8, Msb0; 0; 91 * 2 * 6];
    let encoded_bits = ThreeOutOfSix::encode(&mut encode_buf, frame).unwrap();
    let encoded_bytes = (encoded_bits + 7) / 8; // Round up to nearest byte boundary
    let encoded = &encode_buf.as_raw_slice()[..encoded_bytes];

    // When
    let packet = stack.read(encoded, Mode::ModeTMTO).unwrap();

    // Then
    assert_eq!(frame.len(), FFA::get_frame_length(frame).unwrap());

    let dll = packet.dll.unwrap();
    assert_eq!(
        ManufacturerCode::KAM,
        dll.address.manufacturer_code().unwrap()
    );
    assert_eq!(67042798, dll.address.serial_number());
    assert_eq_hex!(0x30, dll.address.version());
    assert_eq!(DeviceType::Heat, dll.address.device_type().unwrap());

    assert!(packet.ell.is_none());

    let apl = packet.apl;
    assert_eq!(69, apl.len());
    assert_eq_hex!(0x7A, apl[0]);
    assert_eq_hex!(0xAD, *apl.last().unwrap());
}
