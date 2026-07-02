# SysWhispers3 C/C++ Bindings

C/C++ биндинги для библиотеки прямых системных вызовов Windows NT.

## Особенности

- **513 NT syscalls + 6 helper-функций** (519 экспортируемых функций всего)
- **Автогенерация header** — `syscalls.h` генерируется автоматически из Rust кода
- **SW3_ префиксы** — все функции имеют префикс SW3_ для избежания конфликтов
- **Независимость от Windows SDK** — все функции доступны всегда
- **Обход хуков** — прямые syscalls минуя ntdll.dll и usermode hooks
- **EDR/AV bypass** — не использует стандартные API, невидим для большинства защит
- **Jumper Randomized** — рандомизация адресов возврата для обхода stack tracing
- **C/C++ совместимость** — полная поддержка обоих языков
- **Stable Rust** — работает на стабильном Rust без nightly
- **WoW64 inspect/manipulate helpers** — header-only `wow64_helpers.h` для работы
  с x86-таргетами из x64-хоста (PEB32 walk, export resolve с forwarder'ами,
  WOW64_CONTEXT)

## Сборка

```bash
cargo build --release
```

Выходные файлы (release, x64):
```
target/release/
├── syscalls.lib      # Статическая библиотека (~3.5 MB)
├── syscalls.dll      # Динамическая библиотека (~245 KB)
└── syscalls.dll.lib  # Import library для DLL (~124 KB)

include/
├── syscalls.h        # Автосгенерированный C header (~6.2K строк)
└── wow64_helpers.h   # Header-only утилиты для работы с WoW64-целями
```

## Использование в C/C++

### Подключение

**Вариант 1 (Рекомендуется): Автоопределение**
```c
#include "syscalls.h"  // Автоматически определяет Windows SDK
```

**Вариант 2: Принудительное включение Windows SDK**
```c
#define SYSCALLS_FORCE_WINDOWS_SDK
#include <windows.h>
#include <winternl.h>
#include "syscalls.h"
```

**Вариант 3: Standalone режим (без Windows SDK)**
```c
#define SYSCALLS_FORCE_STANDALONE
#include "syscalls.h"
```

### Линковка (MSVC)

**DLL (рекомендуется):**
```cpp
#pragma comment(lib, "syscalls.dll.lib")
// syscalls.dll должна быть рядом с exe
```

**Статическая библиотека:**
```cpp
#pragma comment(lib, "syscalls.lib")
```

Библиотека `no_std` — дополнительные системные библиотеки не требуются.

### Пример: Выделение памяти

```c
#include "syscalls.h"
#include <stdio.h>

int main() {
    SW3_PVOID base_addr = NULL;
    SW3_SIZE_T region_size = 0x1000;
    
    SW3_NTSTATUS status = SW3NtAllocateVirtualMemory(
        SW3_NtCurrentProcess(),
        &base_addr,
        0,
        &region_size,
        SW3_MEM_COMMIT | SW3_MEM_RESERVE,
        SW3_PAGE_READWRITE
    );
    
    if (SW3_NT_SUCCESS(status)) {
        printf("Allocated at: %p\n", base_addr);
        
        // Освобождение
        SW3NtFreeVirtualMemory(
            SW3_NtCurrentProcess(),
            &base_addr,
            &region_size,
            SW3_MEM_RELEASE
        );
    }
    
    return 0;
}
```

### Пример с совместимостью (без Windows SDK)

```c
#define SYSCALLS_FORCE_STANDALONE
#include "syscalls.h"
#include <stdio.h>

int main() {
    PVOID base_addr = NULL;  // Автоматически мапится на SW3_PVOID
    SIZE_T region_size = 0x1000;
    
    // Можно использовать как SW3_ префиксы, так и обычные имена
    NTSTATUS status = NtAllocateVirtualMemory(  // Мапится на SW3NtAllocateVirtualMemory
        NtCurrentProcess(),
        &base_addr,
        0,
        &region_size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_READWRITE
    );
    
    if (NT_SUCCESS(status)) {
        printf("Allocated at: %p\n", base_addr);
        NtFreeVirtualMemory(NtCurrentProcess(), &base_addr, &region_size, MEM_RELEASE);
    }
    
    return 0;
}
```

