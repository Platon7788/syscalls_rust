//! Emit standalone `syscalls.c` — CRT-free runtime (PEB walk, hash table,
//! atomic init gate, `X_Get*` helpers).

pub fn emit() -> String {
    RUNTIME.to_string()
}

const RUNTIME: &str = r####"/*
 * X-Syscalls runtime -- direct NT syscall resolution
 *
 * Auto-generated -- DO NOT EDIT MANUALLY.
 *
 * Standalone / CRT-free:
 *   * No <windows.h>, no CRT calls (no strlen/memcpy/malloc).
 *   * Atomic init via MSVC intrinsic _InterlockedCompareExchange (declared
 *     inline below -- no <intrin.h> required).
 *   * BSS-only static state -- no CRT startup needed.
 */

#include "syscalls.h"

/* ---------- MSVC intrinsic prototypes (avoid pulling <intrin.h>) ---------- */
#if defined(_MSC_VER)
long __cdecl _InterlockedCompareExchange(long volatile *dst, long xchg, long comparand);
long __cdecl _InterlockedExchange(long volatile *dst, long value);
void _mm_pause(void);
#pragma intrinsic(_InterlockedCompareExchange, _InterlockedExchange, _mm_pause)

#if defined(_M_X64)
unsigned __int64 __readgsqword(unsigned long offset);
#pragma intrinsic(__readgsqword)
#elif defined(_M_IX86)
unsigned long __readfsdword(unsigned long offset);
#pragma intrinsic(__readfsdword)
#endif

#define X_CAS(p, xchg, cmp)  _InterlockedCompareExchange((volatile long*)(p), (long)(xchg), (long)(cmp))
#define X_XCHG(p, v)         _InterlockedExchange((volatile long*)(p), (long)(v))
#define X_PAUSE()            _mm_pause()
#else
/* GCC/Clang fallback -- also drop-in for mingw. */
#define X_CAS(p, xchg, cmp)  __sync_val_compare_and_swap((volatile long*)(p), (long)(cmp), (long)(xchg))
#define X_XCHG(p, v)         __sync_lock_test_and_set((volatile long*)(p), (long)(v))
#define X_PAUSE()            __builtin_ia32_pause()
#endif

/* Full memory barrier is provided by the MSVC intrinsics above (both x86 and
 * x64 have implicit full barriers on locked ops). No explicit fence needed. */

/* ---------- Configuration ---------- */
#define X_SEED         0xB8A54425u
#define X_MAX_ENTRIES  600u

/* ---------- Syscall table ---------- */
typedef struct {
    uint32_t Hash;
    uint32_t Address;      /* RVA in ntdll */
    void*    SyscallAddress;
} X_SYSCALL_ENTRY;

/* Count encoding:
 *    0                  -- uninitialized
 *    1 .. X_MAX_ENTRIES -- published; that many valid entries
 *    (long)-1           -- init attempted and failed (ntdll not found)
 */
#define X_COUNT_FAILED ((long)-1)

typedef struct {
    volatile long   Count;
    volatile long   InitStarted;   /* CAS gate: 0 = free, 1 = someone owns init */
    X_SYSCALL_ENTRY Entries[X_MAX_ENTRIES];
} X_SYSCALL_LIST;

static X_SYSCALL_LIST X_SyscallList; /* BSS: all zero on load */

/* Return-address spoofing target (x64 stubs read this):
 * points at a single `ret` (0xC3) inside ntdll. Every `syscall; ret` gadget
 * we resolve ends with a bare `ret` at offset +2, so we lift the address of
 * one of those and cache it here. */
void* X_NtdllRetGadget; /* exported so MASM stubs can EXTERN it */

/* ---------- PEB structures ---------- */
typedef struct _X_LIST_ENTRY {
    struct _X_LIST_ENTRY *Flink;
    struct _X_LIST_ENTRY *Blink;
} X_LIST_ENTRY, *X_PLIST_ENTRY;

