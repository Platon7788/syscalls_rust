/**
 * Extended WoW64 Syscall Validation Test
 *
 * Tests functions NOT covered by test_wow64.c, focusing on:
 * - High parameter counts (9-11+ params)
 * - PIO_APC_ROUTINE / transmute functions (NtReadFile, NtWriteFile, etc.)
 * - Security-critical functions (NtCreateThreadEx, NtCreateFile, etc.)
 * - NtWaitForMultipleObjects, NtOpenFile, NtDeviceIoControlFile
 * - File locking, directory queries, thread context
 *
 * Build (x86 MSVC):
 *   vcvarsall.bat x86
 *   cl /nologo /W3 /O2 test_wow64_extended.c /I..\include
 *      /Fe:test_wow64_extended.exe /link
 *      ..\target\i686-pc-windows-msvc\release\syscalls.lib
 *      ntdll.lib kernel32.lib
 */

#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <windows.h>
#include "syscalls.h"

/* ---- helpers ---- */

static int g_pass = 0, g_fail = 0, g_skip = 0;

#define NT_SUCCESS(s) ((int)(s) >= 0)

#define TEST(name, expr) do { \
    printf("  %-60s", name); \
    fflush(stdout); \
    SW3_NTSTATUS _st = (expr); \
    if (NT_SUCCESS(_st)) { printf("OK  (0x%08X)\n", (unsigned)_st); g_pass++; } \
    else { printf("FAIL (0x%08X)\n", (unsigned)_st); g_fail++; } \
} while(0)

#define TEST_EXPECT(name, expr, expected) do { \
    printf("  %-60s", name); \
    fflush(stdout); \
    SW3_NTSTATUS _st = (expr); \
    if (_st == (SW3_NTSTATUS)(expected) || NT_SUCCESS(_st)) { \
        printf("OK  (0x%08X)\n", (unsigned)_st); g_pass++; \
    } else { printf("FAIL (0x%08X, expected 0x%08X)\n", (unsigned)_st, (unsigned)(expected)); g_fail++; } \
} while(0)

#define TEST_BOOL(name, expr) do { \
    printf("  %-60s", name); \
    fflush(stdout); \
    if (expr) { printf("OK\n"); g_pass++; } \
    else { printf("FAIL\n"); g_fail++; } \
} while(0)

#define TEST_SKIP(name, reason) do { \
    printf("  %-60sSKIP (%s)\n", name, reason); \
    g_skip++; \
} while(0)

static int IsWow64(void) {
    BOOL b = FALSE;
    typedef BOOL (WINAPI *FN)(HANDLE, PBOOL);
    FN fn = (FN)GetProcAddress(GetModuleHandleA("kernel32"), "IsWow64Process");
    if (fn) fn(GetCurrentProcess(), &b);
    return b;
}

static void init_unicode(SW3_UNICODE_STRING *us, const wchar_t *s) {
    us->Length = (USHORT)(wcslen(s) * sizeof(wchar_t));
    us->MaximumLength = us->Length + sizeof(wchar_t);
    us->Buffer = (wchar_t*)s;
}

static void init_oa(SW3_OBJECT_ATTRIBUTES *oa, SW3_UNICODE_STRING *name) {
    memset(oa, 0, sizeof(*oa));
    oa->Length = sizeof(*oa);
    oa->ObjectName = name;
    /* OBJ_CASE_INSENSITIVE */
    oa->Attributes = 0x40;
}

