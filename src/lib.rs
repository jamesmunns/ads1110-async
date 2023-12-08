//! `ads1110-async`
//!
//! A very basic driver for the ADS1110 family from TI.
//!
//! Consider using the [`ads1x1x`](https://docs.rs/ads1x1x) crate if you need blocking
//! or more full-featured support for this family.

#![cfg_attr(not(test), no_std)]

use config::{
    Address, ConversionMode, DataRate, DataReady, Gain, ReadSettings, Start, WriteSettings,
};
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;
pub mod config;

/// Driver error type
#[derive(Debug, PartialEq)]
pub enum Error<I: I2c> {
    /// A timeout occurred while waiting to read the ADC
    Timeout,
    /// An error with the underlying I2C bus
    I2c(I::Error),
}

/// Async driver for the ADS1110 ADC
pub struct Ads1110<I> {
    addr: u8,
    i2c: I,
    sc: ConversionMode,
    dr: DataRate,
    pga: Gain,
}

/// Get all data, including sample value and configuration
async fn get_all<I: I2c>(i2c: &mut I, addr: u8) -> Result<[u8; 3], I::Error> {
    let mut buf = [0u8; 3];
    i2c.read(addr, &mut buf).await?;
    Ok(buf)
}

impl<I> Ads1110<I>
where
    I: I2c,
{
    /// Create a new [Ads1110] with the given [Address] and [I2c] implementation
    ///
    /// This will attempt to read the settings of the device, and return an error
    /// if obtaining the settings failed.
    pub async fn new(mut i2c: I, addr: Address) -> Result<Self, (I, I::Error)> {
        let addru8 = addr.into_addr();
        let [_data_hi, _data_lo, config] = match get_all(&mut i2c, addru8).await {
            Ok(d) => d,
            Err(e) => return Err((i2c, e)),
        };
        let ReadSettings {
            n_drdy: _,
            sc,
            dr,
            pga,
        } = ReadSettings::from(config);

        Ok(Self {
            addr: addru8,
            i2c,
            sc,
            dr,
            pga,
        })
    }

    /// Write the given settings to the device, updating our stored
    /// values if the write succeeded
    pub async fn write_settings(&mut self, settings: WriteSettings) -> Result<(), I::Error> {
        let WriteSettings {
            start: _,
            sc,
            dr,
            pga,
        } = settings;
        let settings_u8 = settings.to_value();
        self.i2c.write(self.addr, &[settings_u8]).await?;

        // If we succeeded, update values
        self.sc = sc;
        self.dr = dr;
        self.pga = pga;

        Ok(())
    }

    /// Attempts to get a raw value from the ADC.
    ///
    /// If the ADC is configured in "OneShot" mode, a conversion will be started, and this
    /// function will automatically sleep for the expected conversion time.
    ///
    /// This function will wait up to 5/4 of a typical sampling interval to obtain data
    /// reported as "Fresh", or previously unread.
    ///
    /// For example, at 15sps, we should receive a sample every 66.7ms. We will wait up to
    /// 83.3ms (5/4) to receive a fresh sample.
    ///
    /// Also note that the raw value is dependent on the sample rate:
    ///
    /// | Data Rate     | Number of Bits    | Min Value     | Max Value     |
    /// | ---:          | :--:              | ---:          | ---:          |
    /// | 15sps         | 16                | -32,768       | 32,767        |
    /// | 30sps         | 15                | -16,384       | 16,383        |
    /// | 60sps         | 14                | -8,192        | 8,191         |
    /// | 240sps        | 12                | -2,048        | 2,047         |
    ///
    /// This function does not consider `gain`, and returns only raw ADC counts
    pub async fn read_value_raw(&mut self) -> Result<i16, Error<I>> {
        let mut quarter_waits = 0;

        if let ConversionMode::OneShot = self.sc {
            // If we are in oneshot mode, start a conversion and wait
            // an appropriate amount of time
            let write = WriteSettings {
                start: Start::StartConversion,
                sc: self.sc,
                dr: self.dr,
                pga: self.pga,
            };
            let write = write.to_value();
            self.i2c
                .write(self.addr, &[write])
                .await
                .map_err(Error::I2c)?;
            let period = self.dr.interval();

            // Don't waste effort polling if we know it will take
            // a whole interval to finish a conversion.
            Timer::after(period).await;
            quarter_waits = 4;
        }

        // Wait up to 5/4 of a period
        let qperiod = self.dr.quarter_interval();
        loop {
            let [data_hi, data_lo, config] = get_all(&mut self.i2c, self.addr)
                .await
                .map_err(Error::I2c)?;

            let read = ReadSettings::from(config);
            if let DataReady::FreshData = read.n_drdy {
                return Ok(i16::from_be_bytes([data_hi, data_lo]));
            }
            if quarter_waits >= 5 {
                return Err(Error::Timeout);
            }
            quarter_waits += 1;
            Timer::after(qperiod).await;
        }
    }

    /// Attempt to read the ADC, with the contents extended to the full `i16` range.
    ///
    /// This compensates for the fact that different sample rates have different min/max
    /// values. See [Self::read_value_raw] for more details.
    ///
    /// This function does not consider `gain`, and returns only ADC counts
    pub async fn read_value_normalized(&mut self) -> Result<i16, Error<I>> {
        let raw = self.read_value_raw().await?;
        Ok(match self.dr {
            // 15sps has full 16-bit range already
            DataRate::Sps15 => raw,
            // 30sps is 15-bits
            DataRate::Sps30 => raw << 1,
            // 60sps is 14-bits
            DataRate::Sps60 => raw << 2,
            // 240sps is 12-bits
            DataRate::Sps240 => raw << 4,
        })
    }

    /// Give back the I2C bus
    pub fn release(self) -> I {
        self.i2c
    }
}
