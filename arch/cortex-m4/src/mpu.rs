//! Implementation of the ARM memory protection unit.

use kernel;
use kernel::common::math;
use kernel::common::registers::{FieldValue, ReadOnly, ReadWrite};
use kernel::common::StaticRef;
use kernel::mpu::Permissions;

/// MPU Registers for the Cortex-M4 family
///
/// Described in section 4.5 of
/// <http://infocenter.arm.com/help/topic/com.arm.doc.dui0553a/DUI0553A_cortex_m4_dgug.pdf>
#[repr(C)]
pub struct MpuRegisters {
    /// Indicates whether the MPU is present and, if so, how many regions it
    /// supports.
    pub mpu_type: ReadOnly<u32, Type::Register>,

    /// The control register:
    ///   * Enables the MPU (bit 0).
    ///   * Enables MPU in hard-fault, non-maskable interrupt (NMI).
    ///   * Enables the default memory map background region in privileged mode.
    pub ctrl: ReadWrite<u32, Control::Register>,

    /// Selects the region number (zero-indexed) referenced by the region base
    /// address and region attribute and size registers.
    pub rnr: ReadWrite<u32, RegionNumber::Register>,

    /// Defines the base address of the currently selected MPU region.
    pub rbar: ReadWrite<u32, RegionBaseAddress::Register>,

    /// Defines the region size and memory attributes of the selected MPU
    /// region. The bits are defined as in 4.5.5 of the Cortex-M4 user guide.
    pub rasr: ReadWrite<u32, RegionAttributes::Register>,
}

register_bitfields![u32,
    Type [
        /// The number of MPU instructions regions supported. Always reads 0.
        IREGION OFFSET(16) NUMBITS(8) [],
        /// The number of data regions supported. If this field reads-as-zero the
        /// processor does not implement an MPU
        DREGION OFFSET(8) NUMBITS(8) [],
        /// Indicates whether the processor support unified (0) or separate
        /// (1) instruction and data regions. Always reads 0 on the
        /// Cortex-M4.
        SEPARATE OFFSET(0) NUMBITS(1) []
    ],

    Control [
        /// Enables privileged software access to the default
        /// memory map
        PRIVDEFENA OFFSET(2) NUMBITS(1) [
            Enable = 0,
            Disable = 1
        ],
        /// Enables the operation of MPU during hard fault, NMI, 
        /// and FAULTMASK handlers
        HFNMIENA OFFSET(1) NUMBITS(1) [
            Enable = 0,
            Disable = 1
        ],
        /// Enables the MPU
        ENABLE OFFSET(0) NUMBITS(1) [
            Disable = 0,
            Enable = 1
        ]
    ],

    RegionNumber [
        /// Region indicating the MPU region referenced by the MPU_RBAR and
        /// MPU_RASR registers. Range 0-7 corresponding to the MPU regions.
        REGION OFFSET(0) NUMBITS(8) []
    ],

    RegionBaseAddress [
        /// Base address of the currently selected MPU region.
        ADDR OFFSET(5) NUMBITS(27) [],
        /// MPU Region Number valid bit.
        VALID OFFSET(4) NUMBITS(1) [
            /// Use the base address specified in Region Number Register (RNR)
            UseRNR = 0,
            /// Use the value of the REGION field in this register (RBAR)
            UseRBAR = 1
        ],
        /// Specifies which MPU region to set if VALID is set to 1.
        REGION OFFSET(0) NUMBITS(4) []
    ],

    RegionAttributes [
        /// Enables instruction fetches/execute permission
        XN OFFSET(28) NUMBITS(1) [
            Enable = 0,
            Disable = 1
        ],
        /// Defines access permissions
        AP OFFSET(24) NUMBITS(3) [
            //                                 Privileged  Unprivileged
            //                                 Access      Access
            NoAccess = 0b000,               // --          --
            PrivilegedOnly = 0b001,         // RW          --
            UnprivilegedReadOnly = 0b010,   // RW          R-
            ReadWrite = 0b011,              // RW          RW
            Reserved = 0b100,               // undef       undef
            PrivilegedOnlyReadOnly = 0b101, // R-          --
            ReadOnly = 0b110,               // R-          R-
            ReadOnlyAlias = 0b111           // R-          R-
        ],
        /// Subregion disable bits
        SRD OFFSET(8) NUMBITS(8) [],
        /// Specifies the region size, being 2^(SIZE+1) (minimum 3)
        SIZE OFFSET(1) NUMBITS(5) [],
        /// Enables the region
        ENABLE OFFSET(0) NUMBITS(1) []
    ]
];

