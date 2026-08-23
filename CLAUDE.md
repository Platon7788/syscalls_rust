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

**syscalls-rust** v0.3.0 -- библиотека прямых Windows NT syscall'ов на Rust (SysWhispers3) + generator standalone C/H/MASM bundle для не-Rust consumer'ов.

- `#![no_std]` (под `cfg_attr(not(test), ...)`) -- нулевые runtime-зависимости
- **Rust edition 2024**, MSRV **1.98**, MSVC-only (MinGW не тестируется)
- 513 NT syscall функций с хеш-обфускацией имён
- Прямые syscall инструкции (x64: `syscall`, x86: `sysenter`/WoW64 gate) -- без вызова ntdll
- Полная WoW64 поддержка (x86 на 64-bit Windows) с dummy return address
- Режим JUMPER_RANDOMIZED + Return-Address Spoofing (RAS) на x64 в generated bundle
- Standalone bundle (drop-in для MSVC): `cargo run -p syscalls-standalone -- --out <dir>`
- MIT лицензия

## Быстрые команды

```bash
cargo build --workspace              # Сборка обоих крейтов workspace
cargo build --workspace --release    # Release (opt-level=z, LTO, strip)
cargo build --features debug         # С int3 breakpoint перед каждым syscall
cargo test --workspace               # Тесты (3 в error::tests)
cargo run --example test_syscalls    # Тестовый пример

# Standalone C/H/MASM bundle (drop-in без Rust-зависимости)
cargo run -p syscalls-standalone -- --out D:\<path>\syscalls
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
lib.rs              -- Основная библиотека (~58K строк): типы, константы, структуры, runtime SW3, 513 syscall функций (x64 + x86/WoW64)
error.rs            -- NtStatus обёртка, NtResult<T>, NtStatusExt trait
examples/
  test_syscalls.rs  -- Тестовый пример: аллокация, запись, чтение, защита, query, free, sleep
syscalls-standalone/
  Cargo.toml        -- bin `syscalls-standalone` в workspace
  src/
    main.rs         -- CLI (--out <dir> [--lib <lib.rs>])
    parse.rs        -- regex-парсер lib.rs (types, structs, constants, functions) + ROR8 hash
    emit_h.rs       -- syscalls.h (X-prefix types, decls, macros)
    emit_c.rs       -- syscalls.c (CRT-free runtime: PEB walk, atomic init, RAS gadget setup)
    emit_asm_x64.rs -- syscallsstubs.x64.asm (MASM PROC × 513 с jumper_randomized + RAS)
    emit_stubs_x86.rs -- syscallsstubs.x86.c (__declspec(naked) × 513 + WoW64 gate)
    emit_props.rs   -- syscalls.props (MSBuild) + README.md
    bin/audit.rs    -- отдельный bin для hash-collision аудита
```

Consumer'ы вне Rust-мира тянут только **сгенерированный bundle** -- никаких
Rust-артефактов не нужно; см. `xhook/syscalls/` как reference-integration через
`syscalls.cmake` (`xsyscalls_attach(target)`).

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

- **Runtime** (крейт `syscalls`): нет (`#![no_std]`)
- **Build** (крейт `syscalls-standalone`): `regex = "1"` (для парсинга lib.rs)
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
- **Атомарная init**: `SW3_SYSCALL_LIST.count: AtomicU32` + `init_started: AtomicBool` (CAS-gate). Winner заполняет entries, публикует count с Release; losers спинятся до появления count > 0 с Acquire. На failure gate освобождается (Rust) или Count получает sentinel (C runtime -- `X_COUNT_FAILED`).
- **Ordering**: `Acquire` при чтении `.count`, `Release` при записи -- publish pattern для lazy init. Реализовано и в `lib.rs`, и в `emit_c.rs` эмитируемом `syscalls.c`.
- x86/x64 TSO гарантирует: store-store и load-load не переупорядочиваются, но store-load может -- для syscall table это не проблема (write-once).

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
- **`#[unsafe(no_mangle)]`, `#[unsafe(link_section)]`, `#[unsafe(naked)]`** -- все атрибуты, помеченные unsafe в edition 2024, оборачивай в `unsafe(...)`.

### FFI / C Consumers (standalone bundle)
- Для не-Rust consumer'ов есть `syscalls-standalone` -- генератор self-contained bundle (`syscalls.h`, `syscalls.c`, `syscallsstubs.x64.asm`, `syscallsstubs.x86.c`, `syscalls.cmake`/`syscalls.props`)
- `X`-префикс на всех symbol'ах (`XNtAllocateVirtualMemory`, `X_HANDLE`, `X_NTSTATUS`) -- не конфликтует с `<windows.h>`
- Runtime **CRT-free**: MSVC intrinsics inlined (`_InterlockedCompareExchange`, `__readgsqword`), BSS-only state
- **При изменении сигнатур в lib.rs** -- регенерируй bundle у downstream consumer'ов: `cargo run -p syscalls-standalone -- --out <dir>`

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
4. **lib.rs огромный (~58K строк)** -- не читай целиком, ищи конкретные функции по имени
5. **Не меняй хеш-seed** (`SW3_SEED = 0xB8A54425`) -- это сломает совместимость с уже развёрнутыми bundle'ами у consumer'ов
6. **Не меняй inline ASM** без крайней необходимости -- это критичный низкоуровневый код
7. **x86/x64 only, MSVC-first** -- архитектура привязана к Windows syscall ABI; MinGW не тестируется (кто хочет -- добавит `[target.*-windows-gnu]` в своём workspace overlay)
8. **Комментарии в коде** -- на английском (сохранять существующий стиль)
9. **После изменения lib.rs** -- регенерируй standalone bundle у downstream consumer'ов и проверяй что build (наш workspace + xhook) остаётся зелёным
10. **Это security-инструмент** -- предназначен для red team, пентест, security research
