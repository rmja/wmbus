#![cfg_attr(not(test), no_std)]
#![feature(generators)]
#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]
#![feature(const_trait_impl)]
#![feature(let_chains)]
#![feature(generic_const_exprs)]

#[macro_use]
extern crate num_derive;

mod address;
#[cfg(feature = "ctrl")]
pub mod ctrl;
pub mod modec;
pub mod modet;
pub mod stack;

#[cfg(feature = "alloc")]
extern crate alloc;

pub use address::WMBusAddress;

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

impl TryFrom<u16> for ManufacturerCode {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u16(value).ok_or(())
    }
}

impl TryFrom<u8> for DeviceType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u8(value).ok_or(())
    }
}