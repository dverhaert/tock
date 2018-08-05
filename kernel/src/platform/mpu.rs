//! Interface for configuring the Memory Protection Unit.

#[derive(Copy, Clone)]
pub enum Permissions {
    ReadWriteExecute,
    ReadWriteOnly,
    ReadExecuteOnly,
    ReadOnly,
    ExecuteOnly,
    NoAccess,
}

pub trait MPU {
    type MpuConfig: Default;

    /// Enables the MPU.
    fn enable_mpu(&self);

    /// Disables the MPU.
    fn disable_mpu(&self);

    /// Returns the total number of regions supported by the MPU.
    fn number_total_regions(&self) -> usize;
    
    /// Chooses the location for a process's memory, and sets up
    /// an MPU region to expose the process-owned portion.
    ///
    /// The implementor must allocate a memory region for the process that is at 
    /// least `min_process_ram_size` bytes in size and lies completely within the 
    /// specified bounds. It must also set up an MPU region exposing at least 
    /// `initial_pam_size` bytes at the beginning of this parent memory region, 
    /// with the specified permissions, and store the region in the `config` variable. 
    /// This MPU region must not reach the point `initial_grant_size` bytes before
    /// the end of the total memory region.
    ///
    /// This function should only be called once during the lifetime of a process. If the
    /// function has been called previously, then the implementor must return None.
    ///
    /// # Arguments
    ///
    /// `lower_bound`           : lower bound for allocating process memory
    /// `upper_bound`           : upper bound for allocating process memory
    /// `min_process_ram_size`  : minimum ram size to allocate for process
    /// `initial_pam_size`      : intial size for the process acessible memory
    /// `initial_grant_size`    : initial size for the process grant.
    /// `permissions`           : permissions for process accessible memory region
    /// `config`                : structure to store MPU configuration 
    ///
    /// # Return Value
    ///
    /// This function returns the start address and the size of the memory 
    /// allocated for the process. If it is infeasible to allocate the memory or the MPU
    /// region, or if the function has already been called, returns None.
    fn setup_process_memory_layout(
        &self, 
        lower_bound: *const u8,
        upper_bound: *const u8,
        min_process_ram_size: usize,
        initial_pam_size: usize,
        initial_grant_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig
    ) -> Option<(*const u8, usize)>;

    /// Updates the MPU region for process accesible memory to reflect a changed location
    /// of the app memory and kernel memory breaks.
    ///
    /// # Arguments
    /// 
    /// `app_memory_break`          : address for the end of process accessible memory 
    /// `kernel_memory_break`       : address for the start of grant memory
    /// `config`                    : configuration data for the MPU
    ///
    /// # Return Value
    /// 
    /// Returns an error if it is infeasible to update the PAM MPU region, or if it was
    /// never created.
    fn update_process_memory_layout(
        &self,
        app_memory_break: *const u8,
        kernel_memory_break: *const u8,
        config: &mut Self::MpuConfig
    ) -> Result<(), ()>;

    /// Adds new MPU region for an arbitrarily-located buffer.
    ///
    /// The implementor must create an MPU region at least `min_buffer_size`
    /// in size within the specified bounds and with the specified permissions,
    /// and store it within `config`.
    ///
    /// # Arguments
    ///
    /// `lower_bound`       : lower bound address for the buffer 
    /// `upper_bound`       : upper bound address for the buffer 
    /// `min_buffer_size`   : minimum size of the buffer
    /// `permissions`       : permissions for the MPU region
    /// `config`            : structure to store MPU configuration
    ///
    /// # Return Value
    ///
    /// Returns the region.
    fn expose_memory_buffer(
        &self,
        lower_bound: *const u8,
        upper_bound: *const u8,
        min_buffer_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig
    ) -> Option<(*const u8, usize)>;

    /// Configures the MPU with the provided region configuration.
    ///
    /// # Arguments
    ///
    /// `config`: region configuration.
    fn configure_mpu(&self, config: &Self::MpuConfig);
}

/// Default implementation of MPU trait
impl MPU for () {
    type MpuConfig = ();

    fn enable_mpu(&self) {}

    fn disable_mpu(&self) {}

    fn number_total_regions(&self) -> usize {
        8
    }

    fn setup_process_memory_layout(
        &self, 
        lower_bound: *const u8,
        upper_bound: *const u8,
        min_app_ram_size: usize,
        _: usize,
        _: usize,
        _: Permissions,
        _: &mut Self::MpuConfig
    ) -> Option<(*const u8, usize)> {
        let available_memory = (upper_bound as usize) - (lower_bound as usize);
        if available_memory < min_app_ram_size {
            None
        } else {
            Some((lower_bound, min_app_ram_size))
        }
    }

    fn update_process_memory_layout(
        &self,
        _: *const u8,
        _: *const u8,
        _: Permissions,
        _: &mut Self::MpuConfig
    ) -> Result<(), ()> {
        Ok(())
    }

    fn expose_memory_buffer(
        &self,
        lower_bound: *const u8,
        upper_bound: *const u8,
        minimum_buffer_size: usize,
        _: Permissions,
        _: &mut Self::MpuConfig
    ) -> Option<(*const u8, usize)> {
        let available_memory = (upper_bound as usize) - (lower_bound as usize);
        if available_memory < minimum_buffer_size {
            None
        } else {
            Some((lower_bound, minimum_buffer_size))
        }
    }

    fn configure_mpu(&self, _: &Self::MpuConfig) {}
}
