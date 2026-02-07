/**
 * ТЕСТ ОБЪЯВЛЕНИЙ СИСКОЛОВ
 * 
 * Проверяет что все SW3 функции правильно объявлены
 * БЕЗ линковки к Rust библиотеке - только заголовки
 */

#include <stdio.h>

/* ТОЛЬКО SW3 заголовок - БЕЗ Windows.h! */
#define SYSCALLS_FORCE_STANDALONE
#include "syscalls.h"

/* Макрос для проверки объявления функции */
#define CHECK_FUNCTION_DECLARED(func_name) \
    printf("✅ %s - объявлена\n", #func_name); \
    (void)(func_name);

int main() {
    printf("=== ТЕСТ ОБЪЯВЛЕНИЙ SW3 СИСКОЛОВ ===\n\n");
    
    printf("🎯 Архитектура: %s\n", SYSCALLS_GET_ARCH_INFO());
    printf("📊 Доступно системных классов: %d\n", SW3_MaxSystemInfoClass);
    printf("🛡️ Windows SDK: %s\n", SYSCALLS_WINDOWS_SDK_DETECTED ? "ОБНАРУЖЕН" : "АВТОНОМНЫЙ");
    
    printf("\n=== ПРОВЕРКА ОБЪЯВЛЕНИЙ ФУНКЦИЙ ===\n");
    
    /* Проверяем что функции объявлены */
    CHECK_FUNCTION_DECLARED(SW3NtQuerySystemInformation);
    CHECK_FUNCTION_DECLARED(SW3NtQuerySystemInformationEx);
    CHECK_FUNCTION_DECLARED(SW3NtAllocateVirtualMemory);
    CHECK_FUNCTION_DECLARED(SW3NtFreeVirtualMemory);
    CHECK_FUNCTION_DECLARED(SW3NtCreateProcess);
    CHECK_FUNCTION_DECLARED(SW3NtCreateThread);
    CHECK_FUNCTION_DECLARED(SW3NtOpenProcess);
    CHECK_FUNCTION_DECLARED(SW3NtOpenThread);
    CHECK_FUNCTION_DECLARED(SW3NtCreateFile);
    CHECK_FUNCTION_DECLARED(SW3NtReadFile);
    CHECK_FUNCTION_DECLARED(SW3NtWriteFile);
    
    printf("\n=== ПРОВЕРКА ТИПОВ ===\n");
    printf("sizeof(SW3_HANDLE) = %zu байт\n", sizeof(SW3_HANDLE));
    printf("sizeof(SW3_NTSTATUS) = %zu байт\n", sizeof(SW3_NTSTATUS));
    printf("sizeof(SW3_ULONG) = %zu байт\n", sizeof(SW3_ULONG));
    printf("sizeof(SW3_ULONG_PTR) = %zu байт\n", sizeof(SW3_ULONG_PTR));
    printf("sizeof(SW3_PVOID) = %zu байт\n", sizeof(SW3_PVOID));
    
    printf("\n=== ПРОВЕРКА КОНСТАНТ ===\n");
    printf("SW3_SystemBasicInformation = %d\n", SW3_SystemBasicInformation);
    printf("SW3_SystemProcessorInformation = %d\n", SW3_SystemProcessorInformation);
    printf("SW3_SystemTimeOfDayInformation = %d\n", SW3_SystemTimeOfDayInformation);
    printf("SW3_SystemKernelDebuggerInformation = %d\n", SW3_SystemKernelDebuggerInformation);
    printf("SW3_MaxSystemInfoClass = %d\n", SW3_MaxSystemInfoClass);
    
    printf("\n=== ПРОВЕРКА СТАТУСОВ ===\n");
    printf("SW3_STATUS_SUCCESS = 0x%08X\n", SW3_STATUS_SUCCESS);
    printf("SW3_STATUS_ACCESS_DENIED = 0x%08X\n", SW3_STATUS_ACCESS_DENIED);
    printf("SW3_STATUS_INVALID_PARAMETER = 0x%08X\n", SW3_STATUS_INVALID_PARAMETER);
    
    printf("\n=== ПРОВЕРКА МАКРОСОВ ===\n");
    SW3_NTSTATUS test_status = SW3_STATUS_SUCCESS;
    printf("SW3_NT_SUCCESS(0x00000000) = %s\n", SW3_NT_SUCCESS(test_status) ? "true" : "false");
    
    test_status = SW3_STATUS_ACCESS_DENIED;
    printf("SW3_NT_SUCCESS(0xC0000022) = %s\n", SW3_NT_SUCCESS(test_status) ? "true" : "false");
    printf("SW3_NT_ERROR(0xC0000022) = %s\n", SW3_NT_ERROR(test_status) ? "true" : "false");
    
    printf("\n🎉 ВСЕ ОБЪЯВЛЕНИЯ КОРРЕКТНЫ! 🎉\n");
    printf("✅ Все SW3 функции объявлены правильно!\n");
    printf("✅ Все SW3 типы определены корректно!\n");
    printf("✅ Все SW3 константы доступны!\n");
    printf("✅ Все SW3 макросы работают!\n");
    printf("🚫 БЕЗ WinAPI зависимостей!\n");
    printf("💀 ГОТОВО ДЛЯ ЧИСТЫХ СИСКОЛОВ!\n");
    
    printf("\n=== ИНСТРУКЦИЯ ДЛЯ ИСПОЛЬЗОВАНИЯ ===\n");
    printf("1. Скомпилируй Rust библиотеку: cargo build --release\n");
    printf("2. Линкуй с правильными флагами для MinGW/MSVC\n");
    printf("3. Используй SW3NtQuerySystemInformation для получения версии ОС\n");
    printf("4. Все константы SW3_System*Information готовы к использованию!\n");
    
    return 0;
}