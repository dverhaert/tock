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

    pub fn set_start(&mut self, start: usize) {
        self.start = start;
    }
    
    pub fn set_end(&mut self, end: usize) {
        self.end = end;
    }
}

#[derive(Copy, Clone)]
pub struct Boundary {
    lower_bound: Option<usize>,
    upper_bound: Option<usize>,
}

impl Boundary {
    pub fn new(
        lower_bound: Option<usize>,
        upper_bound: Option<usize>,
    ) -> Boundary {
        Boundary{
            lower_bound: lower_bound,
            upper_bound: upper_bound,
        }
    }

    pub fn lower_bound(&self) -> Option<usize> {
        return self.lower_bound;
    }
    
    pub fn upper_bound(&self) -> Option<usize> {
        return self.upper_bound;
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
        _: &[Boundary],
    ) -> Result<Self::MpuState, usize> {
        Ok(())
    }

    fn configure_mpu(&self, _: &Self::MpuState) {}
}
