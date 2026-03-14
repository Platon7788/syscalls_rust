/**
 * Comprehensive WoW64 Syscall Test
 * Tests key syscall functions under WoW64 (x86 on x64 Windows).
 * Hashes extracted from lib.rs (seed 0xB8A54425).
 */

#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <windows.h>
#include "syscalls.h"

extern uint32_t SW3Sw3DebugGetCount(void);
extern uint32_t SW3Sw3GetSyscallNumber(uint32_t function_hash);
extern void*    SW3Sw3DebugGetSyscallAddr(size_t index);
extern uint32_t SW3Sw3DebugGetHash(size_t index);

static int g_pass = 0, g_fail = 0;

#define NT_SUCCESS(s) ((int)(s) >= 0)

#define TEST(name, expr) do { \
    printf("  %-55s", name); \
    fflush(stdout); \
    SW3_NTSTATUS _st = (expr); \
    if (NT_SUCCESS(_st)) { printf("OK (0x%08X)\n", (unsigned)_st); g_pass++; } \
    else { printf("FAIL (0x%08X)\n", (unsigned)_st); g_fail++; } \
} while(0)

#define TEST_BOOL(name, expr) do { \
    printf("  %-55s", name); \
    fflush(stdout); \
    if (expr) { printf("OK\n"); g_pass++; } \
    else { printf("FAIL\n"); g_fail++; } \
} while(0)

static int IsWow64(void) {
    BOOL b = FALSE;
    typedef BOOL (WINAPI *FN)(HANDLE, PBOOL);
    FN fn = (FN)GetProcAddress(GetModuleHandleA("kernel32"), "IsWow64Process");
    if (fn) fn(GetCurrentProcess(), &b);
    return b;
}

static void test_infrastructure(void) {
    printf("\n--- Infrastructure ---\n");

    uint32_t count = SW3Sw3DebugGetCount();
    printf("  Syscall table: %u entries\n", count);
    TEST_BOOL("Table populated (>400)", count > 400);

    /* Correct hashes from lib.rs (seed 0xB8A54425) */
    struct { const char* name; uint32_t hash; } funcs[] = {
        {"NtProtectVirtualMemory",    0x039F2B0F},
        {"NtAllocateVirtualMemory",   0x0F993937},
        {"NtFreeVirtualMemory",       0x19820515},
        {"NtQueryVirtualMemory",      0x0D961D39},
        {"NtReadVirtualMemory",       0x07951B01},
        {"NtWriteVirtualMemory",      0x0B9B031B},
        {"NtClose",                   0x0C9DF7C3},
        {"NtCreateFile",              0x46F4885F},
        {"NtOpenProcess",             0x0FAC0030},
        {"NtQueryInformationProcess", 0xE2A1F32F},
        {"NtQuerySystemInformation",  0x5ECC7D99},
        {"NtDelayExecution",          0xF269F2FB},
        {"NtFlushInstructionCache",   0x6D2E9F79},
        {"NtCreateSection",           0x02AB0433},
        {"NtOpenSection",             0xCAE125FC},
        {"NtCreateEvent",             0x0E4BF4CD},
        {"NtOpenKey",                 0x763667AF},
        {"NtQueryValueKey",           0x1C58D904},
        {"NtDuplicateObject",         0x02209C2D},
        {"NtSuspendThread",           0x842D9693},
        {"NtResumeThread",            0x93300A16},
        {"NtGetContextThread",        0x0B98072B},
        {"NtSetContextThread",        0x321EFD35},
        {"NtCreateThreadEx",          0xD63F0B79},
        {"NtQueryInformationThread",  0x069603F7},
        {"NtOpenProcessToken",        0x6F815D02},
        {"NtQueryInformationToken",   0xE3B93D15},
        {"NtSetEvent",                0x594BBC52},
        {"NtResetEvent",              0x10F07518},
        {"NtPulseEvent",              0x26012393},
        {"NtWaitForSingleObject",     0x663C6CA2},
        {"NtCreateMutant",            0x34B628D6},
        {"NtCreateSemaphore",         0xCA982019},
        {"NtReleaseSemaphore",        0x3CA80630},
        {"NtMapViewOfSection",        0x6009669D},
        {"NtUnmapViewOfSection",      0x1688341D},
        {"NtQuerySystemTime",         0x0CAB757F},
        {"NtYieldExecution",          0x0C922A07},
        {"NtTestAlert",               0x14A61334},
    };
    int nfuncs = sizeof(funcs)/sizeof(funcs[0]);

    printf("\n  SSN resolution (%d functions):\n", nfuncs);
    int resolved = 0;
    for (int i = 0; i < nfuncs; i++) {
        uint32_t ssn = SW3Sw3GetSyscallNumber(funcs[i].hash);
        if (ssn != 0xFFFFFFFF) {
            resolved++;
            g_pass++;
        } else {
            printf("  !! %-35s hash=0x%08X NOT FOUND!\n", funcs[i].name, funcs[i].hash);
            g_fail++;
        }
    }
    printf("  Resolved: %d/%d\n", resolved, nfuncs);
}