const MPU_BASE_ADDRESS: StaticRef<MpuRegisters> =
    unsafe { StaticRef::new(0xE000ED90 as *const MpuRegisters) };

/// Constructor field is private to limit who can create a new MPU
pub struct MPU(StaticRef<MpuRegisters>);

impl MPU {
    pub const unsafe fn new() -> MPU {
        MPU(MPU_BASE_ADDRESS)
    }
}

#[derive(Copy, Clone)]
pub struct CortexMConfig {
    memory_info: Option<ProcessMemoryInfo>,
    regions: [Region; 8],
    next_region_num: usize,
}

const APP_MEMORY_REGION_NUM: usize = 0;

impl Default for CortexMConfig {
    fn default() -> CortexMConfig {
        CortexMConfig {
            memory_info: None,
            regions: [
                Region::empty(0),
                Region::empty(1),
                Region::empty(2),
                Region::empty(3),
                Region::empty(4),
                Region::empty(5),
                Region::empty(6),
                Region::empty(7),
            ],
            next_region_num: 1,   // Index 0 is reserved for PAM
        }
    }
}

#[derive(Copy, Clone)]
pub struct ProcessMemoryInfo {
    memory_start: *const u8,
    memory_size: usize,
    permissions: Permissions,
}

#[derive(Copy, Clone)]
pub struct Region {
    base_address: FieldValue<u32, RegionBaseAddress::Register>,
    attributes: FieldValue<u32, RegionAttributes::Register>,
}

impl Region {
    fn new(
        address: u32,
        size: u32,
        region_num: u32,
        subregion_mask: Option<u32>,
        permissions: Permissions,
    ) -> Region {
        // Determine access and execute permissions
        let (access, execute) = match permissions {
            Permissions::ReadWriteExecute => (
                RegionAttributes::AP::ReadWrite,
                RegionAttributes::XN::Enable,
            ),
            Permissions::ReadWriteOnly => (
                RegionAttributes::AP::ReadWrite,
                RegionAttributes::XN::Disable,
            ),
            Permissions::ReadExecuteOnly => {
                (RegionAttributes::AP::ReadOnly, RegionAttributes::XN::Enable)
            }
            Permissions::ReadOnly => (
                RegionAttributes::AP::ReadOnly,
                RegionAttributes::XN::Disable,
            ),
            Permissions::ExecuteOnly => {
                (RegionAttributes::AP::NoAccess, RegionAttributes::XN::Enable)
            }
        };

        // Base address register
        let base_address = RegionBaseAddress::ADDR.val(address >> 5)
            + RegionBaseAddress::VALID::UseRBAR
            + RegionBaseAddress::REGION.val(region_num);

        let size = math::log_base_two(size) - 1;

        // Attributes register
        let mut attributes =
            RegionAttributes::ENABLE::SET + RegionAttributes::SIZE.val(size) + access + execute;

        // If using subregions, add the mask
        if let Some(mask) = subregion_mask {
            attributes += RegionAttributes::SRD.val(mask);
        }

        Region {
            base_address,
            attributes,
        }
    }

    fn empty(region_num: u32) -> Region {
        Region {
            base_address: RegionBaseAddress::VALID::UseRBAR
                + RegionBaseAddress::REGION.val(region_num),
            attributes: RegionAttributes::ENABLE::CLEAR,
        }
    }
}

impl kernel::mpu::MPU for MPU {
    type MpuConfig = CortexMConfig;

    fn enable_mpu(&self) {
        let regs = &*self.0;

        // Enable the MPU, disable it during HardFault/NMI handlers, and allow
        // privileged code access to all unprotected memory.
        regs.ctrl
            .write(Control::ENABLE::SET + Control::HFNMIENA::CLEAR + Control::PRIVDEFENA::SET);
    }

    fn disable_mpu(&self) {
        let regs = &*self.0;
        regs.ctrl.write(Control::ENABLE::CLEAR);
    }

    fn number_total_regions(&self) -> usize {
        let regs = &*self.0;
        regs.mpu_type.read(Type::DREGION) as usize
    }

