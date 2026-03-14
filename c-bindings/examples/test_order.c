#include <stdio.h>
#include <stdint.h>
#include <windows.h>
#include "syscalls.h"

int main(void) {
    printf("=== Order Test ===\n\n");

    /* 2-arg FIRST */
    LARGE_INTEGER delay;
    delay.QuadPart = -1;
    printf("[2-arg] NtDelayExecution: "); fflush(stdout);
    NTSTATUS st = SW3NtDelayExecution(0, (SW3_LARGE_INTEGER*)&delay);
    printf("0x%08X %s\n", (unsigned)st, st==0?"OK":"FAIL");

    /* 1-arg */
    printf("[1-arg] NtClose(bad): "); fflush(stdout);
    st = SW3NtClose((SW3_HANDLE)0x12345678);
    printf("0x%08X %s\n", (unsigned)st, st==(NTSTATUS)0xC0000008?"OK":"FAIL");

    /* 1-arg pointer */
    LARGE_INTEGER t = {0};
    printf("[1-arg] NtQuerySystemTime: "); fflush(stdout);
    st = SW3NtQuerySystemTime((SW3_LARGE_INTEGER*)&t);
    printf("0x%08X %s (t=%lld)\n", (unsigned)st, st==0?"OK":"FAIL", t.QuadPart);

    /* 5-arg: NtCreateEvent */
    HANDLE hEvent = NULL;
    printf("[5-arg] NtCreateEvent: "); fflush(stdout);
    st = SW3NtCreateEvent((SW3_HANDLE*)&hEvent, EVENT_ALL_ACCESS, NULL, 1, 0);
    printf("0x%08X %s (h=%p)\n", (unsigned)st, st==0?"OK":"FAIL", hEvent);
    if (hEvent) SW3NtClose((SW3_HANDLE)hEvent);

    /* 5-arg: NtProtectVirtualMemory */
    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    if (addr) {
        ULONG sz = 4096;
        ULONG oldp = 0;
        printf("[5-arg] NtProtectVM: "); fflush(stdout);
        st = SW3NtProtectVirtualMemory((SW3_HANDLE)(HANDLE)-1, &addr, (unsigned long*)&sz, 0x04, (unsigned long*)&oldp);
        printf("0x%08X %s (oldp=%lu)\n", (unsigned)st, st==0?"OK":"FAIL", oldp);
        VirtualFree(addr, 0, MEM_RELEASE);
    }

    /* 6-arg: NtAllocateVirtualMemory */
    PVOID alloc_addr = NULL;
    SIZE_T alloc_size = 4096;
    printf("[6-arg] NtAllocateVM: "); fflush(stdout);
    st = SW3NtAllocateVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, 0,
                                     (unsigned long*)&alloc_size, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    printf("0x%08X %s (addr=%p)\n", (unsigned)st, st==0?"OK":"FAIL", alloc_addr);
    if (alloc_addr) {
        SIZE_T free_size = 0;
        SW3NtFreeVirtualMemory((SW3_HANDLE)(HANDLE)-1, &alloc_addr, (unsigned long*)&free_size, MEM_RELEASE);
    }

    printf("\n=== Done ===\n");
    return 0;
}
