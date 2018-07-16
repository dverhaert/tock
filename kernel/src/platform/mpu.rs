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
    FlexibleWithBound(usize),
}

#[derive(Copy, Clone)]
pub struct Region {
    start: usize,
    end: usize,
    start_boundary: Boundary,
    end_boundary: Boundary,
    read: Permission,
    write: Permission,
    execute: Permission,
}

impl Region {
    pub fn new(
        start: usize,
        end: usize,
        start_boundary: Boundary,
        end_boundary: Boundary,
        read: Permission,
        write: Permission,
        execute: Permission,
    ) -> Region {
        Region {
            start: start,
            end: end,
            start_boundary: start_boundary,
            end_boundary: end_boundary,
            read: read,
            write: write,
            execute: execute,
        }
    }

    pub fn empty() -> Region {
        Region {
            start: 0,
            end: 0,
            start_boundary: Boundary::Fixed,
            end_boundary: Boundary::Fixed,
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

    pub fn set_start(&mut self, start: usize) {
        self.start = start;
    }
    
    pub fn set_end(&mut self, end: usize) {
        self.end = end;
    }
}

pub trait MPU {
    type MpuState;

    /// Enables the MPU.
    fn enable_mpu(&self);

    /// Disables the MPU.
    fn disable_mpu(&self);

    /// Returns the number of supported MPU regions.
    fn num_supported_regions(&self) -> u32;

    /// Allocates a set of logical regions in the MPU.
    ///
    /// # Arguments
    ///
    /// `regions`   : an array of disjoint logical regions.
    ///
    /// # Return Value 
    ///
    /// Returns MPU configuration data implementing the requested regions. 
    /// If it is infeasible to allocate a memory region, returns its index.
    fn allocate_regions(
        regions: &mut [Region],
    ) -> Result<Self::MpuState, usize>;

    /// Configures memory protection regions in the MPU.
    ///
    /// # Arguments
    ///
    /// `state`: state used to set regions.
    fn configure_mpu(&self, state: &Self::MpuState);
}

/// No-op implementation of MPU trait
impl MPU for () {
    type MpuState = ();   

    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn num_supported_regions(&self) -> u32 {
        8
    }

    fn allocate_regions(
        _: &mut [Region],
    ) -> Result<Self::MpuState, usize> {
        Ok(())
    }

    fn configure_mpu(&self, _: &Self::MpuState) {}
}
