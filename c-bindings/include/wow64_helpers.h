/*
 * wow64_helpers.h - Header-only utilities for inspecting and manipulating
 *                   WoW64 (x86) targets from an x64 host using SW3 syscalls.
 *
 * Scope:
 *   - Detect whether a target process is WoW64
 *   - Read the 32-bit PEB and walk the x86 loader module list
 *   - Locate a module in the x86 image and resolve x86 export addresses
 *     (with forwarded-export following: kernel32 -> KernelBase, etc.)
 *   - Get / set the real x86 register context of a WoW64 thread
 *   - Convenience wrappers for cross-architecture memory I/O
 *
 * Out of scope (do them yourself if needed):
 *   - Ordinal-only forwards ("DLL.#42")
 *   - Manual mapping of x86 PE images
 *   - Heaven's Gate / executing x64 code inside WoW64
 *
 * Compile-time requirements:
 *   - Host build: x86_64 (this helper assumes the *caller* is x64)
 *   - C99 or later; no dependency on <windows.h>
 *   - Pulls in "syscalls.h" for SW3_* types and Nt* declarations
 *
 * Thread-safety: stateless; all state lives on the caller's stack.
 */

#ifndef SW3_WOW64_HELPERS_H
#define SW3_WOW64_HELPERS_H

#include "syscalls.h"

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

/* NTSTATUS shortcuts (SW3_STATUS_*) are provided by syscalls.h. */

/* =============================================================
 * Info classes used here
 * (full list -> winternl.h / ntddk.h; only what we need)
 * ============================================================= */
#define SW3_ProcessBasicInformation       0
#define SW3_ProcessWow64Information       26
#define SW3_ThreadBasicInformation        0
#define SW3_ThreadWow64Context            29

/* =============================================================
 * WOW64_CONTEXT - real x86 register context of a WoW64 thread
 * Layout matches the documented x86 CONTEXT structure.
 * ============================================================= */
/* If <winnt.h> already defined these, just reuse - same numeric values. */
#ifndef WOW64_CONTEXT_i386
#define WOW64_CONTEXT_i386                 0x00010000u
#endif
#ifndef WOW64_CONTEXT_CONTROL
#define WOW64_CONTEXT_CONTROL              (WOW64_CONTEXT_i386 | 0x1u)
#endif
#ifndef WOW64_CONTEXT_INTEGER
#define WOW64_CONTEXT_INTEGER              (WOW64_CONTEXT_i386 | 0x2u)
#endif
#ifndef WOW64_CONTEXT_SEGMENTS
#define WOW64_CONTEXT_SEGMENTS             (WOW64_CONTEXT_i386 | 0x4u)
#endif
#ifndef WOW64_CONTEXT_FLOATING_POINT
#define WOW64_CONTEXT_FLOATING_POINT       (WOW64_CONTEXT_i386 | 0x8u)
#endif
#ifndef WOW64_CONTEXT_DEBUG_REGISTERS
#define WOW64_CONTEXT_DEBUG_REGISTERS      (WOW64_CONTEXT_i386 | 0x10u)
#endif
#ifndef WOW64_CONTEXT_EXTENDED_REGISTERS
#define WOW64_CONTEXT_EXTENDED_REGISTERS   (WOW64_CONTEXT_i386 | 0x20u)
#endif
#ifndef WOW64_CONTEXT_FULL
#define WOW64_CONTEXT_FULL \
    (WOW64_CONTEXT_CONTROL | WOW64_CONTEXT_INTEGER | WOW64_CONTEXT_SEGMENTS)
#endif
#ifndef WOW64_CONTEXT_ALL
#define WOW64_CONTEXT_ALL \
    (WOW64_CONTEXT_FULL | WOW64_CONTEXT_FLOATING_POINT | \
     WOW64_CONTEXT_DEBUG_REGISTERS | WOW64_CONTEXT_EXTENDED_REGISTERS)
#endif

#define WOW64_SIZE_OF_80387_REGISTERS      80
#define WOW64_MAXIMUM_SUPPORTED_EXTENSION  512

