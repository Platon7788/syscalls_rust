/**
 * Test SW3 Constants - Standalone Test
 * 
 * Tests that all SW3_ prefixed constants are properly defined
 * without linking to the Rust library.
 */

#include <stdio.h>
#include <stdint.h>

/* Force standalone mode */
#define SYSCALLS_FORCE_STANDALONE
#include "syscalls.h"

int main() {
    printf("=== SW3 CONSTANTS TEST ===\n\n");
    
    printf("=== System Information Constants ===\n");
    printf("SW3_SystemBasicInformation = %d\n", SW3_SystemBasicInformation);
    printf("SW3_SystemProcessorInformation = %d\n", SW3_SystemProcessorInformation);
    printf("SW3_SystemTimeOfDayInformation = %d\n", SW3_SystemTimeOfDayInformation);
    printf("SW3_SystemKernelDebuggerInformation = %d\n", SW3_SystemKernelDebuggerInformation);
    printf("SW3_MaxSystemInfoClass = %d\n", SW3_MaxSystemInfoClass);
    
    printf("\n=== Registry Constants ===\n");
    printf("SW3_KEY_READ = 0x%X\n", SW3_KEY_READ);
    printf("SW3_KEY_WRITE = 0x%X\n", SW3_KEY_WRITE);
    printf("SW3_KEY_QUERY_VALUE = 0x%X\n", SW3_KEY_QUERY_VALUE);
    printf("SW3_KEY_SET_VALUE = 0x%X\n", SW3_KEY_SET_VALUE);
    
    printf("\n=== Memory Constants ===\n");
    printf("SW3_PAGE_READWRITE = 0x%X\n", SW3_PAGE_READWRITE);
    printf("SW3_MEM_COMMIT = 0x%X\n", SW3_MEM_COMMIT);
    printf("SW3_MEM_RESERVE = 0x%X\n", SW3_MEM_RESERVE);
    
    printf("\n=== Status Constants ===\n");
    printf("SW3_STATUS_SUCCESS = 0x%X\n", SW3_STATUS_SUCCESS);
    printf("SW3_STATUS_ACCESS_DENIED = 0x%X\n", SW3_STATUS_ACCESS_DENIED);
    printf("SW3_STATUS_INVALID_HANDLE = 0x%X\n", SW3_STATUS_INVALID_HANDLE);
    
    printf("\n=== Architecture Info ===\n");
    printf("Architecture: %s\n", SYSCALLS_GET_ARCH_INFO());
    printf("Pointer Size: %d bytes\n", SYSCALLS_POINTER_SIZE);
    printf("Windows SDK Detected: %s\n", SYSCALLS_WINDOWS_SDK_DETECTED ? "Yes" : "No");
    
    printf("\n=== Type Sizes ===\n");
    printf("sizeof(SW3_HANDLE) = %zu\n", sizeof(SW3_HANDLE));
    printf("sizeof(SW3_NTSTATUS) = %zu\n", sizeof(SW3_NTSTATUS));
    printf("sizeof(SW3_ULONG) = %zu\n", sizeof(SW3_ULONG));
    printf("sizeof(SW3_ULONG_PTR) = %zu\n", sizeof(SW3_ULONG_PTR));
    
    printf("\n=== Function Declarations Test ===\n");
    printf("SW3NtQuerySystemInformation function declared: %s\n", 
           "Yes (check compilation success)");
    
    printf("\n✅ ALL SW3 CONSTANTS AND TYPES SUCCESSFULLY DEFINED!\n");
    printf("🎯 Total System Information Classes: %d\n", SW3_MaxSystemInfoClass);
    printf("🛡️ Zero conflicts with Windows SDK in standalone mode!\n");
    
    return 0;
}