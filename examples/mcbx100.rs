use bytes::{BufMut, BytesMut};
use wmbus::{
    modec::FFB_SYNCWORD,
    stack::{dll::DllFields, Mode, Packet, Stack},
    DeviceType, ManufacturerCode, WMBusAddress,
};

fn main() {
    let stack = Stack::new();
    let mut packet: Packet = Packet::new(Mode::ModeCFFB);

    let apl = [
        0xa0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
        0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
    ];
    packet.apl.extend_from_slice(&apl).unwrap();

    for i in 0..100 {
        packet.dll = Some(DllFields {
            control: 0x44,
            address: WMBusAddress::new(
                ManufacturerCode::KAM,
                12345600 + i,
                0x01,
                DeviceType::Repeater,
            ),
        });

        let mut writer = BytesMut::new();
        writer.put_slice(&[0x55, 0x55, 0x55, 0x55]);
        writer.put_slice(&FFB_SYNCWORD);
        stack.write(&mut writer, &packet).unwrap();

        println!("FRAME: {:02x?}", writer.to_vec());
    }
}