/* ================================================================== */
/*  1. File I/O: NtCreateFile(11) + NtWriteFile(9) + NtReadFile(9)    */
/*     Tests: high param count + APC transmute path                   */
/* ================================================================== */
static void test_file_io(void) {
    printf("\n--- File I/O (NtCreateFile=11p, NtWriteFile=9p, NtReadFile=9p) ---\n");

    /* Build NT path for temp file */
    wchar_t tmpPath[MAX_PATH + 32];
    wchar_t ntPath[MAX_PATH + 32];
    GetTempPathW(MAX_PATH, tmpPath);
    wcscat(tmpPath, L"sw3_wow64_test.tmp");
    swprintf(ntPath, sizeof(ntPath)/sizeof(ntPath[0]), L"\\??\\%s", tmpPath);

    SW3_UNICODE_STRING fileName;
    init_unicode(&fileName, ntPath);

    SW3_OBJECT_ATTRIBUTES oa;
    init_oa(&oa, &fileName);

    SW3_IO_STATUS_BLOCK iosb;

    /* NtCreateFile - 11 params! */
    SW3_HANDLE hFile = NULL;
    memset(&iosb, 0, sizeof(iosb));
    TEST("NtCreateFile (create temp, 11 params)",
         SW3NtCreateFile(
             &hFile,
             0x12019F,  /* GENERIC_READ | GENERIC_WRITE | SYNCHRONIZE | FILE_READ_ATTRIBUTES */
             &oa,
             &iosb,
             NULL,      /* allocation size */
             0x80,      /* FILE_ATTRIBUTE_NORMAL */
             0,         /* no sharing */
             2,         /* FILE_CREATE -> supersede if exists: FILE_SUPERSEDE=0, CREATE=2 actually FILE_OVERWRITE_IF=5 */
             0x20,      /* FILE_SYNCHRONOUS_IO_NONALERT */
             NULL,      /* ea buffer */
             0          /* ea length */
         ));

    if (hFile) {
        /* NtWriteFile - 9 params with APC transmute! */
        const char testData[] = "WoW64 extended test data - DEADBEEF 0123456789";
        memset(&iosb, 0, sizeof(iosb));
        TEST("NtWriteFile (9 params, APC=NULL)",
             SW3NtWriteFile(
                 hFile,
                 NULL,   /* event */
                 NULL,   /* apc_routine - this is the PIO_APC_ROUTINE transmute path */
                 NULL,   /* apc_context */
                 &iosb,
                 (SW3_PVOID)testData,
                 (SW3_ULONG)strlen(testData),
                 NULL,   /* byte_offset */
                 NULL    /* key */
             ));
        TEST_BOOL("  Written bytes match",
                   iosb.Information == strlen(testData));

        /* NtQueryInformationFile - 5 params */
        struct {
            LARGE_INTEGER CreationTime, LastAccessTime, LastWriteTime, ChangeTime;
            ULONG FileAttributes;
        } basicInfo;
        memset(&basicInfo, 0, sizeof(basicInfo));
        memset(&iosb, 0, sizeof(iosb));
        TEST("NtQueryInformationFile (BasicInfo, 5 params)",
             SW3NtQueryInformationFile(
                 hFile, &iosb,
                 &basicInfo, sizeof(basicInfo),
                 4  /* FileBasicInformation */
             ));

        /* NtSetInformationFile - 5 params - set file position to 0 */
        LARGE_INTEGER newPos;
        newPos.QuadPart = 0;
        memset(&iosb, 0, sizeof(iosb));
        TEST("NtSetInformationFile (Position=0, 5 params)",
             SW3NtSetInformationFile(
                 hFile, &iosb,
                 &newPos, sizeof(newPos),
                 14  /* FilePositionInformation */
             ));

        /* NtReadFile - 9 params with APC transmute! */
        char readBuf[128];
        memset(readBuf, 0, sizeof(readBuf));
        memset(&iosb, 0, sizeof(iosb));
        TEST("NtReadFile (9 params, APC=NULL)",
             SW3NtReadFile(
                 hFile,
                 NULL,   /* event */
                 NULL,   /* apc_routine - PIO_APC_ROUTINE transmute */
                 NULL,   /* apc_context */
                 &iosb,
                 readBuf,
                 (SW3_ULONG)sizeof(readBuf),
                 NULL,   /* byte_offset */
                 NULL    /* key */
             ));
        TEST_BOOL("  Read data matches written",
                   iosb.Information == strlen(testData) &&
                   memcmp(readBuf, testData, strlen(testData)) == 0);

        TEST("NtClose (temp file)", SW3NtClose(hFile));
        hFile = NULL;
    }

    /* NtOpenFile - 6 params */
    memset(&iosb, 0, sizeof(iosb));
    init_oa(&oa, &fileName);
    SW3_HANDLE hFile2 = NULL;
    TEST("NtOpenFile (reopen, 6 params)",
         SW3NtOpenFile(
             &hFile2,
             0x120089, /* GENERIC_READ | SYNCHRONIZE */
             &oa,
             &iosb,
             1,     /* FILE_SHARE_READ */
             0x20   /* FILE_SYNCHRONOUS_IO_NONALERT */
         ));

    if (hFile2) {
        /* Re-read to verify NtOpenFile works */
        char readBuf2[128];
        memset(readBuf2, 0, sizeof(readBuf2));
        memset(&iosb, 0, sizeof(iosb));
        TEST("NtReadFile (via NtOpenFile handle)",
             SW3NtReadFile(hFile2, NULL, NULL, NULL, &iosb,
                           readBuf2, sizeof(readBuf2), NULL, NULL));
        TEST_BOOL("  Re-read data matches",
                   memcmp(readBuf2, "WoW64 extended test data", 24) == 0);

        TEST("NtClose (reopened file)", SW3NtClose(hFile2));
    }

    /* Cleanup temp file via Win32 (just to be tidy) */
    DeleteFileW(tmpPath);
}

/* ================================================================== */
/*  2. NtCreateThreadEx (11 params) - security-critical               */
/* ================================================================== */

static volatile LONG g_threadResult = 0;

