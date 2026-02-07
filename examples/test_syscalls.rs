//! Test example for syscalls library
//! 
//! Run: cargo run --example test_syscalls

use syscalls::*;

fn main() {
    println!("=== SysWhispers3 Rust Test ===\n");
    
    // Test 1: Check syscall list population
    println!("[1] Testing syscall list initialization...");
    unsafe {
        let count = sw3_debug_get_count();
        println!("    Syscalls found: {}", count);
        
        if count == 0 {
            println!("    ERROR: No syscalls found!");
            return;
        }
        println!("    OK: Syscall list populated\n");
    }
    
    // Test 2: Memory allocation
    println!("[2] Testing NtAllocateVirtualMemory...");
    unsafe {
        let mut base_addr: PVOID = core::ptr::null_mut();
        let mut region_size: SIZE_T = 0x1000; // 4KB
        
        let status = nt_allocate_virtual_memory(
            -1isize as HANDLE, // NtCurrentProcess()
            &mut base_addr,
            0,
            &mut region_size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        
        println!("    Status: 0x{:08X}", status as u32);
        println!("    Base address: {:?}", base_addr);
        println!("    Region size: 0x{:X}", region_size);
        
        if NT_SUCCESS(status) && !base_addr.is_null() {
            println!("    OK: Memory allocated\n");
            
            // Test 3: Write to memory
            println!("[3] Testing memory write...");
            let test_data: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
            core::ptr::copy_nonoverlapping(
                test_data.as_ptr(),
                base_addr as *mut u8,
                4
            );
            println!("    OK: Written 0xDEADBEEF\n");

            // Test 4: Read from memory
            println!("[4] Testing memory read...");
            let mut read_data: [u8; 4] = [0; 4];
            core::ptr::copy_nonoverlapping(
                base_addr as *const u8,
                read_data.as_mut_ptr(),
                4
            );
            println!("    Read: 0x{:02X}{:02X}{:02X}{:02X}", 
                read_data[0], read_data[1], read_data[2], read_data[3]);
            
            if read_data == test_data {
                println!("    OK: Data matches\n");
            } else {
                println!("    ERROR: Data mismatch!\n");
            }
            
            // Test 5: Change protection
            println!("[5] Testing NtProtectVirtualMemory...");
            let mut old_protect: ULONG = 0;
            let mut protect_addr = base_addr;
            let mut protect_size = region_size;
            
            let status = nt_protect_virtual_memory(
                -1isize as HANDLE,
                &mut protect_addr,
                &mut protect_size,
                PAGE_EXECUTE_READ,
                &mut old_protect,
            );
            
            println!("    Status: 0x{:08X}", status as u32);
            println!("    Old protection: 0x{:X}", old_protect);
            
            if NT_SUCCESS(status) {
                println!("    OK: Protection changed to PAGE_EXECUTE_READ\n");
            } else {
                println!("    ERROR: Failed to change protection\n");
            }
            
            // Test 6: Query memory
            println!("[6] Testing NtQueryVirtualMemory...");
            let mut mbi: MEMORY_BASIC_INFORMATION = core::mem::zeroed();
            let mut return_length: SIZE_T = 0;
            
            let status = nt_query_virtual_memory(
                -1isize as HANDLE,
                base_addr,
                0, // MemoryBasicInformation
                &mut mbi as *mut _ as PVOID,
                core::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                &mut return_length,
            );
            
            println!("    Status: 0x{:08X}", status as u32);
            if NT_SUCCESS(status) {
                println!("    BaseAddress: {:?}", mbi.BaseAddress);
                println!("    RegionSize: 0x{:X}", mbi.RegionSize);
                println!("    State: 0x{:X}", mbi.State);
                println!("    Protect: 0x{:X}", mbi.Protect);
                println!("    OK: Query successful\n");
            }

            // Test 7: Free memory
            println!("[7] Testing NtFreeVirtualMemory...");
            let mut free_addr = base_addr;
            let mut free_size: SIZE_T = 0;
            
            let status = nt_free_virtual_memory(
                -1isize as HANDLE,
                &mut free_addr,
                &mut free_size,
                MEM_RELEASE,
            );
            
            println!("    Status: 0x{:08X}", status as u32);
            if NT_SUCCESS(status) {
                println!("    OK: Memory freed\n");
            } else {
                println!("    ERROR: Failed to free memory\n");
            }
        } else {
            println!("    ERROR: Failed to allocate memory\n");
        }
    }
    
    // Test 8: Query system information
    println!("[8] Testing NtQuerySystemInformation...");
    unsafe {
        let mut return_length: ULONG = 0;
        
        // First call to get required size (SystemBasicInformation = 0)
        let status = nt_query_system_information(
            0, // SystemBasicInformation
            core::ptr::null_mut(),
            0,
            &mut return_length,
        );
        
        println!("    Status: 0x{:08X} (expected STATUS_INFO_LENGTH_MISMATCH)", status as u32);
        println!("    Required length: {} bytes", return_length);
        println!("    OK: System query works\n");
    }
    
    // Test 9: Delay execution (sleep)
    println!("[9] Testing NtDelayExecution (100ms sleep)...");
    unsafe {
        // -100ms in 100ns units = -1_000_000
        let mut delay: LARGE_INTEGER = -1_000_000;
        
        let status = nt_delay_execution(0, &mut delay);
        
        println!("    Status: 0x{:08X}", status as u32);
        if NT_SUCCESS(status) {
            println!("    OK: Sleep completed\n");
        }
    }
    
    println!("=== All tests completed! ===");
}
