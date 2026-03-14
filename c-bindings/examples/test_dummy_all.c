#include <stdio.h>
#include <stdint.h>
#include <windows.h>

extern uint32_t SW3Sw3GetSyscallNumber(uint32_t hash);

typedef NTSTATUS (NTAPI *PFN_NtProtectVM)(HANDLE, PVOID*, PULONG, ULONG, PULONG);
typedef NTSTATUS (NTAPI *PFN_NtAllocateVM)(HANDLE, PVOID*, ULONG, PULONG, ULONG, ULONG);
typedef NTSTATUS (NTAPI *PFN_NtDelayExecution)(BOOLEAN, PLARGE_INTEGER);
typedef NTSTATUS (NTAPI *PFN_NtQuerySystemTime)(PLARGE_INTEGER);
typedef NTSTATUS (NTAPI *PFN_NtClose)(HANDLE);
typedef NTSTATUS (NTAPI *PFN_NtCreateEvent)(PHANDLE, ACCESS_MASK, PVOID, DWORD, BOOLEAN);
typedef NTSTATUS (NTAPI *PFN_NtQuerySystemInformation)(ULONG, PVOID, ULONG, PULONG);

int main(void) {
    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");
    DWORD wow64gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov wow64gate, eax
    }

    /* Extract edx_target from any ntdll stub (they all use the same) */
    unsigned char *stub = (unsigned char *)GetProcAddress(hNtdll, "NtProtectVirtualMemory");
    DWORD edx_target = *(DWORD *)(stub + 6);

    printf("=== WoW64 Dummy Return Address Test ===\n");
    printf("gate=0x%08X edx_target=0x%08X\n\n", wow64gate, edx_target);

    /* Test each function WITH and WITHOUT dummy ret addr */

    /* 1-arg: NtClose */
    {
        DWORD ssn = SW3Sw3GetSyscallNumber(0x0C9DF7C3);
        NTSTATUS nts;
        DWORD g = wow64gate;

        /* Without dummy */
        __asm {
            push 012345678h
            mov eax, ssn
            call dword ptr [g]
            add esp, 4
            mov nts, eax
        }
        printf("[1-arg] NtClose WITHOUT dummy: 0x%08X %s\n", (unsigned)nts,
               nts == (NTSTATUS)0xC0000008 ? "OK" : "WRONG");

        /* With dummy */
        __asm {
            push 012345678h
            push 0DEADBEEFh
            mov eax, ssn
            call dword ptr [g]
            add esp, 8
            mov nts, eax
        }
        printf("[1-arg] NtClose WITH    dummy: 0x%08X %s\n\n", (unsigned)nts,
               nts == (NTSTATUS)0xC0000008 ? "OK" : "WRONG");
    }

    /* 1-arg: NtQuerySystemTime */
    {
        DWORD ssn = SW3Sw3GetSyscallNumber(0x0CAB757F);
        LARGE_INTEGER t1 = {0}, t2 = {0};
        NTSTATUS nts;
        DWORD g = wow64gate;

        /* Without dummy */
        __asm {
            lea ecx, t1
            push ecx
            mov eax, ssn
            call dword ptr [g]
            add esp, 4
            mov nts, eax
        }
        printf("[1-arg] NtQuerySysTime WITHOUT dummy: 0x%08X t=%lld\n", (unsigned)nts, t1.QuadPart);

        /* With dummy */
        __asm {
            lea ecx, t2
            push ecx
            push 0DEADBEEFh
            mov eax, ssn
            call dword ptr [g]
            add esp, 8
            mov nts, eax
        }
        printf("[1-arg] NtQuerySysTime WITH    dummy: 0x%08X t=%lld\n\n", (unsigned)nts, t2.QuadPart);
    }

    /* 2-arg: NtDelayExecution */
    {
        DWORD ssn = SW3Sw3GetSyscallNumber(0xF269F2FB);
        LARGE_INTEGER delay;
        NTSTATUS nts;
        DWORD g = wow64gate;

        /* Without dummy */
        delay.QuadPart = -1;
        __asm {
            lea ecx, delay
            push ecx
            push 0
            mov eax, ssn
            call dword ptr [g]
            add esp, 8
            mov nts, eax
        }
        printf("[2-arg] NtDelayExec WITHOUT dummy: 0x%08X\n", (unsigned)nts);

        /* With dummy */
        delay.QuadPart = -1;
        __asm {
            lea ecx, delay
            push ecx
            push 0
            push 0DEADBEEFh
            mov eax, ssn
            call dword ptr [g]
            add esp, 12
            mov nts, eax
        }
        printf("[2-arg] NtDelayExec WITH    dummy: 0x%08X\n\n", (unsigned)nts);
    }

    /* 5-arg: NtProtectVirtualMemory */
    {
        DWORD ssn = SW3Sw3GetSyscallNumber(0x039F2B0F);
        PVOID addr = VirtualAlloc(NULL, 4096, MEM_COMMIT|MEM_RESERVE, PAGE_READWRITE);
        NTSTATUS nts;
        DWORD g = wow64gate;

        /* Without dummy */
        {
            PVOID base = addr;
            ULONG sz = 4096;
            ULONG oldp = 0;
            __asm {
                lea ecx, oldp
                push ecx
                push 4
                lea ecx, sz
                push ecx
                lea ecx, base
                push ecx
                push 0FFFFFFFFh
                mov eax, ssn
                call dword ptr [g]
                add esp, 20
                mov nts, eax
            }
            printf("[5-arg] NtProtectVM WITHOUT dummy: 0x%08X oldp=%lu\n", (unsigned)nts, oldp);
        }

        /* With dummy */
        {
            PVOID base = addr;
            ULONG sz = 4096;
            ULONG oldp = 0;
            __asm {
                lea ecx, oldp
                push ecx
                push 4
                lea ecx, sz
                push ecx
                lea ecx, base
                push ecx
                push 0FFFFFFFFh
                push 0DEADBEEFh
                mov eax, ssn
                call dword ptr [g]
                add esp, 24
                mov nts, eax
            }
            printf("[5-arg] NtProtectVM WITH    dummy: 0x%08X oldp=%lu\n\n", (unsigned)nts, oldp);
        }

        VirtualFree(addr, 0, MEM_RELEASE);
    }

    /* 6-arg: NtAllocateVirtualMemory */
    {
        DWORD ssn = SW3Sw3GetSyscallNumber(0x0F993937);
        NTSTATUS nts;
        DWORD g = wow64gate;

        /* Without dummy */
        {
            PVOID alloc_addr = NULL;
            SIZE_T alloc_size = 4096;
            __asm {
                push 4
                push 03000h
                lea ecx, alloc_size
                push ecx
                push 0
                lea ecx, alloc_addr
                push ecx
                push 0FFFFFFFFh
                mov eax, ssn
                call dword ptr [g]
                add esp, 24
                mov nts, eax
            }
            printf("[6-arg] NtAllocateVM WITHOUT dummy: 0x%08X addr=%p\n", (unsigned)nts, alloc_addr);
            if (alloc_addr) {
                SIZE_T free_sz = 0;
                DWORD ssn_free = SW3Sw3GetSyscallNumber(0x19820515);
                __asm {
                    push 08000h
                    lea ecx, free_sz
                    push ecx
                    lea ecx, alloc_addr
                    push ecx
                    push 0FFFFFFFFh
                    push 0DEADBEEFh
                    mov eax, ssn_free
                    call dword ptr [g]
                    add esp, 20
                }
            }
        }

        /* With dummy */
        {
            PVOID alloc_addr = NULL;
            SIZE_T alloc_size = 4096;
            __asm {
                push 4
                push 03000h
                lea ecx, alloc_size
                push ecx
                push 0
                lea ecx, alloc_addr
                push ecx
                push 0FFFFFFFFh
                push 0DEADBEEFh
                mov eax, ssn
                call dword ptr [g]
                add esp, 28
                mov nts, eax
            }
            printf("[6-arg] NtAllocateVM WITH    dummy: 0x%08X addr=%p\n", (unsigned)nts, alloc_addr);
            if (alloc_addr) {
                SIZE_T free_sz = 0;
                DWORD ssn_free = SW3Sw3GetSyscallNumber(0x19820515);
                __asm {
                    push 08000h
                    lea ecx, free_sz
                    push ecx
                    lea ecx, alloc_addr
                    push ecx
                    push 0FFFFFFFFh
                    push 0DEADBEEFh
                    mov eax, ssn_free
                    call dword ptr [g]
                    add esp, 20
                }
            }
        }
    }

    printf("\n=== Done ===\n");
    return 0;
}