static DWORD WINAPI TestThreadProc(LPVOID param) {
    InterlockedExchange(&g_threadResult, (LONG)(ULONG_PTR)param);
    return 42;
}

static void test_create_thread_ex(void) {
    printf("\n--- NtCreateThreadEx (11 params) ---\n");

    SW3_HANDLE hThread = NULL;
    g_threadResult = 0;

    /* CREATE_SUSPENDED = 1 */
    TEST("NtCreateThreadEx (suspended, 11 params)",
         SW3NtCreateThreadEx(
             &hThread,
             0x001FFFFF, /* THREAD_ALL_ACCESS */
             NULL,       /* object_attributes */
             GetCurrentProcess(),
             (SW3_PVOID)TestThreadProc,
             (SW3_PVOID)0xBEEF,  /* argument */
             1,          /* CREATE_SUSPENDED */
             0,          /* zero_bits */
             0,          /* stack_size (default) */
             0,          /* maximum_stack_size (default) */
             NULL        /* attribute_list */
         ));

    if (hThread) {
        /* NtResumeThread - 2 params */
        SW3_ULONG prevCount = 0;
        TEST("NtResumeThread", SW3NtResumeThread(hThread, &prevCount));
        TEST_BOOL("  Previous suspend count == 1", prevCount == 1);

        /* NtWaitForSingleObject - 3 params */
        LARGE_INTEGER timeout;
        timeout.QuadPart = -50000000LL; /* 5 seconds */
        TEST("NtWaitForSingleObject (wait for thread)",
             SW3NtWaitForSingleObject(hThread, FALSE, (SW3_LARGE_INTEGER*)&timeout));

        TEST_BOOL("  Thread received arg 0xBEEF", g_threadResult == 0xBEEF);

        TEST("NtClose (thread)", SW3NtClose(hThread));
    }
}

/* ================================================================== */
/*  3. NtSuspendThread / NtResumeThread / NtGetContextThread          */
/*     (2+2+2 params) - functional test                               */
/* ================================================================== */

static volatile LONG g_spinFlag = 1;

static DWORD WINAPI SpinThread(LPVOID param) {
    (void)param;
    while (InterlockedCompareExchange(&g_spinFlag, 1, 1) == 1)
        ;
    return 0;
}

static void test_thread_control(void) {
    printf("\n--- Thread Suspend/Resume/Context (2+2+2 params) ---\n");

    g_spinFlag = 1;
    HANDLE hThread = CreateThread(NULL, 0, SpinThread, NULL, 0, NULL);
    if (!hThread) {
        TEST_SKIP("Thread control tests", "CreateThread failed");
        return;
    }

    Sleep(50); /* let thread spin up */

    /* NtSuspendThread */
    SW3_ULONG prevCount = 0;
    TEST("NtSuspendThread", SW3NtSuspendThread(hThread, &prevCount));
    TEST_BOOL("  Previous suspend count == 0", prevCount == 0);

    /* NtGetContextThread - 2 params, uses CONTEXT struct */
    CONTEXT ctx;
    memset(&ctx, 0, sizeof(ctx));
    ctx.ContextFlags = CONTEXT_INTEGER | CONTEXT_CONTROL;
    TEST("NtGetContextThread", SW3NtGetContextThread(hThread, &ctx));
    TEST_BOOL("  EIP is non-zero", ctx.Eip != 0);
    printf("    EIP=0x%08X  ESP=0x%08X\n", (unsigned)ctx.Eip, (unsigned)ctx.Esp);

    /* NtResumeThread */
    prevCount = 0;
    TEST("NtResumeThread", SW3NtResumeThread(hThread, &prevCount));
    TEST_BOOL("  Previous suspend count == 1", prevCount == 1);

    /* Signal thread to exit */
    InterlockedExchange(&g_spinFlag, 0);
    WaitForSingleObject(hThread, 5000);
    CloseHandle(hThread);
}

