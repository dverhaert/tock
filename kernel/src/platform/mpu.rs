//! Interface for configuring the Memory Protection Unit.

use returncode::ReturnCode;

#[derive(Debug)]
pub enum Permission {
    //                      Privileged  Unprivileged
    //                      Access      Access
    NoAccess = 0b00,        // --         --
    PrivilegedOnly = 0b01,  // V          --
    Full = 0b10,            // V          V
}
    
pub struct Region {
    start: usize,
    len: usize,
    read: Permission,
    write: Permission,
    execute: Permission,
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
    /// `regions`   : array of regions 
    /// `overwrite` : whether the permissions of this region should take
    ///               precedence in the event of overlap with an existing
    ///               region.
    fn create_region(
        region_num: usize,
        new_region: Region,
        regions: &mut [Region],
        overwrite: bool
    ) -> ReturnCode;
}

/// Noop implementation of MPU trait
impl MPU for () {
    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn create_region(
        _: usize,
        _: new_region,
        _: &mut [Region],
        _: bool,
    ) -> ReturnCode {
        ReturnCode::SUCCESS 
    } 
}
