//! FMC timing

use super::pac::fmc;

/// Memory access modes
///
/// These define the general shape of a transaction and the meanings of some of the time fields.
/// Refer to the microcontroller reference manual for more details.
#[derive(Debug, Clone)]
pub enum AccessMode {
    ModeA,
    ModeB,
    ModeC,
    ModeD,
}

impl AccessMode {
    pub(crate) fn as_read_variant(&self) -> fmc::btr::ACCMOD_A {
        use fmc::btr::ACCMOD_A;
        match *self {
            AccessMode::ModeA => ACCMOD_A::A,
            AccessMode::ModeB => ACCMOD_A::B,
            AccessMode::ModeC => ACCMOD_A::C,
            AccessMode::ModeD => ACCMOD_A::D,
        }
    }
    pub(crate) fn as_write_variant(&self) -> fmc::bwtr::ACCMOD_A {
        use fmc::bwtr::ACCMOD_A;
        match *self {
            AccessMode::ModeA => ACCMOD_A::A,
            AccessMode::ModeB => ACCMOD_A::B,
            AccessMode::ModeC => ACCMOD_A::C,
            AccessMode::ModeD => ACCMOD_A::D,
        }
    }
}

/// Timing configuration for reading or writing
///
/// A `Timing` object can be created using `Timing::default()` or `Default::default()`.
///
/// The default timing uses access mode C and the slowest possible timings, for maximum
/// compatibility.
///
/// If the LCD controller and wiring allow, you can reduce the times to make transactions faster.
///
/// All time fields are in units of HCLK cycles.
#[derive(Debug, Clone)]
pub struct Timing {
    pub(crate) access_mode: AccessMode,
    pub(crate) bus_turnaround: u8,
    pub(crate) data: u8,
    pub(crate) address_hold: u8,
    pub(crate) address_setup: u8,
}

impl Default for Timing {
    /// Returns a conservative (slow) timing configuration with access mode C
    fn default() -> Self {
        Timing {
            access_mode: AccessMode::ModeC,
            bus_turnaround: Timing::BUS_TURNAROUND_MAX,
            data: 255,
            address_hold: Timing::ADDRESS_HOLD_MAX,
            address_setup: Timing::ADDRESS_SETUP_MAX,
        }
    }
}

impl Timing {
    /// Maximum allowed value of the bus turnaround time
    pub const BUS_TURNAROUND_MAX: u8 = 15;
    /// Minimum allowed value of the data phase time
    pub const DATA_MIN: u8 = 1;
    /// Maximum allowed value of the address hold time
    pub const ADDRESS_HOLD_MIN: u8 = 1;
    /// Maximum allowed value of the address hold time
    pub const ADDRESS_HOLD_MAX: u8 = 15;
    /// Maximum allowed value of the address setup time
    pub const ADDRESS_SETUP_MAX: u8 = 15;

    /// Sets the access mode
    pub fn access_mode(self, access_mode: AccessMode) -> Self {
        Timing {
            access_mode,
            ..self
        }
    }
    /// Sets the bus turnaround time, in units of HCLK cycles
    ///
    /// This corresponds to the BUSTURN field of FMC_BTR or FMC_BWTR.
    pub fn bus_turnaround(self, bus_turnaround: u8) -> Self {
        Timing {
            bus_turnaround,
            ..self
        }
    }
    /// Sets the data phase time, in units of HCLK cycles
    ///
    /// This corresponds to the DATAST field of FMC_BTR or FMC_BWTR.
    pub fn data(self, data: u8) -> Self {
        Timing { data, ..self }
    }
    /// Sets the address hold phase time, in units of HCLK cycles
    ///
    /// This corresponds to the ADDHLD field of FMC_BTR or FMC_BWTR.
    pub fn address_hold(self, address_hold: u8) -> Self {
        Timing {
            address_hold,
            ..self
        }
    }
    /// Sets the address setup phase time, in units of HCLK cycles
    ///
    /// This corresponds to the ADDSET field of FMC_BTR or FMC_BWTR.
    pub fn address_setup(self, address_setup: u8) -> Self {
        Timing {
            address_setup,
            ..self
        }
    }
}