typedef struct _X_PEB_LDR_DATA {
    uint8_t       Reserved1[8];
    void*         Reserved2[3];
    X_LIST_ENTRY  InMemoryOrderModuleList;
} X_PEB_LDR_DATA, *X_PPEB_LDR_DATA;

typedef struct _X_LDR_DATA_TABLE_ENTRY {
    void*         Reserved1[2];
    X_LIST_ENTRY  InMemoryOrderLinks;
    void*         Reserved2[2];
    void*         DllBase;
} X_LDR_DATA_TABLE_ENTRY, *X_PLDR_DATA_TABLE_ENTRY;

typedef struct _X_PEB {
    uint8_t          Reserved1[2];
    uint8_t          BeingDebugged;
    uint8_t          Reserved2[1];
    void*            Reserved3[2];
    X_PPEB_LDR_DATA  Ldr;
} X_PEB, *X_PPEB;

/* ---------- PE structures ---------- */
#pragma pack(push, 1)
typedef struct _X_IMAGE_DOS_HEADER {
    uint16_t e_magic;
    uint16_t e_cblp, e_cp, e_crlc, e_cparhdr;
    uint16_t e_minalloc, e_maxalloc;
    uint16_t e_ss, e_sp, e_csum;
    uint16_t e_ip, e_cs, e_lfarlc, e_ovno;
    uint16_t e_res[4];
    uint16_t e_oemid, e_oeminfo;
    uint16_t e_res2[10];
    int32_t  e_lfanew;
} X_IMAGE_DOS_HEADER;
#pragma pack(pop)

typedef struct _X_IMAGE_EXPORT_DIRECTORY {
    uint32_t Characteristics;
    uint32_t TimeDateStamp;
    uint16_t MajorVersion;
    uint16_t MinorVersion;
    uint32_t Name;
    uint32_t Base;
    uint32_t NumberOfFunctions;
    uint32_t NumberOfNames;
    uint32_t AddressOfFunctions;
    uint32_t AddressOfNames;
    uint32_t AddressOfNameOrdinals;
} X_IMAGE_EXPORT_DIRECTORY;

/* ---------- Utility ---------- */
static X_PPEB X_GetPeb(void) {
#if defined(_M_X64) || defined(__x86_64__)
    return (X_PPEB)(uintptr_t)__readgsqword(0x60);
#elif defined(_M_IX86) || defined(__i386__)
    return (X_PPEB)(uintptr_t)__readfsdword(0x30);
#else
#   error "unsupported architecture"
#endif
}

/* ROR8 hash -- matches the Rust runtime bit for bit. */
static uint32_t X_HashSyscall(const uint8_t *name) {
    uint32_t hash = X_SEED;
    size_t i = 0;
    while (name[i] != 0) {
        uint32_t b1 = (uint32_t)name[i];
        uint32_t b2 = (uint32_t)name[i + 1];
        uint32_t partial = b1 | (b2 << 8);
        uint32_t rot = (hash >> 8) | (hash << 24);
        hash ^= (partial + rot);
        ++i;
    }
    return hash;
}

/* Locate `syscall; ret` (or `sysenter; ret`) near the exported function so
 * jumper mode can spoof the return address into ntdll. */
static void* X_FindSyscallAddress(void *nt_api_address) {
#if defined(_M_X64) || defined(__x86_64__)
    static const uint8_t code[3] = { 0x0F, 0x05, 0xC3 }; /* syscall; ret */
    const size_t distance = 0x12;
#else
    static const uint8_t code[3] = { 0x0F, 0x34, 0xC3 }; /* sysenter; ret */
    const size_t distance = 0x0F;
#endif
    const size_t search_limit = 512;
    uint8_t *base = (uint8_t*)nt_api_address;

    /* Direct offset first. */
    uint8_t *addr = base + distance;
    if (addr[0] == code[0] && addr[1] == code[1] && addr[2] == code[2]) {
        return addr;
    }

    /* HalosGate walk if hooked. */
    for (size_t j = 1; j < search_limit; ++j) {
        addr = base + distance + j * 0x20;
        if (addr[0] == code[0] && addr[1] == code[1] && addr[2] == code[2]) {
            return addr;
        }
        if (distance >= j * 0x20) {
            addr = base + distance - j * 0x20;
            if (addr[0] == code[0] && addr[1] == code[1] && addr[2] == code[2]) {
                return addr;
            }
        }
    }
    return 0;
}

