use bytes::{BufMut, BytesMut};
use wmbus::{
    modec::FFB_SYNCWORD,
    stack::{dll::DllFields, Mode, Packet, Stack},
    DeviceType, ManufacturerCode, WMBusAddress,
};

fn main() {
    let stack = Stack::new();
    let mut packet: Packet = Packet::new(Mode::ModeCFFB);
    packet.dll = Some(DllFields {
        control: 0x44,
        address: WMBusAddress::new(ManufacturerCode::KAM, 12345678, 0x01, DeviceType::Repeater),
    });
    packet
        .apl
        .extend_from_slice(&[0xa0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
        .unwrap();

    let mut writer = BytesMut::new();
    writer.put_slice(&[0x55, 0x55, 0x55, 0x55]);
    writer.put_slice(&FFB_SYNCWORD);
    stack.write(&mut writer, &packet).unwrap();

    println!("FRAME: {:02x?}", writer.to_vec());
}