static void test_memory_operations(void) {
    printf("\n--- Memory Operations ---\n");

    /* NtAllocateVirtualMemory */
    void* alloc1 = NULL;
    SW3_SIZE_T size1 = 4096;
    TEST("NtAllocateVirtualMemory (4K RW)",
         SW3NtAllocateVirtualMemory(GetCurrentProcess(), &alloc1, 0, &size1,
                                     MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));

    if (alloc1) {
        *(volatile uint32_t*)alloc1 = 0xDEADBEEF;
        TEST_BOOL("  Write/Read 0xDEADBEEF", *(volatile uint32_t*)alloc1 == 0xDEADBEEF);

        /* NtProtectVirtualMemory */
        void* pBase = alloc1;
        SW3_SIZE_T pSize = 4096;
        SW3_ULONG oldProt = 0;
        TEST("NtProtectVirtualMemory (RW->RO)",
             SW3NtProtectVirtualMemory(GetCurrentProcess(), &pBase, &pSize,
                                        PAGE_READONLY, &oldProt));

        pBase = alloc1;
        pSize = 4096;
        TEST("NtProtectVirtualMemory (RO->RW)",
             SW3NtProtectVirtualMemory(GetCurrentProcess(), &pBase, &pSize,
                                        PAGE_READWRITE, &oldProt));

        /* NtQueryVirtualMemory */
        SW3_MEMORY_BASIC_INFORMATION mbi;
        memset(&mbi, 0, sizeof(mbi));
        SW3_SIZE_T retLen = 0;
        TEST("NtQueryVirtualMemory",
             SW3NtQueryVirtualMemory(GetCurrentProcess(), alloc1, 0,
                                      &mbi, sizeof(mbi), &retLen));
        if (retLen > 0)
            printf("    Base=%p Size=0x%X Protect=0x%X\n",
                   mbi.BaseAddress, (unsigned)mbi.RegionSize, (unsigned)mbi.Protect);

        /* NtReadVirtualMemory */
        uint32_t readBuf = 0;
        SW3_SIZE_T bytesRead = 0;
        TEST("NtReadVirtualMemory",
             SW3NtReadVirtualMemory(GetCurrentProcess(), alloc1, &readBuf, 4, &bytesRead));
        TEST_BOOL("  Read matches 0xDEADBEEF", readBuf == 0xDEADBEEF);

        /* NtWriteVirtualMemory */
        uint32_t writeVal = 0xCAFEBABE;
        SW3_SIZE_T bytesWritten = 0;
        TEST("NtWriteVirtualMemory",
             SW3NtWriteVirtualMemory(GetCurrentProcess(), alloc1, &writeVal, 4, &bytesWritten));
        TEST_BOOL("  Written matches 0xCAFEBABE", *(volatile uint32_t*)alloc1 == 0xCAFEBABE);

        /* NtFlushInstructionCache */
        TEST("NtFlushInstructionCache",
             SW3NtFlushInstructionCache(GetCurrentProcess(), alloc1, 4096));

        /* Allocate RWX */
        void* alloc2 = NULL;
        SW3_SIZE_T size2 = 4096;
        TEST("NtAllocateVirtualMemory (4K RWX)",
             SW3NtAllocateVirtualMemory(GetCurrentProcess(), &alloc2, 0, &size2,
                                         MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE));
        if (alloc2) {
            SW3_SIZE_T freeSize2 = 0;
            TEST("NtFreeVirtualMemory (RWX)",
                 SW3NtFreeVirtualMemory(GetCurrentProcess(), &alloc2, &freeSize2, MEM_RELEASE));
        }

        /* NtFreeVirtualMemory */
        SW3_SIZE_T freeSize = 0;
        TEST("NtFreeVirtualMemory (main)",
             SW3NtFreeVirtualMemory(GetCurrentProcess(), &alloc1, &freeSize, MEM_RELEASE));
    }
}