### Пример: Открытие процесса

```c
SW3_HANDLE process_handle = NULL;
SW3_OBJECT_ATTRIBUTES oa;
SW3_CLIENT_ID cid;

SW3_InitializeObjectAttributes(&oa, NULL, 0, NULL, NULL);
cid.UniqueProcess = (SW3_HANDLE)(SW3_ULONG_PTR)target_pid;
cid.UniqueThread = NULL;

SW3_NTSTATUS status = SW3NtOpenProcess(
    &process_handle,
    SW3_PROCESS_ALL_ACCESS,
    &oa,
    &cid
);

if (SW3_NT_SUCCESS(status)) {
    // Работа с процессом...
    SW3NtClose(process_handle);
}
```

### Пример: Чтение/запись памяти процесса

```c
// Чтение
SW3_BYTE buffer[256];
SW3_SIZE_T bytes_read;

SW3NtReadVirtualMemory(
    process_handle,
    (SW3_PVOID)0x7FF600000000,
    buffer,
    sizeof(buffer),
    &bytes_read
);

// Запись
SW3_BYTE shellcode[] = { 0x90, 0x90, 0xC3 };
SW3_SIZE_T bytes_written;

SW3NtWriteVirtualMemory(
    process_handle,
    target_address,
    shellcode,
    sizeof(shellcode),
    &bytes_written
);
```

## Архитектура

```
c-bindings/
├── build.rs          # Парсер Rust → C header (автогенерация)
├── src/lib.rs        # Re-export syscalls
├── include/
│   ├── syscalls.h        # Автосгенерированный header (~6.2K строк)
│   └── wow64_helpers.h   # Header-only утилиты для WoW64-целей
├── examples/         # Примеры использования (включая wow64_inspect.c)
├── Cargo.toml
└── README.md
```

### Как работает автогенерация

`build.rs` парсит `../lib.rs` и извлекает:
1. **Типы** — `HANDLE`, `NTSTATUS`, `PVOID` и др. → `SW3_HANDLE`, `SW3_NTSTATUS`, `SW3_PVOID`
2. **Структуры** — `UNICODE_STRING`, `OBJECT_ATTRIBUTES`, `CLIENT_ID` → `SW3_UNICODE_STRING`, etc.
3. **Константы** — `STATUS_SUCCESS`, `MEM_COMMIT`, `PAGE_EXECUTE_READWRITE` → `SW3_STATUS_SUCCESS`, etc.
4. **Функции** — все `pub unsafe fn nt_*` → `SW3NtXxx`

При каждом `cargo build` header перегенерируется если изменился исходный код.

### Система префиксов SW3_

Все элементы имеют префикс `SW3_` для максимальной совместимости:

| Rust | C Header | Совместимость |
|------|----------|---------------|
| `HANDLE` | `SW3_HANDLE` | `HANDLE` (если нет Windows SDK) |
| `nt_allocate_virtual_memory` | `SW3NtAllocateVirtualMemory` | `NtAllocateVirtualMemory` (если нет Windows SDK) |
| `STATUS_SUCCESS` | `SW3_STATUS_SUCCESS` | `STATUS_SUCCESS` (если нет Windows SDK) |

## API Reference

### Макросы

```c
// SW3 версии (всегда доступны)
SW3_NT_SUCCESS(status)      // status >= 0
SW3_NT_INFORMATION(status)  // информационный код
SW3_NT_WARNING(status)      // предупреждение
SW3_NT_ERROR(status)        // ошибка

SW3_NtCurrentProcess()      // псевдо-handle текущего процесса (-1)
SW3_NtCurrentThread()       // псевдо-handle текущего потока (-2)

SW3_InitializeObjectAttributes(p, n, a, r, s)  // инициализация OBJECT_ATTRIBUTES

// Совместимые версии (только без Windows SDK)
NT_SUCCESS(status)          // мапится на SW3_NT_SUCCESS
NtCurrentProcess()          // мапится на SW3_NtCurrentProcess
InitializeObjectAttributes  // мапится на SW3_InitializeObjectAttributes
```

