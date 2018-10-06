//! Interface for configuring the Memory Protection Unit.

/// Access permissions.
#[derive(Copy, Clone)]
pub enum Permission {
    //                 Supervisor  User
    //                 Access      Access
    NoAccess,       // --          --
    SupervisorOnly, // V           --
    Full,           // V           V
}
/// MPU region type.
#[derive(Copy, Clone)]
pub enum RegionType {
    // Absolute region, anchored to start and end
    Absolute {
        start: usize,
        end: usize,
        start_flexibility: usize,
        end_flexibility: usize,
    },

    /// Relative region, flexible in alignment but having a minimum size
    Relative {
        lower_bound: usize,
        upper_bound: usize,
        min_offset: usize,
        min_region_size: usize,
    },
}

/// MPU region.
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

pub trait MPU {
    type MpuConfig: Default = ();

    /// Enables the MPU.
    fn enable_mpu(&self) {}

    /// Disables the MPU.
    fn disable_mpu(&self) {}

    /// Returns the total number of regions supported by the MPU.
    fn number_total_regions(&self) -> usize {
        0
    }

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
    fn allocate_regions(regions: &mut [Region]) -> Result<Self::MpuConfig, usize> {}

    /// Configures the MPU with the provided region configuration.
    ///
    /// An implementation must ensure that all memory locations not covered by
    /// an allocated region are inaccessible in user mode and accessible in
    /// supervisor mode.
    ///
    /// # Arguments
    ///
    /// `config`    : MPU region configuration
    #[allow(unused_variables)]
    fn configure_mpu(&self, config: &Self::MpuConfig) {}
}

/// Implement default MPU trait for unit.
impl MPU for () {}
