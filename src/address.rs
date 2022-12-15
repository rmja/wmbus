use core::{convert::TryInto, fmt::Display};

use bcd_numbers::{BCDConversionError, BCD};

use crate::{DeviceType, ManufacturerCode};

#[derive(PartialEq)]
pub struct WMBusAddress {
    manufacturer_code: u16,
    serial_number: BCD<4>,
    version: u8,
    device_type: u8,
}

#[derive(Debug)]
pub enum WMBusAddressError {
    InvalidSerialNumberBcd,
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
            serial_number: BCD::new(serial_number as u128),
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
                    .map_err(|_| WMBusAddressError::InvalidSerialNumberBcd)?,
                version: value[6],
                device_type: value[7],
            }),
            FieldLayout::Diehl => Ok(Self {
                manufacturer_code: u16::from_le_bytes(value[0..2].try_into().unwrap()),
                serial_number: parse_bcd_le(value[4..8].try_into().unwrap())
                    .map_err(|_| WMBusAddressError::InvalidSerialNumberBcd)?,
                version: value[2],
                device_type: value[3],
            }),
        }
    }

    pub fn manufacturer_code(&self) -> Option<ManufacturerCode> {
        num_traits::FromPrimitive::from_u16(self.manufacturer_code)
    }

    pub fn serial_number(&self) -> u32 {
        self.serial_number.get_number() as u32
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn device_type(&self) -> Option<DeviceType> {
        num_traits::FromPrimitive::from_u8(self.device_type)
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

        if (device_type == 0x04 || device_type == 0x0C) && version == 0x20 {
            // Sharky 775
            if let Ok(serial_number) = parse_bcd_le(value[4..8].try_into().unwrap()) {
                let serial_number = serial_number.get_number();
                if (serial_number >= 44000000 && serial_number < 48350000)
                    || (serial_number >= 51200000 && serial_number < 51273000)
                {
                    return FieldLayout::Diehl;
                }
            }
        } else if device_type == 0x04
            && (version == 0x2A || version == 0x2B || version == 0x2E || version == 0x2F)
        {
            return FieldLayout::Diehl;
        } else if device_type == 0x06 && (version == 0x8B) {
            return FieldLayout::Diehl;
        } else if device_type == 0x0C && (version == 0x2E || version == 0x2F || version == 0x53) {
            return FieldLayout::Diehl;
        } else if device_type == 0x16 && version == 0x25 {
            return FieldLayout::Diehl;
        }
    }

    FieldLayout::Default
}

fn parse_bcd_le(bytes_le: &[u8; 4]) -> Result<BCD<4>, BCDConversionError> {
    let mut bytes_be = [0; 4];
    bytes_be.copy_from_slice(bytes_le);
    bytes_be.reverse();
    BCD::<4>::try_from(bytes_be)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn parse_default() {
        let address =
            WMBusAddress::from_bytes([0x2D, 0x2C, 0x78, 0x56, 0x34, 0x12, 0x01, 0x32]).unwrap();
        assert_eq!(ManufacturerCode::KAM, address.manufacturer_code().unwrap());
        assert_eq!(12345678, address.serial_number.get_number());
        assert_eq!(0x01, address.version);
        assert_eq!(DeviceType::Repeater, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_hydromenter_default() {
        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x95, 0x27, 0x80, 0x49, 0x20, 0x0C]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(49802795, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x59, 0x91, 0x95, 0x49, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(49959159, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x06, 0x34, 0x27, 0x51, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51273406, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x02, 0x84, 0x84, 0x51, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51848402, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x83, 0x70, 0x29, 0x53, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(53297083, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_hydromenter_reversed() {
        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x04, 0x69, 0x02, 0x71, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47710269, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x18, 0x59, 0x78, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47785918, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x53, 0x0C, 0x95, 0x26, 0x86, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47862695, address.serial_number.get_number());
        assert_eq!(0x53, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x61, 0x04, 0x34, 0x48]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(48340461, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x04, 0x02, 0x29, 0x27, 0x51]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51272902, address.serial_number.get_number());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());
    }
}