typedef struct _SW3_WOW64_FLOATING_SAVE_AREA {
    uint32_t ControlWord;
    uint32_t StatusWord;
    uint32_t TagWord;
    uint32_t ErrorOffset;
    uint32_t ErrorSelector;
    uint32_t DataOffset;
    uint32_t DataSelector;
    uint8_t  RegisterArea[WOW64_SIZE_OF_80387_REGISTERS];
    uint32_t Cr0NpxState;
} SW3_WOW64_FLOATING_SAVE_AREA;

typedef struct _SW3_WOW64_CONTEXT {
    uint32_t ContextFlags;
    uint32_t Dr0, Dr1, Dr2, Dr3, Dr6, Dr7;
    SW3_WOW64_FLOATING_SAVE_AREA FloatSave;
    uint32_t SegGs, SegFs, SegEs, SegDs;
    uint32_t Edi, Esi, Ebx, Edx, Ecx, Eax;
    uint32_t Ebp, Eip, SegCs, EFlags, Esp, SegSs;
    uint8_t  ExtendedRegisters[WOW64_MAXIMUM_SUPPORTED_EXTENSION];
} SW3_WOW64_CONTEXT;

/* =============================================================
 * 32-bit mirror structures (target process address space)
 * Layouts are stable across all current Windows x86 builds;
 * trailing fields added in later builds are simply not read.
 * ============================================================= */
typedef struct _SW3_UNICODE_STRING32 {
    uint16_t Length;          /* in bytes, not characters */
    uint16_t MaximumLength;
    uint32_t Buffer;          /* x86 PWSTR (32-bit address) */
} SW3_UNICODE_STRING32;

typedef struct _SW3_LIST_ENTRY32 {
    uint32_t Flink;
    uint32_t Blink;
} SW3_LIST_ENTRY32;

typedef struct _SW3_PEB_LDR_DATA32 {
    uint32_t Length;
    uint8_t  Initialized;
    uint8_t  _pad0[3];
    uint32_t SsHandle;
    SW3_LIST_ENTRY32 InLoadOrderModuleList;
    SW3_LIST_ENTRY32 InMemoryOrderModuleList;
    SW3_LIST_ENTRY32 InInitializationOrderModuleList;
    uint32_t EntryInProgress;
    uint8_t  ShutdownInProgress;
    uint8_t  _pad1[3];
    uint32_t ShutdownThreadId;
} SW3_PEB_LDR_DATA32;

typedef struct _SW3_LDR_DATA_TABLE_ENTRY32 {
    SW3_LIST_ENTRY32     InLoadOrderLinks;
    SW3_LIST_ENTRY32     InMemoryOrderLinks;
    SW3_LIST_ENTRY32     InInitializationOrderLinks;
    uint32_t             DllBase;
    uint32_t             EntryPoint;
    uint32_t             SizeOfImage;
    SW3_UNICODE_STRING32 FullDllName;
    SW3_UNICODE_STRING32 BaseDllName;
    uint32_t             Flags;
    uint16_t             LoadCount;
    uint16_t             TlsIndex;
    SW3_LIST_ENTRY32     HashLinks;
    uint32_t             TimeDateStamp;
    /* (more fields exist on Win8+; we don't read them) */
} SW3_LDR_DATA_TABLE_ENTRY32;

typedef struct _SW3_PEB32 {
    uint8_t  InheritedAddressSpace;
    uint8_t  ReadImageFileExecOptions;
    uint8_t  BeingDebugged;
    uint8_t  BitField;
    uint32_t Mutant;
    uint32_t ImageBaseAddress;
    uint32_t Ldr;              /* -> SW3_PEB_LDR_DATA32* (32-bit) */
    uint32_t ProcessParameters;
    /* (rest omitted - not needed here) */
} SW3_PEB32;

/* =============================================================
 * PE32 headers (x86 images in the WoW64 target)
 * ============================================================= */
