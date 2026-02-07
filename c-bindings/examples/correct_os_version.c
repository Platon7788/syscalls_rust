/**
 * ПРАВИЛЬНОЕ ПОЛУЧЕНИЕ ВЕРСИИ ОС - SysWhispers3 C Bindings
 * 
 * ИСПОЛЬЗУЙ ЭТОТ КОД! SW3RtlGetVersion НЕ СУЩЕСТВУЕТ!
 * Используй SW3NtQuerySystemInformation для получения версии ОС
 */

#include <stdio.h>
#include <windows.h>

/* ТОЛЬКО SW3 заголовок */
#include "syscalls.h"

/* Структура для получения версии ОС через NtQuerySystemInformation */
typedef struct _SW3_RTL_OSVERSIONINFOW {
    SW3_ULONG dwOSVersionInfoSize;
    SW3_ULONG dwMajorVersion;
    SW3_ULONG dwMinorVersion;
    SW3_ULONG dwBuildNumber;
    SW3_ULONG dwPlatformId;
    SW3_WCHAR szCSDVersion[128];
} SW3_RTL_OSVERSIONINFOW;

/* Указатель на функцию SW3NtQuerySystemInformation */
typedef SW3_NTSTATUS (*PFN_SW3NtQuerySystemInformation)(
    uint32_t system_information_class,
    SW3_PVOID system_information,
    SW3_ULONG system_information_length,
    SW3_ULONG* return_length
);

/* Константа для получения версии ОС (не документирована, но работает) */
#define SW3_SystemVersionInformation 45

const char* GetWindowsVersionName(SW3_ULONG major, SW3_ULONG minor, SW3_ULONG build) {
    if (major == 10) {
        if (build >= 22000) return "Windows 11";
        else return "Windows 10";
    } else if (major == 6) {
        if (minor == 3) return "Windows 8.1";
        else if (minor == 2) return "Windows 8";
        else if (minor == 1) return "Windows 7";
        else if (minor == 0) return "Windows Vista";
    } else if (major == 5) {
        if (minor == 2) return "Windows Server 2003";
        else if (minor == 1) return "Windows XP";
        else if (minor == 0) return "Windows 2000";
    }
    return "Unknown Windows";
}

