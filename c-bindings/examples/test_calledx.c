#include <stdio.h>
#include <stdint.h>
#include <windows.h>

typedef NTSTATUS (NTAPI *PFN_NtProtectVirtualMemory)(HANDLE, PVOID*, PULONG, ULONG, PULONG);

int main(void) {
    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");
    PFN_NtProtectVirtualMemory pNtProtect = (PFN_NtProtectVirtualMemory)
        GetProcAddress(hNtdll, "NtProtectVirtualMemory");

    /* Extract edx target from ntdll stub */
    unsigned char *pFunc = (unsigned char *)pNtProtect;
    uint32_t edx_target = *(uint32_t *)(pFunc + 6);  /* after B8 xx xx xx xx BA */
    printf("edx_target = 0x%08X\n", edx_target);

    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    printf("addr = %p\n\n", addr);

    /* Via NTDLL (reference) */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS nts = pNtProtect((HANDLE)-1, &base, &sz, PAGE_READWRITE, &oldp);
        printf("NTDLL:     0x%08X oldp=%lu\n", (unsigned)nts, oldp);
    }

    /* Via call edx (EXACTLY like ntdll does it) */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS nts;
        DWORD target = edx_target;

        /* Push args, then mov eax, SSN; mov edx, target; call edx */
        __asm {
            lea ecx, oldp
            push ecx
            push 4
            lea ecx, sz
            push ecx
            lea ecx, base
            push ecx
            push 0FFFFFFFFh
            mov eax, 050h
            mov edx, target
            call edx
            add esp, 20
            mov nts, eax
        }
        printf("call edx:  0x%08X oldp=%lu\n", (unsigned)nts, oldp);
    }

    /* Hmm wait -- NTDLL is stdcall. The C compiler does:
       push &oldp
       push 4
       push &sz
       push &base
       push -1
       call [pNtProtect]   <-- this pushes ret addr, jumps to ntdll
       Inside ntdll:
           mov eax, SSN
           mov edx, target
           call edx          <-- pushes ANOTHER ret addr (to ntdll ret)
       
       So at the wow64 gate, stack is:
       [esp+0]  = ret to ntdll (after call edx, which is "ret 14h")
       [esp+4]  = ret to C caller (after call [pNtProtect])  
       [esp+8]  = arg1 = -1
       [esp+C]  = arg2 = &base
       [esp+10] = arg3 = &sz
       [esp+14] = arg4 = 4
       [esp+18] = arg5 = &oldp
       
       In OUR case:
       push args
       mov eax, SSN
       call edx/call [gate]
       Stack:
       [esp+0]  = ret to our code
       [esp+4]  = arg1 = -1
       [esp+8]  = arg2 = &base
       [esp+C]  = arg3 = &sz
       [esp+10] = arg4 = 4
       [esp+14] = arg5 = &oldp
       
       THE DIFFERENCE: ntdll has TWO return addresses above the args!
       We have ONE. The wow64 gate may skip [esp+0] (its own ret) and 
       [esp+4] (ntdll's ret) and read args starting at [esp+8]!
       
       If that's the case, we need to push a DUMMY return address!
    */
    
    /* Test: push dummy ret addr before args */
    {
        PVOID base = addr;
        ULONG sz = 4096;
        ULONG oldp = 0;
        NTSTATUS nts;
        DWORD target = edx_target;

        __asm {
            lea ecx, oldp
            push ecx
            push 4
            lea ecx, sz
            push ecx
            lea ecx, base
            push ecx
            push 0FFFFFFFFh    /* arg1 = ProcessHandle */
            push 0DEADBEEFh    /* DUMMY return address (simulating ntdll's ret) */
            mov eax, 050h
            mov edx, target
            call edx           /* this pushes another ret addr, so stack has 2 rets + 5 args */
            add esp, 24        /* 20 (args) + 4 (dummy ret) */
            mov nts, eax
        }
        printf("dummy ret: 0x%08X oldp=%lu\n", (unsigned)nts, oldp);
    }

    VirtualFree(addr, 0, MEM_RELEASE);
    printf("\nDone\n");
    return 0;
}