#define SW3_IMAGE_DOS_SIGNATURE                0x5A4Du   /* 'MZ' */
#define SW3_IMAGE_NT_SIGNATURE                 0x00004550u /* 'PE\0\0' */
#define SW3_IMAGE_NUMBEROF_DIRECTORY_ENTRIES   16
#define SW3_IMAGE_DIRECTORY_ENTRY_EXPORT       0

typedef struct _SW3_IMAGE_DOS_HEADER {
    uint16_t e_magic;
    uint16_t _ignored[29];
    int32_t  e_lfanew;
} SW3_IMAGE_DOS_HEADER;

typedef struct _SW3_IMAGE_FILE_HEADER {
    uint16_t Machine;
    uint16_t NumberOfSections;
    uint32_t TimeDateStamp;
    uint32_t PointerToSymbolTable;
    uint32_t NumberOfSymbols;
    uint16_t SizeOfOptionalHeader;
    uint16_t Characteristics;
} SW3_IMAGE_FILE_HEADER;

typedef struct _SW3_IMAGE_DATA_DIRECTORY {
    uint32_t VirtualAddress;
    uint32_t Size;
} SW3_IMAGE_DATA_DIRECTORY;

typedef struct _SW3_IMAGE_OPTIONAL_HEADER32 {
    uint16_t Magic;                 /* 0x10B for PE32 */
    uint8_t  MajorLinkerVersion;
    uint8_t  MinorLinkerVersion;
    uint32_t SizeOfCode;
    uint32_t SizeOfInitializedData;
    uint32_t SizeOfUninitializedData;
    uint32_t AddressOfEntryPoint;
    uint32_t BaseOfCode;
    uint32_t BaseOfData;
    uint32_t ImageBase;
    uint32_t SectionAlignment;
    uint32_t FileAlignment;
    uint16_t MajorOperatingSystemVersion;
    uint16_t MinorOperatingSystemVersion;
    uint16_t MajorImageVersion;
    uint16_t MinorImageVersion;
    uint16_t MajorSubsystemVersion;
    uint16_t MinorSubsystemVersion;
    uint32_t Win32VersionValue;
    uint32_t SizeOfImage;
    uint32_t SizeOfHeaders;
    uint32_t CheckSum;
    uint16_t Subsystem;
    uint16_t DllCharacteristics;
    uint32_t SizeOfStackReserve;
    uint32_t SizeOfStackCommit;
    uint32_t SizeOfHeapReserve;
    uint32_t SizeOfHeapCommit;
    uint32_t LoaderFlags;
    uint32_t NumberOfRvaAndSizes;
    SW3_IMAGE_DATA_DIRECTORY DataDirectory[SW3_IMAGE_NUMBEROF_DIRECTORY_ENTRIES];
} SW3_IMAGE_OPTIONAL_HEADER32;

typedef struct _SW3_IMAGE_NT_HEADERS32 {
    uint32_t Signature;
    SW3_IMAGE_FILE_HEADER FileHeader;
    SW3_IMAGE_OPTIONAL_HEADER32 OptionalHeader;
} SW3_IMAGE_NT_HEADERS32;

typedef struct _SW3_IMAGE_EXPORT_DIRECTORY {
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
} SW3_IMAGE_EXPORT_DIRECTORY;

/* =============================================================
 * Small primitives
 * ============================================================= */

/* Cast a 32-bit target-space address to SW3_PVOID (zero-extends on x64). */
static inline SW3_PVOID sw3_addr32(uint32_t addr32) {
    return (SW3_PVOID)(uintptr_t)addr32;
}

/* Read `size` bytes from the target at 32-bit address `addr32`. */
static inline SW3_NTSTATUS sw3_wow64_read(SW3_HANDLE hProc,
                                          uint32_t addr32,
                                          void* buf,
                                          SW3_SIZE_T size) {
    SW3_SIZE_T got = 0;
    return SW3NtReadVirtualMemory(hProc, sw3_addr32(addr32),
                                  (SW3_PVOID)buf, size, &got);
}