static void test_process_thread(void) {
    printf("\n--- Process/Thread ---\n");

    /* NtQueryInformationProcess - BasicInformation (0) */
    struct {
        LONG ExitStatus;
        void* PebBaseAddress;
        ULONG_PTR AffinityMask;
        LONG BasePriority;
        ULONG_PTR UniqueProcessId;
        ULONG_PTR InheritedFromUniqueProcessId;
    } pbi;
    memset(&pbi, 0, sizeof(pbi));
    SW3_ULONG retLen = 0;
    TEST("NtQueryInformationProcess (BasicInfo)",
         SW3NtQueryInformationProcess(GetCurrentProcess(), 0, &pbi, sizeof(pbi), &retLen));
    if (retLen > 0)
        printf("    PID=%u PEB=%p\n", (unsigned)pbi.UniqueProcessId, pbi.PebBaseAddress);

    /* NtQueryInformationThread - BasicInformation (0) */
    struct {
        LONG ExitStatus;
        void* TebBaseAddress;
        struct { ULONG_PTR UniqueProcess; ULONG_PTR UniqueThread; } ClientId;
        ULONG_PTR AffinityMask;
        LONG Priority;
        LONG BasePriority;
    } tbi;
    memset(&tbi, 0, sizeof(tbi));
    retLen = 0;
    TEST("NtQueryInformationThread (BasicInfo)",
         SW3NtQueryInformationThread(GetCurrentThread(), 0, &tbi, sizeof(tbi), &retLen));
    if (retLen > 0)
        printf("    TID=%u TEB=%p\n", (unsigned)tbi.ClientId.UniqueThread, tbi.TebBaseAddress);

    /* NtQueryInformationThread - ThreadTimes (1) */
    struct {
        LARGE_INTEGER CreateTime, ExitTime, KernelTime, UserTime;
    } tt;
    memset(&tt, 0, sizeof(tt));
    retLen = 0;
    TEST("NtQueryInformationThread (ThreadTimes)",
         SW3NtQueryInformationThread(GetCurrentThread(), 1, &tt, sizeof(tt), &retLen));

    /* NtDelayExecution (sleep 1ms) */
    LARGE_INTEGER delay;
    delay.QuadPart = -10000LL;
    TEST("NtDelayExecution (1ms)",
         SW3NtDelayExecution(FALSE, (SW3_LARGE_INTEGER*)&delay));

    /* NtYieldExecution */
    TEST("NtYieldExecution",
         SW3NtYieldExecution());

    /* NtDuplicateObject */
    SW3_HANDLE dupHandle = NULL;
    TEST("NtDuplicateObject (dup process handle)",
         SW3NtDuplicateObject(GetCurrentProcess(), GetCurrentProcess(),
                               GetCurrentProcess(), &dupHandle,
                               0, 0, 2 /* DUPLICATE_SAME_ACCESS */));
    if (dupHandle) {
        TEST("NtClose (dup handle)", SW3NtClose(dupHandle));
    }
}

