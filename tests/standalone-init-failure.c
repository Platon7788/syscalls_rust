/* Exercise the actual emitted private publisher with synthetic BSS state.
 * No OS/PEB corruption and no native syscall invocation are involved. */
#include <windows.h>
#include <stdio.h>
#include "syscalls.c"

static DWORD WINAPI wait_for_table(void *unused) {
    (void)unused;
    return (DWORD)X_PopulateSyscallList();
}

int main(int argc, char **argv) {
    HANDLE waiters[4];
    DWORD results[4];
    unsigned int i;
    int published;
    int finished;
    int bad_debug;
    (void)argv;
    if (argc > 2) {
        uint32_t hash = X_HashSyscall((const uint8_t*)"ZwQuerySystemTime");
        uint32_t count = X_DebugGetCount();
        uint32_t number = X_GetSyscallNumber(hash);
        if (count == 0 || count >= X_MAX_ENTRIES || number >= count
            || X_DebugGetHash(number) != hash) return 1;
#if defined(_M_X64)
        if (!X_GetSyscallAddress(hash)) return 1;
#endif
        if (X_DebugGetHash(SIZE_MAX) != 0 || X_DebugGetSyscallAddr(SIZE_MAX) != 0) return 1;
        printf("real Windows table initialized: %u entries\n", count);
        return 0;
    }
    /* This isolated executable owns its own BSS table. Mark an initializer
     * in progress, then finish it through the production completion helper. */
    X_XCHG(&X_SyscallList.InitStarted, 1);
    for (i = 0; i < 4; ++i) {
        waiters[i] = CreateThread(0, 0, wait_for_table, 0, 0, 0);
        if (!waiters[i]) return 2;
    }
    published = X_PublishCount(argc > 1 ? (long)X_MAX_ENTRIES : 0);
    finished = WaitForMultipleObjects(4, waiters, TRUE, 300) == WAIT_OBJECT_0;
    /* Before old-loop cleanup, debug count must never expose -1 as UINT_MAX. */
    bad_debug = finished && (X_DebugGetCount() != 0 || X_DebugGetHash(0) != 0
                 || X_DebugGetSyscallAddr(0) != 0);
    if (!finished) X_XCHG(&X_SyscallList.Count, 1);
    if (WaitForMultipleObjects(4, waiters, TRUE, 2000) != WAIT_OBJECT_0) return 3;
    for (i = 0; i < 4; ++i) {
        if (!GetExitCodeThread(waiters[i], &results[i])) return 4;
        CloseHandle(waiters[i]);
    }
    if (published || !finished || bad_debug) {
        fprintf(stderr, "invalid publication=%d, waiters finished=%d, invalid debug=%d\n", published, finished, bad_debug);
        return 1;
    }
    for (i = 0; i < 4; ++i) if (results[i] != 0) return 1;
    /* The terminal fast path must not attempt a fresh PEB walk. */
    if (X_DebugGetHash(SIZE_MAX) != 0 || X_DebugGetSyscallAddr(SIZE_MAX) != 0) return 1;
    if (X_GetSyscallNumber(0) != 0xFFFFFFFFu || X_GetSyscallAddress(0) != 0
        || X_GetRandomSyscallAddress(0) != 0) return 1;
    puts("empty/full/failed publication finishes all waiters; debug count is zero");
    return 0;
}
