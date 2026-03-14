# Модуль: lib.rs (основная библиотека)

## Путь: `lib.rs`
## Размер: ~57,700 строк, 1.8 MB

## Описание

Главный и единственный модуль библиотеки `syscalls`. Содержит всё: типы, константы, структуры, SW3 runtime и 513 syscall-функций. Автогенерирован из SysWhispers3.

## Атрибуты

```rust
#![no_std]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::too_many_arguments, clippy::missing_safety_doc)]
#![allow(dead_code, unused_imports)]
```

## Секции файла (порядок)

1. **Type aliases** (~200 штук) -- `HANDLE`, `PVOID`, `NTSTATUS`, `SIZE_T`, `WCHAR`, ...
2. **Константы** (~198 штук) -- `STATUS_*`, `PAGE_*`, `MEM_*`, `SEC_*`, `PROCESS_*`, `THREAD_*`, `FILE_*`, `GENERIC_*`
3. **Макросы** (4) -- `NT_SUCCESS`, `NT_INFORMATION`, `NT_WARNING`, `NT_ERROR`
4. **Структуры** (25+) -- `UNICODE_STRING`, `OBJECT_ATTRIBUTES`, `IO_STATUS_BLOCK`, `CLIENT_ID`, `MEMORY_BASIC_INFORMATION`, ...
5. **SW3 Runtime** -- PEB parsing, hash, syscall table, jumper
6. **Syscall функции** (513) -- `nt_allocate_virtual_memory()`, `nt_create_file()`, ...

## SW3 Runtime (ключевые функции)

| Функция | Назначение |
|---------|------------|
| `sw3_hash_syscall()` | ROR8 хеш имени функции с seed |
| `sw3_get_peb()` | Чтение PEB через gs:[0x60] / fs:[0x30] |
| `sw3_populate_syscall_list()` | Инициализация: PEB → ntdll → exports → sort → hash |
| `sw3_get_syscall_number()` | Поиск номера syscall по хешу |
| `sw3_find_syscall_address()` | Поиск адреса syscall инструкции в ntdll |
| `sw3_get_random_syscall_address()` | Случайный syscall адрес (JUMPER mode) |
| `sw3_debug_get_count()` | Количество записей (отладка) |
| `sw3_debug_get_hash()` | Хеш по индексу (отладка) |
| `sw3_debug_get_syscall_addr()` | Адрес по индексу (отладка) |

## Конфигурация (захардкожена)

```rust
const SW3_SEED: u32 = 0xB8A54425;       // Хеш-seed
const SW3_MAX_ENTRIES: usize = 600;      // Макс. записей в таблице
// Recovery method: JUMPER_RANDOMIZED
// WoW64: enabled (runtime detection via fs:[0xC0])
```

## Паттерн syscall-функции

Каждая из 513 функций следует одному паттерну:

### x64
```rust
pub unsafe fn nt_xxx(param1: TYPE1, ...) -> NTSTATUS {
    let hash: u32 = PRECOMPUTED_HASH;
    let number = sw3_get_syscall_number(hash);
    let addr = sw3_get_random_syscall_address(hash);
    // inline ASM: mov r10, rcx; mov eax, number; jmp addr
}
```

### x86 (WoW64)
```rust
pub unsafe fn nt_xxx(param1: TYPE1, ...) -> NTSTATUS {
    let hash: u32 = PRECOMPUTED_HASH;
    let number = sw3_get_syscall_number(hash);
    let addr = sw3_get_random_syscall_address(hash);
    // Runtime check: fs:[0xC0] != 0 → WoW64 mode
    // WoW64: push args, push 0 (dummy ret), call gate, add esp, N+4
    // Native: push args, mov edx, esp, call addr
}
```
