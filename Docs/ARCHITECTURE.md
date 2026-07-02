# ARCHITECTURE.md -- syscalls-rust

## Структура репозитория

```
syscalls-rust/
├── Cargo.toml                    # Основной крейт "syscalls" (edition 2024)
├── Cargo.lock
├── .gitignore                    # Git ignore rules
├── .gitattributes                # Line-ending normalization (2026-07-02)
├── .dockerignore
├── lib.rs                        # Главная библиотека (~57K строк)
├── error.rs                      # NtStatus обёртка
├── README.md                     # Documentation
├── CLAUDE.md                     # Инструкции для AI-ассистента
├── examples/
│   └── test_syscalls.rs          # Rust тестовый пример
├── c-bindings/                   # Подкрейт C/C++ биндингов
│   ├── Cargo.toml                # Крейт "syscalls-c" (edition 2024)
│   ├── build.rs                  # Генератор syscalls.h (~3200 строк)
│   ├── cbindgen.toml             # Конфиг cbindgen (резервный)
│   ├── src/lib.rs                # Re-export + C wrappers
│   ├── include/
│   │   ├── syscalls.h            # Сгенерированный C заголовок
│   │   ├── wow64_helpers.h       # WoW64 helper macros
│   │   └── README.md
│   ├── examples/                 # C примеры (27 файлов)
│   ├── lib/                      # Собранные артефакты
│   │   ├── syscalls.dll          # Dynamic library
│   │   ├── syscalls.lib          # MSVC static lib
│   │   └── syscalls_mingw.a      # MinGW static lib
│   ├── build.bat
│   └── build_all.bat
└── Docs/                         # Документация
```

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

### C биндинги (syscalls-c)

```
c-bindings/build.rs              Парсит lib.rs регулярками, генерирует:
├── c_wrappers.rs (OUT_DIR)      extern "C" fn SW3NtXxx() обёртки
└── include/syscalls.h           C заголовок с типами, константами, функциями

c-bindings/src/lib.rs
├── pub use syscalls::*          Re-export всего из основного крейта
└── include!(c_wrappers.rs)      Подключение сгенерированных обёрток
```

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
