#include <stdio.h>
#include <stdint.h>
#include <windows.h>

int main(void) {
    HMODULE hNtdll = GetModuleHandleA("ntdll.dll");

    /* Dump first 32 bytes of NtProtectVirtualMemory */
    FARPROC pFunc = GetProcAddress(hNtdll, "NtProtectVirtualMemory");
    printf("NtProtectVirtualMemory at %p:\n", pFunc);
    unsigned char *p = (unsigned char *)pFunc;
    for (int i = 0; i < 32; i++) {
        printf("%02X ", p[i]);
        if ((i+1) % 16 == 0) printf("\n");
    }
    printf("\n\n");

    /* Also dump NtDelayExecution for comparison */
    pFunc = GetProcAddress(hNtdll, "NtDelayExecution");
    printf("NtDelayExecution at %p:\n", pFunc);
    p = (unsigned char *)pFunc;
    for (int i = 0; i < 32; i++) {
        printf("%02X ", p[i]);
        if ((i+1) % 16 == 0) printf("\n");
    }
    printf("\n\n");

    /* Dump NtQuerySystemTime */
    pFunc = GetProcAddress(hNtdll, "NtQuerySystemTime");
    printf("NtQuerySystemTime at %p:\n", pFunc);
    p = (unsigned char *)pFunc;
    for (int i = 0; i < 32; i++) {
        printf("%02X ", p[i]);
        if ((i+1) % 16 == 0) printf("\n");
    }
    printf("\n\n");

    /* Dump NtClose */
    pFunc = GetProcAddress(hNtdll, "NtClose");
    printf("NtClose at %p:\n", pFunc);
    p = (unsigned char *)pFunc;
    for (int i = 0; i < 32; i++) {
        printf("%02X ", p[i]);
        if ((i+1) % 16 == 0) printf("\n");
    }
    printf("\n\n");

    /* Dump Wow64SystemServiceCall */
    DWORD gate;
    __asm {
        mov eax, dword ptr fs:[0C0h]
        mov gate, eax
    }
    printf("Wow64 gate at %p:\n", (void*)(uintptr_t)gate);
    p = (unsigned char *)(uintptr_t)gate;
    for (int i = 0; i < 16; i++) {
        printf("%02X ", p[i]);
    }
    printf("\n");

    return 0;
}
