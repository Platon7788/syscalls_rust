/**
 * Simple OS Version Detection - SysWhispers3 C Bindings
 * 
 * Простой пример получения версии ОС без зависимостей от Rust runtime
 */

#include <windows.h>
#include "syscalls.h"

typedef struct _SW3_SYSTEM_BASIC_INFORMATION {
    SW3_ULONG Reserved;
    SW3_ULONG TimerResolution;
    SW3_ULONG PageSize;
    SW3_ULONG NumberOfPhysicalPages;
    SW3_ULONG LowestPhysicalPageNumber;
    SW3_ULONG HighestPhysicalPageNumber;
    SW3_ULONG AllocationGranularity;
    SW3_ULONG_PTR MinimumUserModeAddress;
    SW3_ULONG_PTR MaximumUserModeAddress;
    SW3_ULONG_PTR ActiveProcessorsAffinityMask;
    SW3_CCHAR NumberOfProcessors;
} SW3_SYSTEM_BASIC_INFORMATION;

int main() {
    SW3_SYSTEM_BASIC_INFORMATION basicInfo = {0};
    SW3_ULONG returnLength = 0;
    
    // Test that constants are defined
    SW3_ULONG testConstants[] = {
        SW3_SystemBasicInformation,
        SW3_SystemProcessorInformation,
        SW3_SystemTimeOfDayInformation,
        SW3_SystemKernelDebuggerInformation,
        SW3_MaxSystemInfoClass
    };
    
    // Test that function is available
    SW3_NTSTATUS status = SW3NtQuerySystemInformation(
        SW3_SystemBasicInformation,
        &basicInfo,
        sizeof(basicInfo),
        &returnLength
    );
    
    if (SW3_NT_SUCCESS(status)) {
        // Success - we can get system information
        return 0;
    } else {
        // Failed
        return 1;
    }
}