static void test_events_sync(void) {
    printf("\n--- Events/Sync ---\n");

    SW3_OBJECT_ATTRIBUTES oa;
    memset(&oa, 0, sizeof(oa));
    oa.Length = sizeof(oa);

    /* NtCreateEvent */
    SW3_HANDLE hEvent = NULL;
    TEST("NtCreateEvent",
         SW3NtCreateEvent(&hEvent, 0x001F0003, &oa, 1 /*NotificationEvent*/, FALSE));
    if (hEvent) {
        SW3_ULONG prev = 0;
        TEST("NtSetEvent",       SW3NtSetEvent(hEvent, &prev));
        TEST("NtResetEvent",     SW3NtResetEvent(hEvent, &prev));
        TEST("NtPulseEvent",     SW3NtPulseEvent(hEvent, &prev));

        LARGE_INTEGER timeout;
        timeout.QuadPart = 0;
        TEST("NtWaitForSingleObject (0ms)",
             SW3NtWaitForSingleObject(hEvent, FALSE, (SW3_LARGE_INTEGER*)&timeout));
        TEST("NtClose (event)",  SW3NtClose(hEvent));
    }

    /* NtCreateMutant */
    SW3_HANDLE hMutex = NULL;
    memset(&oa, 0, sizeof(oa)); oa.Length = sizeof(oa);
    TEST("NtCreateMutant",
         SW3NtCreateMutant(&hMutex, 0x001F0001, &oa, FALSE));
    if (hMutex) TEST("NtClose (mutant)", SW3NtClose(hMutex));

    /* NtCreateSemaphore */
    SW3_HANDLE hSem = NULL;
    memset(&oa, 0, sizeof(oa)); oa.Length = sizeof(oa);
    TEST("NtCreateSemaphore",
         SW3NtCreateSemaphore(&hSem, 0x001F0003, &oa, 0, 1));
    if (hSem) {
        SW3_ULONG prevCount = 0;
        TEST("NtReleaseSemaphore", SW3NtReleaseSemaphore(hSem, 1, &prevCount));
        TEST("NtClose (semaphore)", SW3NtClose(hSem));
    }
}

static void test_section(void) {
    printf("\n--- Section ---\n");

    SW3_OBJECT_ATTRIBUTES oa;
    memset(&oa, 0, sizeof(oa));
    oa.Length = sizeof(oa);

    LARGE_INTEGER maxSize;
    maxSize.QuadPart = 4096;

    SW3_HANDLE hSection = NULL;
    TEST("NtCreateSection (4K pagefile)",
         SW3NtCreateSection(&hSection, 0x000F001F,
                             &oa, (SW3_LARGE_INTEGER*)&maxSize,
                             PAGE_READWRITE, 0x08000000 /*SEC_COMMIT*/, NULL));
    if (hSection) {
        void* viewBase = NULL;
        SW3_SIZE_T viewSize = 0;
        TEST("NtMapViewOfSection",
             SW3NtMapViewOfSection(hSection, GetCurrentProcess(), &viewBase,
                                    0, 0, NULL, &viewSize, 1, 0, PAGE_READWRITE));
        if (viewBase) {
            *(volatile uint32_t*)viewBase = 0x12345678;
            TEST_BOOL("  Write/Read shared memory", *(volatile uint32_t*)viewBase == 0x12345678);
            TEST("NtUnmapViewOfSection",
                 SW3NtUnmapViewOfSection(GetCurrentProcess(), viewBase));
        }
        TEST("NtClose (section)", SW3NtClose(hSection));
    }
}

