//! Implementation of the ARM memory protection unit.

use kernel;
use kernel::common::cells::MapCell;
use kernel::common::math;
use kernel::common::math::PowerOfTwo;
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
pub struct MPU(StaticRef<MpuRegisters>, MapCell<Option<CortexMConfig>>);

impl MPU {
    pub const unsafe fn new() -> MPU {
        MPU(MPU_BASE_ADDRESS, MapCell::new(None)) // TODO: remove hack: this should not be stored here.
    }
}

#[derive(Copy, Clone)]
pub struct CortexMConfig {
    memory_info: Option<ProcessMemoryInfo>,
    pam_region: RegionConfig,
    regions: [RegionConfig; 7],
    num_regions_used: usize,
}

const PAM_REGION_NUM: usize = 7;

impl Default for CortexMConfig {
    fn default() -> CortexMConfig {
        CortexMConfig {
            memory_info: None,
            pam_region: RegionConfig::empty(7),
            regions: [
                RegionConfig::empty(0),
                RegionConfig::empty(1),
                RegionConfig::empty(2),
                RegionConfig::empty(3),
                RegionConfig::empty(4),
                RegionConfig::empty(5),
                RegionConfig::empty(6),
            ],
            num_regions_used: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct ProcessMemoryInfo {
    memory_start: *const u8,
    memory_size: usize,
    pam_permissions: Permissions,
}

/// Struct containing the register values to be written
#[derive(Copy, Clone)]
pub struct RegionConfig {
    base_address: FieldValue<u32, RegionBaseAddress::Register>,
    attributes: FieldValue<u32, RegionAttributes::Register>,
}

impl RegionConfig {
    fn new(
        address: u32,
        size: u32,
        region_num: u32,
        subregion_mask: Option<u32>,
        permissions: Permissions,
    ) -> RegionConfig {
        // Matching permissions with register values
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

        // Address register field only takes the 27 MSB TODO
        let base_address = RegionBaseAddress::ADDR.val(address >> 5)
            + RegionBaseAddress::VALID::UseRBAR
            + RegionBaseAddress::REGION.val(region_num);

        // Write region size and permissions
        let mut attributes =
            RegionAttributes::ENABLE::SET + RegionAttributes::SIZE.val(size) + access + execute;

        // If subregions enabled, write them to register
        if let Some(mask) = subregion_mask {
            attributes += RegionAttributes::SRD.val(mask);
        }

        RegionConfig {
            base_address,
            attributes,
        }
    }

    fn empty(region_num: u32) -> RegionConfig {
        RegionConfig {
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
        min_app_ram_size: usize,
        initial_pam_size: usize,
        initial_grant_size: usize,
        permissions: Permissions,
        _config: &mut Self::MpuConfig,
    ) -> Option<(*const u8, usize)> {
        // If the user has not specified enough memory for PAM and grant, round up and fix.
        let app_ram_size = if min_app_ram_size < initial_pam_size + initial_grant_size {
            initial_pam_size + initial_grant_size
        } else {
            min_app_ram_size
        };

        // Some statements for debugging. Will be removed on PR.
        //debug!("Min app ram size: {}", min_app_ram_size);
        //debug!("Initial PAM size: {}", initial_pam_size);
        //debug!("Initial grant size: {}", initial_grant_size);
        //debug!("App ram size: {}", app_ram_size);

        // We'll go through this code with some numeric examples, and
        // indicate this by EX.
        let mut region_len = math::closest_power_of_two(app_ram_size as u32);
        let mut exponent = math::log_base_two(region_len);

        //debug!("Region len: {}", region_len);

        if exponent < 7 {
            // Region sizes must be 128 Bytes or larger in order to support subregions
            exponent = 7;
            region_len = 128;
        } else if exponent > 32 {
            // Region sizes must be 4GB or smaller
            return None;
        }

        //debug!("Region len: {}", region_len);

        // Preferably, the region will start at the start of the parent region
        let mut region_start = parent_start as u32;

        // If the start doesn't align to the length, move region start up until it does
        if region_start % region_len != 0 {
            region_start += region_len - (region_start % region_len);
        }

        // Make sure that the requested region fits in memory
        let parent_end = (parent_start) as u32 + (parent_size as u32);
        if region_start + region_len > parent_end {
            //debug!("Requested region doesn't fit in memory");
            return None;
        }

        //debug!("Parent start: {:#X}", parent_start as usize);
        //debug!("Region start: {:#X}", region_start);
        //debug!("Region len: {}", region_len);

        // The memory initially allocated for the PAM will be aligned to an eigth of the total region length.
        // This allows Cortex-M subregions to control the growth of the PAM/grant in a more linear way.
        // The Cortex-M has a total of 8 subregions per region, which is why we can have precision in
        // eights of total region lengths.
        // EX: subregions_used = (3500 * 8)/8192 + 1 = 4;
        // TODO
        // let subregions_used = (initial_pam_size * 8) as u32 / region_len + 1;
        let subregions_used = 8;

        // EX: 00001111 & 11111111 = 00001111 --> Use the first four subregions (0 = enable)
        let subregion_mask = (0..subregions_used).fold(!0, |res, i| res & !(1 << i)) & 0xff;

        //debug!("Subregions used: {}", subregions_used);

        let size_value = exponent - 1;

        //debug!("Exponent: {}", exponent);

        let region_config = RegionConfig::new(
            region_start,
            size_value,
            PAM_REGION_NUM as u32,
            Some(subregion_mask),
            permissions,
        );

        //debug!("Arg0: {:#b}", region_start);
        //debug!("Arg1: {}", size_value);
        //debug!("Arg2: {}", 1);

        // TODO: do this in config
        let cortexm_config: CortexMConfig = Default::default();
        self.1.replace(Some(cortexm_config));
        self.1.map(|config| match config {
            Some(cortexm_config) => {
                let memory_info = ProcessMemoryInfo {
                    memory_start: region_start as *const u8,
                    memory_size: region_len as usize,
                    pam_permissions: permissions,
                };
                cortexm_config.memory_info = Some(memory_info);
                cortexm_config.pam_region = region_config;
            }
            None => panic!("Config not written correctly"),
        });

        // TODO: do this in config and set PAM region to region 0. Two reasons:
        // (1) More logical to increment the number of used regions when setting up the PAM
        // (2) On future addition of overlapping regions (e.g. it becomes necessary to add a small grant region), this region will have higher priority because the Cortex-M orders region priorities by their index
        // let region_num = config.num_regions_used;
        // debug!("regions used: {}", region_num);
        // config.regions[region_num] = region_config;
        // config.num_regions_used += 1;

        Some((region_start as *const u8, region_len as usize))
    }

    fn update_process_memory_layout(
        &self,
        new_app_memory_break: *const u8,
        new_kernel_memory_break: *const u8,
        _config: &mut Self::MpuConfig,
    ) -> Result<(), ()> {
        // TODO: Use implementation from #1113
        let mut region_start = 0;
        let mut region_len = 0;
        let mut permissions = Permissions::ReadWriteExecute;
        self.1.map(|config| {
            match config {
                Some(cortexm_config) => {
                    match cortexm_config.memory_info {
                        Some(memory_info) => {
                            region_start = memory_info.memory_start as u32;
                            region_len = memory_info.memory_size as u32;
                            permissions = memory_info.pam_permissions;
                        }
                        None => {
                            // PAM was never set up
                            unimplemented!("");
                        }
                    };
                }
                None => panic!("Config not written correctly"),
            }
        });

        //debug!("First update:");
        //debug!("New app memory break: {:#X}", new_app_memory_break as usize);
        //debug!("New new kernel memory break: {:#X}", new_kernel_memory_break as usize);

        // The PAM ends at new_app_memory_break, it's different from the region length.
        let pam_end = new_app_memory_break as u32;
        let grant_start = new_kernel_memory_break as u32;

        if pam_end > grant_start {
            // Error: out of memory for the application. Please allocate more memory for your application.
            return Err(());
        }

        let pam_len = pam_end - region_start;

        // TODO: Measure execution time of these operations. Maybe we can get some optimizations in the future.
        let num_subregions_used = (pam_len * 8) as u32 / region_len + 1;

        return Ok(()); // TODO

        let subregion_mask = (0..num_subregions_used).fold(!0, |res, i| res & !(1 << i)) & 0xff;
        //let subregion_mask = (0..8).fold(!0, |res, i| res & !(1 << i)) & 0xff;

        let size_value = math::log_base_two(region_len) - 1;

        let region_config = RegionConfig::new(
            region_start,
            size_value,
            PAM_REGION_NUM as u32,
            Some(subregion_mask),
            permissions,
        );

        self.1.map(|config| match config {
            Some(cortexm_config) => cortexm_config.pam_region = region_config,
            None => panic!("Config not written correctly"),
        });

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
        let region_num = config.num_regions_used;

        // Only 8 regions supported
        if region_num >= 8 {
            return None;
        }

        // Preferably, the region will start at the start of the parent region
        let mut region_start = parent_start as usize;

        // Regions have to be a power of two
        let mut region_len = math::closest_power_of_two(min_region_size as u32) as usize;

        // Calculate the log base two
        let mut exponent = math::log_base_two(region_len as u32);

        let address_value;
        let size_value;
        let subregion_mask;

        // Case 1: Easy
        if region_start % region_len == 0 {
            if exponent < 5 {
                // Region sizes must be 32 Bytes or larger
                exponent = 5;
                region_len = 32;
            } else if exponent > 32 {
                // Region sizes must be 4GB or smaller
                return None;
            }
            // Region length must not be bigger than parent size
            if region_len > parent_size {
                return None;
            }
            address_value = region_start as u32;
            size_value = exponent - 1;
            subregion_mask = None;
        }

        // Case 2: Hard
        // Things get more difficult if the start doesn't align to the length.
        // If the start still aligns to the region length / 4, 
        // We can use a larger MPU region and expose only MPU subregions, as
        // long as the memory region's base address is aligned to 1/8th of a
        // larger region size.  Possibility 2
        else {
            // Region sizes must be 128 Bytes or larger in order to support subregions.
            // Therefore check region length
            if exponent < 7 {
                exponent = 7; 
                region_len = 128; 
            }

            let mut result = 0;

            // If the start doesn't align to the region length / 4, this means
            // start will have to be changed 
            if region_start % (region_len / 4) != 0 {
                // Move start so that it aligns with region_len / 4
                for x in region_start..(parent_end - region_len) {
                    if region_start % (region_len / 4) == 0 {
                        result = 1;
                        region_start = x;
                        break;
                    } 
                }
                // No region could be found within the parent region and with region_len which suffices Cortex-M requirements
                // Either the parent size should be bigger/differently located, or the region size should be smaller
                if result = 0 {
                    return None;
                }
            }

            // Check just to make sure we're still within parent
            let parent_end = parent_start as usize + parent_size;
            if region_start + region_len > parent_end {
                return None;
            }    

            // Memory base not aligned to memory size
            // Which (power-of-two) subregion size would align with the base
            // address?
            //
            // We find this by taking smallest binary substring of the base
            // address with exactly one bit:
            //
            //      1 << (region_start.trailing_zeros())
            let subregion_size = {
                let tz = region_start.trailing_zeros();
                // `region_start` should never be 0 because of that's taken care of by
                // the previous branch, but in case it is, do the right thing
                // anyway.
                if tz < 32 {
                    (1 as usize) << tz
                } else {
                    0
                }
            };

            // Once we have a subregion size, we get an underlying region size by
            // multiplying it by the number of subregions per region.
            let underlying_region_size = subregion_size * 8;
            
            
            // Finally, we calculate the region base by finding the nearest
            // address below `start` that aligns with the region size.
            let underlying_region_start = region_start - (region_start % region_size);


            }

            // EX: 00001111 & 11111111 = 00001111 --> Use the first four subregions (0 = enable)
            subregion_mask = Some((0..subregions_used).fold(!0, |res, i| res & !(1 << i)) & 0xff);
            unimplemented!("");

            if underlying_region_size + underlying_region_start - region_start < region_size {
                // Basically, check if the length from region_start until underlying_region_end is greater than region_len
                // TODO: Should honestly never happen, remove?
                return None;
            }
            if len % subregion_size != 0 {
                // Basically, check if the subregions actually align to the length
                // TODO: Should always happen because of this line:
                // if region_start % (region_len / 4) != 0
                // So remove?

                // Sanity check that there is some integer X such that
                // subregion_size * X == len so none of `len` is left over when
                // we take the max_subregion.
                return None;
            }

            // The index of the first subregion to activate is the number of
            // regions between `region_start` (MPU) and `start` (memory).
            let min_subregion = (region_start - underlying_region_start) / subregion_size;
            // The index of the last subregion to activate is the number of
            // regions that fit in `len`, plus the `min_subregion`, minus one
            // (because subregions are zero-indexed).
            let max_subregion = min_subregion + region_len / subregion_size - 1;
            // Turn the min/max subregion into a bitfield where all bits are `1`
            // except for the bits whose index lie within
            // [min_subregion, max_subregion]
            //
            // Note: Rust ranges are minimum inclusive, maximum exclusive, hence
            // max_subregion + 1.
            subregion_mask =
                Some((min_subregion..(max_subregion + 1)).fold(!0, |res, i| res & !(1 << i)) & 0xff);

            address_value = region_start as u32;
            size_value = exponent - 1;
        }

        let region_config = RegionConfig::new(
            address_value,
            size_value,
            region_num as u32,
            subregion_mask,
            permissions,
        );

        config.regions[region_num] = region_config;
        config.num_regions_used += 1;

        Some((region_start as *const u8, region_len))
    }

    fn configure_mpu(&self, config: &Self::MpuConfig) {
        let regs = &*self.0;

        // Set MPU regions
        for region_config in config.regions.iter() {
            regs.rbar.write(region_config.base_address);
            regs.rasr.write(region_config.attributes);
        }

        // Set PAM region
        // TODO: use config for this
        self.1.map(|config| match config {
            Some(cortexm_config) => {
                let region_config = cortexm_config.pam_region;
                regs.rbar.write(region_config.base_address);
                regs.rasr.write(region_config.attributes);
            }
            None => panic!("Config not written correctly"),
        });
    }
}