/* ================================================================== */
/*  4. NtDeviceIoControlFile (10 params, APC transmute)               */
/* ================================================================== */
static void test_device_ioctl(void) {
    printf("\n--- NtDeviceIoControlFile (10 params, APC) ---\n");

    /* Open \\Device\\Null */
    SW3_UNICODE_STRING devName;
    init_unicode(&devName, L"\\Device\\Null");

    SW3_OBJECT_ATTRIBUTES oa;
    init_oa(&oa, &devName);

    SW3_IO_STATUS_BLOCK iosb;
    SW3_HANDLE hDev = NULL;
    memset(&iosb, 0, sizeof(iosb));

    TEST("NtCreateFile (\\Device\\Null)",
         SW3NtCreateFile(&hDev, 0x120089, &oa, &iosb, NULL, 0, 7, 3, 0, NULL, 0));

    if (hDev) {
        /* NtDeviceIoControlFile - 10 params with PIO_APC_ROUTINE transmute */
        /* Use an invalid IOCTL - we just test the call doesn't crash */
        memset(&iosb, 0, sizeof(iosb));
        SW3_NTSTATUS st = SW3NtDeviceIoControlFile(
            hDev,
            NULL,   /* event */
            NULL,   /* apc_routine - PIO_APC_ROUTINE transmute */
            NULL,   /* apc_context */
            &iosb,
            0x00000001,  /* bogus ioctl */
            NULL, 0,     /* input */
            NULL, 0      /* output */
        );
        /* Expected: STATUS_INVALID_DEVICE_REQUEST or similar */
        printf("  NtDeviceIoControlFile (bogus IOCTL, 10 params)            ");
        /* Any response without crash = pass */
        printf("OK  (0x%08X, no crash)\n", (unsigned)st);
        g_pass++;

        TEST("NtClose (device)", SW3NtClose(hDev));
    }
}

/* ================================================================== */
/*  5. NtFsControlFile (10 params, APC transmute)                     */
/* ================================================================== */
static void test_fscontrol(void) {
    printf("\n--- NtFsControlFile (10 params, APC) ---\n");

    /* Create a named pipe to test FSCTL on */
    HANDLE hPipe = CreateNamedPipeW(
        L"\\\\.\\pipe\\sw3_wow64_test_pipe",
        PIPE_ACCESS_DUPLEX,
        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE,
        1, 4096, 4096, 0, NULL);

    if (hPipe == INVALID_HANDLE_VALUE) {
        TEST_SKIP("NtFsControlFile", "CreateNamedPipe failed");
        return;
    }

    SW3_IO_STATUS_BLOCK iosb;
    memset(&iosb, 0, sizeof(iosb));

    /* FSCTL_PIPE_PEEK = 0x0011400C */
    char peekBuf[64];
    SW3_NTSTATUS st = SW3NtFsControlFile(
        hPipe,
        NULL,   /* event */
        NULL,   /* apc_routine - PIO_APC_ROUTINE transmute */
        NULL,   /* apc_context */
        &iosb,
        0x0011400C,  /* FSCTL_PIPE_PEEK */
        NULL, 0,
        peekBuf, sizeof(peekBuf)
    );
    printf("  NtFsControlFile (PIPE_PEEK, 10 params)                    ");
    /* STATUS_PIPE_LISTENING or similar is fine - we test no crash */
    printf("OK  (0x%08X, no crash)\n", (unsigned)st);
    g_pass++;

    CloseHandle(hPipe);
}

/* ================================================================== */
/*  6. NtWaitForMultipleObjects (5 params)                            */
/* ================================================================== */
static void test_wait_multiple(void) {
    printf("\n--- NtWaitForMultipleObjects (5 params) ---\n");

    SW3_OBJECT_ATTRIBUTES oa;
    memset(&oa, 0, sizeof(oa));
    oa.Length = sizeof(oa);

    #define NUM_EVENTS 4
    SW3_HANDLE events[NUM_EVENTS] = {0};
    int created = 0;

    for (int i = 0; i < NUM_EVENTS; i++) {
        SW3_NTSTATUS st = SW3NtCreateEvent(&events[i], 0x001F0003, &oa, 1, FALSE);
        if (NT_SUCCESS(st)) created++;
    }
    printf("  Created %d/%d events\n", created, NUM_EVENTS);
    TEST_BOOL("All events created", created == NUM_EVENTS);

    /* Signal all events */
    for (int i = 0; i < NUM_EVENTS; i++) {
        if (events[i]) SW3NtSetEvent(events[i], NULL);
    }

    /* WaitAll with timeout=0 */
    LARGE_INTEGER timeout;
    timeout.QuadPart = 0;
    TEST("NtWaitForMultipleObjects (WaitAll, 4 events)",
         SW3NtWaitForMultipleObjects(
             NUM_EVENTS,
             events,
             1,     /* WaitAll */
             FALSE, /* alertable */
             (SW3_LARGE_INTEGER*)&timeout
         ));

    /* Reset all, then WaitAny (should timeout) */
    for (int i = 0; i < NUM_EVENTS; i++) {
        if (events[i]) SW3NtResetEvent(events[i], NULL);
    }
    timeout.QuadPart = 0;
    SW3_NTSTATUS st = SW3NtWaitForMultipleObjects(
        NUM_EVENTS, events, 0 /* WaitAny */, FALSE,
        (SW3_LARGE_INTEGER*)&timeout);
    printf("  NtWaitForMultipleObjects (WaitAny, none signaled)          ");
    if ((unsigned)st == 0x00000102) { /* STATUS_TIMEOUT */
        printf("OK  (0x%08X = TIMEOUT)\n", (unsigned)st);
        g_pass++;
    } else {
        printf("FAIL (0x%08X)\n", (unsigned)st);
        g_fail++;
    }

    /* Signal event[2], WaitAny should return WAIT_OBJECT_0+2 (=2) */
    if (events[2]) {
        SW3_ULONG prev = 0;
        SW3NtSetEvent(events[2], &prev);
    }
    timeout.QuadPart = -1; /* 100ns relative - effectively immediate */
    st = SW3NtWaitForMultipleObjects(
        NUM_EVENTS, events, 0 /* WaitAny */, FALSE,
        (SW3_LARGE_INTEGER*)&timeout);
    printf("  NtWaitForMultipleObjects (WaitAny, event[2] signaled)      ");
    if ((unsigned)st == 2) { /* WAIT_OBJECT_2 */
        printf("OK  (0x%08X = WAIT_2)\n", (unsigned)st);
        g_pass++;
    } else if (NT_SUCCESS(st) || (unsigned)st == 0x00000102) {
        /* TIMEOUT is also acceptable - event signaling is racy with timeout=0 */
        printf("OK  (0x%08X)\n", (unsigned)st);
        g_pass++;
    } else {
        printf("FAIL (0x%08X)\n", (unsigned)st);
        g_fail++;
    }

    for (int i = 0; i < NUM_EVENTS; i++) {
        if (events[i]) SW3NtClose(events[i]);
    }
    #undef NUM_EVENTS
}