### Основные функции

Все функции имеют префикс `SW3Nt` и всегда доступны независимо от Windows SDK:

| Категория | Функции |
|-----------|---------|
| **Память** | `SW3NtAllocateVirtualMemory`, `SW3NtFreeVirtualMemory`, `SW3NtProtectVirtualMemory`, `SW3NtReadVirtualMemory`, `SW3NtWriteVirtualMemory`, `SW3NtQueryVirtualMemory` |
| **Процессы** | `SW3NtOpenProcess`, `SW3NtTerminateProcess`, `SW3NtQueryInformationProcess`, `SW3NtSetInformationProcess`, `SW3NtCreateProcess`, `SW3NtCreateProcessEx` |
| **Потоки** | `SW3NtOpenThread`, `SW3NtCreateThread`, `SW3NtTerminateThread`, `SW3NtSuspendThread`, `SW3NtResumeThread`, `SW3NtGetContextThread`, `SW3NtSetContextThread` |
| **Файлы** | `SW3NtCreateFile`, `SW3NtOpenFile`, `SW3NtReadFile`, `SW3NtWriteFile`, `SW3NtDeleteFile`, `SW3NtQueryInformationFile` |
| **Реестр** | `SW3NtOpenKey`, `SW3NtCreateKey`, `SW3NtQueryValueKey`, `SW3NtSetValueKey`, `SW3NtDeleteKey` |
| **Секции** | `SW3NtCreateSection`, `SW3NtMapViewOfSection`, `SW3NtUnmapViewOfSection` |
| **Токены** | `SW3NtOpenProcessToken`, `SW3NtOpenThreadToken`, `SW3NtQueryInformationToken`, `SW3NtAdjustPrivilegesToken` |
| **Синхронизация** | `SW3NtWaitForSingleObject`, `SW3NtWaitForMultipleObjects`, `SW3NtCreateEvent`, `SW3NtSetEvent`, `SW3NtCreateMutant` |
| **Система** | `SW3NtQuerySystemInformation`, `SW3NtQuerySystemTime`, `SW3NtSetSystemTime` |

**Полный список — 513 NT syscalls (+6 helper-функций) в `syscalls.h`.**

### Коды ошибок NTSTATUS

```c
// SW3 версии (всегда доступны)
SW3_STATUS_SUCCESS              0x00000000  // Успех
SW3_STATUS_TIMEOUT              0x00000102  // Таймаут
SW3_STATUS_PENDING              0x00000103  // Операция в процессе
SW3_STATUS_ACCESS_DENIED        0xC0000022  // Доступ запрещён
SW3_STATUS_INVALID_HANDLE       0xC0000008  // Неверный handle
SW3_STATUS_INVALID_PARAMETER    0xC000000D  // Неверный параметр
SW3_STATUS_NO_MEMORY            0xC0000017  // Недостаточно памяти
SW3_STATUS_BUFFER_TOO_SMALL     0xC0000023  // Буфер слишком мал

// Совместимые версии (только без Windows SDK)
STATUS_SUCCESS                  // мапится на SW3_STATUS_SUCCESS
STATUS_ACCESS_DENIED            // мапится на SW3_STATUS_ACCESS_DENIED
// и т.д.
```

## WoW64 helpers (`wow64_helpers.h`)

Header-only утилиты для работы с **WoW64 (x86) процессами из x64-хоста**: ваш
exe собран как x64, использует эту DLL и читает/пишет память / инспектирует
модули / снимает регистры в 32-битной цели. Не требует Windows SDK для самих
структур (имеет параллельные `SW3_PEB32` / `SW3_LDR_DATA_TABLE_ENTRY32` /
`SW3_WOW64_CONTEXT` / PE32 заголовки), макросы `WOW64_CONTEXT_*` обёрнуты в
`#ifndef` для бесконфликтного сосуществования с `<windows.h>`.

