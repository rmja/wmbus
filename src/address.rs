use core::fmt::Display;

use nobcd::{BcdError, BcdNumber};

use crate::{DeviceType, ManufacturerCode};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct WMBusAddress {
    pub manufacturer_code: u16,
    pub serial_number: BcdNumber<4>,
    pub version: u8,
    pub device_type: u8,
}

#[derive(Debug, PartialEq)]
pub enum WMBusAddressError {
    SerialNumberBcd,
}

enum FieldLayout {
    Default, // The default layout according to EN13757, i.e. Manufacturer, serial number, version, type
    Diehl, // The layout used by Diehl on some of its meters, i.e. Manufacturer, version, type, serial number
}

impl Display for WMBusAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:#x}:{:?}/{:?}/{:?}",
            self.manufacturer_code, self.serial_number, self.version, self.device_type
        )
    }
}

impl WMBusAddress {
    pub fn new(
        manufacturer_code: ManufacturerCode,
        serial_number: u32,
        version: u8,
        device_type: DeviceType,
    ) -> Self {
        Self {
            manufacturer_code: manufacturer_code as u16,
            serial_number: BcdNumber::new(serial_number).unwrap(),
            version,
            device_type: device_type as u8,
        }
    }

    pub fn from_bytes(value: [u8; 8]) -> Result<WMBusAddress, WMBusAddressError> {
        let layout = get_layout(&value);
        match layout {
            FieldLayout::Default => Ok(Self {
                manufacturer_code: u16::from_le_bytes(value[0..2].try_into().unwrap()),
                serial_number: parse_bcd_le(value[2..6].try_into().unwrap())
                    .map_err(|_| WMBusAddressError::SerialNumberBcd)?,
                version: value[6],
                device_type: value[7],
            }),
            FieldLayout::Diehl => Ok(Self {
                manufacturer_code: u16::from_le_bytes(value[0..2].try_into().unwrap()),
                serial_number: parse_bcd_le(value[4..8].try_into().unwrap())
                    .map_err(|_| WMBusAddressError::SerialNumberBcd)?,
                version: value[2],
                device_type: value[3],
            }),
        }
    }

    pub fn manufacturer_code(&self) -> Option<ManufacturerCode> {
        self.manufacturer_code.try_into().ok()
    }

    pub fn serial_number(&self) -> u32 {
        self.serial_number.value()
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn device_type(&self) -> Option<DeviceType> {
        self.device_type.try_into().ok()
    }

    pub fn get_bytes(&self) -> [u8; 8] {
        let mut bytes = [0; 8];
        bytes[0..2].copy_from_slice(self.manufacturer_code.to_le_bytes().as_ref());

        let mut index = 2;
        for byte in self.serial_number.into_iter().rev() {
            bytes[index] = byte;
            index += 1;
        }

        assert_eq!(6, index);
        bytes[6] = self.version;
        bytes[7] = self.device_type;

        bytes
    }
}

impl TryFrom<&[u8; 8]> for WMBusAddress {
    type Error = WMBusAddressError;

