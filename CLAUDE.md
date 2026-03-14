# CLAUDE.md - syscalls-rust

## Проект

**syscalls-rust** v0.1.0 -- библиотека прямых Windows NT syscall'ов на Rust (SysWhispers3), с полными C/C++ биндингами.

- `#![no_std]` -- нулевые runtime-зависимости
- 513 NT syscall функций с хеш-обфускацией имён
- Прямые syscall инструкции (x64: `syscall`, x86: `sysenter`/WoW64 gate) -- без вызова ntdll
- Полная WoW64 поддержка (x86 на 64-bit Windows) с dummy return address
- Режим JUMPER_RANDOMIZED -- подмена return address через случайный адрес syscall в ntdll
- C/C++ биндинги: staticlib + cdylib + автогенерация `syscalls.h`
- MIT лицензия

## Быстрые команды

```bash
cargo build                          # Сборка основного крейта
cargo build --release                # Release (opt-level=z, LTO, strip)
cargo build --features debug         # С int3 breakpoint перед каждым syscall
cargo run --example test_syscalls    # Запуск тестового примера

# C биндинги
cd c-bindings
cargo build --release                # Сборка DLL + статической библиотеки
```

## Архитектура

### Модель работы
1. При первом вызове любого syscall -- инициализация через PEB:
   - Поиск ntdll.dll через PEB -> Ldr -> InLoadOrderModuleList
   - Парсинг export table, сбор Zw* функций
   - Сортировка по адресу (индекс = номер syscall)
   - Хеширование имён (ROR8 + seed `0xB8A54425`)
   - Поиск адресов syscall инструкций (для JUMPER mode)
2. При вызове конкретной функции:
   - Поиск по хешу в таблице → номер syscall
   - Выбор случайного syscall адреса (JUMPER_RANDOMIZED)
   - Прямой вызов через inline ASM

### Карта файлов

```
lib.rs              -- Основная библиотека (~57700 строк): типы, константы, структуры, runtime SW3, 513 syscall функций (x64 + x86/WoW64)
error.rs            -- NtStatus обёртка, NtResult<T>, NtStatusExt trait (378 строк)
examples/
  test_syscalls.rs  -- Тестовый пример: аллокация, запись, чтение, защита, query, free, sleep
c-bindings/
  Cargo.toml        -- Крейт syscalls-c (staticlib + cdylib)
  build.rs          -- Автогенерация syscalls.h из Rust исходников (~116KB, ~3200 строк)
  cbindgen.toml     -- Конфигурация cbindgen (резервная)
  src/lib.rs        -- Re-export + include сгенерированных C wrappers
  include/
    syscalls.h      -- Сгенерированный C заголовок
  examples/         -- 26 C примеров (OS version, dynamic loading, constants, WoW64, debug и др.)
  lib/              -- Собранные артефакты (syscalls.dll, .lib, _mingw.a)
  build.bat         -- Windows build script
  build_all.bat     -- Multi-target build
```

## Ключевые типы

| Тип | Модуль | Назначение |
|-----|--------|------------|
| HANDLE, PVOID, NTSTATUS | lib.rs | Базовые Windows типы |
| UNICODE_STRING | lib.rs | NT строка (Length, MaximumLength, Buffer) |
| OBJECT_ATTRIBUTES | lib.rs | Параметры создания объектов |
| IO_STATUS_BLOCK | lib.rs | Статус I/O операции |
| CLIENT_ID | lib.rs | Идентификация процесса/потока |
| MEMORY_BASIC_INFORMATION | lib.rs | Информация о виртуальной памяти |
| NtStatus | error.rs | Обёртка NTSTATUS с именами и severity |
| NtResult\<T\> | error.rs | Result\<T, NtStatus\> |
| SW3SyscallEntry | lib.rs (внутр.) | Запись таблицы syscall'ов (hash, address, syscall_address) |

## Зависимости

- **Runtime**: нет (`#![no_std]`)
- **Build (c-bindings)**: `regex = "1"` (для парсинга Rust → C header)
- **System**: ntdll.dll (в памяти, не линкуется)

## Правила для AI-ассистента

1. **Перед началом работы** -- прочитай `Docs/CURRENT_STATE.md` и `Docs/ARCHITECTURE.md`
2. **Веди журнал** -- обновляй `Docs/PROGRESS.md` при завершении задач
3. **Фиксируй заметки** -- записывай наблюдения в `Docs/NOTES.md`
4. **lib.rs огромный (~57K строк)** -- не читай целиком, ищи конкретные функции по имени
5. **Не меняй хеш-seed** (`SW3_SEED = 0xB8A54425`) -- это сломает совместимость с C биндингами
6. **Не меняй inline ASM** без крайней необходимости -- это критичный низкоуровневый код
7. **x86/x64 only** -- архитектура привязана к Windows syscall ABI
8. **Комментарии в коде** -- на английском (сохранять существующий стиль)
9. **После изменения lib.rs** -- проверяй что c-bindings/build.rs корректно генерирует header
10. **Это security-инструмент** -- предназначен для red team, пентест, security research
