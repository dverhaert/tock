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
pub enum RegionType {
    /// Fixed region
    Fixed { 
        start_address: usize, 
        end_address: usize,
    },

    /// End fixed, start can be lower
    LeftGrowing {
        start_address: usize,
        start_flexibility: usize,
        end_address: usize,
    },

    /// Start fixed, end can be higher 
    RightGrowing {
        start_address: usize,
        end_address: usize,
        end_flexibility: usize,
    },

    /// Put as close to end of previous region as possible,
    /// make the length at least `length`, and as small as possible
    Packed {
        length: usize,
    }
}

#[derive(Copy, Clone)]
pub struct Region {
    region_type: RegionType,
    read: Permission,
    write: Permission,
    execute: Permission,
}

impl Region {
    pub fn new(
        region_type: RegionType,
        read: Permission,
        write: Permission,
        execute: Permission,
    ) -> Region {
        Region {
            region_type: region_type,
            read: read,
            write: write,
            execute: execute,
        }
    }

    pub fn get_type(&self) -> RegionType {
        self.region_type
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

    pub fn set_type(&mut self, region_type: RegionType) {
        self.region_type = region_type;
    }
}

impl Default for Region {
    fn default() -> Region {
        Region {
            region_type: RegionType::Fixed { 
                start_address: 0,
                end_address: 0,
            },
            read: Permission::NoAccess,
            write: Permission::NoAccess,
            execute: Permission::NoAccess,
        }
    }
}

pub trait MPU {
    type MpuConfig;

    /// Enables the MPU.
    fn enable_mpu(&self);

    /// Disables the MPU.
    fn disable_mpu(&self);

    /// Returns the number of supported MPU regions.
    fn number_supported_regions(&self) -> u32;

    /// Allocates a set of logical regions in the MPU.
    ///
    /// # Arguments
    ///
    /// `regions`: an array of disjoint logical regions.
    ///
    /// # Return Value
    ///
    /// Returns MPU configuration data implementing the requested regions.
    /// If it is infeasible to allocate a memory region, returns its index.
    fn allocate_regions(regions: &mut [Region]) -> Result<Self::MpuConfig, usize>;

    /// Configures memory protection regions in the MPU.
    ///
    /// # Arguments
    ///
    /// `config`: configuration used to set regions.
    fn configure_mpu(&self, config: &Self::MpuConfig);
}

/// No-op implementation of MPU trait
impl MPU for () {
    type MpuConfig = ();

    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn number_supported_regions(&self) -> u32 {
        8
    }

    fn allocate_regions(_: &mut [Region]) -> Result<Self::MpuConfig, usize> {
        Ok(())
    }

    fn configure_mpu(&self, _: &Self::MpuConfig) {}
}