    fn setup_process_memory_layout(
        &self,
        parent_start: *const u8,
        parent_size: usize,
        min_memory_size: usize,
        initial_app_memory_size: usize,
        initial_kernel_memory_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig,
    ) -> Option<(*const u8, usize)> {
        // Make sure there is enough memory for app memory and kernel memory 
        let memory_size = {
            if min_memory_size < initial_app_memory_size + initial_kernel_memory_size {
                initial_app_memory_size + initial_kernel_memory_size
            } else {
                min_memory_size
            }
        };

        // We'll use a running example for the purposes of clarity. // TODO

        let mut region_size = math::closest_power_of_two(memory_size as u32) as usize;
        let exponent = math::log_base_two(region_size as u32);
        
        if exponent < 8 {
            // Region sizes must be 256 Bytes or larger in order to support subregions
            region_size = 256;
        } else if exponent > 32 {
            // Region sizes must be 4GB or smaller
            return None;
        }

        // Ideally, the region will start at the start of the parent memory block 
        let mut region_start = parent_start as usize;

        // If the start and length don't align, move region up until it does
        if region_start % region_size != 0 {
            region_start += region_size - (region_start % region_size);
        }

        // The memory initially allocated for app memory will be aligned to an eigth of the total region length.
        // This allows Cortex-M subregions to cover incrementally growing app memory in linear way.
        // The Cortex-M has a total of 8 subregions per region, which is why we can have precision in
        // eights of total region lengths.
        //
        // EX: subregions_used = (3500 * 8)/8192 + 1 = 4;
        let subregions_used = (initial_app_memory_size * 8) / region_size + 1;

        // EX: 01111111 & 11111111 = 01111111 --> Use only the first subregions (0 = enable)
        let subregion_mask = (0..subregions_used).fold(!0, |res, i| res & !(1 << i)) & 0xff;

        // Make sure the region fits in the parent memory block
        if region_start + region_size > (parent_start as usize) + parent_size {
            return None;
        }
        
        let memory_info = ProcessMemoryInfo {
            memory_start: region_start as *const u8,
            memory_size: region_size,
            permissions: permissions,
        };

        let region = Region::new(
            region_start as u32,
            region_size as u32,
            APP_MEMORY_REGION_NUM as u32,
            Some(subregion_mask),
            permissions,
        );

        config.memory_info = Some(memory_info);
        config.regions[APP_MEMORY_REGION_NUM] = region;

        Some((region_start as *const u8, region_size))
    }

    fn update_process_memory_layout(
        &self,
        app_memory_break: *const u8,
        kernel_memory_break: *const u8,
        config: &mut Self::MpuConfig,
    ) -> Result<(), ()> {
        let (region_start, region_size, permissions) = match config.memory_info {
            Some(memory_info) => (
                memory_info.memory_start as u32,
                memory_info.memory_size as u32,
                memory_info.permissions,
            ),
            None => panic!(
                "Error: Process tried to update app memory MPU region before it was created."
            ),
        };

        let app_memory_break = app_memory_break as u32;
        let kernel_memory_break = kernel_memory_break as u32;

        // Out of memory
        if app_memory_break > kernel_memory_break {
            return Err(());
        }

        let app_memory_size = app_memory_break - region_start;

        // TODO: Measure execution time of these operations. Maybe we can implement some optimizations in the future.
        let num_subregions_used = (app_memory_size * 8) / region_size + 1;

        // We can no longer cover app memory with an MPU region without overlapping kernel memory 
        let subregion_size = region_size / 8;
        let subregions_end = region_start + subregion_size * num_subregions_used;
        if subregions_end > kernel_memory_break {
            return Err(());
        }

        let subregion_mask = (0..num_subregions_used).fold(!0, |res, i| res & !(1 << i)) & 0xff;

        let region = Region::new(
            region_start,
            region_size,
            APP_MEMORY_REGION_NUM as u32,
            Some(subregion_mask),
            permissions,
        );

        config.regions[APP_MEMORY_REGION_NUM] = region;

        Ok(())
    }

