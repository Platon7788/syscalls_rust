#include <stdio.h>
#include <stdint.h>
#include <windows.h>
#include "syscalls.h"

extern uint32_t SW3Sw3GetSyscallNumber(uint32_t function_hash);

static int passed = 0;
static int failed = 0;
static int total = 0;

static void check(const char *name, NTSTATUS status, NTSTATUS expected, const char *extra) {
    total++;
    if (status == expected) {
        passed++;
        printf("  [PASS] %s: 0x%08X %s\n", name, (unsigned)status, extra ? extra : "");
    } else {
        failed++;
        printf("  [FAIL] %s: 0x%08X (expected 0x%08X) %s\n", name, (unsigned)status,
               (unsigned)expected, extra ? extra : "");
    }
}

static void check_ok(const char *name, NTSTATUS status, const char *extra) {
    check(name, status, 0, extra);
}

int main(void) {
    printf("=== Comprehensive WoW64 Syscall Test ===\n\n");

    /* ---- 0-arg functions ---- */
    printf("--- 0-arg functions ---\n");
    {
        NTSTATUS st = SW3NtYieldExecution();
        total++; 
        if (st == 0 || st == 0x40000024) { passed++; printf("  [PASS] NtYieldExecution: 0x%08X\n", (unsigned)st); }
        else { failed++; printf("  [FAIL] NtYieldExecution: 0x%08X\n", (unsigned)st); }
    }
    check_ok("NtTestAlert", SW3NtTestAlert(), "");

    /* ---- 1-arg functions ---- */
    printf("\n--- 1-arg functions ---\n");
    check("NtClose(bad)", SW3NtClose((SW3_HANDLE)0x12345678), (NTSTATUS)0xC0000008, "STATUS_INVALID_HANDLE");
    
    {
        LARGE_INTEGER t = {0};
        check_ok("NtQuerySystemTime", SW3NtQuerySystemTime((SW3_LARGE_INTEGER*)&t), "");
        printf("    t=%lld\n", t.QuadPart);
    }

    /* ---- 2-arg functions ---- */
    printf("\n--- 2-arg functions ---\n");
    {
        LARGE_INTEGER delay;
        delay.QuadPart = -1;
        check_ok("NtDelayExecution", SW3NtDelayExecution(0, (SW3_LARGE_INTEGER*)&delay), "");
    }

    /* ---- 3-arg functions ---- */
    printf("\n--- 3-arg functions ---\n");
    {
        PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        if (addr) {
            SIZE_T free_size = 0;
            NTSTATUS st = SW3NtFreeVirtualMemory((SW3_HANDLE)(HANDLE)-1, &addr, (unsigned long*)&free_size, MEM_RELEASE);
            check_ok("NtFreeVirtualMemory", st, "");
        }
    }

    /* ---- 4-arg functions ---- */
    printf("\n--- 4-arg functions ---\n");
    {
        BYTE buf[256] = {0};
        ULONG ret_len = 0;
        NTSTATUS st = SW3NtQuerySystemInformation(0 /*SystemBasicInformation*/, buf, sizeof(buf), (unsigned long*)&ret_len);
        check_ok("NtQuerySystemInformation(Basic)", st, "");
        if (st == 0) {
            /* SystemBasicInformation: PageSize at offset 4, NumberOfProcessors at offset 40 */
            ULONG page_size = *(ULONG*)(buf + 4);
            printf("    PageSize=%lu, RetLen=%lu\n", page_size, ret_len);
        }
    }
    {
        ULONG exit_code = 0;
        ULONG ret_len = 0;
        /* ProcessBasicInformation = 0, needs PROCESS_BASIC_INFORMATION (24 bytes on x86) */
        BYTE pbi[24] = {0};
        NTSTATUS st = SW3NtQueryInformationProcess((SW3_HANDLE)(HANDLE)-1, 0, pbi, sizeof(pbi), (unsigned long*)&ret_len);
        check_ok("NtQueryInformationProcess(Basic)", st, "");
        if (st == 0) {
            ULONG pid = *(ULONG*)(pbi + 16);
            printf("    PID=%lu\n", pid);
        }
    }

    /* ---- 5-arg functions ---- */
    printf("\n--- 5-arg functions ---\n");
    {
        HANDLE hEvent = NULL;
        NTSTATUS st = SW3NtCreateEvent((SW3_HANDLE*)&hEvent, EVENT_ALL_ACCESS, NULL, 1 /*NotificationEvent*/, 0);
        check_ok("NtCreateEvent", st, "");
        if (st == 0) {
            printf("    hEvent=%p\n", hEvent);
            SW3NtClose((SW3_HANDLE)hEvent);
        }
    }
    {
        PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        if (addr) {
            ULONG sz = 4096;
            ULONG oldp = 0;
            NTSTATUS st = SW3NtProtectVirtualMemory((SW3_HANDLE)(HANDLE)-1, &addr, (unsigned long*)&sz, PAGE_READWRITE, (unsigned long*)&oldp);
            check_ok("NtProtectVirtualMemory", st, "");
            printf("    oldp=%lu\n", oldp);
            VirtualFree(addr, 0, MEM_RELEASE);
        }
    }
    {
        BYTE buf[512] = {0};
        ULONG ret_len = 0;
        /* SystemProcessorPerformanceInformation = 8 */
        NTSTATUS st = SW3NtQuerySystemInformation(8, buf, sizeof(buf), (unsigned long*)&ret_len);
        total++;
        if (st == 0 || st == (NTSTATUS)0xC0000004 /*STATUS_INFO_LENGTH_MISMATCH*/) {
            passed++;
            printf("  [PASS] NtQuerySysInfo(ProcPerf): 0x%08X retLen=%lu\n", (unsigned)st, ret_len);
        } else {
            failed++;
            printf("  [FAIL] NtQuerySysInfo(ProcPerf): 0x%08X\n", (unsigned)st);
        }
    }

    /* ---- 6-arg functions ---- */
    printf("\n--- 6-arg functions ---\n");
    {
        PVOID alloc_addr = NULL;
        SIZE_T alloc_size = 4096;
        NTSTATUS st = SW3NtAllocateVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, 0,
                                                  (unsigned long*)&alloc_size, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        check_ok("NtAllocateVirtualMemory", st, "");
        if (st == 0) {
            printf("    addr=%p size=%lu\n", alloc_addr, (unsigned long)alloc_size);
            /* Write and read back */
            *(volatile DWORD*)alloc_addr = 0xDEADBEEF;
            DWORD val = *(volatile DWORD*)alloc_addr;
            total++;
            if (val == 0xDEADBEEF) { passed++; printf("  [PASS] Memory read/write OK\n"); }
            else { failed++; printf("  [FAIL] Memory read/write: got 0x%08X\n", val); }

            /* Protect to PAGE_READONLY */
            ULONG sz = 4096;
            ULONG oldp = 0;
            st = SW3NtProtectVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, (unsigned long*)&sz, PAGE_READONLY, (unsigned long*)&oldp);
            check_ok("NtProtectVM(READONLY)", st, "");
            printf("    oldp=%lu (should be 4=PAGE_READWRITE)\n", oldp);

            /* Protect back to RW */
            sz = 4096;
            oldp = 0;
            st = SW3NtProtectVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, (unsigned long*)&sz, PAGE_READWRITE, (unsigned long*)&oldp);
            check_ok("NtProtectVM(back to RW)", st, "");
            printf("    oldp=%lu (should be 2=PAGE_READONLY)\n", oldp);

            /* Free */
            SIZE_T free_sz = 0;
            st = SW3NtFreeVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, (unsigned long*)&free_sz, MEM_RELEASE);
            check_ok("NtFreeVirtualMemory", st, "");
        }
    }

    /* ---- 7-arg: NtCreateSection ---- */
    printf("\n--- 7-arg functions ---\n");
    {
        HANDLE hSection = NULL;
        LARGE_INTEGER maxSize;
        maxSize.QuadPart = 4096;
        NTSTATUS st = SW3NtCreateSection((SW3_HANDLE*)&hSection, SECTION_ALL_ACCESS, NULL,
                                          (SW3_LARGE_INTEGER*)&maxSize, PAGE_READWRITE, SEC_COMMIT, NULL);
        check_ok("NtCreateSection", st, "");
        if (st == 0) {
            printf("    hSection=%p\n", hSection);
            SW3NtClose((SW3_HANDLE)hSection);
        }
    }

    /* ---- 8-arg: NtCreateFile ---- */
    printf("\n--- 8+ arg functions ---\n");
    {
        /* NtQueryVirtualMemory (6 args) */
        PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        if (addr) {
            BYTE mbi[28] = {0}; /* MEMORY_BASIC_INFORMATION for x86 is 28 bytes */
            SIZE_T ret_len = 0;
            NTSTATUS st = SW3NtQueryVirtualMemory((SW3_HANDLE)(HANDLE)-1, addr, 0 /*MemoryBasicInformation*/,
                                                   mbi, sizeof(mbi), (unsigned long*)&ret_len);
            check_ok("NtQueryVirtualMemory", st, "");
            if (st == 0) {
                ULONG protect = *(ULONG*)(mbi + 20);
                ULONG state = *(ULONG*)(mbi + 16);
                printf("    State=0x%X Protect=0x%X RetLen=%lu\n", state, protect, (unsigned long)ret_len);
            }
            VirtualFree(addr, 0, MEM_RELEASE);
        }
    }
    {
        /* NtWriteVirtualMemory (5 args) */
        PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        if (addr) {
            DWORD data = 0x12345678;
            ULONG written = 0;
            NTSTATUS st = SW3NtWriteVirtualMemory((SW3_HANDLE)(HANDLE)-1, addr, &data, sizeof(data), (unsigned long*)&written);
            check_ok("NtWriteVirtualMemory", st, "");
            printf("    written=%lu, *addr=0x%08X\n", written, *(DWORD*)addr);
            
            /* NtReadVirtualMemory (5 args) */
            DWORD read_val = 0;
            ULONG bytes_read = 0;
            st = SW3NtReadVirtualMemory((SW3_HANDLE)(HANDLE)-1, addr, &read_val, sizeof(read_val), (unsigned long*)&bytes_read);
            check_ok("NtReadVirtualMemory", st, "");
            printf("    read=0x%08X bytes=%lu\n", read_val, bytes_read);
            
            VirtualFree(addr, 0, MEM_RELEASE);
        }
    }

    /* ---- Summary ---- */
    printf("\n========================================\n");
    printf("TOTAL: %d  PASSED: %d  FAILED: %d\n", total, passed, failed);
    printf("========================================\n");

    return failed > 0 ? 1 : 0;
}
