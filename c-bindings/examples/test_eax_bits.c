#include <stdio.h>
#include <stdint.h>
#include <windows.h>

extern uint32_t SW3Sw3GetSyscallNumber(uint32_t hash);

int main(void) {
    DWORD wow64gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov wow64gate, eax
    }
    printf("Gate: 0x%08X\n\n", wow64gate);

    /* Get the FULL eax value from ntdll for NtProtectVirtualMemory */
    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");
    unsigned char *pNtProtect = (unsigned char *)GetProcAddress(hNtdll, "NtProtectVirtualMemory");
    unsigned char *pNtDelay = (unsigned char *)GetProcAddress(hNtdll, "NtDelayExecution");
    unsigned char *pNtClose = (unsigned char *)GetProcAddress(hNtdll, "NtClose");
    unsigned char *pNtQueryTime = (unsigned char *)GetProcAddress(hNtdll, "NtQuerySystemTime");

    /* Read the full 4-byte eax from mov eax, imm32 (opcode B8) */
    uint32_t ntdll_eax_protect = *(uint32_t *)(pNtProtect + 1);
    uint32_t ntdll_eax_delay = *(uint32_t *)(pNtDelay + 1);
    uint32_t ntdll_eax_close = *(uint32_t *)(pNtClose + 1);
    uint32_t ntdll_eax_qtime = *(uint32_t *)(pNtQueryTime + 1);

    uint32_t our_ssn_protect = SW3Sw3GetSyscallNumber(0x039F2B0F);
    uint32_t our_ssn_delay = SW3Sw3GetSyscallNumber(0xF269F2FB);
    uint32_t our_ssn_close = SW3Sw3GetSyscallNumber(0x0C9DF7C3);
    uint32_t our_ssn_qtime = SW3Sw3GetSyscallNumber(0x0CAB757F);

    printf("Function               ntdll_eax    our_SSN    match_low16?\n");
    printf("NtProtectVirtualMemory 0x%08X   0x%04X     %s\n", ntdll_eax_protect, our_ssn_protect,
           (ntdll_eax_protect & 0xFFFF) == our_ssn_protect ? "YES" : "NO");
    printf("NtDelayExecution       0x%08X   0x%04X     %s\n", ntdll_eax_delay, our_ssn_delay,
           (ntdll_eax_delay & 0xFFFF) == our_ssn_delay ? "YES" : "NO");
    printf("NtClose                0x%08X   0x%04X     %s\n", ntdll_eax_close, our_ssn_close,
           (ntdll_eax_close & 0xFFFF) == our_ssn_close ? "YES" : "NO");
    printf("NtQuerySystemTime      0x%08X   0x%04X     %s\n", ntdll_eax_qtime, our_ssn_qtime,
           (ntdll_eax_qtime & 0xFFFF) == our_ssn_qtime ? "YES" : "NO");

    /* Now test NtProtectVM with FULL ntdll eax vs our SSN */
    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);

    /* Test with our SSN (low 16 bits only) */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        DWORD ssn_val = our_ssn_protect;  /* 0x50 */

        __asm {
            mov eax, ssn_val
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
        printf("\nNtProtectVM with our SSN (0x%08X): 0x%08X\n", ssn_val, (unsigned)ntstatus);
    }

    /* Test with FULL ntdll eax value */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        DWORD ssn_val = ntdll_eax_protect;  /* should be 0x00000050 for NtProtectVM */

        __asm {
            mov eax, ssn_val
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
        printf("NtProtectVM with ntdll eax (0x%08X): 0x%08X\n", ssn_val, (unsigned)ntstatus);
    }

    /* Test NtDelayExecution with FULL ntdll eax vs our SSN */
    {
        LARGE_INTEGER delay;
        delay.QuadPart = -1;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        DWORD ssn_val = our_ssn_delay;

        __asm {
            mov eax, ssn_val
            lea ecx, delay
            push ecx
            push 0
            call dword ptr [g]
            add esp, 8
            mov ntstatus, eax
        }
        printf("\nNtDelayExecution with our SSN (0x%08X): 0x%08X\n", ssn_val, (unsigned)ntstatus);
    }

    {
        LARGE_INTEGER delay;
        delay.QuadPart = -1;
        NTSTATUS ntstatus;
        DWORD g = wow64gate;
        DWORD ssn_val = ntdll_eax_delay;

        __asm {
            mov eax, ssn_val
            lea ecx, delay
            push ecx
            push 0
            call dword ptr [g]
            add esp, 8
            mov ntstatus, eax
        }
        printf("NtDelayExecution with ntdll eax (0x%08X): 0x%08X\n", ssn_val, (unsigned)ntstatus);
    }

    VirtualFree(addr, 0, MEM_RELEASE);
    printf("\nDone\n");
    return 0;
}