    fn expose_memory_region(
        &self,
        parent_start: *const u8,
        parent_size: usize,
        min_region_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig,
    ) -> Option<(*const u8, usize)> {
        let region_num = config.next_region_num;

        // Cortex-M supports 8 regions 
        if region_num >= 8 {
            return None;
        }

        // Preferably, the region will start at the start of the parent region
        let mut region_start = parent_start as usize;
        let parent_end = parent_start as usize + parent_size;

        // Region start always has to align to at least 32 bits
        if region_start % 32 != 0 {
            region_start += 32 - (region_start % 32);
        }

        // Regions have to be a power of two
        // https://www.youtube.com/watch?v=ovo6zwv6DX4
        let mut region_size = math::closest_power_of_two(min_region_size as u32) as usize;

        // Calculate the log base two
        let mut exponent = math::log_base_two(region_size as u32);

        if exponent < 5 {
            // Region sizes must be 32 Bytes or larger
            exponent = 5;
            region_size = 32;
        }
        if exponent > 32 {
            // Region sizes must be 4GB or smaller
            return None;
        }

        let mut subregion_mask = None;

        // There are two possibilities we support:
        //
        // 1. The base address is aligned exactly to the size of the region,
        //    which uses an MPU region with the exact base address and size of
        //    the memory region. In this case, we just do some basic checks
        //    after which we write to the registers.
        //
        // 2. Otherwise, we can use a larger MPU region and expose only MPU
        //    subregions, as long as the memory region's base address is aligned
        //    to 1/8th of a larger underlying region size.
        //

        // Case 1: Easy
        // Region start aligns to the length, so we can handle this normally!
        // Only need to check if the region size is not greater than the parent size.
        if region_start % region_size == 0 {
            // Region length must not be bigger than parent size
            if region_size > parent_size {
                return None;
            }
        }
        // Case 2: Hard
        // Things get more difficult if the start doesn't align to the region length.
        // If the start aligns to the region length / 4, we can use a
        // larger MPU region and expose only MPU subregions. Therefore, we
        // check if this is the case, and otherwise change start so that it does align to region length / 4
        // Note that if start aligns to region length / 8 but not to region length / 4,
        // it's impossible to create a valid region since for the region itself
        // 8 subregions are already required, so no subregions are left to 
        // fix the alignment.
        else {
            // If the start doesn't align to the region length / 4, this means
            // start will have to be changed
            if region_start % (region_size / 4) != 0 {
                region_start += (region_size / 4) - (region_start % (region_size / 4));
                // No valid region could be found within the parent region and with
                // region_size which suffices Cortex-M requirements. Either the
                // parent size should be bigger/differently located, or the
                // region length (and so min_app_ram_size) should be smaller
                if region_start + region_size > parent_end {
                    debug!("No region could be found within the parent region and with region_size which suffices Cortex-M requirements.");
                    return None;
                }
            }

            // We have now found an address which can definitely be supported,
            // be it with or without subregions. If it perfectly aligns, we're 
            // done. Otherwise, create with subregions.
            if region_start % region_size != 0 {
                // Memory base not aligned to memory size
                // Which (power-of-two) subregion size would align with the base
                // address?
                //
                // We find this by taking smallest binary substring of the base
                // address with exactly one bit:
                //
                //      1 << (region_start.trailing_zeros())
                let subregion_size = (1 as usize) << region_start.trailing_zeros();
                let enabled_subregions_start = region_start;
                let enabled_subregions_size = region_size;

                // Once we have a subregion size, we get a region size by
                // multiplying it by the number of subregions per region.
                region_size = subregion_size * 8;

                // Finally, we calculate the new region base by finding the nearest
                // address below `enabled_subregions_start` that aligns with the region size.
                region_start = enabled_subregions_start - (enabled_subregions_start % region_size);

                // The index of the first subregion to activate is the number of
                // regions between `enabled_subregions_start` (MPU) and `region_start` (memory).
                let min_subregion = (enabled_subregions_start - region_start) / subregion_size;
                // The index of the last subregion to activate is the number of
                // regions that fit in `region_size`, plus the `min_subregion`, minus one
                // (because subregions are zero-indexed).
                let max_subregion = min_subregion + enabled_subregions_size / subregion_size - 1;
                // Turn the min/max subregion into a bitfield where all bits are `1`
                // except for the bits whose index lie within
                // [min_subregion, max_subregion]
                //
                // Note: Rust ranges are minimum inclusive, maximum exclusive, hence
                // max_subregion + 1.
                subregion_mask = Some(
                    (min_subregion..(max_subregion + 1)).fold(!0, |res, i| res & !(1 << i)) & 0xff,
                );

                // debug!(
                //     "Subregions used: {} through {}",
                //     min_subregion, max_subregion
                // );
                // debug!("Underlying region start address: {:#X}", address_value);
                // debug!("Underlying region size: {}", size_value);
            }
        }

        // debug!("Region start: {:#X}", region_start);
        // debug!("Region length: {}", region_size);

        let region = Region::new(
            region_start as u32,
            region_size as u32,
            region_num as u32,
            subregion_mask,
            permissions,
        );

        config.regions[region_num] = region;
        config.next_region_num += 1;

        Some((region_start as *const u8, region_size))
    }

    fn configure_mpu(&self, config: &Self::MpuConfig) {
        let regs = &*self.0;

        // Set MPU regions
        for region in config.regions.iter() {
            regs.rbar.write(region.base_address);
            regs.rasr.write(region.attributes);
        }
    }
}
