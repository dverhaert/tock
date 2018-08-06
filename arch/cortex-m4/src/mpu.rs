//! Implementation of the ARM memory protection unit.

use kernel;
use kernel::common::math::PowerOfTwo;
use kernel::common::registers::{FieldValue, ReadOnly, ReadWrite};
use kernel::common::StaticRef;
use kernel::common::cells::MapCell;
use kernel::mpu::Permissions;

#[repr(C)]
/// MPU Registers for the Cortex-M4 family
///
/// Described in section 4.5 of
/// <http://infocenter.arm.com/help/topic/com.arm.doc.dui0553a/DUI0553A_cortex_m4_dgug.pdf>
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
    pam_region: Option<PAMRegionInfo>,
    regions: [Option<RegionConfig>; 7],
}

impl Default for CortexMConfig {
    fn default() -> CortexMConfig {
        CortexMConfig {
            pam_region: None,
            regions: [None; 7],
        }
    }
}

#[derive(Copy, Clone)]
pub struct PAMRegionInfo {
    base_address: u32,
    size: u32,
    permissions: Permissions,
    num_subregions_used: u32,
    region: RegionConfig,
}

#[derive(Copy, Clone)]
pub struct RegionConfig {
    base_address: FieldValue<u32, RegionBaseAddress::Register>,
    attributes: FieldValue<u32, RegionAttributes::Register>,
}