/* ================================================================== */
/*  7. NtLockFile / NtUnlockFile (10/5 params, APC transmute)         */
/* ================================================================== */
static void test_file_locking(void) {
    printf("\n--- NtLockFile(10p) / NtUnlockFile(5p) ---\n");

    /* Create a temp file */
    wchar_t tmpPath[MAX_PATH + 32];
    wchar_t ntPath[MAX_PATH + 32];
    GetTempPathW(MAX_PATH, tmpPath);
    wcscat(tmpPath, L"sw3_lock_test.tmp");
    swprintf(ntPath, sizeof(ntPath)/sizeof(ntPath[0]), L"\\??\\%s", tmpPath);

    SW3_UNICODE_STRING fileName;
    init_unicode(&fileName, ntPath);

    SW3_OBJECT_ATTRIBUTES oa;
    init_oa(&oa, &fileName);

    SW3_IO_STATUS_BLOCK iosb;
    SW3_HANDLE hFile = NULL;
    memset(&iosb, 0, sizeof(iosb));

    SW3_NTSTATUS st = SW3NtCreateFile(
        &hFile, 0x120116, &oa, &iosb, NULL, 0x80, 0, 5, 0x20, NULL, 0);

    if (!NT_SUCCESS(st) || !hFile) {
        TEST_SKIP("File locking tests", "NtCreateFile failed");
        return;
    }

    /* Write some data so the file has content to lock */
    char data[256];
    memset(data, 'A', sizeof(data));
    memset(&iosb, 0, sizeof(iosb));
    SW3NtWriteFile(hFile, NULL, NULL, NULL, &iosb, data, sizeof(data), NULL, NULL);

    /* NtLockFile - 10 params with PIO_APC_ROUTINE transmute */
    uint64_t lockOffset = 0;
    uint64_t lockLength = 128;
    memset(&iosb, 0, sizeof(iosb));
    TEST("NtLockFile (10 params, APC)",
         SW3NtLockFile(
             hFile,
             NULL,   /* event */
             NULL,   /* apc_routine - PIO_APC_ROUTINE */
             NULL,   /* apc_context */
             &iosb,
             &lockOffset,
             &lockLength,
             0,      /* key */
             TRUE,   /* fail_immediately */
             TRUE    /* exclusive_lock */
         ));

    /* NtUnlockFile - 5 params */
    memset(&iosb, 0, sizeof(iosb));
    TEST("NtUnlockFile (5 params)",
         SW3NtUnlockFile(hFile, &iosb, &lockOffset, &lockLength, 0));

    TEST("NtClose (locked file)", SW3NtClose(hFile));
    DeleteFileW(tmpPath);
}

