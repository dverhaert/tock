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
pub enum Boundary {
    Fixed,
    Flexible(usize),
    Relative(usize),
}

#[derive(Copy, Clone)]
pub struct Region {
    start_address: usize,
    end_address: usize,
    start_boundary: Boundary,
    end_boundary: Boundary,
    read: Permission,
    write: Permission,
    execute: Permission,
}

impl Region {
    pub fn new(
        start_address: usize,
        end_address: usize,
        start_boundary: Boundary,
        end_boundary: Boundary,
        read: Permission,
        write: Permission,
        execute: Permission,
    ) -> Region {
        Region {
            start_address,
            end_address,
            start_boundary,
            end_boundary,
            read,
            write,
            execute,
        }
    }

    pub fn empty() -> Region {
        Region {
            start_address: 0,
            end_address: 0,
            start_boundary: Boundary::Fixed,
            end_boundary: Boundary::Fixed,
            read: Permission::NoAccess,
            write: Permission::NoAccess,
            execute: Permission::NoAccess,
        }
    }

    pub fn get_start_address(&self) -> usize {
        self.start_address
    }

    pub fn get_end_address(&self) -> usize {
        self.end_address
    }

    pub fn get_start_boundary(&self) -> Boundary {
        self.start_boundary
    }

    pub fn get_end_boundary(&self) -> Boundary {
        self.end_boundary
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

    pub fn set_start_address(&mut self, start_address: usize) {
        self.start_address = start_address;
    }

    pub fn set_end_address(&mut self, end_address: usize) {
        self.end_address = end_address;
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
