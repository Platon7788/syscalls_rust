/**
 * Debug test - compare ntdll direct call vs Rust wrapper
 * This tells us if the issue is in our syscall mechanism or parameter passing.
 */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <windows.h>
#include "syscalls.h"

/* Direct ntdll function pointers (via GetProcAddress) */
typedef NTSTATUS (NTAPI *PFN_NtQueryInformationProcess)(HANDLE, ULONG, PVOID, ULONG, PULONG);
typedef NTSTATUS (NTAPI *PFN_NtAllocateVirtualMemory)(HANDLE, PVOID*, ULONG, PSIZE_T, ULONG, ULONG);
typedef NTSTATUS (NTAPI *PFN_NtFreeVirtualMemory)(HANDLE, PVOID*, PSIZE_T, ULONG);
typedef NTSTATUS (NTAPI *PFN_NtDelayExecution)(BOOLEAN, PLARGE_INTEGER);
typedef NTSTATUS (NTAPI *PFN_NtCreateEvent)(PHANDLE, ACCESS_MASK, PVOID, ULONG, BOOLEAN);
typedef NTSTATUS (NTAPI *PFN_NtClose)(HANDLE);
typedef NTSTATUS (NTAPI *PFN_NtQuerySystemInformation)(ULONG, PVOID, ULONG, PULONG);
typedef NTSTATUS (NTAPI *PFN_NtProtectVirtualMemory)(HANDLE, PVOID*, PSIZE_T, ULONG, PULONG);
typedef NTSTATUS (NTAPI *PFN_NtOpenProcessToken)(HANDLE, ACCESS_MASK, PHANDLE);

#define LOAD(name) PFN_##name p##name = (PFN_##name)GetProcAddress(hNtdll, #name)