static void test_system_info(void) {
    printf("\n--- System Info ---\n");

    /* SystemBasicInformation (0) */
    struct {
        ULONG Reserved, TimerResolution, PageSize, NumberOfPhysicalPages;
        ULONG LowestPhysicalPageNumber, HighestPhysicalPageNumber, AllocationGranularity;
        ULONG_PTR MinimumUserModeAddress, MaximumUserModeAddress, ActiveProcessorsAffinityMask;
        UCHAR NumberOfProcessors;
    } sysBasic;
    memset(&sysBasic, 0, sizeof(sysBasic));
    SW3_ULONG retLen = 0;
    TEST("NtQuerySystemInformation (BasicInfo)",
         SW3NtQuerySystemInformation(0, &sysBasic, sizeof(sysBasic), &retLen));
    if (retLen > 0)
        printf("    PageSize=%u CPUs=%u\n", sysBasic.PageSize, sysBasic.NumberOfProcessors);

    /* SystemPerformanceInformation (2) */
    char perfBuf[512];
    retLen = 0;
    TEST("NtQuerySystemInformation (PerfInfo)",
         SW3NtQuerySystemInformation(2, perfBuf, sizeof(perfBuf), &retLen));

    /* NtQuerySystemTime */
    LARGE_INTEGER sysTime = {0};
    TEST("NtQuerySystemTime",
         SW3NtQuerySystemTime((SW3_LARGE_INTEGER*)&sysTime));
}

static void test_registry(void) {
    printf("\n--- Registry ---\n");

    SW3_UNICODE_STRING keyName;
    wchar_t keyPath[] = L"\\Registry\\Machine\\Software";
    keyName.Length = (USHORT)(wcslen(keyPath) * sizeof(wchar_t));
    keyName.MaximumLength = keyName.Length + 2;
    keyName.Buffer = keyPath;

    SW3_OBJECT_ATTRIBUTES oa;
    memset(&oa, 0, sizeof(oa));
    oa.Length = sizeof(oa);
    oa.ObjectName = &keyName;

    SW3_HANDLE hKey = NULL;
    TEST("NtOpenKey (HKLM\\SOFTWARE)",
         SW3NtOpenKey(&hKey, 0x20019 /*KEY_READ*/, &oa));
    if (hKey) TEST("NtClose (reg key)", SW3NtClose(hKey));
}

static void test_token(void) {
    printf("\n--- Token ---\n");

    SW3_HANDLE hToken = NULL;
    TEST("NtOpenProcessToken",
         SW3NtOpenProcessToken(GetCurrentProcess(), 0x0008 /*TOKEN_QUERY*/, &hToken));
    if (hToken) {
        char buf[256];
        SW3_ULONG retLen = 0;
        TEST("NtQueryInformationToken (TokenUser)",
             SW3NtQueryInformationToken(hToken, 1, buf, sizeof(buf), &retLen));
        retLen = 0;
        TEST("NtQueryInformationToken (TokenStatistics)",
             SW3NtQueryInformationToken(hToken, 10, buf, sizeof(buf), &retLen));
        TEST("NtClose (token)", SW3NtClose(hToken));
    }
}

static void test_zero_param(void) {
    printf("\n--- Zero-param Functions ---\n");
    TEST("NtYieldExecution", SW3NtYieldExecution());
    TEST("NtTestAlert",      SW3NtTestAlert());

    LARGE_INTEGER t = {0};
    TEST("NtQuerySystemTime", SW3NtQuerySystemTime((SW3_LARGE_INTEGER*)&t));
}

int main(void) {
    printf("============================================\n");
    printf("  Comprehensive WoW64 Syscall Test\n");
    printf("============================================\n");
    printf("WoW64: %s  PID: %u\n", IsWow64() ? "YES" : "NO", GetCurrentProcessId());

    test_infrastructure();
    test_memory_operations();
    test_process_thread();
    test_events_sync();
    test_section();
    test_system_info();
    test_registry();
    test_token();
    test_zero_param();

    printf("\n============================================\n");
    printf("  Results: %d PASSED, %d FAILED\n", g_pass, g_fail);
    printf("============================================\n");
    return g_fail > 0 ? 1 : 0;
}