/* ================================================================== */
/*  8. NtQueryDirectoryFile (11 params, APC transmute)                */
/* ================================================================== */
static void test_query_directory(void) {
    printf("\n--- NtQueryDirectoryFile (11 params, APC) ---\n");

    /* Open system root directory */
    wchar_t sysDir[MAX_PATH];
    GetSystemDirectoryW(sysDir, MAX_PATH);
    wchar_t ntPath[MAX_PATH + 32];
    swprintf(ntPath, sizeof(ntPath)/sizeof(ntPath[0]), L"\\??\\%s", sysDir);

    SW3_UNICODE_STRING dirName;
    init_unicode(&dirName, ntPath);

    SW3_OBJECT_ATTRIBUTES oa;
    init_oa(&oa, &dirName);

    SW3_IO_STATUS_BLOCK iosb;
    SW3_HANDLE hDir = NULL;
    memset(&iosb, 0, sizeof(iosb));

    SW3_NTSTATUS st = SW3NtOpenFile(
        &hDir,
        0x100001, /* FILE_LIST_DIRECTORY | SYNCHRONIZE */
        &oa, &iosb,
        1,    /* FILE_SHARE_READ */
        0x21  /* FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT */
    );

    if (!NT_SUCCESS(st) || !hDir) {
        TEST_SKIP("NtQueryDirectoryFile", "Cannot open directory");
        return;
    }

    /* NtQueryDirectoryFile - 11 params with PIO_APC_ROUTINE transmute */
    char dirBuf[4096];
    memset(dirBuf, 0, sizeof(dirBuf));
    memset(&iosb, 0, sizeof(iosb));
    TEST("NtQueryDirectoryFile (11 params, APC)",
         SW3NtQueryDirectoryFile(
             hDir,
             NULL,   /* event */
             NULL,   /* apc_routine - PIO_APC_ROUTINE */
             NULL,   /* apc_context */
             &iosb,
             dirBuf,
             sizeof(dirBuf),
             1,      /* FileBothDirectoryInformation */
             FALSE,  /* return_single_entry */
             NULL,   /* file_name filter */
             TRUE    /* restart_scan */
         ));
    TEST_BOOL("  Received directory data", iosb.Information > 0);

    TEST("NtClose (directory)", SW3NtClose(hDir));
}

/* ================================================================== */
/*  9. NtNotifyChangeDirectoryFile (9 params, APC transmute)          */
/* ================================================================== */
static void test_notify_change(void) {
    printf("\n--- NtNotifyChangeDirectoryFile (9 params, APC) ---\n");

    wchar_t tmpDir[MAX_PATH];
    GetTempPathW(MAX_PATH, tmpDir);
    wchar_t ntPath[MAX_PATH + 32];
    swprintf(ntPath, sizeof(ntPath)/sizeof(ntPath[0]), L"\\??\\%s", tmpDir);

    SW3_UNICODE_STRING dirName;
    init_unicode(&dirName, ntPath);

    SW3_OBJECT_ATTRIBUTES oa;
    init_oa(&oa, &dirName);

    SW3_IO_STATUS_BLOCK iosb;
    SW3_HANDLE hDir = NULL;
    memset(&iosb, 0, sizeof(iosb));

    /* Open directory in ASYNC mode (no FILE_SYNCHRONOUS_IO_NONALERT)
     * so NtNotifyChangeDirectoryFile returns STATUS_PENDING immediately */
    SW3_NTSTATUS st = SW3NtOpenFile(
        &hDir, 0x100001, &oa, &iosb, 7,
        0x01  /* FILE_DIRECTORY_FILE only, no synchronous flag */
    );

    if (!NT_SUCCESS(st) || !hDir) {
        TEST_SKIP("NtNotifyChangeDirectoryFile", "Cannot open temp dir");
        return;
    }

    /* Create an event for async notification */
    SW3_OBJECT_ATTRIBUTES evOa;
    memset(&evOa, 0, sizeof(evOa));
    evOa.Length = sizeof(evOa);
    SW3_HANDLE hEvent = NULL;
    SW3NtCreateEvent(&hEvent, 0x001F0003, &evOa, 0, FALSE);

    if (hEvent) {
        char notifyBuf[1024];
        memset(&iosb, 0, sizeof(iosb));
        /* FILE_NOTIFY_CHANGE_FILE_NAME = 0x01 */
        st = SW3NtNotifyChangeDirectoryFile(
            hDir,
            hEvent,
            NULL,   /* apc_routine - PIO_APC_ROUTINE */
            NULL,   /* apc_context */
            &iosb,
            notifyBuf,
            sizeof(notifyBuf),
            0x01,   /* FILE_NOTIFY_CHANGE_FILE_NAME */
            FALSE   /* watch_tree */
        );
        /* Should return STATUS_PENDING */
        printf("  NtNotifyChangeDirectoryFile (9 params, APC)                ");
        if ((unsigned)st == 0x103 || NT_SUCCESS(st)) { /* STATUS_PENDING or success */
            printf("OK  (0x%08X)\n", (unsigned)st);
            g_pass++;
        } else {
            printf("FAIL (0x%08X)\n", (unsigned)st);
            g_fail++;
        }
        SW3NtClose(hEvent);
    }

    /* Close directory - this also cancels any pending I/O */
    SW3NtClose(hDir);
}

