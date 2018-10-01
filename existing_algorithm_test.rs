use std::u32;
use std::f32;

fn main() {
    // All of these have the same output for both existing and new.
    test(1,256,32);
    test(33,256,32);
    test(0,256,0);
    test(0,256,32);
    test(0,256,64);
    test(0,256,128);
    test(0,256,256);
    test(0,256,512);
    test(0,4096,512); 
    test(0,4096,1024);
    test(0,4096,2048);
    test(0,4096,4096);
    test(300,512,0);
    test(300,512,32); 
    test(300,512,64); 
    test(300,512,128);
    test(300,512,512); // Both fail (correct behaviour)
    test(1,400,32); // 32 to 64
    test(1,400,64); // 32 to 96
    test(1,400,128); // 32 to 160
    test(768, 1280, 1280); 
    test(32,224,224); // 32 to 256
    test(416,400,128); 
    test(416,96,96); // 256 to 512


    // These have different results for existing and proposed algorithm
    test(1,400,256); // 64 to 320 --> Existing algorithm uses 256 to 512
    test(300,512,256); // 384 to 640 --> Existing algorithm uses 512 to 768
    test(1, 6000, 4096); // 1024 to 5120 --> Existing algorithm fails
    test(512,8192,4096); // 1024 to 5120 --> Existing algorithm uses 4096 to 8192
    test(1,4096,256); // 64 to 320 --> Existing uses 256 to 512
    test(1,4096,512); // 128 to 640 --> Existing uses 256 to 512
    test(1,4096,1024); // 256 to 1280 --> Existing uses 1024 to 2048
    test(1234,5678,2345); // 1536 to 4096 --> Existing fails
    test(10000,20000,15000); // 12288 to 28672 --> Existing fails
}

fn test(parent_start: usize, parent_size: usize, min_region_size: usize) {
// Algorithm test. Change the following three parameters at will and be amazed at the results.
if let None = allocate_memory_region(
    parent_start, // parent_start
    parent_size, // parent_size
    min_region_size, // min_region_size
) {
    println!("Failed for values {}, {} and {}\n", parent_start, parent_size, min_region_size);
}
}

fn allocate_memory_region(
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
    let mut subregion_mask = None;
    
    // Region start always has to align to 32 bytes 
    if start % 32 != 0 {
        start += 32 - (start % 32);
    }

    // Regions must be at least 32 bytes
    if size < 32 {
        size = 32;
    }

    // We can only create an MPU region if the size is a power of two and it divides 
    // the start address. If this is not the case, the first thing we try to do to 
    // cover the memory region is to use a larger MPU region and expose certain subregions.
    if size.count_ones() > 1 || start % size != 0 {
        // Which (power-of-two) subregion size would align with the base
        // address?
        //
        // We find this by taking smallest binary substring of the base
        // address with exactly one bit:
        //
        //      1 << (start.trailing_zeros())
        let subregion_size = {
            let tz = start.trailing_zeros();
            if tz < 32 {
                (1 as usize) << tz
            } else {
                // This case means `start` is 0.                    
                let mut ceil = u32::next_power_of_two(size as u32) as usize;
                if ceil < 256 {
                    ceil = 256
                }
                ceil / 8
            }
        };

        // Once we have a subregion size, we get a region size by
        // multiplying it by the number of subregions per region.
        let underlying_region_size = subregion_size * 8;

        // Finally, we calculate the region base by finding the nearest
        // address below `start` that aligns with the region size.
        let underlying_region_start = start - (start % underlying_region_size);
            
        // If `size` doesn't align to the subregion size, extend it.
        if size % subregion_size != 0 {
            size += subregion_size - (size % subregion_size);
        }

        let end = start + size;
        let underlying_region_end = underlying_region_start + underlying_region_size;

        // To use subregions, the region must be at least 256 bytes. Also, we need
        // the amount of left over space in the region after `start` to be at least as 
        // large as the memory region we want to cover.
        if subregion_size >= 32 && underlying_region_end >= end {
            // The index of the first subregion to activate is the number of
            // regions between `region_start` (MPU) and `start` (memory).
            let min_subregion = (start - underlying_region_start) / subregion_size;

            // The index of the last subregion to activate is the number of
            // regions that fit in `len`, plus the `min_subregion`, minus one
            // (because subregions are zero-indexed).
            let max_subregion = min_subregion + size / subregion_size - 1;

            // Turn the min/max subregion into a bitfield where all bits are `1`
            // except for the bits whose index lie within
            // [min_subregion, max_subregion]
            //
            // Note: Rust ranges are minimum inclusive, maximum exclusive, hence
            // max_subregion + 1.
            let mask =
                (min_subregion..(max_subregion + 1)).fold(!0, |res, i| res & !(1 << i)) & 0xff;

            region_start = underlying_region_start;
            region_size = underlying_region_size;

            subregion_mask = Some(mask);

            println!(
                "Subregions used: {} through {}",
                min_subregion, max_subregion
            );
            println!("Underlying region start address: {}", underlying_region_start);
            println!("Underlying region size: {}", underlying_region_size);

        } else {
            // In this case, we can't use subregions to solve the alignment
            // problem. Instead, we will round up `size` to a power of two and
            // shift `start` up in memory to make it align with `size`.
            size = u32::next_power_of_two(size as u32) as usize;                
            start += size - (start % size); 

            region_start = start;
            region_size = size;
        }    
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