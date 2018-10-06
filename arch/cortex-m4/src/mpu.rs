//! Implementation of the ARM memory protection unit.

use kernel;
use kernel::common::math;
use kernel::common::registers::{FieldValue, ReadOnly, ReadWrite};
use kernel::common::StaticRef;
use kernel::mpu;

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

/// Struct storing region configuration for the Cortex-M MPU.
#[derive(Copy, Clone)]
pub struct CortexMConfig {
    regions: [CortexMRegion; 8],
}

const APP_MEMORY_REGION_NUM: usize = 0;

impl Default for CortexMConfig {
    fn default() -> CortexMConfig {
        CortexMConfig {
            regions: [
                CortexMRegion::empty(0),
                CortexMRegion::empty(1),
                CortexMRegion::empty(2),
                CortexMRegion::empty(3),
                CortexMRegion::empty(4),
                CortexMRegion::empty(5),
                CortexMRegion::empty(6),
                CortexMRegion::empty(7),
            ],
        }
    }
}

/// Struct storing configuration for a Cortex-M MPU region.
#[derive(Copy, Clone)]
pub struct CortexMRegion {
    location: Option<(*const u8, usize)>,
    base_address: FieldValue<u32, RegionBaseAddress::Register>,
    attributes: FieldValue<u32, RegionAttributes::Register>,
}

impl CortexMRegion {
    fn new(
        logical_start: *const u8,
        logical_size: usize,
        region_start: *const u8,
        region_size: usize,
        region_num: usize,
        subregion_mask: Option<u32>,
    ) -> CortexMRegion {
        // Determine permissions
        let read = region.get_read_permission();
        let write = region.get_write_permission();
        let execute = region.get_execute_permission();

        // Convert execute permission to a bitfield
        let execute_value = match execute {
            Permission::NoAccess => RegionAttributes::XN::Disable,
            Permission::Full => RegionAttributes::XN::Enable,
            _ => {
                return Err(i);
            } // Not supported
        };

        // Convert read & write permissions to bitfields
        let access_value = match read {
            Permission::NoAccess => RegionAttributes::AP::NoAccess,
            Permission::PrivilegedOnly => {
                match write {
                    Permission::NoAccess => RegionAttributes::AP::PrivilegedOnlyReadOnly,
                    Permission::PrivilegedOnly => RegionAttributes::AP::PrivilegedOnly,
                    _ => {
                        return Err(i);
                    } // Not supported
                }
            }
            Permission::Full => match write {
                Permission::NoAccess => RegionAttributes::AP::ReadOnly,
                Permission::PrivilegedOnly => RegionAttributes::AP::UnprivilegedReadOnly,
                Permission::Full => RegionAttributes::AP::ReadWrite,
            },
        };

        // Base address register
        let base_address = RegionBaseAddress::ADDR.val((region_start as u32) >> 5)
            + RegionBaseAddress::VALID::UseRBAR
            + RegionBaseAddress::REGION.val(region_num as u32);

        let size_value = math::log_base_two(region_size as u32) - 1;

        // Attributes register
        let mut attributes = RegionAttributes::ENABLE::SET
            + RegionAttributes::SIZE.val(size_value)
            + access
            + execute;

        // If using subregions, add the mask
        if let Some(mask) = subregion_mask {
            attributes += RegionAttributes::SRD.val(mask);
        }

        CortexMRegion {
            location: Some((logical_start, logical_size)),
            base_address: base_address,
            attributes: attributes,
        }
    }

    fn empty(region_num: usize) -> CortexMRegion {
        CortexMRegion {
            location: None,
            base_address: RegionBaseAddress::VALID::UseRBAR
                + RegionBaseAddress::REGION.val(region_num as u32),
            attributes: RegionAttributes::ENABLE::CLEAR,
        }
    }

    fn location(&self) -> Option<(*const u8, usize)> {
        self.location
    }

    fn base_address(&self) -> FieldValue<u32, RegionBaseAddress::Register> {
        self.base_address
    }

    fn attributes(&self) -> FieldValue<u32, RegionAttributes::Register> {
        self.attributes
    }
}

