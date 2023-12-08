//! Configuration types read from and written to the ADS1110

use embassy_time::Duration;

/// The ADS1110 comes in one eight variants, with the address bits
/// set in the factory. Your part will have a part number, like:
///
/// ```text
/// ADS1110A0IDBVR  | Full Name
/// -------         | - Common family name (ADS1110)
///        --       | - Address (A0)
///          -----  | - Other packaging info (IDBVR)
/// ```
///
/// You'll need to pick an [Address] variant that matches your part number(s).
///
/// | Address/Part | Address (binary) | Address (hex, right aligned)    |
/// | :---         | :---             | :---                            |
/// | ADS1110A0    | `0b1001_000x`    | `0x48`                          |
/// | ADS1110A1    | `0b1001_001x`    | `0x49`                          |
/// | ADS1110A2    | `0b1001_010x`    | `0x4A`                          |
/// | ADS1110A3    | `0b1001_011x`    | `0x4B`                          |
/// | ADS1110A4    | `0b1001_100x`    | `0x4C`                          |
/// | ADS1110A5    | `0b1001_101x`    | `0x4D`                          |
/// | ADS1110A6    | `0b1001_110x`    | `0x4E`                          |
/// | ADS1110A7    | `0b1001_111x`    | `0x4F`                          |
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Address {
    /// Address 0, marked as `ED0` on-package
    A0,
    /// Address 1, marked as `ED1` on-package
    A1,
    /// Address 2, marked as `ED2` on-package
    A2,
    /// Address 3, marked as `ED3` on-package
    A3,
    /// Address 4, marked as `ED4` on-package
    A4,
    /// Address 5, marked as `ED5` on-package
    A5,
    /// Address 6, marked as `ED6` on-package
    A6,
    /// Address 7, marked as `ED7` on-package
    A7,
}

impl Address {
    /// Convert into the right-aligned 7-bit address
    pub fn into_addr(&self) -> u8 {
        match self {
            Address::A0 => 0x48,
            Address::A1 => 0x49,
            Address::A2 => 0x4A,
            Address::A3 => 0x4B,
            Address::A4 => 0x4C,
            Address::A5 => 0x4D,
            Address::A6 => 0x4E,
            Address::A7 => 0x4F,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DataRate {
    /// 15 samples per second - every 66.7ms
    Sps15,
    /// 30 samples per second - every 33.3ms
    Sps30,
    /// 60 samples per second - every 16.7ms
    Sps60,
    /// 240 samples per second - every 4.2ms
    Sps240,
}

impl DataRate {
    /// Get the interval between samples as a [`Duration`].
    pub fn interval(&self) -> Duration {
        match self {
            DataRate::Sps15 => Duration::from_micros(66_667),
            DataRate::Sps30 => Duration::from_micros(33_333),
            DataRate::Sps60 => Duration::from_micros(16_667),
            DataRate::Sps240 => Duration::from_micros(4_167),
        }
    }

    /// Get 1/4 of the interval between samples as a [`Duration`].
    pub fn quarter_interval(&self) -> Duration {
        match self {
            DataRate::Sps15 => Duration::from_micros(16_667),
            DataRate::Sps30 => Duration::from_micros(8_333),
            DataRate::Sps60 => Duration::from_micros(4_167),
            DataRate::Sps240 => Duration::from_micros(1_042),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Start {
    /// Write: 0, nothing happens
    DontStart,
    /// Write: 1, conversion started
    StartConversion,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DataReady {
    /// Read: 0, New unread data
    FreshData,
    /// Read: 1, Data has been read
    StaleData,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConversionMode {
    /// R/W: 0
    Continuous,
    /// R/W: 1
    OneShot,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Gain {
    /// R/W: 00
    X1,
    /// R/W: 01
    X2,
    /// R/W: 10
    X4,
    /// R/W: 11
    X8,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct WriteSettings {
    pub start: Start,
    pub sc: ConversionMode,
    pub dr: DataRate,
    pub pga: Gain,
}

impl Default for WriteSettings {
    fn default() -> Self {
        Self {
            start: Start::DontStart,
            sc: ConversionMode::Continuous,
            dr: DataRate::Sps15,
            pga: Gain::X1,
        }
    }
}

impl WriteSettings {
    pub fn to_value(&self) -> u8 {
        let mut output = 0u8;
        output |= match self.start {
            Start::DontStart => 0b0000_0000,
            Start::StartConversion => 0b1000_0000,
        };
        output |= match self.sc {
            ConversionMode::Continuous => 0b0000_0000,
            ConversionMode::OneShot => 0b0001_0000,
        };
        output |= match self.dr {
            DataRate::Sps15 => 0b0000_1100,
            DataRate::Sps30 => 0b0000_1000,
            DataRate::Sps60 => 0b0000_0100,
            DataRate::Sps240 => 0b0000_0000,
        };
        output |= match self.pga {
            Gain::X1 => 0b0000_0000,
            Gain::X2 => 0b0000_0001,
            Gain::X4 => 0b0000_0010,
            Gain::X8 => 0b0000_0011,
        };
        output
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ReadSettings {
    pub n_drdy: DataReady,
    pub sc: ConversionMode,
    pub dr: DataRate,
    pub pga: Gain,
}

impl From<u8> for ReadSettings {
    fn from(value: u8) -> Self {
        let n_drdy = if (value & 0b1000_0000) == 0 {
            DataReady::FreshData
        } else {
            DataReady::StaleData
        };

        let sc = if (value & 0b0001_0000) == 0 {
            ConversionMode::Continuous
        } else {
            ConversionMode::OneShot
        };

        let dr = match value & 0b0000_1100 {
            0b0000_0000 => DataRate::Sps240,
            0b0000_0100 => DataRate::Sps60,
            0b0000_1000 => DataRate::Sps30,
            0b0000_1100 => DataRate::Sps15,
            _ => unreachable!(),
        };

        let pga = match value & 0b0000_0011 {
            0b0000_0000 => Gain::X1,
            0b0000_0001 => Gain::X2,
            0b0000_0010 => Gain::X4,
            0b0000_0011 => Gain::X8,
            _ => unreachable!(),
        };

        Self {
            n_drdy,
            sc,
            dr,
            pga,
        }
    }
}
