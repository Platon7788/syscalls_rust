# SysWhispers3 C/C++ Bindings

Полнофункциональные C/C++ биндинги для библиотеки SysWhispers3 с поддержкой всех 519 NT syscalls.

## ✅ Что работает:

- **519 NT syscalls** - все функции доступны
- **Автоматическое определение архитектуры** (x86/x64/WOW64)
- **Совместимость с Windows SDK** - автоматическое избежание конфликтов
- **Standalone режим** - работает без Windows SDK
- **Все NTSTATUS коды** и константы
- **Основные структуры** Windows NT
- **Макросы** - NT_SUCCESS, NtCurrentProcess(), InitializeObjectAttributes

## Быстрый старт:

### 1. Включить заголовочный файл:
```c
#include <windows.h>
#include <winternl.h>
#include "syscalls.h"  // Автоматически определит Windows SDK
```

### 2. Использовать syscalls:
```c
// Выделение памяти
PVOID base_address = NULL;
SIZE_T region_size = 0x1000;
NTSTATUS status = NtAllocateVirtualMemory(
    NtCurrentProcess(),
    &base_address,
    0,
    &region_size,
    MEM_COMMIT | MEM_RESERVE,
    PAGE_READWRITE
);

if (NT_SUCCESS(status)) {
    printf("Memory allocated at: %p\n", base_address);
}
```

### 3. Компиляция:
```bash
# MinGW
gcc -I./include -std=c99 your_program.c -o program.exe

# MSVC  
cl /I./include your_program.c /out:program.exe

# Clang
clang -I./include your_program.c -o program.exe
```

## Тестирование:

```bash
# Тест совместимости заголовочного файла
gcc -I./include examples/header_test.c -o header_test.exe
./header_test.exe
```

## Архитектурная информация:

```c
printf("Architecture: %s\n", SYSCALLS_GET_ARCH_INFO());  // "x64" или "x86" или "x86 (WOW64)"
printf("Pointer size: %d\n", SYSCALLS_POINTER_SIZE);     // 8 для x64, 4 для x86
```

## Режимы работы:

### С Windows SDK (рекомендуется):
```c
#include <windows.h>
#include <winternl.h>
#include "syscalls.h"
```

### Standalone (без SDK):
```c
#define SYSCALLS_FORCE_STANDALONE
#include "syscalls.h"
```

### Принудительный SDK:
```c
#define SYSCALLS_FORCE_WINDOWS_SDK
#include <windows.h>
#include <winternl.h>
#include "syscalls.h"
```

## Доступные функции:

Все 519 NT syscalls, включая:
- **Память**: NtAllocateVirtualMemory, NtFreeVirtualMemory, NtMapViewOfSection
- **Процессы**: NtCreateProcess, NtOpenProcess, NtTerminateProcess
- **Потоки**: NtCreateThread, NtOpenThread, NtSuspendThread
- **Файлы**: NtCreateFile, NtReadFile, NtWriteFile, NtQueryInformationFile
- **Реестр**: NtCreateKey, NtOpenKey, NtSetValueKey, NtQueryValueKey
- **Синхронизация**: NtCreateEvent, NtWaitForSingleObject
- **ALPC**: NtAlpcCreatePort, NtAlpcConnectPort, NtAlpcSendWaitReceivePort
- **И многие другие...**

## Константы и макросы:

```c
// NTSTATUS коды
STATUS_SUCCESS, STATUS_ACCESS_DENIED, STATUS_INVALID_PARAMETER

// Права доступа
GENERIC_READ, GENERIC_WRITE, PROCESS_ALL_ACCESS, THREAD_ALL_ACCESS

// Память
PAGE_READWRITE, MEM_COMMIT, MEM_RESERVE, MEM_RELEASE

// Файлы
FILE_READ_DATA, FILE_WRITE_DATA, FILE_GENERIC_READ

// Макросы
NT_SUCCESS(status)
NtCurrentProcess()
NtCurrentThread()
InitializeObjectAttributes(&oa, name, flags, root, security)
```

## Структуры:

```c
UNICODE_STRING, OBJECT_ATTRIBUTES, IO_STATUS_BLOCK, CLIENT_ID
ALPC_PORT_ATTRIBUTES, THREAD_BASIC_INFORMATION, PROCESS_BASIC_INFORMATION
TOKEN_STATISTICS
```

---

**Статус**: ✅ Заголовочный файл полностью готов и протестирован  
**Совместимость**: MinGW, MSVC, Clang, BCC64X  
**Архитектуры**: x86, x64, WOW64  
**Функций**: 519 NT syscalls