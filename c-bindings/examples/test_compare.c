#include <stdio.h>
#include <stdint.h>
#include <windows.h>

extern uint32_t SW3Sw3GetSyscallNumber(uint32_t hash);
typedef NTSTATUS (NTAPI *PFN_NtProtectVirtualMemory)(HANDLE, PVOID*, PULONG, ULONG, PULONG);

int main(void) {
    DWORD wow64gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov wow64gate, eax
    }
    printf("Gate: 0x%08X\n\n", wow64gate);

    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");
    PFN_NtProtectVirtualMemory pNtProtect = (PFN_NtProtectVirtualMemory)
        GetProcAddress(hNtdll, "NtProtectVirtualMemory");

    uint32_t ssn = SW3Sw3GetSyscallNumber(0x039F2B0F);
    printf("SSN=%u\n", ssn);

    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    printf("VirtualAlloc: %p\n", addr);

    /* Via NTDLL */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS nts = pNtProtect((HANDLE)-1, &base, &sz, PAGE_READWRITE, &oldp);
        printf("NTDLL: 0x%08X oldp=%lu\n", (unsigned)nts, oldp);
    }

    /* Via gate - direct push */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;

        __asm {
            mov eax, ssn
            lea ecx, oldp
            push ecx
            push 4
            lea ecx, sz
            push ecx
            lea ecx, base
            push ecx
            push 0FFFFFFFFh
            call dword ptr [g]
            add esp, 20
            mov ntstatus, eax
        }
        printf("Gate: 0x%08X oldp=%lu\n", (unsigned)ntstatus, oldp);
    }

    VirtualFree(addr, 0, MEM_RELEASE);
    printf("\nDone\n");
    return 0;
}