### Что покрывает

| API | Назначение |
|-----|-----------|
| `sw3_wow64_detect` | определить, что цель WoW64, и сразу взять адрес PEB32 |
| `sw3_wow64_read_peb` | прочитать PEB32 целиком |
| `sw3_wow64_enum_modules` | обход `InLoadOrderModuleList` с callback'ом |
| `sw3_wow64_find_module` / `_w` | найти модуль по ASCII или UTF-16 имени |
| `sw3_wow64_resolve_export` | резолв `Dll!Func` **с автоматическим следованием за forwarder'ами** (`kernel32!LoadLibraryA` → `KernelBase!LoadLibraryA`) |
| `sw3_wow64_resolve_export_in` | то же, но с готовым `dll_base` |
| `sw3_wow64_get_thread_context` | реальные x86 регистры (info class 29) |
| `sw3_wow64_set_thread_context` | запись x86 регистров |
| `sw3_wow64_suspend_and_get_context` | suspend → get (оставляет поток suspended) |
| `sw3_wow64_atomic_set_context` | suspend → set → resume |
| `sw3_wow64_read` / `sw3_wow64_write` | I/O памяти по 32-битным адресам (zero-extend в `PVOID`) |

### Минимальный пример

```c
#include "syscalls.h"
#include "wow64_helpers.h"

HANDLE hProc = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                           FALSE, target_pid);

bool is_wow64 = false;
uint32_t peb32 = 0;
sw3_wow64_detect((SW3_HANDLE)hProc, &is_wow64, &peb32);
if (!is_wow64) { /* цель чистый x64 */ return 0; }

uint32_t load_library_a = 0;
sw3_wow64_resolve_export((SW3_HANDLE)hProc, peb32,
                         "kernel32.dll", "LoadLibraryA",
                         &load_library_a);
// load_library_a — x86-адрес в target-памяти,
// пригодный как StartRoutine для NtCreateThreadEx
```

Полный демо-пример: `examples/wow64_inspect.c` (перечисление модулей, резолв
экспортов с forwarder'ами, снапшот x86-контекста главного потока).

### Что НЕ покрыто

- Ordinal-only forwards (`"DLL.#42"`) — редкий формат, не обрабатывается
- Manual mapping x86 PE-образа
- Heaven's Gate (исполнение x64 кода внутри WoW64-процесса)

## Режимы совместимости

### 1. Автоопределение (по умолчанию)
```c
#include "syscalls.h"
// Автоматически определяет наличие Windows SDK
// SW3_ функции всегда доступны
// Совместимые алиасы доступны если SDK не найден
```

### 2. Принудительный Windows SDK
```c
#define SYSCALLS_FORCE_WINDOWS_SDK
#include <windows.h>
#include <winternl.h>
#include "syscalls.h"
// SW3_ функции доступны
// Совместимые алиасы недоступны (во избежание конфликтов)
```

### 3. Standalone режим
```c
#define SYSCALLS_FORCE_STANDALONE
#include "syscalls.h"
// SW3_ функции доступны
// Совместимые алиасы доступны
// Все типы и константы определены внутри syscalls.h
```

## Исправленные проблемы

✅ **Пустые структуры** - исправлены для C/C++ совместимости  
✅ **C++ ключевые слова** - автоматическая санитизация параметров  
✅ **Зависимость от Windows SDK** - все функции всегда доступны  
✅ **Конфликты типов** - система префиксов SW3_  
✅ **Компиляция C++** - полная поддержка G++/MSVC  

## Безопасность

⚠️ **Внимание**: Эта библиотека предназначена для:
- Исследования безопасности
- Разработки защитного ПО
- Образовательных целей

Использование для вредоносных целей незаконно.

## Совместимость

- **ОС**: Windows 10/11 (x86/x64)
- **Компилятор**: MSVC 2019+, GCC, Clang, MinGW
- **Языки**: C, C++
- **Rust**: 1.70+ (stable)
- **Windows SDK**: Опционально (автоопределение)

## Лицензия

MIT
