//! Interface for configuring the Memory Protection Unit.

use returncode::ReturnCode;
use platform::mpu::Permission::NoAccess;

#[derive(Copy, Clone)]
pub enum Permission {
    //                      Privileged  Unprivileged
    //                      Access      Access
    NoAccess = 0b00,        // --         --
    PrivilegedOnly = 0b01,  // V          --
    Full = 0b10,            // V          V
}
    
#[derive(Copy, Clone)]
pub struct Region {
    pub start: usize,
    pub len: usize,
    pub read: Permission,
    pub write: Permission,
    pub execute: Permission,
}

impl Region {
    pub fn empty() -> Region {
        Region {
            start: 0,
            len: 0,
            read: NoAccess,
            write: NoAccess,
            execute: NoAccess,
        }
    }
}

pub trait MPU {
    /// Enable the MPU.
    fn enable_mpu(&self);

    /// Completely disable the MPU.
    fn disable_mpu(&self);

    /// Creates a new MPU-specific memory protection region
    ///
    /// `region_num`: an MPU region number
    /// `new_region`: region to be created
    /// `regions`   : array of option-wrapped regions 
    /// `overwrite` : whether the permissions of this region should take
    ///               precedence in the event of overlap with an existing
    ///               region.
    fn create_region(
        &self,
        region_num: usize,
        new_region: Region,
        regions: &mut [Option<Region>],
        overwrite: bool
    ) -> ReturnCode;
}

/// Noop implementation of MPU trait
impl MPU for () {
    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn create_region(
        &self,
        _: usize,
        _: Region,
        _: &mut [Option<Region>],
        _: bool,
    ) -> ReturnCode {
        ReturnCode::SUCCESS 
    } 
}
