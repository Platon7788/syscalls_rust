#include <stdio.h>
#include <stdint.h>
#include <windows.h>

int main(void) {
    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");

    /* Extract the edx target from NtProtectVirtualMemory */
    /* B8 xx xx xx xx BA yy yy yy yy FF D2 C2 14 00 */
    unsigned char *pFunc = (unsigned char *)GetProcAddress(hNtdll, "NtProtectVirtualMemory");
    uint32_t ssn = *(uint32_t *)(pFunc + 1);          /* after B8 */
    uint32_t edx_target = *(uint32_t *)(pFunc + 6);    /* after BA */
    printf("NtProtectVM: SSN=0x%08X, edx_target=0x%08X\n", ssn, edx_target);

    /* Also get wow64 gate for comparison */
    DWORD wow64gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov wow64gate, eax
    }
    printf("TEB gate: 0x%08X\n\n", wow64gate);

    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);

    /* Test 1: via edx (ntdll style) */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS ntstatus;
        DWORD target = edx_target;

        __asm {
            mov eax, 050h
            lea ecx, oldp
            push ecx
            push 4
            lea ecx, sz
            push ecx
            lea ecx, base
            push ecx
            push 0FFFFFFFFh
            mov edx, target
            call edx
            add esp, 20
            mov ntstatus, eax
        }
        printf("Via edx target: 0x%08X oldp=%lu\n", (unsigned)ntstatus, oldp);
    }

    /* Test 2: via TEB gate */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;

        __asm {
            mov eax, 050h
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
        printf("Via TEB gate:   0x%08X oldp=%lu\n", (unsigned)ntstatus, oldp);
    }

    /* Test 3: what if the edx value matters? ntdll sets edx = target BEFORE call */
    /* Maybe wow64 gate reads edx as well as eax? */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        DWORD target = edx_target;

        __asm {
            mov eax, 050h
            lea ecx, oldp
            push ecx
            push 4
            lea ecx, sz
            push ecx
            lea ecx, base
            push ecx
            push 0FFFFFFFFh
            mov edx, target    /* set edx like ntdll does */
            call dword ptr [g] /* but call via TEB gate */
            add esp, 20
            mov ntstatus, eax
        }
        printf("Via gate+edx:   0x%08X oldp=%lu\n", (unsigned)ntstatus, oldp);
    }

    VirtualFree(addr, 0, MEM_RELEASE);
    printf("\nDone\n");
    return 0;
}
