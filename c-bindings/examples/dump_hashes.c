/**
 * Dump all syscall hashes from the table
 */
#include <stdio.h>
#include <stdint.h>
#include <windows.h>
#include "syscalls.h"

extern uint32_t SW3Sw3DebugGetCount(void);
extern uint32_t SW3Sw3DebugGetHash(size_t index);

int main(void) {
    uint32_t count = SW3Sw3DebugGetCount();
    printf("Syscall table: %u entries\n\n", count);
    for (uint32_t i = 0; i < count; i++) {
        printf("[%3u] hash=0x%08X\n", i, SW3Sw3DebugGetHash(i));
    }
    return 0;
}
