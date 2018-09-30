use std::u32;
use std::f32;

fn main() {

    // test(1,400,32); // 32, 32
    // test(1,400,64); // 32, 64 (0 to 256)
    // test(1,400,128); // 32, 128 (0 to 256)
    test(1,400,256); // 256, 256 ==> Optimal 64 to 320.
    test(768, 1280, 1280); // Optimum is 768 from 1280. Case in which Danilo's algorithm would (incorrectly) fail
    test(1, 6000, 4096); // Optimum is 4096 from 1024. Case in which Conor's algorithm would (incorrectly) fail
    test(32,224,224); // Optimum is 224 from 0. Both algorithms would fail.
}


    fn test(parent_start: usize, parent_size: usize, min_region_size: usize) {
    // Algorithm test. Change the following three parameters at will and be amazed at the results.
    if let None = expose_memory_region(
        parent_start, // parent_start
        parent_size, // parent_size
        min_region_size, // min_region_size
    ) {
        println!("Failed for values {}, {} and {}\n", parent_start, parent_size, min_region_size);
    }
    }

    fn expose_memory_region(
        parent_start: usize,
        parent_size: usize,
        min_region_size: usize,
    ) -> Option<(*const u8, usize)> {
        let region_num = 1;

        // Cortex-M only supports 8 regions 
        if region_num >= 8 {
            return None;
        }
        
        // Logical region 
        let mut start = parent_start as usize;
        let mut size = min_region_size;
        
        // Physical MPU region
        let mut region_start = start;
        let mut region_size = size;
        
        // Region start always has to align to at least 32 bytes 
        if start % 32 != 0 {
            start += 32 - (start % 32);
        }

        // Regions must be at least 32 bytes
        if size < 32 {
            size = 32;
        }
        // Do nothing if already aligns
        if size.count_ones() > 1 || start % size != 0 {

        if start % (region_size / 4) != 0 {
                start += (region_size / 4) - (start % (region_size / 4));
            }

            // We have now found an address which can definitely be supported,
            // be it with or without subregions. Check for both.
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
                    // TODO: Remove another useless sanity check?
                    } else {
                    // This case means `start` is 0.                    
                    let mut ceil = u32::next_power_of_two(size as u32) as usize;
                    if ceil < 256 {
                        ceil = 256
                    }
                    ceil / 8
                    }
                };

                // Once we have a subregion size, we get an underlying region size by
                // multiplying it by the number of subregions per region.
                let underlying_region_size = subregion_size * 8;

                // Finally, we calculate the region base by finding the nearest
                // address below `start` that aligns with the region size.
                let underlying_region_start =
                    start - (start % underlying_region_size);

                if underlying_region_size + underlying_region_start - start < size {
                    // Basically, check if the size from start until
                    // underlying_region_end is greater than size
                    // TODO: Should honestly never happen, remove?
                    return None;
                }
                if size % subregion_size != 0 {
                    // Basically, check if the subregions actually align to the size
                    // TODO: Should always happen because of this line:
                    // if start % (region_size / 4) != 0
                    // So remove?

                    // Sanity check that there is some integer X such that
                    // subregion_size * X == size so none of `size` is left over when
                    // we take the max_subregion.
                    return None;
                }

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
                // subregion_mask = Some(
                //     (min_subregion..(max_subregion + 1)).fold(!0, |res, i| res & !(1 << i)) & 0xff,
                // );
                region_start = underlying_region_start;
                region_size = underlying_region_size;

                println!(
                    "Subregions used: {} through {}",
                    min_subregion, max_subregion
                );
                println!("Underlying region start address: {}", underlying_region_start);
                println!("Underlying region size: {}", underlying_region_size);
            }

        println!("Region start: {}", start);
        println!("Region length: {}\n", size);  

        // Regions can't be greater than 4 GB. 
        if f32::log2(size as f32) >= 32 as f32 {
            return None;
        }

        // Check that our region fits in memory.
        if start + size > (parent_start as usize) + parent_size {
            return None;
        }
        Some((start as *const u8, size))
    }