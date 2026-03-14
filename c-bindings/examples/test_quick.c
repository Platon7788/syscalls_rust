#include <stdio.h>
#include <stdint.h>
#include <windows.h>
#include "syscalls.h"

typedef NTSTATUS (NTAPI *PFN_NtDelayExecution)(BOOLEAN, PLARGE_INTEGER);
typedef NTSTATUS (NTAPI *PFN_NtClose)(HANDLE);
typedef NTSTATUS (NTAPI *PFN_NtProtectVirtualMemory)(HANDLE, PVOID*, PULONG, ULONG, PULONG);

int main(void) {
    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");

    printf("=== Quick WoW64 Test ===\n\n");

    /* 0-arg: NtYieldExecution */
    NTSTATUS st = SW3NtYieldExecution();
    printf("0-arg NtYieldExecution: 0x%08X %s\n", (unsigned)st,
           (st == 0 || st == 0x40000024) ? "OK" : "FAIL");

    /* 1-arg: NtClose(invalid) */
    st = SW3NtClose((SW3_HANDLE)0x12345678);
    printf("1-arg NtClose(bad):     0x%08X %s\n", (unsigned)st,
           st == (NTSTATUS)0xC0000008 ? "OK" : "FAIL");

    /* 1-arg: NtQuerySystemTime */
    LARGE_INTEGER t = {0};
    st = SW3NtQuerySystemTime((SW3_LARGE_INTEGER*)&t);
    printf("1-arg NtQuerySysTime:   0x%08X %s (t=%lld)\n", (unsigned)st,
           st == 0 ? "OK" : "FAIL", t.QuadPart);

    /* 2-arg: NtDelayExecution with 0 delay */
    LARGE_INTEGER delay;
    delay.QuadPart = 0; /* immediate return */
    st = SW3NtDelayExecution(0, (SW3_LARGE_INTEGER*)&delay);
    printf("2-arg NtDelayExec(0):   0x%08X %s\n", (unsigned)st,
           st == 0 ? "OK" : "FAIL");

    /* 5-arg: NtProtectVirtualMemory */
    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    if (addr) {
        ULONG sz = 4096;
        ULONG oldp = 0;
        HANDLE proc = (HANDLE)-1;
        st = SW3NtProtectVirtualMemory((SW3_HANDLE)proc, &addr, (unsigned long*)&sz, 0x04, (unsigned long*)&oldp);
        printf("5-arg NtProtectVM:      0x%08X %s (oldp=%lu)\n", (unsigned)st,
               st == 0 ? "OK" : "FAIL", oldp);
        VirtualFree(addr, 0, MEM_RELEASE);
    }

    /* 6-arg: NtAllocateVirtualMemory */
    PVOID alloc_addr = NULL;
    SIZE_T alloc_size = 4096;
    st = SW3NtAllocateVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, 0,
                                     (unsigned long*)&alloc_size, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    printf("6-arg NtAllocateVM:     0x%08X %s (addr=%p)\n", (unsigned)st,
           st == 0 ? "OK" : "FAIL", alloc_addr);
    if (alloc_addr) {
        SW3NtFreeVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, (unsigned long*)&alloc_size, MEM_RELEASE);
    }

    printf("\n=== Done ===\n");
    return 0;
}
