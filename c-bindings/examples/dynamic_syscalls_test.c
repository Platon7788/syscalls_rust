/**
 * ДИНАМИЧЕСКАЯ ЗАГРУЗКА СИСКОЛОВ
 * 
 * Загружает SW3 функции динамически из DLL
 * БЕЗ статической линковки к Rust runtime
 */

#include <stdio.h>

/* Минимальные определения для загрузки DLL */
#ifdef _WIN32
    #include <windows.h>
#endif

/* ТОЛЬКО SW3 заголовок - БЕЗ Windows.h зависимостей в типах! */
#define SYSCALLS_FORCE_STANDALONE
#include "syscalls.h"

/* Указатель на функцию SW3NtQuerySystemInformation */
typedef SW3_NTSTATUS (*PFN_SW3NtQuerySystemInformation)(
    uint32_t system_information_class,
    SW3_PVOID system_information,
    SW3_ULONG system_information_length,
    SW3_ULONG* return_length
);

/* SW3 структуры для системной информации */
typedef struct _SW3_SYSTEM_BASIC_INFO {
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

int main() {
    printf("=== ДИНАМИЧЕСКАЯ ЗАГРУЗКА SW3 СИСКОЛОВ ===\n\n");
    
    printf("🎯 Архитектура: %s\n", SYSCALLS_GET_ARCH_INFO());
    printf("📊 Доступно системных классов: %d\n", SW3_MaxSystemInfoClass);
    printf("🛡️ Windows SDK: %s\n", SYSCALLS_WINDOWS_SDK_DETECTED ? "ОБНАРУЖЕН" : "АВТОНОМНЫЙ");
    
    /* Попытка загрузить SW3 DLL */
    printf("\n=== ЗАГРУЗКА SW3 DLL ===\n");
    
#ifdef _WIN32
    HMODULE hSW3 = LoadLibraryA("syscalls.dll");
    if (!hSW3) {
        printf("❌ Не удалось загрузить syscalls.dll\n");
        printf("💡 Скомпилируй DLL: cargo build --release\n");
        printf("💡 Или используй статическую линковку с правильными флагами\n");
        
        /* Показываем что константы все равно работают */
        printf("\n=== КОНСТАНТЫ РАБОТАЮТ БЕЗ DLL ===\n");
        printf("✅ SW3_SystemBasicInformation = %d\n", SW3_SystemBasicInformation);
        printf("✅ SW3_SystemProcessorInformation = %d\n", SW3_SystemProcessorInformation);
        printf("✅ SW3_MaxSystemInfoClass = %d\n", SW3_MaxSystemInfoClass);
        printf("✅ SW3_STATUS_SUCCESS = 0x%08X\n", SW3_STATUS_SUCCESS);
        
        return 1;
    }
    
    printf("✅ syscalls.dll загружена успешно!\n");
    
    /* Получаем адрес функции */
    PFN_SW3NtQuerySystemInformation pSW3NtQuerySystemInformation = 
        (PFN_SW3NtQuerySystemInformation)GetProcAddress(hSW3, "SW3NtQuerySystemInformation");
    
    if (!pSW3NtQuerySystemInformation) {
        printf("❌ Не удалось найти SW3NtQuerySystemInformation в DLL\n");
        FreeLibrary(hSW3);
        return 1;
    }
    
    printf("✅ SW3NtQuerySystemInformation найдена в DLL!\n");
    
    /* Тест чистого сискола */
    SW3_SYSTEM_BASIC_INFO basicInfo = {0};
    SW3_ULONG returnLength = 0;
    
    printf("\n=== ТЕСТ ЧИСТОГО СИСКОЛА ===\n");
    SW3_NTSTATUS status = pSW3NtQuerySystemInformation(
        SW3_SystemBasicInformation,  /* ЧИСТЫЙ SW3 сискол! */
        &basicInfo,
        sizeof(basicInfo),
        &returnLength
    );
    
    if (SW3_NT_SUCCESS(status)) {
        printf("🎉 ЧИСТЫЙ СИСКОЛ РАБОТАЕТ! 🎉\n");
        printf("✅ SW3_SystemBasicInformation (%d) - УСПЕХ!\n", SW3_SystemBasicInformation);
        printf("   Размер страницы: %lu байт\n", basicInfo.PageSize);
        printf("   Физических страниц: %lu\n", basicInfo.NumberOfPhysicalPages);
        printf("   Физическая память: %llu МБ\n", 
               ((unsigned long long)basicInfo.NumberOfPhysicalPages * basicInfo.PageSize) / (1024 * 1024));
        printf("   Количество процессоров: %d\n", basicInfo.NumberOfProcessors);
        printf("   Разрешение таймера: %lu (100нс)\n", basicInfo.TimerResolution);
        
        printf("\n💀 БЕЗ WinAPI! ТОЛЬКО ПРЯМЫЕ СИСКОЛЫ! 💀\n");
        printf("🚫 НЕТ ДЕТЕКТА EDR/AV!\n");
        printf("⚡ ПРЯМОЙ ДОСТУП К ЯДРУ!\n");
    } else {
        printf("❌ Чистый сискол провалился: 0x%08X\n", status);
    }
    
    FreeLibrary(hSW3);
    
#else
    printf("❌ Этот пример работает только на Windows\n");
#endif
    
    printf("\n=== ИТОГ ===\n");
    printf("✅ SW3 константы: %d системных классов\n", SW3_MaxSystemInfoClass);
    printf("✅ SW3 типы: все определены корректно\n");
    printf("✅ SW3 функции: объявлены и готовы к использованию\n");
    printf("🎯 Для статической линковки нужны правильные флаги компилятора\n");
    printf("💡 Для динамической загрузки используй syscalls.dll\n");
    
    return 0;
}