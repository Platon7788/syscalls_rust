# CURRENT_STATE.md -- syscalls-rust

## Версия: 0.3.0

## Toolchain

| | Value |
|---|---|
| Rust edition | **2024** |
| MSRV | **1.98** |
| Nightly required? | **Нет** -- всё на стабильном компиляторе |
| `#![no_std]` | Да (под `cfg_attr(not(test), ...)` -- тесты работают) |
| Cargo workspace | Да -- 2 крейта (`syscalls` + `syscalls-standalone`) |

## Структура workspace

| Крейт | Тип | Роль |
|---|---|---|
| `syscalls` | rlib | Основная библиотека для Rust-consumer'ов (edition 2024, no_std) |
| `syscalls-standalone` | bin | Генератор self-contained C/H/MASM drop-in bundle с `X`-префиксом для MSVC-проектов без Rust-зависимости (target: xhook) |

`c-bindings/` крейт удалён 2026-07-28 -- его роль полностью покрыл
`syscalls-standalone` (см. ADR-17 в `DECISIONS.md`).

## Статус проекта

**Активный.** Primary крейт NT-syscall'ов для всех Rust-проектов в
`D:/GitHub/Rust_Projects/`. С 2026-07-28 также обслуживает C/C++ проекты
через standalone bundle -- в `xhook` (`D:/GitHub/VsProjects/xhook`)
интегрирован через `syscalls.cmake` + `xsyscalls_attach(target)`.


## Что реализовано

### Основная библиотека (lib.rs)

| Компонент | Статус | Количество |
|-----------|--------|------------|
| Type aliases | Готово | 200+ |
| Константы (STATUS_*, PAGE_*, MEM_*, ...) | Готово | 198 |
| Макросы (NT_SUCCESS, NT_ERROR, ...) | Готово | 4 |
| Структуры (repr(C)) | Готово | 25+ |
| SW3 Runtime (PEB, hash, table) | Готово | 12 функций |
| Syscall функции | Готово | 513 |
| WoW64 поддержка (x86) | Готово | Все 513 функций |

### Категории syscall функций

| Категория | Количество | Примеры |
|-----------|-----------|---------|
| Memory | 12 | NtAllocateVirtualMemory, NtFreeVirtualMemory, NtProtectVirtualMemory |
| Process | 32 | NtCreateProcess, NtOpenProcess, NtTerminateProcess |
| Thread | 33 | NtCreateThread, NtSuspendThread, NtResumeThread |
| File/IO | 163 | NtCreateFile, NtReadFile, NtWriteFile |
| Registry | 49 | NtCreateKey, NtOpenKey, NtSetValueKey |
| Token | 21 | NtOpenProcessToken, NtAdjustPrivilegesToken |
| Synchronization | 45 | NtCreateEvent, NtWaitForSingleObject, NtCreateMutant |
| Object | 22 | NtClose, NtDuplicateObject, NtQueryObject |
| System | 9 | NtQuerySystemInformation, NtQuerySystemTime |
| Other | 127 | ALPC, atoms, audit, power, debug, ... |

### Error module (error.rs)

| Компонент | Статус |
|-----------|--------|
| NtStatus обёртка | Готово |
| 120+ именованных кодов | Готово |
| NtResult\<T\> | Готово |
| NtStatusExt trait | Готово |
| Display/Debug форматирование | Готово |
| Unit тесты (3 теста) | Готово |

### Standalone C/H/MASM bundle (syscalls-standalone/)

| Компонент | Статус |
|-----------|--------|
| CLI (`--out <dir>`) | Готово |
| Парсер lib.rs (regex) | Готово |
| Emit syscalls.h (X-prefix decls) | Готово |
| Emit syscalls.c (CRT-free runtime, атомик init) | Готово |
| Emit syscallsstubs.x64.asm (MASM + RAS) | Готово |
| Emit syscallsstubs.x86.c (naked + WoW64 gate) | Готово |
| Emit syscalls.props (MSBuild) + syscalls.cmake | Готово |
| hash-collision audit bin | Готово |
| Verified: xhook integration (Debug/Release × Win32/x64) | Готово |

