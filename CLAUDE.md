# CLAUDE.md - syscalls-rust

> **Статус: активный проект.** Это основной крейт прямых NT-syscall'ов
> для всех Rust-проектов в `D:\GitHub\Rust_Projects\` (IMGUI_NXT,
> IMGUI_SPF, NX_DRV-MOD, PE-Protect, Vex0r, Auth-Workspace, useful-lib
> и др.). Repo: https://github.com/Platon7788/syscalls_rust.
>
> Ранее существовавший «преемник» — отдельный репо `SysCalls (RSC)` —
> удалён в июле 2026: единственный его потребитель (`useful-lib`)
> мигрирован обратно на `syscalls-rust`, все прочие consumer'ы никогда
> с RSC не переезжали. Ветка `SysCalls` сохранена на GitHub
> (https://github.com/Platon7788/SysCalls) как исторический референс.

## Проект

**syscalls-rust** v0.1.0 -- библиотека прямых Windows NT syscall'ов на Rust (SysWhispers3), с полными C/C++ биндингами.

- `#![no_std]` -- нулевые runtime-зависимости
- **Rust edition 2024**, MSRV 1.97
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

## Low-Level Quality Standards

Правила качества на основе [lowleveldevskills.com](https://www.lowleveldevskills.com/) и [rust-skills](https://github.com/actionbook/rust-skills) — применяй при написании и ревью кода.

### Assembly / x86 & x86_64 (assembly-x86)
- **Calling conventions**: x64 = Microsoft x64 (RCX, RDX, R8, R9 + shadow space 0x28), x86 = stdcall/cdecl — критично для naked syscall функций
- **Register preservation**: x64 callee-saved: RBX, RBP, RDI, RSI, R12-R15; x86 callee-saved: EBX, ESI, EDI, EBP
- **Naked functions** (`#[unsafe(naked)]`): компилятор не генерирует prologue/epilogue — всё вручную через `naked_asm!`
- **Inline ASM** (`core::arch::asm!`): clobbers, in/out constraints, `sym` для вызова Rust-функций из ASM
- **Syscall ABI**: x64: syscall number в EAX, param1 = R10 (не RCX!), остальные RDX, R8, R9; x86: EAX + EDX = ESP
- **WoW64 gate**: `fs:[0xC0]` (WoW64Reserved) — если != 0, процесс WoW64, вызов через gate вместо sysenter
- При модификации ASM — проверяй **оба** пути (x64 native и x86/WoW64)

### PE Parsing & PEB (binary inspection)
- **PEB доступ**: x64 = `gs:[0x60]`, x86 = `fs:[0x30]` — через inline ASM, не через NtQueryInformationProcess
- **InLoadOrderModuleList**: PEB → Ldr → InLoadOrderModuleList — linked list `LIST_ENTRY` (Flink/Blink)
- **PE header validation**: всегда проверяй `e_magic == 0x5A4D` (MZ) и `Signature == 0x00004550` (PE) перед парсингом
- **Export table walk**: Export Directory → AddressOfNames → binary/linear search → AddressOfNameOrdinals → AddressOfFunctions
- **RVA to pointer**: `base + RVA` — проверяй что RVA в пределах секции перед dereference
- Все парсинг-структуры (`ImageDosHeader`, `ImageNtHeaders`, `ImageExportDirectory`) — raw pointer cast, не `transmute`

### Memory Model & Atomics (memory-model, concurrency)
- **Static mut**: `SW3_SYSCALL_LIST` — глобальный mutable state, race condition при первом вызове из нескольких потоков идемпотентен (все вычисляют одинаковый результат)
- **Ordering**: `Acquire` при чтении `.count`, `Release` при записи — publish pattern для lazy init
- x86/x64 TSO гарантирует: store-store и load-load не переупорядочиваются, но store-load может — для syscall table это не проблема (write-once)

### Hash Obfuscation & Anti-Detection (binary-hardening)
- **ROR8 hash**: seed `0xB8A54425` — deterministic, но не reversible без brute-force
- Хеши вычисляются **compile-time** (hardcoded в каждой syscall функции) — не меняй seed без пересчёта всех хешей
- **HalosGate pattern**: если функция в ntdll захучена (начинается не с `0x4C, 0x8B, 0xD1, 0xB8`), ищет соседние функции ±512 для восстановления номера syscall
- **Jumper mode**: прыжок на `syscall; ret` в случайной функции ntdll — скрывает реальный return address от call stack analysis

### Unsafe Rust & Safety (rust-unsafe)
- **Масштаб unsafe**: весь lib.rs — inherently unsafe (inline ASM, raw pointers, static mut, FFI)
- **Safety invariants** для каждого unsafe блока: документируй через `// SAFETY:` комментарий
- **Naked functions**: `#[unsafe(naked)]` — компилятор не вставляет prologue, не проверяет ABI — полная ответственность на разработчике
- **Raw pointer arithmetic**: `.add()`, `.sub()`, `.offset()` — всё unsafe, проверяй bounds перед dereference
- **Никогда** `transmute` для PE-структур — используй `ptr as *const ImageDosHeader`
- **`#![allow(unsafe_op_in_unsafe_fn)]`** на уровне крейта (edition 2024) — сознательное решение: вся кодовая база inherently unsafe, wrapping каждой операции в `unsafe {}` добавляет шум без safety-value. **Но** — при добавлении **новых** helper-функций, где unsafe оп не очевиден (например, ты вычислил указатель и разыменовываешь его через 20 строк), всё равно ставь **явный `unsafe { }` блок** и `// SAFETY:` комментарий — это дисциплина, не lint.
- **`#[unsafe(no_mangle)]`, `#[unsafe(link_section)]`, `#[unsafe(naked)]`** — все атрибуты, помеченные unsafe в edition 2024, оборачивай в `unsafe(...)`. C-bindings `build.rs` эмитит `#[unsafe(no_mangle)]` для всех генерируемых wrapper'ов.

### FFI / C Bindings (rust-ffi)
- `#[unsafe(no_mangle)] extern "C"` для всех экспортируемых функций (edition 2024 требует `unsafe(...)` обёртку)
- `#[repr(C)]` для структур в header
- C wrappers в `c-bindings/src/lib.rs` — автогенерируются через `build.rs`
- При изменении сигнатур в lib.rs — **всегда** проверяй что `c-bindings/build.rs` корректно парсит и `syscalls.h` обновился
- Не передавай Rust-типы (String, Vec, Box) через FFI — только raw pointers + primitive types

### Build & Optimization (rustc, profiling)
- `opt-level = "z"` — оптимизация по размеру (критично для injectable библиотеки)
- `lto = true` + `codegen-units = 1` — максимальный inlining, один объектный файл
- `panic = "abort"` — без unwinding (уменьшает размер, нет CRT зависимости)
- `strip = true` — удаление debug symbols из release
- Feature `debug` — вставляет `int3` перед каждым syscall для отладки в отладчике

### Error Handling (error-handling)
- `NtStatus` wrapper: `is_success()`, `is_error()`, `name()` — 260+ именованных кодов
- `NtResult<T>` = `Result<T, NtStatus>` — стандартный паттерн
- `NtStatusExt` trait: `.to_result()` для конвертации сырого NTSTATUS
- На FFI границе: возвращай сырой `NTSTATUS` (i32) — C-код не работает с Rust Result
- **Никогда panic** в библиотечном коде — все ошибки через Result

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
