use core::{convert::TryInto, fmt::Display};

use bcd::{BcdError, BcdNumber};

use crate::{DeviceType, ManufacturerCode};

#[derive(Clone, Debug, PartialEq)]
pub struct WMBusAddress {
    pub manufacturer_code: u16,
    pub serial_number: BcdNumber<4>,
    pub version: u8,
    pub device_type: u8,
}

#[derive(Debug)]
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
            serial_number: BcdNumber::from_u32(serial_number),
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

fn parse_bcd_le(bytes_le: &[u8; 4]) -> Result<BcdNumber<4>, BcdError> {
    let mut bytes_be = [0; 4];
    bytes_be.copy_from_slice(bytes_le);
    bytes_be.reverse();
    BcdNumber::<4>::try_from(bytes_be)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn parse_default() {
        let address =
            WMBusAddress::from_bytes([0x2D, 0x2C, 0x78, 0x56, 0x34, 0x12, 0x01, 0x32]).unwrap();
        assert_eq!(ManufacturerCode::KAM, address.manufacturer_code().unwrap());
        assert_eq!(12345678u32, address.serial_number.value());
        assert_eq!(0x01, address.version);
        assert_eq!(DeviceType::Repeater, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_hydromenter_default() {
        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x95, 0x27, 0x80, 0x49, 0x20, 0x0C]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(49802795u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x59, 0x91, 0x95, 0x49, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(49959159u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x06, 0x34, 0x27, 0x51, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51273406u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x02, 0x84, 0x84, 0x51, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51848402u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x83, 0x70, 0x29, 0x53, 0x20, 0x04]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(53297083u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());
    }

    #[test]
    pub fn parse_hydromenter_reversed() {
        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x04, 0x69, 0x02, 0x71, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47710269u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x18, 0x59, 0x78, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47785918u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x53, 0x0C, 0x95, 0x26, 0x86, 0x47]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(47862695u32, address.serial_number.value());
        assert_eq!(0x53, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x0C, 0x61, 0x04, 0x34, 0x48]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(48340461u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::HeatInlet, address.device_type().unwrap());

        let address =
            WMBusAddress::from_bytes([0x24, 0x23, 0x20, 0x04, 0x02, 0x29, 0x27, 0x51]).unwrap();
        assert_eq!(ManufacturerCode::HYD, address.manufacturer_code().unwrap());
        assert_eq!(51272902u32, address.serial_number.value());
        assert_eq!(0x20, address.version);
        assert_eq!(DeviceType::Heat, address.device_type().unwrap());
    }
}