/* Write `size` bytes to the target at 32-bit address `addr32`. */
static inline SW3_NTSTATUS sw3_wow64_write(SW3_HANDLE hProc,
                                           uint32_t addr32,
                                           const void* buf,
                                           SW3_SIZE_T size) {
    SW3_SIZE_T put = 0;
    return SW3NtWriteVirtualMemory(hProc, sw3_addr32(addr32),
                                   (SW3_PVOID)(uintptr_t)buf, size, &put);
}

/* Length of a null-terminated UTF-16 string (in code units). */
static inline size_t sw3_wcslen16(const uint16_t* s) {
    size_t n = 0;
    while (s[n]) n++;
    return n;
}

/* ASCII-only case-insensitive UTF-16 compare. Good enough for DLL names. */
static inline int sw3_wcs_eq_iascii(const uint16_t* a, size_t la,
                                    const uint16_t* b, size_t lb) {
    if (la != lb) return 0;
    for (size_t i = 0; i < la; i++) {
        uint16_t ca = a[i], cb = b[i];
        if (ca >= 'A' && ca <= 'Z') ca = (uint16_t)(ca - 'A' + 'a');
        if (cb >= 'A' && cb <= 'Z') cb = (uint16_t)(cb - 'A' + 'a');
        if (ca != cb) return 0;
    }
    return 1;
}

/* =============================================================
 * Detection + PEB32 access
 * ============================================================= */

/*
 * Is the target a WoW64 process? Also returns the 32-bit PEB address.
 *
 * `out_peb32_addr` may be NULL.
 * Required process access: PROCESS_QUERY_INFORMATION (or _LIMITED_).
 */
static inline SW3_NTSTATUS sw3_wow64_detect(SW3_HANDLE hProc,
                                            bool* out_is_wow64,
                                            uint32_t* out_peb32_addr) {
    /* ProcessWow64Information writes sizeof(PVOID) bytes -
     * 8 on x64 host, 4 on x86 host. uintptr_t covers both. */
    uintptr_t peb32_val = 0;
    SW3_ULONG ret = 0;
    SW3_NTSTATUS s = SW3NtQueryInformationProcess(
        hProc, SW3_ProcessWow64Information,
        &peb32_val, (SW3_ULONG)sizeof(peb32_val), &ret);
    if (s < 0) return s;
    if (out_is_wow64)    *out_is_wow64 = (peb32_val != 0);
    if (out_peb32_addr)  *out_peb32_addr = (uint32_t)peb32_val;
    return SW3_STATUS_SUCCESS;
}

/* Read the PEB32 of a WoW64 target. */
static inline SW3_NTSTATUS sw3_wow64_read_peb(SW3_HANDLE hProc,
                                              uint32_t peb32_addr,
                                              SW3_PEB32* out_peb) {
    return sw3_wow64_read(hProc, peb32_addr, out_peb, sizeof(*out_peb));
}

/* =============================================================
 * Module enumeration
 * ============================================================= */

/*
 * Callback invoked for every x86 loader entry.
 *   entry             - the (partially read) LDR_DATA_TABLE_ENTRY32
 *   base_name_utf16   - UTF-16LE BaseDllName, NUL-terminated within `name_chars + 1`
 *   base_name_chars   - length in code units (no terminator)
 *   ctx               - caller-supplied pointer
 * Return true to keep walking, false to stop early.
 */
typedef bool (*sw3_wow64_module_cb)(const SW3_LDR_DATA_TABLE_ENTRY32* entry,
                                    const uint16_t* base_name_utf16,
                                    size_t base_name_chars,
                                    void* ctx);

/*
 * Walk InLoadOrderModuleList of a WoW64 target.
 * Stops gracefully on read errors and on a sanity cap of 4096 entries
 * (a corrupted / hostile target won't deadlock the caller).
 */