    fn try_from(value: &[u8; 8]) -> Result<WMBusAddress, Self::Error> {
        WMBusAddress::from_bytes(*value)
    }
}

fn get_layout(value: &[u8; 8]) -> FieldLayout {
    let manufacturer_code = u16::from_le_bytes(value[0..2].try_into().unwrap());
    if manufacturer_code == ManufacturerCode::HYD as u16 {
        // These indexes are not correct according to the standard, but are used by Diehl
        let version = value[2];
        let device_type = value[3];

        #[allow(clippy::if_same_then_else)]
        if (device_type == 0x04 || device_type == 0x0C) && version == 0x20 {
            // Sharky 775
            if let Ok(serial_number) = parse_bcd_le(value[4..8].try_into().unwrap()) {
                let serial_number: u32 = serial_number.value();
                if (44000000..48350000).contains(&serial_number)
                    || (51200000..51273000).contains(&serial_number)
                {
                    return FieldLayout::Diehl;
                }
            }
        } else if device_type == 0x04
            && (version == 0x2A || version == 0x2B || version == 0x2E || version == 0x2F)
        {
            return FieldLayout::Diehl;
        } else if device_type == 0x06 && version == 0x8B {
            return FieldLayout::Diehl;
        } else if device_type == 0x07 && (version == 0x85 || version == 0x86 || version == 0x8B) {
            return FieldLayout::Diehl;
        } else if device_type == 0x0C && (version == 0x2E || version == 0x2F || version == 0x53) {
            return FieldLayout::Diehl;
        } else if device_type == 0x16 && version == 0x25 {
            return FieldLayout::Diehl;
        }
    } else if manufacturer_code == ManufacturerCode::DME as u16 {
        // These indexes are not correct according to the standard, but are used by Diehl
        let version = value[2];
        let device_type = value[3];

        if device_type == 0x07 && version == 0x78 {
            return FieldLayout::Diehl;
        }
    }

    FieldLayout::Default
}

fn parse_bcd_le(bytes_le: &[u8; 4]) -> Result<BcdNumber<4>, BcdError> {
    let mut bytes_be = [0; 4];
    bytes_be.copy_from_slice(bytes_le);
    bytes_be.reverse();
    BcdNumber::<4>::from_bcd_bytes(bytes_be)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn parse_default() {
        let address =
            WMBusAddress::from_bytes([0x2D, 0x2C, 0x78, 0x56, 0x34, 0x12, 0x01, 0x32]).unwrap();
        assert_eq!(ManufacturerCode::KAM, address.manufacturer_code().unwrap());
        assert_eq!(12345678, address.serial_number.value::<u32>());
        assert_eq!(0x01, address.version);
        assert_eq!(DeviceType::Repeater, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_hydromenter_default() {
        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x14, 0x89, 0x81, 0x44, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(44818914, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());
        assert_eq!(
            [0x24, 0x23, 0x14, 0x89, 0x81, 0x44, 0x20, 0x04],
            address.get_bytes()
        );

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x91, 0x56, 0x39, 0x48, 0x20, 0x0C]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(48395691, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x95, 0x27, 0x80, 0x49, 0x20, 0x0C]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(49802795, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x59, 0x91, 0x95, 0x49, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(49959159, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x93, 0x56, 0x13, 0x51, 0x20, 0x0C]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51135693, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x06, 0x34, 0x27, 0x51, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51273406, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x02, 0x84, 0x84, 0x51, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51848402, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x83, 0x70, 0x29, 0x53, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(53297083, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_hydromenter_reversed() {
        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x85, 0x07, 0x47, 0x35, 0x04, 0x09]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(09043547, address.serial_number.value::<u32>());
        assert_eq!(0x85, address.version);
        assert_eq!(DeviceType::Water, address.device_type().unwrap());
        assert_ne!(
            [0x24, 0x23, 0x85, 0x07, 0x47, 0x35, 0x04, 0x09],
            address.get_bytes()
        );
        assert_eq!(
            [0x24, 0x23, 0x47, 0x35, 0x04, 0x09, 0x85, 0x07],
            address.get_bytes()
        );

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x85, 0x07, 0x25, 0x56, 0x00, 0x11]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(11005625, address.serial_number.value::<u32>());
        assert_eq!(0x85, address.version);
        assert_eq!(DeviceType::Water, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x31, 0x87, 0x81, 0x44]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(44818731, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x86, 0x88, 0x81, 0x44]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(44818886, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x70, 0x90, 0x81, 0x44]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(44819070, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x28, 0x87, 0x16, 0x46]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(46168728, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x04, 0x69, 0x02, 0x71, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47710269, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x18, 0x59, 0x78, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47785918, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x2B, 0x04, 0x41, 0x44, 0x87, 0x29]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(29874441, address.serial_number.value::<u32>());
        assert_eq!(0x2B, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x53, 0x0C, 0x95, 0x26, 0x86, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47862695, address.serial_number.value::<u32>());
        assert_eq!(0x53, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x61, 0x04, 0x34, 0x48]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(48340461, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x04, 0x02, 0x29, 0x27, 0x51]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51272902, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x8B, 0x06, 0x29, 0x32, 0x26, 0x63]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(63263229, address.serial_number.value::<u32>());
        assert_eq!(0x8B, address.version);
        assert_eq!(DeviceType::WarmWater, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_diehl_default() {
        let address =
            WMBusAddress::from_bytes([0xA5, 0x11, 0x55, 0x07, 0x16, 0x75, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::DME, address.manufacturer_code().unwrap());
        assert_eq!(75160755, address.serial_number.value::<u32>());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_diehl_reversed() {
        let address =
            WMBusAddress::from_bytes([0xA5, 0x11, 0x78, 0x07, 0x79, 0x19, 0x48, 0x20]).unwrap();
        assert_eq!(ManufacturerCode::DME, address.manufacturer_code().unwrap());
        assert_eq!(20481979, address.serial_number.value::<u32>());
        assert_eq!(0x78, address.version);
        assert_eq!(DeviceType::Water, address.device_type().unwrap());
    }

    #[test]
    fn parse_error() {
        assert_eq!(
            Err(WMBusAddressError::SerialNumberBcd),
            WMBusAddress::from_bytes([0xE4, 0x20, 0x00, 0xD0, 0x60, 0xC9, 0x00, 0x20])
        );
    }
}
