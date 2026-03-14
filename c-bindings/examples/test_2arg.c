#include <stdio.h>
#include <stdint.h>
#include <windows.h>
#include "syscalls.h"

int main(void) {
    printf("Testing 2-arg NtDelayExecution via RUST...\n");
    LARGE_INTEGER delay;
    delay.QuadPart = -1; /* 100ns = near instant */
    printf("  delay addr=%p, val=%lld\n", &delay, delay.QuadPart);
    printf("  Calling...\n");
    fflush(stdout);
    NTSTATUS st = SW3NtDelayExecution(0, (SW3_LARGE_INTEGER*)&delay);
    printf("  Result: 0x%08X\n", (unsigned)st);
    return 0;
}
