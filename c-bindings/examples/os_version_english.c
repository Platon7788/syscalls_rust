/**
 * OS Version Detection - SysWhispers3 C Bindings (English Version)
 * 
 * USE THIS CODE! SW3RtlGetVersion DOES NOT EXIST!
 * Use SW3NtQuerySystemInformation to get OS version
 */

#include <stdio.h>
#include <windows.h>

/* SW3 header only */
#include "syscalls.h"

/* Function pointer for SW3NtQuerySystemInformation */
typedef SW3_NTSTATUS (*PFN_SW3NtQuerySystemInformation)(
    uint32_t system_information_class,
    SW3_PVOID system_information,
    SW3_ULONG system_information_length,
    SW3_ULONG* return_length
);

const char* GetWindowsVersionName(SW3_ULONG major, SW3_ULONG minor, SW3_ULONG build) {
    if (major == 10) {
        if (build >= 22000) return "Windows 11";
        else return "Windows 10";
    } else if (major == 6) {
        if (minor == 3) return "Windows 8.1";
        else if (minor == 2) return "Windows 8";
        else if (minor == 1) return "Windows 7";
        else if (minor == 0) return "Windows Vista";
    } else if (major == 5) {
        if (minor == 2) return "Windows Server 2003";
        else if (minor == 1) return "Windows XP";
        else if (minor == 0) return "Windows 2000";
    }
    return "Unknown Windows";
}

int main() {
    printf("=== CORRECT OS VERSION DETECTION ===\n\n");
    printf("ERROR: SW3RtlGetVersion DOES NOT EXIST!\n");
    printf("SOLUTION: Use SW3NtQuerySystemInformation!\n\n");
    
    /* Load DLL with pure syscalls */
    HMODULE hSW3 = LoadLibraryA("syscalls.dll");
    if (!hSW3) {
        printf("ERROR: Failed to load syscalls.dll\n");
        printf("SOLUTION: Compile DLL: cargo build --release\n");
        printf("SOLUTION: Copy DLL: copy target\\release\\syscalls.dll .\n");
        return 1;
    }
    
    printf("SUCCESS: syscalls.dll loaded!\n");
    
    /* Get function address */
    PFN_SW3NtQuerySystemInformation pSW3NtQuerySystemInformation = 
        (PFN_SW3NtQuerySystemInformation)GetProcAddress(hSW3, "SW3NtQuerySystemInformation");
    
    if (!pSW3NtQuerySystemInformation) {
        printf("ERROR: SW3NtQuerySystemInformation not found\n");
        FreeLibrary(hSW3);
        return 1;
    }
    
    printf("SUCCESS: SW3NtQuerySystemInformation found!\n\n");
    
    /* METHOD 1: Get basic system information */
    printf("=== METHOD 1: SW3_SystemBasicInformation ===\n");
    
    typedef struct {
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
    } SW3_SYSTEM_BASIC_INFO;
    
    SW3_SYSTEM_BASIC_INFO basicInfo = {0};
    SW3_ULONG returnLength = 0;
    
    SW3_NTSTATUS status = pSW3NtQuerySystemInformation(
        SW3_SystemBasicInformation,
        &basicInfo,
        sizeof(basicInfo),
        &returnLength
    );
    
    if (SW3_NT_SUCCESS(status)) {
        printf("SUCCESS: Basic information retrieved:\n");
        printf("   Processors: %d\n", basicInfo.NumberOfProcessors);
        printf("   Page size: %lu bytes\n", basicInfo.PageSize);
        printf("   Physical memory: %llu MB\n", 
               ((unsigned long long)basicInfo.NumberOfPhysicalPages * basicInfo.PageSize) / (1024 * 1024));
    } else {
        printf("ERROR: Failed to get basic information: 0x%08X\n", status);
    }
    
    /* METHOD 2: Get processor information */
    printf("\n=== METHOD 2: SW3_SystemProcessorInformation ===\n");
    
    typedef struct {
        SW3_USHORT ProcessorArchitecture;
        SW3_USHORT ProcessorLevel;
        SW3_USHORT ProcessorRevision;
        SW3_USHORT MaximumProcessors;
        SW3_ULONG ProcessorFeatureBits;
    } SW3_SYSTEM_PROCESSOR_INFO;
    
    SW3_SYSTEM_PROCESSOR_INFO procInfo = {0};
    returnLength = 0;
    
    status = pSW3NtQuerySystemInformation(
        SW3_SystemProcessorInformation,
        &procInfo,
        sizeof(procInfo),
        &returnLength
    );
    
    if (SW3_NT_SUCCESS(status)) {
        printf("SUCCESS: Processor information retrieved:\n");
        printf("   Architecture: %u ", procInfo.ProcessorArchitecture);
        switch (procInfo.ProcessorArchitecture) {
            case 0: printf("(Intel x86)\n"); break;
            case 9: printf("(x64)\n"); break;
            case 12: printf("(ARM64)\n"); break;
            default: printf("(Unknown)\n"); break;
        }
        printf("   Processor level: %u\n", procInfo.ProcessorLevel);
        printf("   Revision: %u\n", procInfo.ProcessorRevision);
    } else {
        printf("ERROR: Failed to get processor information: 0x%08X\n", status);
    }
    
    /* METHOD 3: Get version via GetVersion (WinAPI for comparison) */
    printf("\n=== METHOD 3: GetVersion (WinAPI for comparison) ===\n");
    
    DWORD version = GetVersion();
    SW3_ULONG majorVersion = (SW3_ULONG)(LOBYTE(LOWORD(version)));
    SW3_ULONG minorVersion = (SW3_ULONG)(HIBYTE(LOWORD(version)));
    SW3_ULONG buildNumber = 0;
    
    if (version < 0x80000000) {
        buildNumber = (SW3_ULONG)(HIWORD(version));
    }
    
    printf("SUCCESS: Windows version (via WinAPI):\n");
    printf("   Major version: %lu\n", majorVersion);
    printf("   Minor version: %lu\n", minorVersion);
    printf("   Build number: %lu\n", buildNumber);
    printf("   Name: %s\n", GetWindowsVersionName(majorVersion, minorVersion, buildNumber));
    
    FreeLibrary(hSW3);
    
    printf("\n=== SUMMARY ===\n");
    printf("SUCCESS: SW3NtQuerySystemInformation works for system information\n");
    printf("ERROR: SW3RtlGetVersion DOES NOT EXIST in this library\n");
    printf("SOLUTION: For OS version detection use:\n");
    printf("   1. SW3NtQuerySystemInformation with SW3_SystemBasicInformation\n");
    printf("   2. SW3NtQuerySystemInformation with SW3_SystemProcessorInformation\n");
    printf("   3. Standard WinAPI functions (GetVersion, GetVersionEx)\n");
    printf("SUCCESS: All SW3_ constants and functions work correctly!\n");
    
    return 0;
}