impl RegionConfig {
    fn new(
        base_address: u32,
        size: u32,
        region_num: u32,
        subregion_mask: Option<u32>,
        permissions: Permissions,
    ) -> RegionConfig {
        let (access_value, execute_value) = match permissions {
            Permissions::ReadWriteExecute => (RegionAttributes::AP::ReadWrite, RegionAttributes::XN::Enable),
            Permissions::ReadWriteOnly => (RegionAttributes::AP::ReadWrite, RegionAttributes::XN::Disable),
            Permissions::ReadExecuteOnly => (RegionAttributes::AP::ReadOnly, RegionAttributes::XN::Enable),
            Permissions::ReadOnly => (RegionAttributes::AP::ReadOnly, RegionAttributes::XN::Disable),
            Permissions::ExecuteOnly => (RegionAttributes::AP::PrivilegedOnly, RegionAttributes::XN::Enable), // TODO
            Permissions::NoAccess => (RegionAttributes::AP::PrivilegedOnly, RegionAttributes::XN::Disable), // TODO
        };

        let base_address = RegionBaseAddress::ADDR.val(base_address)
            + RegionBaseAddress::VALID::UseRBAR
            + RegionBaseAddress::REGION.val(region_num);

        let mut attributes = RegionAttributes::ENABLE::SET
            + RegionAttributes::SIZE.val(size)
            + access_value
            + execute_value;

        // Subregions enabled
        if let Some(value) = subregion_mask {
            attributes += RegionAttributes::SRD.val(value);
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

const PAM_REGION_NUM: usize = 7;

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
        _config: &mut Self::MpuConfig
    ) -> Option<(*const u8, usize)> {
        // If the user has not given enough memory, round up and fix.
        let app_ram_size = if min_app_ram_size < initial_pam_size + initial_grant_size {
            initial_pam_size + initial_grant_size
        } else {
            min_app_ram_size
        };

        // We'll go through this code with some numeric examples, and 
        // indicate this by EX.
        // EX: region_len = PowerOfTwo::ceiling(4500) = 8192
        let region_len_poweroftwo = PowerOfTwo::ceiling(app_ram_size as u32);
        
        let mut region_len = PowerOfTwo::as_num(region_len_poweroftwo);
        // exponent = log2(region_len)  
        let mut exponent = region_len_poweroftwo.exp::<u32>();

        if exponent < 7 {
            // Region sizes must be 128 Bytes or larger in order to support subregions
            exponent = 7;
            region_len = 128;
        } else if exponent > 32 {
            // Region sizes must be 4GB or smaller
            return None;
        }       
        
        // Preferably, the start of the region is equal to the lower bound
        let mut region_start = parent_start as u32;

        // If the start doesn't align to the length, make sure it does
        if region_start % region_len != 0 {
            region_start = region_start + region_len - region_start % region_len;
        }

        // Make sure that the requested region fits in memory
        let upper_bound = parent_start as u32 + parent_size as u32;
        if region_start + region_len > upper_bound {
            return None;
        }
         
        // The memory initially allocated for the PAM will be aligned to an eigth of the total region length. 
        // This allows Cortex-M subregions to control the growth of the PAM/grant in a more linear way.
        // The Cortex-M has a total of 8 subregions per region, which is why we can have precision in 
        // eights of total region lengths.
        // EX: subregions_used = 3500/8192 * 8 + 1 = 4;
        let subregions_used = initial_pam_size as u32/region_len * 8 + 1;
        
        // EX: 00001111 & 11111111 = 00001111 --> Use the first four subregions (0 = enable)
        let subregion_mask = (0..subregions_used).fold(!0, |res, i| res & !(1 << i)) & 0xff;

        let region_len_value = exponent - 1;

        let region_config = RegionConfig::new(region_start, region_len_value, PAM_REGION_NUM as u32, Some(subregion_mask), permissions);

        // TODO: do this in config
        let cortexm_config = Default::default();
        self.1.replace(Some(cortexm_config));

        self.1.map(|config| {
            if let Some(cortexm_config) = config {
                let pam_region = PAMRegionInfo {
                    base_address: region_start,
                    size: region_len,
                    permissions: permissions,
                    num_subregions_used: subregions_used,
                    region: region_config,
                };

                cortexm_config.pam_region = Some(pam_region);
            }
        });

        None
    }

    fn update_process_memory_layout(
        &self,
        new_app_memory_break: *const u8,
        new_kernel_memory_break: *const u8,
        _config: &mut Self::MpuConfig
    ) -> Result<(), ()> {
        // get old memory region data from somewhere, let's see how we get this
        
        // 
        let mut region_start = 0;
        let mut region_len = 0;
        let mut permissions = Permissions::ReadWriteExecute;
        let mut num_subregions_used = 0; 

        self.1.map(|config| {
            if let Some(cortexm_config) = config {
                match cortexm_config.pam_region {
                    Some(pam_region) => {
                        region_start = pam_region.base_address;
                        region_len = pam_region.size;
                        permissions = pam_region.permissions;
                        num_subregions_used = pam_region.num_subregions_used;

                    },
                    None => { unimplemented!(""); },
                };
            }
        });

        // The PAM ends at new_app_memory_break, it's different from the region length.
        let pam_end = new_app_memory_break as usize;
        let grant_start = new_kernel_memory_break as usize;

        let pam_size = pam_end - region_start as usize;

        if pam_end > grant_start {
            // Error: out of memory for the application. Please allocate more memory for your application.
            return Err(());
        }

        // TODO: Measure execution time of these operations. Maybe we can get some optimizations in the future.
        let new_subregions_used = pam_size as u32/region_len * 8 + 1;
        
        if num_subregions_used == new_subregions_used {
            return Ok(());
        }
        else {
            let subregion_mask = (0..new_subregions_used).fold(!0, |res, i| res & !(1 << i)) & 0xff;// Recompute the exponent so we can pass it back into the region config
            // TODO: Move calculation of of region_len_value in Region create function
            let region_len_poweroftwo = PowerOfTwo::ceiling(region_len as u32);        
            let exponent = region_len_poweroftwo.exp::<u32>();
            let region_len_value = exponent - 1;

            let region_config = RegionConfig::new(region_start, region_len_value, PAM_REGION_NUM as u32, Some(subregion_mask), permissions); 

            self.1.map(|config| {
                if let Some(cortexm_config) = config {
                    let pam_region = PAMRegionInfo {
                        base_address: region_start,
                        size: region_len,
                        permissions: permissions,
                        num_subregions_used: new_subregions_used,
                        region: region_config,
                    };
                    cortexm_config.pam_region = Some(pam_region);
                }
            });
            Ok(())
        }

        
    }

    fn expose_memory_region(
        &self,
        parent_start: *const u8,
        parent_size: usize,
        min_region_size: usize,
        permissions: Permissions,
        config: &mut Self::MpuConfig
    ) -> Option<(*const u8, usize)>  {
        let mut region_num = 0; 

        // Find unused region number
        for (index, region) in config.regions.iter().enumerate() {
            if let None = region {
                region_num = index;
            } else if index == config.regions.len() - 1 {
                return None;
            }
        }

        // Only 8 regions supported
        if region_num >= 8 {
            return None;
        }

        // TODO
        if parent_size != min_region_size {
            unimplemented!("Flexible region requests not yet implemented");
        }
        
        let start = parent_start as usize;
        let len = min_region_size;

        // There are two possibilities we support:
        //
        // 1. The base address is aligned exactly to the size of the region,
        //    which uses an MPU region with the exact base address and size of
        //    the memory region.
        //
        // 2. Otherwise, we can use a larger MPU region and expose only MPU
        //    subregions, as long as the memory region's base address is aligned
        //    to 1/8th of a larger region size.

        // Possibility 1
        if start % len == 0 {
            // Memory base aligned to memory size - straight forward case
            let region_len = PowerOfTwo::floor(len as u32);

            // exponent = log2(region_len)
            let exponent = region_len.exp::<u32>();

            if exponent < 5 {
                // Region sizes must be 32 Bytes or larger
                return None;
            } else if exponent > 32 {
                // Region sizes must be 4GB or smaller
                return None;
            }

            let address_value = (start >> 5) as u32;
            let region_len_value = exponent - 1;
            
            let region_config = RegionConfig::new(
                address_value,
                region_len_value,
                region_num as u32,
                None,
                permissions,
            );
            
            config.regions[region_num] = Some(region_config); 
        }
        // Possibility 2
        else {
            // Memory base not aligned to memory size

            // Which (power-of-two) subregion size would align with the base
            // address?
            //
            // We find this by taking smallest binary substring of the base
            // address with exactly one bit:
            //
            //      1 << (start.trailing_zeros())
            let subregion_size = {
                let tz = start.trailing_zeros();
                // `start` should never be 0 because of that's taken care of by
                // the previous branch, but in case it is, do the right thing
                // anyway.
                if tz < 32 {
                    (1 as usize) << tz
                } else {
                    0
                }
            };

            // Once we have a subregion size, we get a region size by
            // multiplying it by the number of subregions per region.
            let region_size = subregion_size * 8;
            // Finally, we calculate the region base by finding the nearest
            // address below `start` that aligns with the region size.
            let region_start = start - (start % region_size);

            if region_size + region_start - start < len {
                // Sanity check that the amount left over space in the region
                // after `start` is at least as large as the memory region we
                // want to reference.
                return None;
            }
            if len % subregion_size != 0 {
                // Sanity check that there is some integer X such that
                // subregion_size * X == len so none of `len` is left over when
                // we take the max_subregion.
                return None;
            }

            // The index of the first subregion to activate is the number of
            // regions between `region_start` (MPU) and `start` (memory).
            let min_subregion = (start - region_start) / subregion_size;
            // The index of the last subregion to activate is the number of
            // regions that fit in `len`, plus the `min_subregion`, minus one
            // (because subregions are zero-indexed).
            let max_subregion = min_subregion + len / subregion_size - 1;

            let region_len = PowerOfTwo::floor(region_size as u32);
            // exponent = log2(region_len)
            let exponent = region_len.exp::<u32>();
            if exponent < 7 {
                // Subregions only supported for regions sizes 128 bytes and up.
                return None;
            } else if exponent > 32 {
                // Region sizes must be 4GB or smaller
                return None;
            }

            // Turn the min/max subregion into a bitfield where all bits are `1`
            // except for the bits whose index lie within
            // [min_subregion, max_subregion]
            //
            // Note: Rust ranges are minimum inclusive, maximum exclusive, hence
            // max_subregion + 1.
            let subregion_mask =
                (min_subregion..(max_subregion + 1)).fold(!0, |res, i| res & !(1 << i)) & 0xff;

            let address_value = (region_start >> 5) as u32;
            let region_len_value = exponent - 1;

            let region_config = RegionConfig::new(
                address_value,
                region_len_value,
                region_num as u32,
                Some(subregion_mask),
                permissions
            );
            
            config.regions[region_num] = Some(region_config); 
        }

        Some((start as *const u8, len)) 
    }

    fn configure_mpu(&self, config: &Self::MpuConfig) {
        let regs = &*self.0;

        // Set MPU regions
        for (index, region) in config.regions.iter().enumerate() {
            let region_config = match region {
                Some(region_config) => region_config.clone(),
                None => RegionConfig::empty(index as u32),
            };

            regs.rbar.write(region_config.base_address);
            regs.rasr.write(region_config.attributes);
        }

        // Set PAM region
        // TODO: use config for this
        self.1.map(|config| {
            let region_config = match config {
                Some(cortexm_config) => {
                    match cortexm_config.pam_region {
                        Some(pam_region) => pam_region.region,
                        None => RegionConfig::empty(PAM_REGION_NUM as u32),
                    }
                },
                None => RegionConfig::empty(PAM_REGION_NUM as u32),
            };

            regs.rbar.write(region_config.base_address);
            regs.rasr.write(region_config.attributes);
        });
    }
}
