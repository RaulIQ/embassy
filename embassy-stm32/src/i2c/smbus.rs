//! SMBus support for the STM32 I2C peripheral.
//!
//! [`Smbus`] wraps an existing [`I2c`] driver and configures the peripheral for SMBus
//! operation by setting the appropriate control-register bits.

use core::ops::{Deref, DerefMut};

use super::*;
use crate::mode::Mode;
use crate::pac::i2c;

/// SMBus role (I2C v1 hardware only).
///
/// On I2C v2/v3, host and device default address matching are controlled independently
/// via [`Config::host_address`] and [`Config::device_default_address`].
#[cfg(i2c_v1)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Role {
    /// SMBus device.
    Device,
    /// SMBus host.
    Host,
}

/// SMBus peripheral configuration.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub struct Config {
    /// Respond to the SMBus host address (`0b00010`, 7-bit `0x08`).
    ///
    /// I2C v2/v3: `SMBHEN`. I2C v1: selects host role when [`Role::Host`] is set.
    pub host_address: bool,
    /// Respond to the SMBus device default address (`0b1100001`, 7-bit `0x61`).
    ///
    /// I2C v2/v3: `SMBDEN`.
    pub device_default_address: bool,
    /// Enable the SMBus alert response protocol.
    pub alert: bool,
    /// Enable packet error checking (PEC).
    pub pec: bool,
    /// SMBus role (I2C v1 only).
    #[cfg(i2c_v1)]
    pub role: Role,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host_address: false,
            device_default_address: false,
            alert: false,
            pec: false,
            #[cfg(i2c_v1)]
            role: Role::Device,
        }
    }
}

/// SMBus wrapper around [`I2c`].
///
/// All I2C transfer APIs remain available via [`Deref`] / [`DerefMut`].
pub struct Smbus<'d, M: Mode, IM: MasterMode> {
    i2c: I2c<'d, M, IM>,
}

impl<'d, M: Mode, IM: MasterMode> Smbus<'d, M, IM> {
    /// Wrap an [`I2c`] instance and configure the peripheral for SMBus.
    pub fn new(i2c: I2c<'d, M, IM>, config: Config) -> Self {
        apply_config(i2c.info, &config);
        Self { i2c }
    }

    /// Consume the wrapper and return the underlying [`I2c`] driver.
    pub fn into_i2c(self) -> I2c<'d, M, IM> {
        self.i2c
    }
}

impl<'d, M: Mode, IM: MasterMode> Deref for Smbus<'d, M, IM> {
    type Target = I2c<'d, M, IM>;

    fn deref(&self) -> &Self::Target {
        &self.i2c
    }
}

impl<'d, M: Mode, IM: MasterMode> DerefMut for Smbus<'d, M, IM> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.i2c
    }
}

fn apply_config(info: &'static Info, config: &Config) {
    let pe = info.regs.cr1().read().pe();

    info.regs.cr1().modify(|reg| reg.set_pe(false));

    #[cfg(i2c_v1)]
    {
        use i2c::vals::{Smbtype, Smbus as SmbusMode};

        info.regs.cr1().modify(|reg| {
            reg.set_smbus(SmbusMode::SmBus);
            reg.set_smbtype(match config.role {
                Role::Device => Smbtype::Device,
                Role::Host => Smbtype::Host,
            });
            reg.set_enpec(config.pec);
            reg.set_alert(config.alert);
        });
    }

    #[cfg(any(i2c_v2, i2c_v3))]
    {
        info.regs.cr1().modify(|reg| {
            reg.set_smbhen(config.host_address);
            reg.set_smbden(config.device_default_address);
            reg.set_alerten(config.alert);
            reg.set_pecen(config.pec);
        });
    }

    if pe {
        info.regs.cr1().modify(|reg| reg.set_pe(true));
    }
}