static int X_PopulateSyscallList(void) {
    /* Fast path: publication already happened. */
    {
        long snap = X_SyscallList.Count;
        if (snap > 0) return 1;
        if (snap == X_COUNT_FAILED) return 0;
    }

    /* Serialise initialisers -- winner walks ntdll, losers spin on Count. */
    if (X_CAS(&X_SyscallList.InitStarted, 1, 0) != 0) {
        for (;;) {
            long c = X_SyscallList.Count;
            if (c > 0) return 1;
            if (c == X_COUNT_FAILED) return 0;
            X_PAUSE();
        }
    }

    /* --- winner path --- */
    X_PPEB peb = X_GetPeb();
    if (!peb || !peb->Ldr) {
        X_XCHG(&X_SyscallList.Count, X_COUNT_FAILED);
        return 0;
    }

    X_PLIST_ENTRY head = &peb->Ldr->InMemoryOrderModuleList;
    X_PLIST_ENTRY cur  = head->Flink;

    while (cur != head) {
#if defined(_M_X64) || defined(__x86_64__)
        X_PLDR_DATA_TABLE_ENTRY entry = (X_PLDR_DATA_TABLE_ENTRY)((uint8_t*)cur - 0x10);
#else
        X_PLDR_DATA_TABLE_ENTRY entry = (X_PLDR_DATA_TABLE_ENTRY)((uint8_t*)cur - 0x08);
#endif
        void *dll_base = entry->DllBase;
        if (!dll_base) { cur = cur->Flink; continue; }

        X_IMAGE_DOS_HEADER *dos = (X_IMAGE_DOS_HEADER*)dll_base;
        if (dos->e_magic != 0x5A4D) { cur = cur->Flink; continue; }

        uint8_t *nt = (uint8_t*)dll_base + dos->e_lfanew;
#if defined(_M_X64) || defined(__x86_64__)
        uint32_t export_rva = *(uint32_t*)(nt + 0x88);
#else
        uint32_t export_rva = *(uint32_t*)(nt + 0x78);
#endif
        if (!export_rva) { cur = cur->Flink; continue; }

        X_IMAGE_EXPORT_DIRECTORY *exp =
            (X_IMAGE_EXPORT_DIRECTORY*)((uint8_t*)dll_base + export_rva);
        uint8_t *dll_name = (uint8_t*)dll_base + exp->Name;

        /* Case-insensitive check for "ntdll" (four low bytes). */
        uint32_t name_check = (*(uint32_t*)dll_name) | 0x20202020u;
        if (name_check != 0x6C64746Eu) { cur = cur->Flink; continue; }

        /* Found ntdll -- enumerate Zw* exports (backwards, matches SW3 C). */
        uint32_t num_names = exp->NumberOfNames;
        uint32_t *names     = (uint32_t*)((uint8_t*)dll_base + exp->AddressOfNames);
        uint32_t *functions = (uint32_t*)((uint8_t*)dll_base + exp->AddressOfFunctions);
        uint16_t *ordinals  = (uint16_t*)((uint8_t*)dll_base + exp->AddressOfNameOrdinals);

        uint32_t count = 0;
        uint32_t i = num_names;
        while (i > 0) {
            --i;
            uint32_t name_rva = names[i];
            uint8_t *func_name = (uint8_t*)dll_base + name_rva;
            if (func_name[0] == 'Z' && func_name[1] == 'w') {
                uint16_t ord      = ordinals[i];
                uint32_t func_rva = functions[ord];
                X_SYSCALL_ENTRY *slot = &X_SyscallList.Entries[count];
                slot->Hash           = X_HashSyscall(func_name);
                slot->Address        = func_rva;
                slot->SyscallAddress = X_FindSyscallAddress((uint8_t*)dll_base + func_rva);
                ++count;
                if (count >= X_MAX_ENTRIES) break;
            }
        }

        /* Selection sort by Address (small N, no CRT). */
        for (uint32_t ii = 0; ii < count; ++ii) {
            uint32_t min_idx = ii;
            for (uint32_t j = ii + 1; j < count; ++j) {
                if (X_SyscallList.Entries[j].Address < X_SyscallList.Entries[min_idx].Address) {
                    min_idx = j;
                }
            }
            if (min_idx != ii) {
                X_SYSCALL_ENTRY tmp = X_SyscallList.Entries[ii];
                X_SyscallList.Entries[ii] = X_SyscallList.Entries[min_idx];
                X_SyscallList.Entries[min_idx] = tmp;
            }
        }

        /* Pick a ntdll `ret` for RAS: byte +2 past any resolved `syscall; ret`
         * gadget IS a bare `ret` (0xC3). Cache the first non-null one. */
        for (uint32_t k = 0; k < count; ++k) {
            uint8_t *g = (uint8_t*)X_SyscallList.Entries[k].SyscallAddress;
            if (g) { X_NtdllRetGadget = g + 2; break; }
        }

        /* Publish: any thread seeing Count>0 sees fully-initialised entries
         * AND the retaddr-spoofing gadget. */
        X_XCHG(&X_SyscallList.Count, (long)count);
        return 1;
    }

    /* ntdll not found on the module list -- mark as failed, wake spinners. */
    X_XCHG(&X_SyscallList.Count, X_COUNT_FAILED);
    return 0;
}

