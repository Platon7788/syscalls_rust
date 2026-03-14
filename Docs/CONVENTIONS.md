# CONVENTIONS.md -- syscalls-rust

## Rust (основной крейт)

### Общие правила
- `#![no_std]` -- без стандартной библиотеки
- `#![allow(non_snake_case, non_camel_case_types)]` -- Windows типы сохраняют оригинальные имена
- `#![allow(clippy::too_many_arguments)]` -- syscall'ы имеют до 17 параметров
- Комментарии на английском

### Именование
- **Типы Windows**: `HANDLE`, `PVOID`, `NTSTATUS`, `UNICODE_STRING` -- SCREAMING_CASE / PascalCase (как в Windows SDK)
- **Константы**: `STATUS_SUCCESS`, `PAGE_READWRITE`, `MEM_COMMIT` -- SCREAMING_SNAKE_CASE
- **Syscall функции (Rust)**: `nt_allocate_virtual_memory()` -- snake_case с префиксом `nt_`
- **Syscall функции (C export)**: `SW3NtAllocateVirtualMemory()` -- PascalCase с префиксом `SW3`
- **SW3 runtime**: `sw3_hash_syscall()`, `sw3_get_peb()` -- snake_case с префиксом `sw3_`

### Unsafe
- Все syscall функции -- `pub unsafe fn` или `pub unsafe extern "C" fn`
- Все вызовы PEB/ASM -- в `unsafe` блоках
- Причина: прямые syscall'ы не проверяют параметры

### Структуры
- Все `#[repr(C)]` для совместимости с C
- Поля в SCREAMING_CASE / PascalCase (как в Windows SDK)
- Пример: `UNICODE_STRING { Length, MaximumLength, Buffer }`

## C (биндинги)

### Именование
- **Типы**: `SW3_` префикс для избежания конфликтов с Windows SDK (`SW3_HANDLE`, `SW3_NTSTATUS`)
- **Функции**: `SW3NtXxx()` -- PascalCase с префиксом SW3
- **Константы**: `SW3_STATUS_SUCCESS`, `SW3_PAGE_READWRITE` и т.д.
- **Макросы**: `SW3_NT_SUCCESS()`, `SW3_NtCurrentProcess()`

### Header guard
```c
#ifndef SYSCALLS_H
#define SYSCALLS_H
// ...
#endif /* SYSCALLS_H */
```

### Автодетекция
- Архитектура: `_M_X64` / `__x86_64__` (x64) vs `_M_IX86` / `__i386__` (x86)
- Windows SDK: `#ifdef _WINDEF_` -- если подключен, используются нативные типы
- C++: `extern "C" { }` обёртки

## Release Profile

```toml
opt-level = "z"      # Оптимизация размера
lto = true           # Link-time optimization
codegen-units = 1    # Один codegen unit
panic = "abort"      # Без unwinding
strip = true         # Убрать символы
```
