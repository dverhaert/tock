//! Interface for configuring the Memory Protection Unit.

#[derive(Copy, Clone)]
pub enum Permission {
    //                 Privileged  Unprivileged
    //                 Access      Access
    NoAccess,       // --          --
    PrivilegedOnly, // V           --
    Full,           // V           V
}

#[derive(Copy, Clone)]
pub struct Region {
    start: usize,
    end: usize,
    read: Permission,
    write: Permission,
    execute: Permission,
}

impl Region {
    pub fn new(
        start: usize,
        end: usize,
        read: Permission,
        write: Permission,
        execute: Permission,
    ) -> Region {
        Region {
            start: start,
            end: end,
            read: read,
            write: write,
            execute: execute,
        }
    }

    pub fn empty() -> Region {
        Region {
            start: 0,
            end: 0,
            read: Permission::NoAccess,
            write: Permission::NoAccess,
            execute: Permission::NoAccess,
        }
    }

    pub fn get_start(&self) -> usize {
        self.start
    }

    pub fn get_end(&self) -> usize {
        self.end
    }

    pub fn get_read_permission(&self) -> Permission {
        self.read
    }

    pub fn get_write_permission(&self) -> Permission {
        self.write
    }

    pub fn get_execute_permission(&self) -> Permission {
        self.execute
    }
}

pub trait MPU {
    /// Enables the MPU.
    fn enable_mpu(&self);

    /// Disables the MPU.
    fn disable_mpu(&self);

    /// Returns the number of supported MPU regions.
    fn num_supported_regions(&self) -> u32;

    /// Requests approval from the MPU for a new region.
    ///
    /// # Arguments
    ///
    /// `start`      : the base address of the region
    /// `end`        : the end address of the region
    /// `start_fixed`: whether the MPU can adjust the start address or not
    /// `end_fixed`  : whether the MPU can adjust the end address or not
    /// `read`       : read permission of the region
    /// `write`      : write permission of the region
    /// `execute`    : execute permission of the region.
    /// `existing`   : regions that have previously been approved by the MPU.
    ///                The implementor must ensure that the new region does
    ///                not overlap with any of the previous regions.
    ///
    /// # Return Value 
    ///
    /// The function returns a Region struct approved by the MPU, which may
    /// have a different start or end address than requested if the client
    /// specified that these addresses are not fixed. If the request was not
    /// feasible, returns None.
    fn request_region(
        start: usize,
        end: usize,
        start_fixed: bool,
        end_fixed: bool,
        read: Permission,
        write: Permission,
        execute: Permission,
        existing: &[Region],
    ) -> Option<Region>; 

    /// Sets memory protection regions in the MPU.
    ///
    /// # Arguments
    ///
    /// `regions`: array of regions to be allocated
    fn set_regions(&self, regions: &[Region]);
}

/// No-op implementation of MPU trait
impl MPU for () {
    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn num_supported_regions(&self) -> u32 {
        8
    }

    fn request_region(
        _: usize,
        _: usize,
        _: bool,
        _: bool,
        _: Permission,
        _: Permission,
        _: Permission,
        _: &[Region],
    ) -> Option<Region> {
        Some(Region::empty())
    }

    fn set_regions(&self, _: &[Region]) {}
}
