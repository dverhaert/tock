use std::f32;
use std::u32;

fn main() {

    // Algorithm test. Change the following three parameters at will and be amazed at the results.
    if let None = expose_memory_region(
        1, // parent_start
        400, // parent_size
        256, // min_region_size
    ) {
        println!("Failed, try again next time");
    }

    fn expose_memory_region(
        parent_start: usize,
        parent_size: usize,
        min_region_size: usize,
    ) -> Option<(*const u8, usize)> {
        let region_num = 1;

        // Only 8 regions supported
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
        let mut region_len = u32::next_power_of_two(min_region_size as u32) as usize;

        // Calculate the log base two
        let mut floatexponent = f32::log2(region_len as f32);
        let mut exponent = floatexponent as usize;

        if exponent < 5 {
            // Region sizes must be 32 Bytes or larger
            exponent = 5;
            region_len = 32;
        }
        if exponent > 32 {
            // Region sizes must be 4GB or smaller
            return None;
        }

        // There are three possibilities we support:
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
        if region_start % region_len == 0 {

            // Region length must not be bigger than parent size
            if region_len > parent_size {
                return None;
            }
        }
        // Case 2: Hard
        // Things get more difficult if the start doesn't align to the length.
        // If the start still aligns to the region length / 4, we can use a
        // larger MPU region and expose only MPU subregions.
        // Note that if start aligns to region length / 8 but not to region length / 4,
        // it's impossible to create a valid region since for this 9 subregions
        // are required: 8 after the start for the region itself and one to
        // before the start to align it.
        else {
            // If the start doesn't align to the region length / 4, this means
            // start will have to be changed to align to htis
            if region_start % (region_len / 8) != 0 {
                region_start += (region_len / 8) - (region_start % (region_len / 8));
                // No region could be found within the parent region and with
                // region_len which suffices Cortex-M requirements. Either the
                // parent size should be bigger/differently located, or the
                // region length (and so min_app_ram_size) should be smaller
                if region_start + region_len > parent_end {
                    println!("No region could be found within the parent region and with region_len which suffices Cortex-M requirements.");
                    return None;
                }
            }

            // We have now found an address which can definitely be supported,
            // be it with or without subregions. Check for both.
            if region_start % region_len == 0 {
            } else {
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
                    // TODO: Remove another useless sanity check?
                    } else {
                        0
                    }
                };

                // Once we have a subregion size, we get an underlying region size by
                // multiplying it by the number of subregions per region.
                let underlying_region_size = subregion_size * 8;

                // Finally, we calculate the region base by finding the nearest
                // address below `region_start` that aligns with the region size.
                let underlying_region_start =
                    region_start - (region_start % underlying_region_size);

                floatexponent = f32::log2(underlying_region_size as f32);
                exponent = floatexponent as usize;

                if underlying_region_size + underlying_region_start - region_start < region_len {
                    // Basically, check if the length from region_start until
                    // underlying_region_end is greater than region_len
                    // TODO: Should honestly never happen, remove?
                    return None;
                }
                if region_len % subregion_size != 0 {
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
                // subregion_mask = Some(
                //     (min_subregion..(max_subregion + 1)).fold(!0, |res, i| res & !(1 << i)) & 0xff,
                // );

                println!(
                    "Subregions used: {} through {}",
                    min_subregion, max_subregion
                );
                println!("Underlying region start address: {}", underlying_region_start);
                println!("Underlying region size: {}", underlying_region_size);
            }
        }
        println!("exponent: {}", exponent); 
        println!("Region start: {}", region_start);
        println!("Region length: {}", region_len);      

        Some((region_start as *const u8, region_len))
    }
}