static inline SW3_NTSTATUS sw3_wow64_enum_modules(SW3_HANDLE hProc,
                                                  uint32_t peb32_addr,
                                                  sw3_wow64_module_cb cb,
                                                  void* ctx) {
    if (!cb) return SW3_STATUS_INVALID_PARAMETER;

    SW3_PEB32 peb;
    SW3_NTSTATUS s = sw3_wow64_read_peb(hProc, peb32_addr, &peb);
    if (s < 0) return s;
    if (peb.Ldr == 0) return SW3_STATUS_UNSUCCESSFUL;

    SW3_PEB_LDR_DATA32 ldr;
    s = sw3_wow64_read(hProc, peb.Ldr, &ldr, sizeof(ldr));
    if (s < 0) return s;

    uint32_t head_addr = peb.Ldr +
        (uint32_t)offsetof(SW3_PEB_LDR_DATA32, InLoadOrderModuleList);
    uint32_t cur = ldr.InLoadOrderModuleList.Flink;
    int sanity = 4096;

    while (cur != 0 && cur != head_addr && sanity-- > 0) {
        SW3_LDR_DATA_TABLE_ENTRY32 mod;
        s = sw3_wow64_read(hProc, cur, &mod, sizeof(mod));
        if (s < 0) return s;

        uint16_t name[260];
        size_t name_chars = (size_t)(mod.BaseDllName.Length / sizeof(uint16_t));
        if (name_chars > 259) name_chars = 259;
        if (name_chars > 0 && mod.BaseDllName.Buffer) {
            s = sw3_wow64_read(hProc, mod.BaseDllName.Buffer,
                               name, name_chars * sizeof(uint16_t));
            if (s < 0) return s;
        }
        name[name_chars] = 0;

        if (!cb(&mod, name, name_chars, ctx)) {
            return SW3_STATUS_SUCCESS;   /* callback asked to stop */
        }

        cur = mod.InLoadOrderLinks.Flink;
    }
    return SW3_STATUS_SUCCESS;
}

/* Internal context for find-by-name. */
typedef struct {
    const uint16_t* needle;
    size_t needle_chars;
    uint32_t out_base;
    uint32_t out_size;
    bool found;
} sw3_wow64_find_ctx_t;

static inline bool sw3_wow64_find_cb_(const SW3_LDR_DATA_TABLE_ENTRY32* e,
                                      const uint16_t* name, size_t nlen,
                                      void* ctx_) {
    sw3_wow64_find_ctx_t* fc = (sw3_wow64_find_ctx_t*)ctx_;
    if (sw3_wcs_eq_iascii(name, nlen, fc->needle, fc->needle_chars)) {
        fc->out_base = e->DllBase;
        fc->out_size = e->SizeOfImage;
        fc->found = true;
        return false;
    }
    return true;
}

/* Find an x86 module by name (UTF-16LE). Returns STATUS_DLL_NOT_FOUND if absent. */
static inline SW3_NTSTATUS sw3_wow64_find_module_w(SW3_HANDLE hProc,
                                                   uint32_t peb32_addr,
                                                   const uint16_t* dll_name_utf16,
                                                   uint32_t* out_dll_base,
                                                   uint32_t* out_size_of_image) {
    sw3_wow64_find_ctx_t fc;
    fc.needle       = dll_name_utf16;
    fc.needle_chars = sw3_wcslen16(dll_name_utf16);
    fc.out_base     = 0;
    fc.out_size     = 0;
    fc.found        = false;
    SW3_NTSTATUS s = sw3_wow64_enum_modules(hProc, peb32_addr,
                                            sw3_wow64_find_cb_, &fc);
    if (s < 0) return s;
    if (!fc.found) return SW3_STATUS_DLL_NOT_FOUND;
    if (out_dll_base)       *out_dll_base       = fc.out_base;
    if (out_size_of_image)  *out_size_of_image  = fc.out_size;
    return SW3_STATUS_SUCCESS;
}