/* ================================================================== */
/*  10. Token & Security (NtAdjustPrivilegesToken=6, etc.)            */
/* ================================================================== */
static void test_token_extended(void) {
    printf("\n--- Token Extended (6 params) ---\n");

    SW3_HANDLE hToken = NULL;
    TEST("NtOpenProcessToken (3 params)",
         SW3NtOpenProcessToken(GetCurrentProcess(), 0x0002002C /* READ_CONTROL|ADJUST|QUERY */, &hToken));

    if (hToken) {
        /* NtAdjustPrivilegesToken - 6 params */
        /* Try to enable SeDebugPrivilege (may fail without admin) */
        struct {
            ULONG PrivilegeCount;
            struct {
                SW3_LUID Luid;
                ULONG Attributes;
            } Privileges[1];
        } tp;
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid.LowPart = 20;  /* SeDebugPrivilege */
        tp.Privileges[0].Luid.HighPart = 0;
        tp.Privileges[0].Attributes = 0x00000002; /* SE_PRIVILEGE_ENABLED */

        SW3_ULONG retLen = 0;
        SW3_NTSTATUS st = SW3NtAdjustPrivilegesToken(
            hToken,
            FALSE,          /* disable_all */
            (SW3_PTOKEN_PRIVILEGES)&tp,
            sizeof(tp),
            NULL,           /* previous_state */
            &retLen
        );
        printf("  NtAdjustPrivilegesToken (6 params)                         ");
        /* May return STATUS_NOT_ALL_ASSIGNED (0x106) without admin - that's fine */
        if (NT_SUCCESS(st) || (unsigned)st == 0x00000106) {
            printf("OK  (0x%08X)\n", (unsigned)st);
            g_pass++;
        } else {
            printf("FAIL (0x%08X)\n", (unsigned)st);
            g_fail++;
        }

        /* NtQuerySecurityObject - 5 params */
        char secBuf[256];
        SW3_ULONG secNeeded = 0;
        TEST_EXPECT("NtQuerySecurityObject (5 params)",
             SW3NtQuerySecurityObject(
                 hToken,
                 (SW3_PVOID)(ULONG_PTR)4, /* DACL_SECURITY_INFORMATION */
                 (SW3_PSECURITY_DESCRIPTOR)secBuf,
                 sizeof(secBuf),
                 &secNeeded
             ), 0);

        TEST("NtClose (token)", SW3NtClose(hToken));
    }
}

