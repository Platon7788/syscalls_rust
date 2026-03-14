#include <stdio.h>
#include <stdint.h>
#include <windows.h>

typedef NTSTATUS (NTAPI *PFN_NtProtectVirtualMemory)(HANDLE, PVOID*, PULONG, ULONG, PULONG);

int main(void) {
    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");
    PFN_NtProtectVirtualMemory pNtProtect = (PFN_NtProtectVirtualMemory)
        GetProcAddress(hNtdll, "NtProtectVirtualMemory");

    DWORD wow64gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov wow64gate, eax
    }

    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    printf("addr = %p\n\n", addr);

    /* Test via NTDLL first */
    PVOID base1 = addr;
    ULONG sz1 = 4096;
    ULONG oldp1 = 0;
    printf("NTDLL call: ProcessHandle=-1, BaseAddress=%p (&=%p), RegionSize=%lu (&=%p), NewProt=4, OldProt (&=%p)\n",
           base1, &base1, sz1, &sz1, &oldp1);
    NTSTATUS nts1 = pNtProtect((HANDLE)-1, &base1, &sz1, PAGE_READWRITE, &oldp1);
    printf("NTDLL: 0x%08X oldp=%lu\n\n", (unsigned)nts1, oldp1);

    /* Now reproduce EXACTLY what ntdll does internally */
    /* ntdll does: mov eax, SSN; mov edx, target; call edx; ret 0x14 */
    /* The caller (us calling pNtProtect) pushes 5 args and calls ntdll */
    /* Inside ntdll: eax=SSN, then call Wow64SystemServiceCall */
    /* At the point of call edx, stack = [ret_to_ntdll, arg1, arg2, arg3, arg4, arg5] */
    /* But WE are doing: push args manually, then call gate */
    /* At our call gate, stack = [ret_to_us, arg1, arg2, arg3, arg4, arg5] */
    /* These should be identical! */

    /* Let's try: call ntdll function directly (not via function pointer) */
    /* Just to make ABSOLUTELY sure the function pointer call works */
    PVOID base2 = addr;
    ULONG sz2 = 1;  /* Try size 1 instead of 4096 */
    ULONG oldp2 = 0;
    printf("NTDLL call2: base=%p sz=%lu\n", base2, sz2);
    NTSTATUS nts2 = pNtProtect((HANDLE)-1, &base2, &sz2, PAGE_EXECUTE_READ, &oldp2);
    printf("NTDLL: 0x%08X oldp=%lu base=%p sz=%lu\n\n", (unsigned)nts2, oldp2, base2, sz2);

    /* Now exact same via gate */
    PVOID base3 = addr;
    ULONG sz3 = 1;
    ULONG oldp3 = 0;
    NTSTATUS nts3;
    DWORD g = wow64gate;

    /* Print addresses to compare */
    printf("Gate call: &base3=%p base3=%p, &sz3=%p sz3=%lu, &oldp3=%p\n",
           &base3, base3, &sz3, sz3, &oldp3);

    __asm {
        lea ecx, oldp3
        push ecx
        push 020h       /* PAGE_EXECUTE_READ */
        lea ecx, sz3
        push ecx
        lea ecx, base3
        push ecx
        push 0FFFFFFFFh /* (HANDLE)-1 */
        mov eax, 050h   /* NtProtectVirtualMemory SSN */
        call dword ptr [g]
        add esp, 20
        mov nts3, eax
    }
    printf("Gate: 0x%08X oldp=%lu base=%p sz=%lu\n\n", (unsigned)nts3, oldp3, base3, sz3);

    /* What if we DON'T push but instead let C do it? */
    /* Create a function pointer that points to the gate */
    /* Hmm, gate is not stdcall compatible... */

    /* Instead: disassemble what C compiler generates for pNtProtect call */
    /* by putting a breakpoint equivalent - just print the stack */
    printf("Stack comparison:\n");
    printf("  Before NTDLL call, pNtProtect calls internally:\n");
    printf("    mov eax, 0x50\n");
    printf("    mov edx, 0x770491F0\n");
    printf("    call edx\n");
    printf("  At call edx, stack = [ret_inside_ntdll, arg1..arg5]\n\n");
    printf("  Our gate call:\n");
    printf("    push args, mov eax, 0x50, call [gate]\n");
    printf("  At call [gate], stack = [ret_to_our_asm, arg1..arg5]\n");
    printf("  These stacks should be equivalent.\n\n");

    /* Hmm wait - call dword ptr [g] is an INDIRECT call through memory */
    /* call [g] = call *(&g) -- reads the value AT address &g, then calls that */
    /* We want call (g) which calls the value of g directly */
    /* MSVC __asm: call dword ptr [g] means: read dword from variable g, call that addr */
    /* That should be correct... g = wow64gate = the gate address */

    /* But wait: what about ecx? We use ecx for lea before push, */
    /* then after all pushes, ecx still has the last lea result. */
    /* Does the gate care about ecx? */

    /* Let's try without using ecx - use a different approach */
    PVOID base4 = addr;
    ULONG sz4 = 1;
    ULONG oldp4 = 0;
    NTSTATUS nts4;
    DWORD arg1 = (DWORD)(HANDLE)-1;
    DWORD arg2 = (DWORD)&base4;
    DWORD arg3 = (DWORD)&sz4;
    DWORD arg4 = 0x20;  /* PAGE_EXECUTE_READ */
    DWORD arg5 = (DWORD)&oldp4;

    printf("Gate call2 (pre-computed args):\n");
    printf("  args: %08X %08X %08X %08X %08X\n", arg1, arg2, arg3, arg4, arg5);

    __asm {
        push arg5
        push arg4
        push arg3
        push arg2
        push arg1
        mov eax, 050h
        call dword ptr [g]
        add esp, 20
        mov nts4, eax
    }
    printf("Gate2: 0x%08X oldp=%lu\n", (unsigned)nts4, oldp4);

    VirtualFree(addr, 0, MEM_RELEASE);
    printf("\nDone\n");
    return 0;
}