/* ASCII convenience: "kernel32.dll" -> finds it via UTF-16 walk. */
static inline SW3_NTSTATUS sw3_wow64_find_module(SW3_HANDLE hProc,
                                                 uint32_t peb32_addr,
                                                 const char* dll_name_ascii,
                                                 uint32_t* out_dll_base,
                                                 uint32_t* out_size_of_image) {
    uint16_t wname[260];
    size_t n = 0;
    while (dll_name_ascii[n] && n < 259) {
        wname[n] = (uint16_t)(unsigned char)dll_name_ascii[n];
        n++;
    }
    wname[n] = 0;
    return sw3_wow64_find_module_w(hProc, peb32_addr, wname,
                                   out_dll_base, out_size_of_image);
}

/* =============================================================
 * Export resolution (with forwarder following)
 * ============================================================= */

/* Read up to `cap-1` ASCII bytes from target, terminating at NUL. */
static inline SW3_NTSTATUS sw3_wow64_read_cstr_(SW3_HANDLE hProc,
                                                uint32_t addr32,
                                                char* dst, size_t cap) {
    if (cap == 0) return SW3_STATUS_BUFFER_TOO_SMALL;
    SW3_NTSTATUS s = sw3_wow64_read(hProc, addr32, dst, cap - 1);
    if (s < 0) { dst[0] = 0; return s; }
    dst[cap - 1] = 0;
    /* Best-effort NUL guarantee: stop at first NUL if any. */
    for (size_t i = 0; i < cap; i++) {
        if (dst[i] == 0) return SW3_STATUS_SUCCESS;
    }
    dst[cap - 1] = 0;
    return SW3_STATUS_SUCCESS;
}

#ifndef SW3_WOW64_MAX_EXPORTS
/* kernel32 ~1500, ntdll ~2500. 4096 covers all standard DLLs. */
#define SW3_WOW64_MAX_EXPORTS 4096
#endif

#ifndef SW3_WOW64_MAX_FORWARD_DEPTH
#define SW3_WOW64_MAX_FORWARD_DEPTH 8
#endif

/* Forward decl - export resolution is recursive on forwarders. */
static inline SW3_NTSTATUS sw3_wow64_resolve_export_at_(
    SW3_HANDLE hProc, uint32_t peb32_addr,
    uint32_t dll_base, const char* func_name,
    uint32_t* out_addr, int depth);

/*
 * Resolve `dll_name!func_name` in the x86 target.
 * Follows forwarded exports (e.g. kernel32!LoadLibraryA -> KernelBase!LoadLibraryA).
 *
 * On success, *out_addr is a 32-bit target-space address ready to be used
 * as a remote thread start routine, hook target, etc.
 */
static inline SW3_NTSTATUS sw3_wow64_resolve_export(SW3_HANDLE hProc,
                                                    uint32_t peb32_addr,
                                                    const char* dll_name,
                                                    const char* func_name,
                                                    uint32_t* out_addr) {
    if (!dll_name || !func_name || !out_addr)
        return SW3_STATUS_INVALID_PARAMETER;
    uint32_t base = 0, size = 0;
    SW3_NTSTATUS s = sw3_wow64_find_module(hProc, peb32_addr, dll_name,
                                           &base, &size);
    if (s < 0) return s;
    return sw3_wow64_resolve_export_at_(hProc, peb32_addr, base,
                                        func_name, out_addr, 0);
}

/*
 * Like sw3_wow64_resolve_export() but given an explicit module base
 * (useful when you already enumerated modules and have the base in hand).
 * Still follows forwarders, so peb32_addr is required.
 */
static inline SW3_NTSTATUS sw3_wow64_resolve_export_in(SW3_HANDLE hProc,
                                                      uint32_t peb32_addr,
                                                      uint32_t dll_base,
                                                      const char* func_name,
                                                      uint32_t* out_addr) {
    if (!func_name || !out_addr || dll_base == 0)
        return SW3_STATUS_INVALID_PARAMETER;
    return sw3_wow64_resolve_export_at_(hProc, peb32_addr, dll_base,
                                        func_name, out_addr, 0);
}