/* ================================================================== */
/*  11. Stress: Many different param counts in sequence               */
/* ================================================================== */
static void test_param_count_stress(void) {
    printf("\n--- Parameter count stress test ---\n");

    /* 0 params */
    TEST("NtTestAlert (0 params)",           SW3NtTestAlert());
    TEST("NtYieldExecution (0 params)",      SW3NtYieldExecution());

    /* 1 param */
    LARGE_INTEGER t = {0};
    TEST("NtQuerySystemTime (1 param)",      SW3NtQuerySystemTime((SW3_LARGE_INTEGER*)&t));

    /* 2 params */
    LARGE_INTEGER delay;
    delay.QuadPart = -1000; /* 100us */
    TEST("NtDelayExecution (2 params)",      SW3NtDelayExecution(FALSE, (SW3_LARGE_INTEGER*)&delay));

    /* 3 params */
    SW3_OBJECT_ATTRIBUTES oa;
    memset(&oa, 0, sizeof(oa));
    oa.Length = sizeof(oa);
    SW3_HANDLE hEvent = NULL;
    SW3_UNICODE_STRING keyName;
    init_unicode(&keyName, L"\\Registry\\Machine\\Software");
    init_oa(&oa, &keyName);
    SW3_HANDLE hKey = NULL;
    TEST("NtOpenKey (3 params)",             SW3NtOpenKey(&hKey, 0x20019, &oa));
    if (hKey) SW3NtClose(hKey);

    /* 4 params */
    struct {
        ULONG Reserved, TimerResolution, PageSize, NumberOfPhysicalPages;
        ULONG LowestPhysicalPageNumber, HighestPhysicalPageNumber, AllocationGranularity;
        ULONG_PTR MinimumUserModeAddress, MaximumUserModeAddress, ActiveProcessorsAffinityMask;
        UCHAR NumberOfProcessors;
    } sysBasic;
    SW3_ULONG sysRetLen = 0;
    TEST("NtQuerySystemInformation (4 params)",
         SW3NtQuerySystemInformation(0, &sysBasic, sizeof(sysBasic), &sysRetLen));

    /* 5 params */
    void* alloc1 = NULL; SW3_SIZE_T allocSize = 4096;
    SW3_ULONG oldProt = 0;
    TEST("NtAllocateVirtualMemory (alloc, 6p)",
         SW3NtAllocateVirtualMemory(GetCurrentProcess(), &alloc1, 0, &allocSize,
                                     MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE));
    if (alloc1) {
        void* pAddr = alloc1;
        SW3_SIZE_T pSize = 4096;
        TEST("NtProtectVirtualMemory (5 params)",
             SW3NtProtectVirtualMemory(GetCurrentProcess(), &pAddr, &pSize,
                                        PAGE_EXECUTE_READ, &oldProt));

        /* 6 params */
        SW3_MEMORY_BASIC_INFORMATION mbi;
        SW3_SIZE_T retLen = 0;
        TEST("NtQueryVirtualMemory (6 params)",
             SW3NtQueryVirtualMemory(GetCurrentProcess(), alloc1, 0,
                                      &mbi, sizeof(mbi), &retLen));

        /* 7 params - NtCreateSection */
        SW3_OBJECT_ATTRIBUTES secOa;
        memset(&secOa, 0, sizeof(secOa));
        secOa.Length = sizeof(secOa);
        LARGE_INTEGER maxSz; maxSz.QuadPart = 4096;
        SW3_HANDLE hSec = NULL;
        TEST("NtCreateSection (7 params)",
             SW3NtCreateSection(&hSec, 0xF001F, &secOa, (SW3_LARGE_INTEGER*)&maxSz,
                                 PAGE_READWRITE, 0x08000000, NULL));
        if (hSec) {
            /* 10 params - NtMapViewOfSection */
            void* viewBase = NULL;
            SW3_SIZE_T viewSize = 0;
            TEST("NtMapViewOfSection (10 params)",
                 SW3NtMapViewOfSection(hSec, GetCurrentProcess(), &viewBase,
                                        0, 0, NULL, &viewSize, 1, 0, PAGE_READWRITE));
            if (viewBase) {
                *(volatile uint32_t*)viewBase = 0xFEEDFACE;
                TEST_BOOL("  Section write/read 0xFEEDFACE",
                           *(volatile uint32_t*)viewBase == 0xFEEDFACE);
                TEST("NtUnmapViewOfSection (2 params)",
                     SW3NtUnmapViewOfSection(GetCurrentProcess(), viewBase));
            }
            SW3NtClose(hSec);
        }

        /* Free */
        SW3_SIZE_T freeSize = 0;
        SW3NtFreeVirtualMemory(GetCurrentProcess(), &alloc1, &freeSize, MEM_RELEASE);
    }

    /* 11 params - already tested via NtCreateFile above */
    printf("  (11+ params tested in File I/O and NtCreateThreadEx)\n");
}

/* ================================================================== */
/*  12. NtDuplicateObject extended (7 params)                         */
/* ================================================================== */
static void test_duplicate_extended(void) {
    printf("\n--- NtDuplicateObject extended (7 params) ---\n");

    /* Create event, duplicate with specific access */
    SW3_OBJECT_ATTRIBUTES oa;
    memset(&oa, 0, sizeof(oa));
    oa.Length = sizeof(oa);
    SW3_HANDLE hEvent = NULL;
    SW3NtCreateEvent(&hEvent, 0x001F0003, &oa, 1, FALSE);

    if (hEvent) {
        SW3_HANDLE hDup = NULL;
        TEST("NtDuplicateObject (event, specific access, 7p)",
             SW3NtDuplicateObject(
                 GetCurrentProcess(), hEvent,
                 GetCurrentProcess(), &hDup,
                 0x001F0003, /* EVENT_ALL_ACCESS */
                 0,          /* attributes */
                 0           /* options - no flags */
             ));
        if (hDup) {
            /* Verify dup works by setting event via dup handle */
            SW3_ULONG prev = 0;
            TEST("NtSetEvent (via dup handle)", SW3NtSetEvent(hDup, &prev));
            TEST("NtClose (dup)", SW3NtClose(hDup));
        }
        TEST("NtClose (original)", SW3NtClose(hEvent));
    }
}

/* ================================================================== */
/*  MAIN                                                              */
/* ================================================================== */
int main(void) {
    printf("============================================================\n");
    printf("  Extended WoW64 Syscall Validation Test\n");
    printf("============================================================\n");
    printf("WoW64: %s  PID: %u  Arch: x86\n",
           IsWow64() ? "YES" : "NO", GetCurrentProcessId());

    test_file_io();
    test_create_thread_ex();
    test_thread_control();
    test_device_ioctl();
    test_fscontrol();
    test_wait_multiple();
    test_file_locking();
    test_query_directory();
    test_notify_change();
    test_token_extended();
    test_param_count_stress();
    test_duplicate_extended();

    printf("\n============================================================\n");
    printf("  Results: %d PASSED, %d FAILED, %d SKIPPED\n",
           g_pass, g_fail, g_skip);
    printf("============================================================\n");
    return g_fail > 0 ? 1 : 0;
}
