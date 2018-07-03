//! Interface for configuring the Memory Protection Unit.

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
    start: usize,
    len: usize,
    read: Permission,
    write: Permission,
    execute: Permission,
}

impl Region {
    pub fn new(
        start: usize,
        len: usize,
        read: Permission,
        write: Permission,
        execute: Permission
    ) -> Region {
        Region {
            start: start,
            len: len,
            read: read,
            write: write,
            execute: execute,
        }
    }

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
    /// Enables the MPU.
    fn enable_mpu(&self);

    /// Disables the MPU.
    fn disable_mpu(&self);

    /// Allocates memory protection regions.
    ///
    /// `regions`: array of regions to be allocated. The index of the array
    ///            encodes the priority of the region. In the event of an 
    ///            overlap between regions, the implementor must ensure 
    ///            that the permissions of the region with higher priority
    ///            take precendence.
    fn set_regions(&self, regions: &[Option<Region>]) -> Result<(), &'static str>;
}

/// No-op implementation of MPU trait
impl MPU for () {
    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn set_regions(&self, _: &[Option<Region>]) -> Result<(), &'static str> { Ok(()) }
}
