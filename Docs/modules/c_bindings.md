# Модуль: c-bindings/

## Путь: `c-bindings/`
## Крейт: `syscalls-c` v0.1.0 (edition 2024)

## Описание

Подкрейт для создания C/C++ совместимых библиотек (staticlib + cdylib). Автоматически генерирует `syscalls.h` заголовок и C-обёртки для всех 513 syscall-функций.

## Сборка

```bash
cd c-bindings
cargo build --release
```

**Результат:**
- `target/release/syscalls.dll` -- динамическая библиотека
- `target/release/syscalls.dll.lib` -- import library (MSVC)
- `target/release/libsyscalls.a` -- статическая библиотека (MinGW)

## build.rs (генератор заголовка)

Парсит `../lib.rs` регулярными выражениями и генерирует:

1. **`$OUT_DIR/c_wrappers.rs`** -- Rust-обёртки с `extern "C"` для каждой функции
2. **`include/syscalls.h`** -- Полный C заголовок

### Этапы генерации

1. Чтение lib.rs
2. `extract_type_aliases()` -- 200+ `pub type X = Y;`
3. `extract_constants()` -- 198 `pub const X: T = V;`
4. `extract_structs()` -- 25+ `#[repr(C)] pub struct X { ... }`
5. `extract_functions()` -- 513 `pub unsafe fn nt_xxx(...) -> NTSTATUS`
6. `rust_to_c_type()` -- маппинг типов (Rust → C с SW3_ prefix)
7. `to_pascal_case()` -- snake_case → PascalCase для C имён
8. `generate_header()` -- сборка .h файла

### Маппинг имён

| Rust | C |
|------|---|
| `nt_allocate_virtual_memory` | `SW3NtAllocateVirtualMemory` |
| `HANDLE` | `SW3_HANDLE` |
| `NTSTATUS` | `SW3_NTSTATUS` |
| `UNICODE_STRING` | `SW3_UNICODE_STRING` |

## src/lib.rs

```rust
#![allow(non_snake_case, clippy::missing_safety_doc, clippy::too_many_arguments)]
// Edition 2024: генерируемые wrappers пробрасывают unsafe fn без явных unsafe { }
#![allow(unsafe_op_in_unsafe_fn)]

pub use syscalls::*;
include!(concat!(env!("OUT_DIR"), "/c_wrappers.rs"));
```

Re-экспорт всех типов из основного крейта + включение сгенерированных обёрток.

**Edition 2024**: build.rs эмитит `#[unsafe(no_mangle)]` (не старый `#[no_mangle]`)
для всех ~519 C wrappers -- это обязательная edition-2024 форма unsafe-атрибута.

## Примеры (27 файлов)

| Файл | Описание |
|------|----------|
| simple_os_version.c | Базовое определение версии ОС |
| os_version_english.c | Версия ОС на английском |
| correct_os_version.c | Корректная реализация версии |
| dynamic_syscalls_test.c | Динамическая загрузка DLL |
| syscalls_declarations_test.c | Проверка объявлений функций |
| test_constants.c | Доступность констант |
| sw3_full_compatibility_test.c | Полная совместимость SW3 |
| test_wow64.c | WoW64 syscall вызовы |
| test_wow64_extended.c | Расширенные WoW64 тесты |
| test_2arg.c | Тест с 2 аргументами |
| test_asm.c | Проверка ASM кода |
| test_calledx.c | Тест call edx |
| test_compare.c | Сравнение результатов |
| test_comprehensive.c | Комплексный тест |
| test_debug.c | Отладочный тест |
| test_disasm_ntdll.c | Дизассемблирование ntdll стабов |
| test_dummy_all.c | Тест dummy return address |
| test_eax_bits.c | Тест eax/SSN |
| test_edx_call.c | Тест call через edx |
| test_exact.c | Точный тест syscall |
| test_gate2.c | Тест WoW64 gate |
| test_minimal.c | Минимальный тест |
| test_order.c | Порядок параметров |
| test_quick.c | Быстрый тест |
| test_systematic.c | Систематический тест |
| dump_hashes.c | Дамп хешей функций |
| wow64_inspect.c | WoW64 gate/env inspection (2026-03) |

## Артефакты (lib/)

| Файл | Размер | Тип |
|------|--------|-----|
| syscalls.dll | 417 KB | Dynamic library |
| syscalls.lib | - | MSVC import library |
| syscalls_mingw.a | - | MinGW static library |

## Зависимости

- **Runtime**: `syscalls` (path = "..") -- основной крейт
- **Build**: `regex = "1"` -- для парсинга Rust кода