int main(void) {
    printf("=== NTDLL vs Rust Wrapper Comparison ===\n\n");

    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");
    if (!hNtdll) { printf("ntdll not found!\n"); return 1; }

    LOAD(NtQueryInformationProcess);
    LOAD(NtAllocateVirtualMemory);
    LOAD(NtFreeVirtualMemory);
    LOAD(NtDelayExecution);
    LOAD(NtCreateEvent);
    LOAD(NtClose);
    LOAD(NtQuerySystemInformation);
    LOAD(NtProtectVirtualMemory);
    LOAD(NtOpenProcessToken);

    int pass = 0, fail = 0;

    /* --- NtProtectVirtualMemory (5 args) --- */
    printf("--- NtProtectVirtualMemory (5 args) ---\n");
    {
        void* addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        SIZE_T sz = 4096;
        ULONG oldp = 0;

        void* a1 = addr; SIZE_T s1 = sz;
        NTSTATUS st_ntdll = pNtProtectVirtualMemory(GetCurrentProcess(), &a1, &s1, PAGE_READONLY, &oldp);
        printf("  NTDLL: 0x%08X oldp=0x%X\n", (unsigned)st_ntdll, oldp);
        pNtProtectVirtualMemory(GetCurrentProcess(), &a1, &s1, PAGE_READWRITE, &oldp);

        void* a2 = addr; SW3_SIZE_T s2 = sz; SW3_ULONG oldp2 = 0;
        NTSTATUS st_rust = SW3NtProtectVirtualMemory(GetCurrentProcess(), &a2, &s2, PAGE_READONLY, &oldp2);
        printf("  RUST:  0x%08X oldp=0x%X\n", (unsigned)st_rust, (unsigned)oldp2);
        if (st_rust == 0) { SW3NtProtectVirtualMemory(GetCurrentProcess(), &a2, &s2, PAGE_READWRITE, &oldp2); pass++; }
        else fail++;

        VirtualFree(addr, 0, MEM_RELEASE);
    }

    /* --- NtQueryInformationProcess (5 args) --- */
    printf("\n--- NtQueryInformationProcess (5 args) ---\n");
    {
        struct {
            LONG ExitStatus; void* PebBaseAddress; ULONG_PTR AffinityMask;
            LONG BasePriority; ULONG_PTR UniqueProcessId; ULONG_PTR InheritedFromUniqueProcessId;
        } pbi;
        ULONG retLen = 0;

        memset(&pbi, 0, sizeof(pbi));
        NTSTATUS st_ntdll = pNtQueryInformationProcess(GetCurrentProcess(), 0, &pbi, sizeof(pbi), &retLen);
        printf("  NTDLL: 0x%08X PID=%u\n", (unsigned)st_ntdll, (unsigned)pbi.UniqueProcessId);

        memset(&pbi, 0, sizeof(pbi)); retLen = 0;
        NTSTATUS st_rust = SW3NtQueryInformationProcess(GetCurrentProcess(), 0, &pbi, sizeof(pbi), &retLen);
        printf("  RUST:  0x%08X PID=%u\n", (unsigned)st_rust, (unsigned)pbi.UniqueProcessId);
        if (st_rust == 0) pass++; else fail++;
    }

    /* --- NtAllocateVirtualMemory (6 args) --- */
    printf("\n--- NtAllocateVirtualMemory (6 args) ---\n");
    {
        void* a1 = NULL; SIZE_T s1 = 4096;
        NTSTATUS st_ntdll = pNtAllocateVirtualMemory(GetCurrentProcess(), &a1, 0, &s1, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        printf("  NTDLL: 0x%08X addr=%p\n", (unsigned)st_ntdll, a1);
        if (a1) { SIZE_T fs = 0; pNtFreeVirtualMemory(GetCurrentProcess(), &a1, &fs, MEM_RELEASE); }

        void* a2 = NULL; SW3_SIZE_T s2 = 4096;
        NTSTATUS st_rust = SW3NtAllocateVirtualMemory(GetCurrentProcess(), &a2, 0, &s2, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        printf("  RUST:  0x%08X addr=%p\n", (unsigned)st_rust, a2);
        if (st_rust == 0) pass++; else fail++;
        if (a2) { SW3_SIZE_T fs = 0; SW3NtFreeVirtualMemory(GetCurrentProcess(), &a2, &fs, MEM_RELEASE); }
    }

    /* --- NtDelayExecution (2 args) --- */
    printf("\n--- NtDelayExecution (2 args) ---\n");
    {
        LARGE_INTEGER delay; delay.QuadPart = -10000LL;
        NTSTATUS st_ntdll = pNtDelayExecution(FALSE, &delay);
        printf("  NTDLL: 0x%08X\n", (unsigned)st_ntdll);

        delay.QuadPart = -10000LL;
        NTSTATUS st_rust = SW3NtDelayExecution(FALSE, (SW3_LARGE_INTEGER*)&delay);
        printf("  RUST:  0x%08X\n", (unsigned)st_rust);
        if (st_rust == 0) pass++; else fail++;
    }

    /* --- NtCreateEvent (5 args) --- */
    printf("\n--- NtCreateEvent (5 args) ---\n");
    {
        HANDLE h1 = NULL;
        SW3_OBJECT_ATTRIBUTES oa; memset(&oa, 0, sizeof(oa)); oa.Length = sizeof(oa);
        NTSTATUS st_ntdll = pNtCreateEvent(&h1, 0x001F0003, &oa, 1, FALSE);
        printf("  NTDLL: 0x%08X handle=%p\n", (unsigned)st_ntdll, h1);
        if (h1) pNtClose(h1);

        SW3_HANDLE h2 = NULL;
        memset(&oa, 0, sizeof(oa)); oa.Length = sizeof(oa);
        NTSTATUS st_rust = SW3NtCreateEvent(&h2, 0x001F0003, &oa, 1, FALSE);
        printf("  RUST:  0x%08X handle=%p\n", (unsigned)st_rust, h2);
        if (st_rust == 0) pass++; else fail++;
        if (h2) SW3NtClose(h2);
    }

    /* --- NtQuerySystemInformation (4 args) --- */
    printf("\n--- NtQuerySystemInformation (4 args) ---\n");
    {
        struct {
            ULONG Reserved, TimerResolution, PageSize, NumberOfPhysicalPages;
            ULONG Lowest, Highest, Granularity;
            ULONG_PTR MinAddr, MaxAddr, ActiveMask;
            UCHAR NumCPUs;
        } info;
        ULONG retLen = 0;

        memset(&info, 0, sizeof(info));
        NTSTATUS st_ntdll = pNtQuerySystemInformation(0, &info, sizeof(info), &retLen);
        printf("  NTDLL: 0x%08X PageSize=%u CPUs=%u\n", (unsigned)st_ntdll, info.PageSize, info.NumCPUs);

        memset(&info, 0, sizeof(info)); retLen = 0;
        NTSTATUS st_rust = SW3NtQuerySystemInformation(0, &info, sizeof(info), &retLen);
        printf("  RUST:  0x%08X PageSize=%u CPUs=%u\n", (unsigned)st_rust, info.PageSize, info.NumCPUs);
        if (st_rust == 0) pass++; else fail++;
    }

    /* --- NtOpenProcessToken (3 args) --- */
    printf("\n--- NtOpenProcessToken (3 args) ---\n");
    {
        HANDLE h1 = NULL;
        NTSTATUS st_ntdll = pNtOpenProcessToken(GetCurrentProcess(), 0x0008, &h1);
        printf("  NTDLL: 0x%08X handle=%p\n", (unsigned)st_ntdll, h1);
        if (h1) pNtClose(h1);

        SW3_HANDLE h2 = NULL;
        NTSTATUS st_rust = SW3NtOpenProcessToken(GetCurrentProcess(), 0x0008, &h2);
        printf("  RUST:  0x%08X handle=%p\n", (unsigned)st_rust, h2);
        if (st_rust == 0) pass++; else fail++;
        if (h2) SW3NtClose(h2);
    }

    printf("\n=== Results: %d pass, %d fail ===\n", pass, fail);
    return fail > 0 ? 1 : 0;
}