/* ---------- Public helpers (called by the MASM stubs) ---------- */

uint32_t X_GetSyscallNumber(uint32_t function_hash) {
    if (!X_PopulateSyscallList()) return 0xFFFFFFFFu;
    long count = X_SyscallList.Count;
    for (long i = 0; i < count; ++i) {
        if (X_SyscallList.Entries[i].Hash == function_hash) {
            return (uint32_t)i;
        }
    }
    return 0xFFFFFFFFu;
}

void* X_GetSyscallAddress(uint32_t function_hash) {
    if (!X_PopulateSyscallList()) return 0;
    long count = X_SyscallList.Count;
    for (long i = 0; i < count; ++i) {
        if (X_SyscallList.Entries[i].Hash == function_hash) {
            void *addr = X_SyscallList.Entries[i].SyscallAddress;
            if (addr) return addr;
            break;
        }
    }
    /* Fallback: any valid syscall address. */
    for (long i = 0; i < count; ++i) {
        void *addr = X_SyscallList.Entries[i].SyscallAddress;
        if (addr) return addr;
    }
    return 0;
}

void* X_GetRandomSyscallAddress(uint32_t function_hash) {
    if (!X_PopulateSyscallList()) return 0;
    long count = X_SyscallList.Count;
    if (count == 0) return 0;

    long index = (long)(function_hash % (uint32_t)count);
    long attempts = 0;
    while ((X_SyscallList.Entries[index].Hash == function_hash
         || X_SyscallList.Entries[index].SyscallAddress == 0)
        && attempts < count) {
        index = (index + 1) % count;
        ++attempts;
    }
    return X_SyscallList.Entries[index].SyscallAddress;
}

uint32_t X_DebugGetCount(void) {
    X_PopulateSyscallList();
    return (uint32_t)X_SyscallList.Count;
}

uint32_t X_DebugGetHash(size_t index) {
    if ((long)index < X_SyscallList.Count) {
        return X_SyscallList.Entries[index].Hash;
    }
    return 0;
}

void* X_DebugGetSyscallAddr(size_t index) {
    if ((long)index < X_SyscallList.Count) {
        return X_SyscallList.Entries[index].SyscallAddress;
    }
    return 0;
}
"####;
