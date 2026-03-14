/**
 * Minimal test - dump stack state before syscall
 */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <windows.h>
#include "syscalls.h"

extern uint32_t SW3Sw3GetSyscallNumber(uint32_t function_hash);

typedef NTSTATUS (NTAPI *PFN_NtDelayExecution)(BOOLEAN, PLARGE_INTEGER);

int main(void) {
    printf("=== Minimal WoW64 Debug ===\n\n");

    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");
    PFN_NtDelayExecution pNtDelay = (PFN_NtDelayExecution)GetProcAddress(hNtdll, "NtDelayExecution");

    /* NtDelayExecution(Alertable=FALSE, DelayInterval=&delay)
       This is the simplest: only 2 args */
    LARGE_INTEGER delay;
    delay.QuadPart = -10000LL; /* 1ms */

    /* Via ntdll */
    printf("NTDLL: NtDelayExecution(FALSE, %p)...\n", &delay);
    NTSTATUS st1 = pNtDelay(FALSE, &delay);
    printf("NTDLL: 0x%08X\n\n", (unsigned)st1);

    /* Via Rust: SW3NtDelayExecution
       Let's print what we're passing */
    delay.QuadPart = -10000LL;
    printf("RUST:  SW3NtDelayExecution(FALSE, %p)...\n", &delay);
    printf("  &delay = %p, delay.QuadPart = %lld\n", &delay, delay.QuadPart);

    /* Verify the SSN */
    uint32_t ssn = SW3Sw3GetSyscallNumber(0xF269F2FB);
    printf("  SSN = %u\n", ssn);

    NTSTATUS st2 = SW3NtDelayExecution(FALSE, (SW3_LARGE_INTEGER*)&delay);
    printf("RUST:  0x%08X\n\n", (unsigned)st2);

    /* Now try the simplest possible: NtYieldExecution (0 args) */
    printf("RUST: NtYieldExecution (0 args)...\n");
    NTSTATUS st3 = SW3NtYieldExecution();
    printf("RUST: 0x%08X\n\n", (unsigned)st3);

    /* NtTestAlert (0 args) */
    printf("RUST: NtTestAlert (0 args)...\n");
    NTSTATUS st4 = SW3NtTestAlert();
    printf("RUST: 0x%08X\n\n", (unsigned)st4);

    /* NtQuerySystemTime (1 arg) */
    LARGE_INTEGER t = {0};
    printf("RUST: NtQuerySystemTime (1 arg, &t=%p)...\n", &t);
    NTSTATUS st5 = SW3NtQuerySystemTime((SW3_LARGE_INTEGER*)&t);
    printf("RUST: 0x%08X t=%lld\n\n", (unsigned)st5, t.QuadPart);

    /* NtClose with invalid handle (1 arg) - should return STATUS_INVALID_HANDLE (0xC0000008) */
    printf("RUST: NtClose(0x12345678) - expect 0xC0000008...\n");
    NTSTATUS st6 = SW3NtClose((SW3_HANDLE)0x12345678);
    printf("RUST: 0x%08X %s\n\n", (unsigned)st6,
           st6 == (NTSTATUS)0xC0000008 ? "CORRECT!" : "WRONG!");

    /* NtClose with NULL handle */
    printf("RUST: NtClose(NULL) - expect error...\n");
    NTSTATUS st7 = SW3NtClose(NULL);
    printf("RUST: 0x%08X\n\n", (unsigned)st7);

    printf("=== Summary: 0-arg funcs work, testing 1-arg and 2-arg ===\n");
    return 0;
}
