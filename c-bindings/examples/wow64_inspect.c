/*
 * wow64_inspect.c - Demonstrates the wow64_helpers.h API:
 *   1. Detect that a target PID is WoW64
 *   2. Enumerate its x86 modules
 *   3. Find kernel32.dll
 *   4. Resolve LoadLibraryA (follows the forwarder to KernelBase)
 *   5. Snapshot the real x86 register context of the main thread
 *
 * Build (MSVC):
 *   cl /W4 /O2 wow64_inspect.c /I..\include ..\lib\syscalls_msvc.lib
 *
 * Usage:
 *   wow64_inspect.exe <PID>
 *
 * Run against any 32-bit process (e.g. a WoW64 game / browser / installer).
 * Requires PROCESS_QUERY_INFORMATION | PROCESS_VM_READ on the target,
 * and THREAD_GET_CONTEXT | THREAD_SUSPEND_RESUME for the thread snapshot.
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <tlhelp32.h>

#include "syscalls.h"
#include "wow64_helpers.h"

/* ---- module listing ------------------------------------------------- */

static bool print_module_cb(const SW3_LDR_DATA_TABLE_ENTRY32* e,
                            const uint16_t* name, size_t nlen, void* ctx)
{
    (void)ctx;
    printf("  0x%08X  size 0x%08X  ", e->DllBase, e->SizeOfImage);
    for (size_t i = 0; i < nlen; i++) {
        putchar((char)(name[i] < 128 ? name[i] : '?'));
    }
    putchar('\n');
    return true;
}

/* ---- main ----------------------------------------------------------- */

static DWORD find_first_thread_id(DWORD pid)
{
    HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if (snap == INVALID_HANDLE_VALUE) return 0;
    THREADENTRY32 te;
    te.dwSize = sizeof(te);
    DWORD tid = 0;
    if (Thread32First(snap, &te)) {
        do {
            if (te.th32OwnerProcessID == pid) { tid = te.th32ThreadID; break; }
        } while (Thread32Next(snap, &te));
    }
    CloseHandle(snap);
    return tid;
}

int main(int argc, char** argv)
{
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <PID>\n", argv[0]);
        return 1;
    }
    DWORD pid = (DWORD)strtoul(argv[1], NULL, 0);

    HANDLE hProc = OpenProcess(
        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
        FALSE, pid);
    if (!hProc) {
        fprintf(stderr, "OpenProcess(%lu) failed: %lu\n",
                pid, GetLastError());
        return 1;
    }

    /* --- 1. Detect WoW64 ------------------------------------------- */
    bool is_wow64 = false;
    uint32_t peb32 = 0;
    SW3_NTSTATUS s = sw3_wow64_detect((SW3_HANDLE)hProc, &is_wow64, &peb32);
    if (s < 0) {
        fprintf(stderr, "sw3_wow64_detect: NTSTATUS 0x%08X\n", (unsigned)s);
        CloseHandle(hProc);
        return 1;
    }
    printf("PID %lu is %sWoW64\n", pid, is_wow64 ? "" : "NOT ");
    if (!is_wow64) { CloseHandle(hProc); return 0; }
    printf("PEB32 @ 0x%08X\n\n", peb32);

    /* --- 2. Enumerate x86 modules ---------------------------------- */
    printf("Loaded modules (x86 view):\n");
    s = sw3_wow64_enum_modules((SW3_HANDLE)hProc, peb32, print_module_cb, NULL);
    if (s < 0) {
        fprintf(stderr, "sw3_wow64_enum_modules: 0x%08X\n", (unsigned)s);
    }
    putchar('\n');

    /* --- 3. Find kernel32.dll -------------------------------------- */
    uint32_t k32_base = 0, k32_size = 0;
    s = sw3_wow64_find_module((SW3_HANDLE)hProc, peb32,
                              "kernel32.dll", &k32_base, &k32_size);
    if (s < 0) {
        fprintf(stderr, "kernel32.dll not found: 0x%08X\n", (unsigned)s);
        CloseHandle(hProc);
        return 1;
    }
    printf("kernel32.dll @ 0x%08X (size 0x%08X)\n", k32_base, k32_size);

    /* --- 4. Resolve LoadLibraryA (follows forwarder) --------------- */
    uint32_t lla = 0;
    s = sw3_wow64_resolve_export((SW3_HANDLE)hProc, peb32,
                                 "kernel32.dll", "LoadLibraryA", &lla);
    if (s == SW3_STATUS_SUCCESS) {
        printf("LoadLibraryA  @ 0x%08X "
               "(resolved via forwarder chain if needed)\n", lla);
    } else {
        printf("LoadLibraryA  : NTSTATUS 0x%08X\n", (unsigned)s);
    }

    uint32_t ga = 0;
    s = sw3_wow64_resolve_export((SW3_HANDLE)hProc, peb32,
                                 "kernel32.dll", "GetProcAddress", &ga);
    if (s == SW3_STATUS_SUCCESS) {
        printf("GetProcAddress@ 0x%08X\n", ga);
    }
    putchar('\n');

    /* --- 5. Snapshot real x86 registers of first thread ------------ */
    DWORD tid = find_first_thread_id(pid);
    if (tid == 0) {
        fprintf(stderr, "No threads enumerable for PID %lu\n", pid);
        CloseHandle(hProc);
        return 0;
    }
    HANDLE hThr = OpenThread(
        THREAD_GET_CONTEXT | THREAD_SUSPEND_RESUME,
        FALSE, tid);
    if (!hThr) {
        fprintf(stderr, "OpenThread(%lu) failed: %lu\n",
                tid, GetLastError());
        CloseHandle(hProc);
        return 0;
    }

    SW3_WOW64_CONTEXT ctx;
    s = sw3_wow64_suspend_and_get_context(
        (SW3_HANDLE)hThr, &ctx, WOW64_CONTEXT_FULL);
    if (s == SW3_STATUS_SUCCESS) {
        printf("Thread %lu x86 context:\n", tid);
        printf("  EIP=%08X  EFLAGS=%08X\n", ctx.Eip, ctx.EFlags);
        printf("  EAX=%08X  EBX=%08X  ECX=%08X  EDX=%08X\n",
               ctx.Eax, ctx.Ebx, ctx.Ecx, ctx.Edx);
        printf("  ESI=%08X  EDI=%08X  EBP=%08X  ESP=%08X\n",
               ctx.Esi, ctx.Edi, ctx.Ebp, ctx.Esp);
        printf("  CS=%04X DS=%04X ES=%04X FS=%04X GS=%04X SS=%04X\n",
               ctx.SegCs, ctx.SegDs, ctx.SegEs,
               ctx.SegFs, ctx.SegGs, ctx.SegSs);
    } else {
        fprintf(stderr, "get_context failed: 0x%08X\n", (unsigned)s);
    }

    /* Always resume what we suspended. */
    SW3_ULONG prev = 0;
    SW3NtResumeThread((SW3_HANDLE)hThr, &prev);

    CloseHandle(hThr);
    CloseHandle(hProc);
    return 0;
}
