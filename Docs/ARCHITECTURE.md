# ARCHITECTURE.md -- syscalls-rust

## Структура репозитория

```
syscalls-rust/
├── Cargo.toml                    # Workspace root + main "syscalls" крейт (edition 2024)
├── Cargo.lock
├── .cargo/config.toml            # +crt-static для MSVC targets
├── .gitignore
├── .gitattributes                # Line-ending normalization (2026-07-02)
├── .dockerignore
├── lib.rs                        # Главная библиотека (~58K строк)
├── error.rs                      # NtStatus обёртка
├── README.md
├── CLAUDE.md                     # Инструкции для AI-ассистента
├── examples/
│   └── test_syscalls.rs          # Rust тестовый пример
├── syscalls-standalone/          # bin: генератор self-contained C/H/MASM bundle
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # CLI (--out <dir> [--lib <lib.rs>])
│       ├── parse.rs              # regex-парсер lib.rs + ROR8 hash
│       ├── emit_h.rs             # syscalls.h emit (X-prefix)
│       ├── emit_c.rs             # syscalls.c emit (CRT-free runtime)
│       ├── emit_asm_x64.rs       # syscallsstubs.x64.asm emit (MASM + RAS)
│       ├── emit_stubs_x86.rs     # syscallsstubs.x86.c emit (__declspec(naked))
│       ├── emit_props.rs         # syscalls.props (MSBuild) + README template
│       └── bin/audit.rs          # hash-collision audit bin
└── Docs/                         # Документация
```

**История**: раньше был подкрейт `c-bindings/` (`syscalls-c`), эмитил
`syscalls.h` + `staticlib`+`cdylib`. Удалён 2026-07-28 -- полностью заменён
на `syscalls-standalone` (модель "source drop-in" вместо "pre-built binary
+ Rust build у consumer'а"). См. `DECISIONS.md`, ADR-17.

## Модули и зависимости

### Основной крейт (syscalls)

```
lib.rs
├── Type aliases (200+)          HANDLE, PVOID, NTSTATUS, SIZE_T, ...
├── Constants (198)              STATUS_*, PAGE_*, MEM_*, PROCESS_*, THREAD_*, ...
├── Macros (4)                   NT_SUCCESS, NT_INFORMATION, NT_WARNING, NT_ERROR
├── Structures (25+)             UNICODE_STRING, OBJECT_ATTRIBUTES, IO_STATUS_BLOCK, ...
├── SW3 Runtime                  PEB parsing, hash, syscall table, jumper
│   ├── sw3_hash_syscall()       ROR8 хеш с seed
│   ├── sw3_get_peb()            Чтение PEB из GS/FS
│   ├── sw3_populate_syscall_list()  Инициализация таблицы
│   ├── sw3_get_syscall_number() Поиск номера по хешу
│   ├── sw3_find_syscall_address()  Поиск syscall инструкции
│   └── sw3_get_random_syscall_address()  Случайный адрес (JUMPER)
└── Syscall functions (513)      nt_allocate_virtual_memory(), nt_create_file(), ...

error.rs
├── NtStatus                     #[repr(transparent)] обёртка i32
├── NtResult<T>                  Result<T, NtStatus>
├── NtStatusExt trait            .to_result(), .to_result_with()
└── Constants (24)               STATUS_SUCCESS, STATUS_ACCESS_DENIED, ...
```

### Standalone bundle generator (syscalls-standalone)

```
syscalls-standalone/src/         Парсит lib.rs (regex) и эмитит:
├── parse.rs                     - Function/Param/Parsed структуры + ROR8 hash
├── emit_h.rs                    → syscalls.h (X-prefix типы, decls, macros)
├── emit_c.rs                    → syscalls.c (CRT-free runtime + PEB walk +
│                                              атомик init + RAS gadget setup)
├── emit_asm_x64.rs              → syscallsstubs.x64.asm (513 MASM PROC,
│                                              jumper_randomized + RAS,
│                                              per-stub args-shift для N>4)
├── emit_stubs_x86.rs            → syscallsstubs.x86.c (513 __declspec(naked)
│                                              с WoW64 gate runtime-detect)
├── emit_props.rs                → syscalls.props + README template
└── main.rs                      CLI orchestrator + I/O
```

Consumer после генерации подключает bundle **без Rust-runtime зависимости**:
для CMake-проектов -- через `syscalls.cmake` (`xsyscalls_attach(target)`),
для .vcxproj -- через `<Import Project="syscalls.props"/>`.

## Поток данных при вызове syscall

```
Rust код                    C код
    |                         |
nt_allocate_virtual_memory()  SW3NtAllocateVirtualMemory()
    |                         |
    +--------+-------+--------+
             |
    sw3_get_syscall_number(hash)
             |
    [первый вызов?] → sw3_populate_syscall_list()
             |                    |
             |               PEB → ntdll → exports → Zw* → sort → hash
             |
    syscall_number + random_syscall_address
             |
    inline ASM: mov r10, rcx; mov eax, syscall_number; jmp [random_addr]
             |
    Windows Kernel → NTSTATUS
```

## Calling Convention

### x64
```
Параметр 1 → RCX (копируется в R10 для syscall)
Параметр 2 → RDX
Параметр 3 → R8
Параметр 4 → R9
Параметры 5+ → Stack (RSP+0x28, RSP+0x30, ...)
Номер syscall → EAX
Возврат → EAX (NTSTATUS)
```

### x86 (WoW64)
```
Параметры → Stack (stdcall)
Номер syscall → EAX
WoW64 gate → fs:[0xC0] (Wow64SystemServiceCall)
Возврат → EAX (NTSTATUS)

Два режима (runtime-определение через fs:[0xC0]):
1. WoW64 (32-bit на 64-bit Windows):
   - push args (reverse order), push 0 (dummy return addr), call gate
   - Dummy return address обязателен: WoW64 gate ожидает стек как у ntdll stub
   - ≤4 params: прямые push через in(reg) операнды
   - >4 params: массив параметров на стеке, push через [params_ptr + offset]

2. Native x86:
   - push args, mov edx, esp, call [addr] (sysenter convention)
```

## Потокобезопасность

- `SW3_SYSCALL_LIST` -- `static mut`, инициализация не thread-safe
- На практике безопасно: таблица заполняется одинаково при любом вызове
- Гонка при первом вызове из нескольких потоков -- результат идемпотентен
