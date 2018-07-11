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

    pub fn set_start(&self, start: usize) {
        self.start = start;
    }
    
    pub fn set_end(&self, end: usize) {
        self.end = end;
    }
}

#[derive(Copy, Clone)]
pub struct Boundary {
    start_fixed: bool,
    end_fixed: bool,
}

impl Boundary {
    pub fn new(
        start_fixed: bool, 
        end_fixed: bool
    ) -> Boundary {
        Boundary{
            start_fixed: start_fixed,
            end_fixed: end_fixed,
        }
    }

    pub fn start_fixed(&self) -> bool {
        return self.start_fixed;
    }
    
    pub fn end_fixed(&self) -> bool {
        return self.end_fixed;
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
    /// `boundaries`: an array of region boundary parameters. The parameters 
    ///               at each index specify whether the region at that index
    ///               in `regions` has fixed start and end addresses that
    ///               must be respected by the MPU, or whether the MPU is 
    ///               allowed to extend them downward or upward respectively. 
    ///               The size of this array must equal that of `regions`.
    /// `state`     : MPU state. The MPU writes configuration data
    ///               to this field implementing the client's requested 
    ///               regions.
    ///
    /// # Return Value 
    ///
    /// If it is infeasible to allocate a memory region, returns its index
    /// wrapped in a Result.
    fn allocate_regions(
        regions: &mut [Region],
        boundaries: &[Boundary],
        state: &mut Self::MpuState,
    ) -> Result<(), usize>; 

    /// Configures memory protection regions in the MPU.
    ///
    /// # Arguments
    ///
    /// `state`: state used to set regions.
    fn configure_mpu(&self, state: &Self::MpuState);
}

/// No-op implementation of MPU trait
impl MPU for () {
    type MpuState = [Region; 8];   

    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn num_supported_regions(&self) -> u32 {
        8
    }

    fn allocate_regions(
        _: &mut [Region],
        _: &[Boundary],
        _: &mut Self::MpuState,
    ) -> Result<(), usize> {
        Ok(())
    }

    fn configure_mpu(&self, _: &Self::MpuState) {}
}