fn round_up_to_nearest_multiple(x: usize, y: usize) -> usize {
    if x % y == 0 {
        x
    } else {
        x + y - (x % y)
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

    fn allocate_regions(regions: &mut [Region]) -> Result<Self::MpuConfig, usize> {
        let mut config = [
            RegionConfig::empty(0),
            RegionConfig::empty(1),
            RegionConfig::empty(2),
            RegionConfig::empty(3),
            RegionConfig::empty(4),
            RegionConfig::empty(5),
            RegionConfig::empty(6),
            RegionConfig::empty(7),
        ];

        for (i, region) in regions.iter().enumerate() {
            // Only support 8 regions
            if i >= 8 {
                return Err(i);
            }

            let region_num = i as u32;

            // Handle flexible start and end and relative locations
            let (lower_bound, min_region_size, upper_bound) = match region.get_type() {
                // TODO: Properly implement absolute
                RegionType::Absolute {
                    start_address,
                    end_address,
                    start_flexibility,
                    end_flexibility,
                } => (
                    start_address - start_flexibility,
                    end_address - start_address,
                    end_address + end_flexibility,
                ),
                RegionType::Relative {
                    lower_bound,
                    upper_bound,
                    min_offset,
                    min_region_size,
                } => (lower_bound + min_offset, min_region_size, upper_bound),
            };

            // Logical region
            let mut start = lower_bound as usize;
            let mut size = min_region_size;

            // Regions must be at least 32 bytes
            if size < 32 {
                size = 32;
            }

            // If we have to resort to using subregions, we calculate what our
            // underlying region size and subregion size would look like.
            let mut size_pow_two = math::closest_power_of_two(size as u32) as usize;
            if size_pow_two < 256 {
                size_pow_two = 256
            }
            let mut subregion_size = size_pow_two / 8;

            // Rounds start up to subregion_size (which is always higher than 32).
            start = round_up_to_nearest_multiple(start, subregion_size);

            // These values form the physical MPU region: the values we write to
            // the registers. The physical MPU region might be larger than
            // the logical region if some subregions are disabled.
            let mut region_start = start;
            let mut region_size = size;
            let mut subregion_mask = None;

            // This loop checks if we can make subregions work for the subregion size
            // being equal to size_pow_two/8, size_pow_two/4, size_pow_two/2 and
            // finally size_pow_two.
            // If none of these cases works, it is impossible to create a region,
            // and we fail.
            while subregion_size <= size_pow_two {
                // If `size` doesn't align to the subregion size, extend it.
                size = round_up_to_nearest_multiple(size, subregion_size);

                // If the size is a power of two and start % size = 0, we have a valid
                // region. If this is not the case, we try to cover the memory
                // region by using a larger MPU region and expose certain subregions.
                if size.count_ones() == 1 && start % size == 0 {
                    region_start = start;
                    region_size = size;
                    break;
                }

                // If we increased our subregion size, we need to increase
                // the underlying_region_size accordingly.
                let underlying_region_size = subregion_size * 8;

                // We calculate the underlying_region_start by finding the nearest
                // address below `start` that aligns with the region size.
                let underlying_region_start = start - (start % underlying_region_size);

                let end = start + size;
                let underlying_region_end = underlying_region_start + underlying_region_size;

                // We have found a suitable subregion setup if the end of the
                // underlying region covers the end of our memory. If so, we set this up
                // and break. Otherwise, we repeat this while loop.
                if underlying_region_end >= end {
                    // The index of the first subregion to activate is the number of
                    // regions between `region_start` (MPU) and `start` (memory).
                    let min_subregion = (start - underlying_region_start) / subregion_size;

                    // The index of the last subregion to activate is the number of
                    // regions that fit in `size`, plus the `min_subregion`, minus one
                    // (because subregions are zero-indexed).
                    let max_subregion = min_subregion + size / subregion_size - 1;

                    // Turn the min/max subregion into a bitfield where all bits are `1`
                    // except for the bits whose index lie within
                    // [min_subregion, max_subregion]
                    //
                    // Note: Rust ranges are minimum inclusive, maximum exclusive, hence
                    // max_subregion + 1.
                    let mask = (min_subregion..(max_subregion + 1))
                        .fold(!0, |res, i| res & !(1 << i))
                        & 0xff;

                    region_start = underlying_region_start;
                    region_size = underlying_region_size;
                    subregion_mask = Some(mask);

                    break;
                }
                // We just tried aligning a certain start and size. Apparently, it
                // didn't work out, so we try aligning for a bigger region instead.
                subregion_size *= 2;
                start = round_up_to_nearest_multiple(start, subregion_size);
            }

            // Check if we found a suitable region
            if subregion_size > size_pow_two {
                return None;
            }

            // Cortex-M regions can't be greater than 4 GB.
            if math::log_base_two(region_size as u32) >= 32 {
                return None;
            }

            // Check that our logical region fits in memory.
            if start + size > upper_bound {
                return None;
            }

            config[i] = CortexMRegion::new(
                start as *const u8,
                size,
                region_start as *const u8,
                region_size,
                region_num,
                subregion_mask,
            );
        }
        Some(mpu::Region::new(start as *const u8, size))
    }

    fn configure_mpu(&self, config: &Self::MpuConfig) {
        let regs = &*self.0;

        // Set MPU regions
        for region in config.regions.iter() {
            regs.rbar.write(region.base_address());
            regs.rasr.write(region.attributes());
        }
    }
}