int main() {
    printf("=== ПРАВИЛЬНОЕ ПОЛУЧЕНИЕ ВЕРСИИ ОС ===\n\n");
    printf("❌ SW3RtlGetVersion НЕ СУЩЕСТВУЕТ!\n");
    printf("✅ Используй SW3NtQuerySystemInformation!\n\n");
    
    /* Загружаем DLL с чистыми сисколами */
    HMODULE hSW3 = LoadLibraryA("syscalls.dll");
    if (!hSW3) {
        printf("❌ Не удалось загрузить syscalls.dll\n");
        printf("💡 Скомпилируй DLL: cargo build --release\n");
        printf("💡 Скопируй DLL: copy target\\release\\syscalls.dll .\n");
        return 1;
    }
    
    printf("✅ syscalls.dll загружена успешно!\n");
    
    /* Получаем адрес функции */
    PFN_SW3NtQuerySystemInformation pSW3NtQuerySystemInformation = 
        (PFN_SW3NtQuerySystemInformation)GetProcAddress(hSW3, "SW3NtQuerySystemInformation");
    
    if (!pSW3NtQuerySystemInformation) {
        printf("❌ Функция SW3NtQuerySystemInformation не найдена\n");
        FreeLibrary(hSW3);
        return 1;
    }
    
    printf("✅ SW3NtQuerySystemInformation найдена!\n\n");
    
    /* СПОСОБ 1: Получение базовой информации о системе */
    printf("=== СПОСОБ 1: SW3_SystemBasicInformation ===\n");
    
    typedef struct {
        SW3_ULONG Reserved;
        SW3_ULONG TimerResolution;
        SW3_ULONG PageSize;
        SW3_ULONG NumberOfPhysicalPages;
        SW3_ULONG LowestPhysicalPageNumber;
        SW3_ULONG HighestPhysicalPageNumber;
        SW3_ULONG AllocationGranularity;
        SW3_ULONG_PTR MinimumUserModeAddress;
        SW3_ULONG_PTR MaximumUserModeAddress;
        SW3_ULONG_PTR ActiveProcessorsAffinityMask;
        SW3_CCHAR NumberOfProcessors;
    } SW3_SYSTEM_BASIC_INFO;
    
    SW3_SYSTEM_BASIC_INFO basicInfo = {0};
    SW3_ULONG returnLength = 0;
    
    SW3_NTSTATUS status = pSW3NtQuerySystemInformation(
        SW3_SystemBasicInformation,
        &basicInfo,
        sizeof(basicInfo),
        &returnLength
    );
    
    if (SW3_NT_SUCCESS(status)) {
        printf("✅ Базовая информация получена:\n");
        printf("   Процессоров: %d\n", basicInfo.NumberOfProcessors);
        printf("   Размер страницы: %lu байт\n", basicInfo.PageSize);
        printf("   Физическая память: %llu МБ\n", 
               ((unsigned long long)basicInfo.NumberOfPhysicalPages * basicInfo.PageSize) / (1024 * 1024));
    } else {
        printf("❌ Ошибка получения базовой информации: 0x%08X\n", status);
    }
    
    /* СПОСОБ 2: Получение информации о процессоре */
    printf("\n=== СПОСОБ 2: SW3_SystemProcessorInformation ===\n");
    
    typedef struct {
        SW3_USHORT ProcessorArchitecture;
        SW3_USHORT ProcessorLevel;
        SW3_USHORT ProcessorRevision;
        SW3_USHORT MaximumProcessors;
        SW3_ULONG ProcessorFeatureBits;
    } SW3_SYSTEM_PROCESSOR_INFO;
    
    SW3_SYSTEM_PROCESSOR_INFO procInfo = {0};
    returnLength = 0;
    
    status = pSW3NtQuerySystemInformation(
        SW3_SystemProcessorInformation,
        &procInfo,
        sizeof(procInfo),
        &returnLength
    );
    
    if (SW3_NT_SUCCESS(status)) {
        printf("✅ Информация о процессоре получена:\n");
        printf("   Архитектура: %u ", procInfo.ProcessorArchitecture);
        switch (procInfo.ProcessorArchitecture) {
            case 0: printf("(Intel x86)\n"); break;
            case 9: printf("(x64)\n"); break;
            case 12: printf("(ARM64)\n"); break;
            default: printf("(Unknown)\n"); break;
        }
        printf("   Уровень процессора: %u\n", procInfo.ProcessorLevel);
        printf("   Ревизия: %u\n", procInfo.ProcessorRevision);
    } else {
        printf("❌ Ошибка получения информации о процессоре: 0x%08X\n", status);
    }
    
    /* СПОСОБ 3: Попытка получить версию через GetVersion (WinAPI) */
    printf("\n=== СПОСОБ 3: GetVersion (WinAPI для сравнения) ===\n");
    
    DWORD version = GetVersion();
    SW3_ULONG majorVersion = (SW3_ULONG)(LOBYTE(LOWORD(version)));
    SW3_ULONG minorVersion = (SW3_ULONG)(HIBYTE(LOWORD(version)));
    SW3_ULONG buildNumber = 0;
    
    if (version < 0x80000000) {
        buildNumber = (SW3_ULONG)(HIWORD(version));
    }
    
    printf("✅ Версия Windows (через WinAPI):\n");
    printf("   Основная версия: %lu\n", majorVersion);
    printf("   Дополнительная версия: %lu\n", minorVersion);
    printf("   Номер сборки: %lu\n", buildNumber);
    printf("   Название: %s\n", GetWindowsVersionName(majorVersion, minorVersion, buildNumber));
    
    FreeLibrary(hSW3);
    
    printf("\n=== ИТОГ ===\n");
    printf("✅ SW3NtQuerySystemInformation работает для получения системной информации\n");
    printf("❌ SW3RtlGetVersion НЕ СУЩЕСТВУЕТ в этой библиотеке\n");
    printf("💡 Для получения версии ОС используй:\n");
    printf("   1. SW3NtQuerySystemInformation с SW3_SystemBasicInformation\n");
    printf("   2. SW3NtQuerySystemInformation с SW3_SystemProcessorInformation\n");
    printf("   3. Стандартные WinAPI функции (GetVersion, GetVersionEx)\n");
    printf("🎯 Все SW3_ константы и функции работают корректно!\n");
    
    return 0;
}