#include <stdio.h>
#include <stdint.h>
#include <windows.h>

int main(void) {
    /* Dump what ntdll uses as the call target */
    unsigned char *p = (unsigned char *)0x770491F0;
    printf("ntdll call target (0x770491F0):\n");
    for (int i = 0; i < 32; i++) {
        printf("%02X ", p[i]);
        if ((i+1) % 16 == 0) printf("\n");
    }
    printf("\n\n");

    /* Dump wow64 gate from TEB */
    DWORD gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov gate, eax
    }
    printf("TEB Wow64 gate (0x%08X):\n", gate);
    p = (unsigned char *)(uintptr_t)gate;
    for (int i = 0; i < 32; i++) {
        printf("%02X ", p[i]);
        if ((i+1) % 16 == 0) printf("\n");
    }
    printf("\n\n");

    /* Check: are they the same? */
    printf("Same address? %s\n", (gate == 0x770491F0) ? "YES" : "NO");

    /* Now let's try calling NtProtectVirtualMemory via edx method (like ntdll does) */
    PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
    if (!addr) { printf("VirtualAlloc failed\n"); return 1; }

    PVOID base = addr;
    ULONG sz = 4096;
    ULONG oldp = 0;
    NTSTATUS ntstatus;
    DWORD target = 0x770491F0;

    printf("\nTest via ntdll-style edx call (SSN=0x50):\n");
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
    printf("Result: 0x%08X oldp=%lu\n", (unsigned)ntstatus, oldp);

    /* Now via TEB gate */
    base = addr;
    sz = 4096;
    oldp = 0;
    DWORD g = gate;
    printf("\nTest via TEB gate call (SSN=0x50):\n");
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
    printf("Result: 0x%08X oldp=%lu\n", (unsigned)ntstatus, oldp);

    VirtualFree(addr, 0, MEM_RELEASE);
    printf("\nDone\n");
    return 0;
}
