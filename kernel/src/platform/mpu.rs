//! Interface for configuring the Memory Protection Unit.

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
    fn number_total_regions(&self) -> usize {}

    /// Chooses the location for a process's memory, and sets up
    /// an MPU region to expose the process-owned portion.
    ///
    /// The implementor must allocate a memory region for the process that is at
    /// least `min_process_ram_size` bytes in size and lies completely within the
    /// specified parent region. It must also allocate an MPU region covering the first
    /// `initial_pam_size` bytes at the beginning of this parent memory region,
    /// with the specified permissions, and store the region in the `config` variable.
    /// This MPU region must not intersect the last `initial_grant_size` bytes of the
    /// total memory region.
    ///
    /// # Arguments
    ///
    /// `parent_start`          : start of the parent region
    /// `parent_size`           : size of the parent region
    /// `min_app_ram_size`      : minimum ram size to allocate for process
    /// `initial_pam_size`      : initial size for the PAM (process accessible memory)
    /// `initial_grant_size`    : initial size for the process grant
    /// `permissions`           : permissions for the PAM MPU region
    /// `config`                : MPU region configuration
    ///
    /// # Return Value
    ///
    /// This function returns the start address and the size of the memory
    /// allocated for the process. If it is infeasible to allocate the memory or the MPU
    /// region, or if the function has been previously called, returns None.
    #[allow(unused_variables)]
    fn setup_process_memory_layout(
        &self,
        parent_start: *const u8,
        parent_size: usize,
        min_app_ram_size: usize,
        initial_pam_size: usize,
        initial_grant_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig,
    ) -> Option<(*const u8, usize)> {
        let app_ram_size = if min_app_ram_size < initial_pam_size + initial_grant_size {
            initial_pam_size + initial_grant_size
        } else {
            min_app_ram_size
        };
        if min_app_ram_size > parent_size {
            None
        } else {
            Some((parent_start, app_ram_size))
        }
    }

    /// Updates the MPU region for PAM to reflect any change in the app memory
    /// and/or kernel memory breaks.
    ///
    /// The implementor must update the PAM MPU region stored in `config` to extend
    /// past `app_memory_break`, but not past `kernel_memory_break`.
    ///
    /// # Arguments
    ///
    /// `app_memory_break`      : new address for the end of PAM
    /// `kernel_memory_break`   : new address for the start of grant
    /// `config`                : MPU region configuration
    ///
    /// # Return Value
    ///
    /// Returns an error if it is infeasible to update the PAM MPU region, or if it was
    /// never created.
    #[allow(unused_variables)]
    fn update_process_memory_layout(
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

    /// Adds a new MPU region exposing a region in memory.
    ///
    /// The implementor must create an MPU region at least `min_region_size`
    /// in size within the specified parent region, with the specified permissions,
    /// and store it within `config`.
    ///
    /// # Arguments
    ///
    /// `lower_bound`       : lower bound address for the region
    /// `upper_bound`       : upper bound address for the region
    /// `min_region_size`   : minimum size of the region
    /// `permissions`       : permissions for the MPU region
    /// `config`            : MPU region configuration
    ///
    /// # Return Value
    ///
    /// Returns the MPU region allocated. If it is infeasible to allocate the
    /// region, returns None.
    #[allow(unused_variables)]
    fn expose_memory_region(
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