static inline SW3_NTSTATUS sw3_wow64_resolve_export_at_(
    SW3_HANDLE hProc, uint32_t peb32_addr,
    uint32_t dll_base, const char* func_name,
    uint32_t* out_addr, int depth)
{
    if (depth > SW3_WOW64_MAX_FORWARD_DEPTH)
        return SW3_STATUS_ENTRYPOINT_NOT_FOUND;

    SW3_IMAGE_DOS_HEADER dos;
    SW3_NTSTATUS s = sw3_wow64_read(hProc, dll_base, &dos, sizeof(dos));
    if (s < 0) return s;
    if (dos.e_magic != SW3_IMAGE_DOS_SIGNATURE)
        return SW3_STATUS_INVALID_IMAGE_FORMAT;

    SW3_IMAGE_NT_HEADERS32 nt;
    s = sw3_wow64_read(hProc, dll_base + (uint32_t)dos.e_lfanew,
                       &nt, sizeof(nt));
    if (s < 0) return s;
    if (nt.Signature != SW3_IMAGE_NT_SIGNATURE)
        return SW3_STATUS_INVALID_IMAGE_FORMAT;

    uint32_t exp_rva  = nt.OptionalHeader
        .DataDirectory[SW3_IMAGE_DIRECTORY_ENTRY_EXPORT].VirtualAddress;
    uint32_t exp_size = nt.OptionalHeader
        .DataDirectory[SW3_IMAGE_DIRECTORY_ENTRY_EXPORT].Size;
    if (exp_rva == 0 || exp_size == 0)
        return SW3_STATUS_ENTRYPOINT_NOT_FOUND;

    SW3_IMAGE_EXPORT_DIRECTORY exp;
    s = sw3_wow64_read(hProc, dll_base + exp_rva, &exp, sizeof(exp));
    if (s < 0) return s;
    if (exp.NumberOfNames == 0 ||
        exp.NumberOfNames > SW3_WOW64_MAX_EXPORTS)
        return SW3_STATUS_ENTRYPOINT_NOT_FOUND;

    /* Bulk-read parallel arrays (cheaper than per-name round trips). */
    uint32_t name_rvas[SW3_WOW64_MAX_EXPORTS];
    uint16_t name_ords[SW3_WOW64_MAX_EXPORTS];
    s = sw3_wow64_read(hProc, dll_base + exp.AddressOfNames,
                       name_rvas, exp.NumberOfNames * sizeof(uint32_t));
    if (s < 0) return s;
    s = sw3_wow64_read(hProc, dll_base + exp.AddressOfNameOrdinals,
                       name_ords, exp.NumberOfNames * sizeof(uint16_t));
    if (s < 0) return s;

    size_t func_len = 0;
    while (func_name[func_len]) func_len++;

    for (uint32_t i = 0; i < exp.NumberOfNames; i++) {
        char nm[256];
        s = sw3_wow64_read_cstr_(hProc, dll_base + name_rvas[i],
                                 nm, sizeof(nm));
        if (s < 0) continue;
        if (strncmp(nm, func_name, func_len + 1) != 0) continue;

        uint16_t ord = name_ords[i];
        uint32_t func_rva = 0;
        s = sw3_wow64_read(hProc,
            dll_base + exp.AddressOfFunctions + ord * (uint32_t)sizeof(uint32_t),
            &func_rva, sizeof(func_rva));
        if (s < 0) return s;
        if (func_rva == 0) return SW3_STATUS_ENTRYPOINT_NOT_FOUND;

        /* Forwarded export: RVA points inside the export directory.
         * The data there is an ASCII "TargetDll.TargetFunc" string. */
        if (func_rva >= exp_rva && func_rva < exp_rva + exp_size) {
            char fwd[256];
            s = sw3_wow64_read_cstr_(hProc, dll_base + func_rva,
                                     fwd, sizeof(fwd));
            if (s < 0) return s;

            char* dot = NULL;
            for (size_t k = 0; fwd[k]; k++) {
                if (fwd[k] == '.') { dot = &fwd[k]; break; }
            }
            if (!dot) return SW3_STATUS_ENTRYPOINT_NOT_FOUND;
            *dot = 0;
            const char* fwd_func = dot + 1;

            /* Ordinal forwards ("DLL.#42") - not handled here. */
            if (fwd_func[0] == '#') return SW3_STATUS_ENTRYPOINT_NOT_FOUND;

            /* "<TargetDll>.dll" */
            char fwd_dll[96];
            size_t k = 0;
            while (fwd[k] && k < sizeof(fwd_dll) - 5) {
                fwd_dll[k] = fwd[k]; k++;
            }
            fwd_dll[k++] = '.';
            fwd_dll[k++] = 'd';
            fwd_dll[k++] = 'l';
            fwd_dll[k++] = 'l';
            fwd_dll[k]   = 0;

            uint32_t fb = 0, fs2 = 0;
            s = sw3_wow64_find_module(hProc, peb32_addr, fwd_dll, &fb, &fs2);
            if (s < 0) return s;
            return sw3_wow64_resolve_export_at_(hProc, peb32_addr, fb,
                                                fwd_func, out_addr, depth + 1);
        }

        *out_addr = dll_base + func_rva;
        return SW3_STATUS_SUCCESS;
    }
    return SW3_STATUS_ENTRYPOINT_NOT_FOUND;
}

