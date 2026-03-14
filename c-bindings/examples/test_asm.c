#include <stdio.h>
#include <stdint.h>
#include <windows.h>

extern uint32_t SW3Sw3GetSyscallNumber(uint32_t hash);

int main(void) {
    printf("=== Direct ASM WoW64 Test ===\n\n");
    
    DWORD wow64gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov wow64gate, eax
    }
    printf("WoW64 gate: 0x%08X\n", wow64gate);
    if (!wow64gate) { printf("Not WoW64\n"); return 1; }
    
    /* Test 1: NtQuerySystemTime (1 arg) - direct push */
    {
        uint32_t ssn = SW3Sw3GetSyscallNumber(0x0CAB757F);
        printf("NtQuerySystemTime SSN=%u\n", ssn);
        
        LARGE_INTEGER systime = {0};
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        
        __asm {
            mov eax, ssn
            lea ecx, systime
            push ecx
            call dword ptr [g]
            add esp, 4
            mov ntstatus, eax
        }
        printf("  Direct ASM: 0x%08X, t=%lld\n", (unsigned)ntstatus, systime.QuadPart);
    }
    
    /* Test 2: NtProtectVirtualMemory (5 args) - direct push */
    {
        uint32_t ssn = SW3Sw3GetSyscallNumber(0x039F2B0F);
        printf("\nNtProtectVirtualMemory SSN=%u\n", ssn);
        
        PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        ULONG sz = 4096;
        ULONG oldp = 0;
        HANDLE proc = (HANDLE)-1;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        
        __asm {
            mov eax, ssn
            lea ecx, oldp
            push ecx
            push 4
            lea ecx, sz
            push ecx
            lea ecx, addr
            push ecx
            push proc
            call dword ptr [g]
            add esp, 20
            mov ntstatus, eax
        }
        printf("  Direct push ASM: 0x%08X, oldp=%lu\n", (unsigned)ntstatus, oldp);
        if (addr) VirtualFree(addr, 0, MEM_RELEASE);
    }
    
    /* Test 3: NtProtectVM via array (like Rust params array) */
    {
        uint32_t ssn = SW3Sw3GetSyscallNumber(0x039F2B0F);
        printf("\nNtProtectVM via array:\n");
        
        PVOID addr2 = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        ULONG sz2 = 4096;
        ULONG oldp2 = 0;
        HANDLE proc2 = (HANDLE)-1;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        
        uint32_t params[5];
        params[0] = (uint32_t)proc2;
        params[1] = (uint32_t)&addr2;
        params[2] = (uint32_t)&sz2;
        params[3] = (uint32_t)4;
        params[4] = (uint32_t)&oldp2;
        
        __asm {
            mov eax, ssn
            lea edx, params
            push dword ptr [edx+16]
            push dword ptr [edx+12]
            push dword ptr [edx+8]
            push dword ptr [edx+4]
            push dword ptr [edx]
            call dword ptr [g]
            add esp, 20
            mov ntstatus, eax
        }
        printf("  Array ASM: 0x%08X, oldp=%lu\n", (unsigned)ntstatus, oldp2);
        if (addr2) VirtualFree(addr2, 0, MEM_RELEASE);
    }
    
    printf("\n=== Done ===\n");
    return 0;
}