### Примеры

| Пример | Язык | Что тестирует |
|--------|------|---------------|
| test_syscalls.rs | Rust | Alloc, write, read, protect, query, free, sleep |
| simple_os_version.c | C | Базовое определение версии ОС |
| os_version_english.c | C | Версия ОС на английском |
| correct_os_version.c | C | Корректная реализация версии |
| dynamic_syscalls_test.c | C | Динамическая загрузка DLL |
| syscalls_declarations_test.c | C | Проверка объявлений функций |
| test_constants.c | C | Доступность констант |
| sw3_full_compatibility_test.c | C | Полная совместимость SW3 |
| test_wow64.c | C | WoW64 syscall вызовы |
| test_wow64_extended.c | C | Расширенные WoW64 тесты |
| test_2arg.c | C | Тест с 2 аргументами |
| test_asm.c | C | Проверка ASM кода |
| test_calledx.c | C | Тест call edx |
| test_compare.c | C | Сравнение результатов |
| test_comprehensive.c | C | Комплексный тест |
| test_debug.c | C | Отладочный тест |
| test_disasm_ntdll.c | C | Дизассемблирование ntdll стабов |
| test_dummy_all.c | C | Тест dummy return address |
| test_eax_bits.c | C | Тест eax/SSN |
| test_edx_call.c | C | Тест call через edx |
| test_exact.c | C | Точный тест syscall |
| test_gate2.c | C | Тест WoW64 gate |
| test_minimal.c | C | Минимальный тест |
| test_order.c | C | Порядок параметров |
| test_quick.c | C | Быстрый тест |
| test_systematic.c | C | Систематический тест |
| dump_hashes.c | C | Дамп хешей функций |

## Поддерживаемые платформы

| Target | Статус |
|--------|--------|
| x86_64-pc-windows-msvc | Готово, tested |
| i686-pc-windows-msvc | Готово (с WoW64), tested |

MinGW (`*-windows-gnu`) не тестируется. Кому нужно -- добавит соответствующий
`[target.*-windows-gnu]` блок в свой workspace overlay поверх нашего
`.cargo/config.toml` (см. также ADR-17).

## Известные ограничения

1. **Windows only** -- syscall'ы специфичны для Windows NT
2. **Номера syscall'ов меняются между версиями Windows** -- runtime-определение решает это
3. **lib.rs ~58K строк** -- сложно читать/редактировать целиком
4. **Bundle stealth уровня baseline** -- есть RAS на x64, но нет signature
   diversification и HalosGate fallback (см. `NOTES.md`, раздел
   «Отложенные улучшения bundle» для приоритетов)

## Известные проблемы

- Нет CI/CD (см. `NOTES.md` -- CI regression tests для generator в отложенных)

## Downstream consumers (path-dep)

Проверять при изменении public API:

| Проект | Крейт | Как связано |
|---|---|---|
| IMGUI_NXT | engine, engine-loading | `path = "../syscalls-rust"` (workspace dep) |
| IMGUI_SPF | engine | `path = "../syscalls-rust"` (workspace dep) |
| NX_DRV-MOD | engine | `path = "../syscalls-rust"` |
| PE-Protect | stub | `default-features = false`, `path = "../syscalls-rust"` |
| Vex0r/Client | introspect, usermode_debugger | абсолютный путь `D:/GitHub/Rust_Projects/syscalls-rust` |
| Auth-Workspace | client-sdk-windows | `path = "../../../syscalls-rust"` |
| useful-lib | proc_enum, drv_enum | `path = "../syscalls-rust"` (workspace dep) |

Верификация проводится через `cargo build -p <crate> --release` в
соответствующем workspace.