/* =============================================================
 * WoW64 thread context (real x86 registers)
 *
 * Always populate ContextFlags BEFORE calling Get; the kernel
 * uses it to decide which register groups to fill in.
 *
 * For reliable values you should NtSuspendThread() the target
 * thread first (and NtResumeThread() afterwards).
 * ============================================================= */

static inline SW3_NTSTATUS sw3_wow64_get_thread_context(
    SW3_HANDLE hThread, SW3_WOW64_CONTEXT* ctx, uint32_t flags)
{
    if (!ctx) return SW3_STATUS_INVALID_PARAMETER;
    memset(ctx, 0, sizeof(*ctx));
    ctx->ContextFlags = flags ? flags : WOW64_CONTEXT_FULL;
    SW3_ULONG ret = 0;
    return SW3NtQueryInformationThread(
        hThread, SW3_ThreadWow64Context,
        ctx, (SW3_ULONG)sizeof(*ctx), &ret);
}

static inline SW3_NTSTATUS sw3_wow64_set_thread_context(
    SW3_HANDLE hThread, const SW3_WOW64_CONTEXT* ctx)
{
    if (!ctx) return SW3_STATUS_INVALID_PARAMETER;
    return SW3NtSetInformationThread(
        hThread, SW3_ThreadWow64Context,
        (SW3_PVOID)(uintptr_t)ctx, (SW3_ULONG)sizeof(*ctx));
}

/* Convenience: suspend, get context with `flags`, leave thread suspended.
 * Caller is responsible for resuming via SW3NtResumeThread. */
static inline SW3_NTSTATUS sw3_wow64_suspend_and_get_context(
    SW3_HANDLE hThread, SW3_WOW64_CONTEXT* ctx, uint32_t flags)
{
    SW3_ULONG prev = 0;
    SW3_NTSTATUS s = SW3NtSuspendThread(hThread, &prev);
    if (s < 0) return s;
    s = sw3_wow64_get_thread_context(hThread, ctx, flags);
    if (s < 0) {
        SW3_ULONG p = 0;
        SW3NtResumeThread(hThread, &p);   /* best-effort recovery */
    }
    return s;
}

/* Atomic: suspend -> set context -> resume. */
static inline SW3_NTSTATUS sw3_wow64_atomic_set_context(
    SW3_HANDLE hThread, const SW3_WOW64_CONTEXT* ctx)
{
    SW3_ULONG prev = 0;
    SW3_NTSTATUS s = SW3NtSuspendThread(hThread, &prev);
    if (s < 0) return s;
    s = sw3_wow64_set_thread_context(hThread, ctx);
    SW3_ULONG p = 0;
    SW3NtResumeThread(hThread, &p);
    return s;
}

#ifdef __cplusplus
}
#endif

#endif /* SW3_WOW64_HELPERS_H */
