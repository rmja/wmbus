#![cfg_attr(not(test), no_std)]
#![feature(generators)]

extern crate alloc;

#[macro_use]
extern crate num_derive;

mod address;
pub mod modec;
pub mod modet;
pub mod stack;
#[cfg(feature = "ctrl")]
pub mod ctrl;

#[derive(Clone, Copy, Debug, PartialEq, FromPrimitive)]
#[repr(u16)]
pub enum ManufacturerCode {
    APT = 0x8614, // Apator
    DME = 0x11A5, // Diehl
    GAV = 0x1C36, // Carlo Gavazzi
    HYD = 0x2324, // Hydrometer
    KAM = 0x2C2D, // Kamstrup
    LUG = 0x32A7, // Landis+Gyr GmbH
    SON = 0x4DEE, // Sontex
    TCH = 0x5068, // Techem
}

#[derive(Clone, Copy, Debug, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum DeviceType {
    Other = 0x00,
    Electricity = 0x02,
    Heat = 0x04,
    WarmWater = 0x06,
    Water = 0x07,
    Cooling = 0x0A,
    CoolingInlet = 0x0B,
    HeatInlet = 0x0C,
    HeatCooling = 0x0D,
    Unknown = 0x0F,
    ColdWater = 0x16,
    Repeater = 0x32,
}
