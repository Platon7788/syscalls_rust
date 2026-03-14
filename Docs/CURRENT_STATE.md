# CURRENT_STATE.md -- syscalls-rust

## Версия: 0.1.0

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

### C биндинги (c-bindings/)

| Компонент | Статус |
|-----------|--------|
| build.rs генератор | Готово |
| syscalls.h (автогенерация) | Готово |
| staticlib (.lib, .a) | Готово |
| cdylib (.dll) | Готово |
| C примеры (26 файлов) | Готово |
| build.bat / build_all.bat | Готово |

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
| x86_64-pc-windows-msvc | Готово |
| x86_64-pc-windows-gnu | Готово |
| i686-pc-windows-msvc | Готово (с WoW64) |
| i686-pc-windows-gnu | Готово (с WoW64) |

## Известные ограничения

1. **Windows only** -- syscall'ы специфичны для Windows NT
2. **Не thread-safe при первом вызове** -- `static mut` таблица, но идемпотентна
3. **Номера syscall'ов меняются между версиями Windows** -- runtime-определение решает это
4. **lib.rs ~57K строк** -- сложно читать/редактировать целиком

## Известные проблемы

- Нет CI/CD
