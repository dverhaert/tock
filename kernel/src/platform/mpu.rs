//! Interface for configuring the Memory Protection Unit.

use returncode::ReturnCode;

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

    /// Returns the number of MPU regions still available.
    fn number_available_regions(&self) -> usize;
    
    /// Resets MPU region configuration.
    fn reset_mpu_config(&self, config: Self::MpuConfig);

    /// Sets up MPU regions for process RAM.
    ///
    /// # Arguments
    ///
    /// `lower_bound`           : lower bound for allocating process memory
    /// `upper_bound`           : upper bound for allocating process memory
    /// `min_process_ram_size`  : minimum ram size to allocate for process
    /// `initial_pam_size`      : intial size for the process acessible memory
    /// `initial_grant_size`    : initial size for the process grant.
    /// `config`                : configuration data for the MPU
    ///
    /// # Return Value
    ///
    /// This function returns the start address and the size of the memory 
    /// allocated for the process.
    fn setup_ram_mpu_regions(
        &self, 
        lower_bound: *const u8,
        upper_bound: *const u8,
        min_process_ram_size: usize,
        initial_pam_size: usize,
        initial_grant_size: usize,
        config: Self::MpuConfig
    ) -> Option<(*const u8, usize)>;

    /// Updates MPU regions for process RAM. 
    ///
    /// # Arguments
    /// 
    /// `new_app_memory_break`      : new address for the end of process acessible memory 
    /// `new_kernel_memory_break`   : new address for the start of grant
    /// `config`                    : configuration data for the MPU
    fn update_ram_mpu_regions(
        &self,
        new_app_memory_break: *const u8,
        new_kernel_memory_break: *const u8,
        config: Self::MpuConfig
    ) -> ReturnCode;

    /// Adds new MPU region for a buffer.
    ///
    /// # Arguments
    fn add_new_mpu_region(
        &self,
        lower_bound: *const u8,
        upper_bound: *const u8,
        min_buffer_size: usize,
        permissions: Permissions
    ) -> ReturnCode;

    /// Configures the MPU.
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

    fn number_total_regions(&self) -> usize {
        8
    }
    
    fn number_available_regions(&self) -> usize {
        8
    }
    
    fn reset_mpu_config(&self, _: Self::MpuConfig) {}

    fn setup_ram_mpu_regions(
        &self, 
        lower_bound: *const u8,
        _: *const u8,
        min_app_ram_size: usize,
        _: usize,
        _: usize,
        _: Self::MpuConfig
    ) -> Option<(*const u8, usize)> {
        Some((lower_bound, min_app_ram_size))
    }

    fn update_ram_mpu_regions(
        &self,
        new_app_memory_break: *const u8,
        new_kernel_memory_break: *const u8,
        config: Self::MpuConfig
    ) -> ReturnCode {
        ReturnCode::SUCCESS
    }

    /// Adds new MPU region for a buffer.
    ///
    /// # Arguments
    fn add_new_mpu_region(
        &self,
        lower_bound: *const u8,
        upper_bound: *const u8,
        min_buffer_size: usize,
        permissions: Permissions
    ) -> ReturnCode {
        ReturnCode::SUCCESS
    }

    fn configure_mpu(&self, _: &Self::MpuConfig) {}
}
