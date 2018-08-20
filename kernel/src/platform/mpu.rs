//! Interface for configuring the Memory Protection Unit.

/// User mode permission levels for accesses to a memory region.
#[derive(Copy, Clone)]
pub enum Permissions {
    ReadWriteExecute,
    ReadWriteOnly,
    ReadExecuteOnly,
    ReadOnly,
    ExecuteOnly,
}

pub trait MPU {
    type MpuConfig: Default = ();

    /// Enables the MPU.
    fn enable_mpu(&self) {}

    /// Disables the MPU.
    fn disable_mpu(&self) {}

    /// Returns the total number of regions supported by the MPU.
    fn number_total_regions(&self) -> usize {
        8
    }

    /// Chooses the location for a process's memory, and allocates an MPU region
    /// covering the app-owned portion.
    ///
    /// An implementation must allocate a contiguous block of memory that is at
    /// least `min_memory_size` bytes in size and lies completely within the
    /// specified parent region. 
    ///
    /// It must also allocate an MPU region with the following properties:
    ///
    /// 1.  The region covers at least the first `initial_app_memory_size` bytes at the
    ///     beginning of the memory block.
    /// 2.  The region does not intersect with the last `initial_kernel_memory_size`
    ///     bytes.
    /// 3.  The region has the permissions specified by `permissions`.
    ///
    /// The end of app memory will likely increase in the future, so the implementation 
    /// should choose the location of the process memory block such that it is possible 
    /// for the MPU region covering app memory to grow along with it.
    /// The implementation must store state for the allocated region in the `config` 
    /// variable.
    ///
    /// # Arguments
    ///
    /// `parent_start`              : start of the parent region
    /// `parent_size`               : size of the parent region
    /// `min_memory_size`           : minimum total memory to allocate for process
    /// `initial_app_memory_size`   : initial size for app memory
    /// `initial_kernel_memory_size`: initial size for kernel memory 
    /// `permissions`               : permissions for the MPU region
    /// `config`                    : MPU region configuration
    ///
    /// # Return Value
    ///
    /// This function returns the start address and the size of the memory block
    /// allocated for the process. If it is infeasible to allocate the block or 
    /// the MPU region, or if the function has already been called, returns None.
    #[allow(unused_variables)]
    fn allocate_app_memory_region(
        &self,
        parent_start: *const u8,
        parent_size: usize,
        min_memory_size: usize,
        initial_app_memory_size: usize,
        initial_kernel_memory_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig,
    ) -> Option<(*const u8, usize)> {
        let memory_size = {
            if min_memory_size < initial_app_memory_size + initial_kernel_memory_size {
                initial_app_memory_size + initial_kernel_memory_size
            } else {
                min_memory_size
            }
        };
        if memory_size > parent_size {
            None
        } else {
            Some((parent_start, memory_size))
        }
    }

    /// Updates the MPU region for app memory to reflect any changes in the
    /// position of the app memory break and/or kernel memory break.
    ///
    /// The implementor must reallocate the app memory MPU region stored in `config` 
    /// to maintain the 3 conditions described in `setup_process_memory_layout`.
    ///
    /// # Arguments
    ///
    /// `app_memory_break`      : new address for the end of app memory 
    /// `kernel_memory_break`   : new address for the start of kernel memory 
    /// `config`                : MPU region configuration
    ///
    /// # Return Value
    ///
    /// Returns an error if it is infeasible to update the MPU region, or if it was
    /// never created.
    #[allow(unused_variables)]
    fn update_app_memory_region(
        &self,
        app_memory_break: *const u8,
        kernel_memory_break: *const u8,
        config: &mut Self::MpuConfig,
    ) -> Result<(), ()> {
        if (app_memory_break as usize) > (kernel_memory_break as usize) {
            Err(())
        } else {
            Ok(())
        }
    }

    /// Allocates a new MPU region.
    ///
    /// The implementor must create an MPU region at least `min_region_size`
    /// in size within the specified parent region, with the specified permissions,
    /// and store it within `config`.
    ///
    /// # Arguments
    ///
    /// `parent_start`      : start of the parent region
    /// `parent_size`       : size of the parent region
    /// `min_region_size`   : minimum size of the region
    /// `permissions`       : permissions for the MPU region
    /// `config`            : MPU region configuration
    ///
    /// # Return Value
    ///
    /// Returns the region exposed by the MPU region. If it is infeasible to allocate 
    /// the MPU region, returns None.
    #[allow(unused_variables)]
    fn allocate_region(
        &self,
        parent_start: *const u8,
        parent_size: usize,
        min_region_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig,
    ) -> Option<(*const u8, usize)> {
        if min_region_size > parent_size {
            None
        } else {
            Some((parent_start, min_region_size))
        }
    }

    /// Configures the MPU with the provided region configuration.
    ///
    /// # Arguments
    ///
    /// `config`    : MPU region configuration
    #[allow(unused_variables)]
    fn configure_mpu(&self, config: &Self::MpuConfig) {}
}

/// Implement default MPU trait for unit.
impl MPU for () {}
