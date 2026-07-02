# DECISIONS.md -- syscalls-rust

## Архитектурные решения

### 1. `#![no_std]`
**Решение**: Библиотека не использует стандартную библиотеку Rust.
**Причина**: Минимизация зависимостей и размера бинарника. Offensive-инструменты должны быть компактными и не тянуть лишний runtime. Syscall'ы работают на уровне ядра -- std не нужен.

### 2. Один файл lib.rs (49K строк)
**Решение**: Все типы, константы, структуры, runtime и 513 функций в одном файле.
**Причина**: Автогенерация из SysWhispers3. Каждая syscall-функция содержит inline ASM с хеш-константой, уникальной для данного seed. Разбиение на модули усложнило бы генерацию и регенерацию.

### 3. Хеш-обфускация (ROR8 + seed)
**Решение**: Имена функций хешируются с seed `0xB8A54425`. В бинарнике хранятся только хеши.
**Причина**: Строковые сигнатуры вида "NtAllocateVirtualMemory" легко обнаруживаются AV/EDR. Хеши не reverse'ятся без seed (односторонняя функция).

### 4. JUMPER_RANDOMIZED mode
**Решение**: Вместо прямого `syscall` -- `jmp` на случайный syscall адрес в ntdll.
**Причина**: Прямой `syscall` из не-ntdll модуля легко обнаруживается stack trace анализом. Переход на адрес в ntdll маскирует вызов под легитимный.

### 5. Runtime определение номеров syscall
**Решение**: Номера syscall определяются при первом вызове через PEB + export table.
**Причина**: Номера syscall меняются между версиями Windows (и даже patch'ами). Статические номера работали бы только на одной версии.

### 6. Сортировка Zw* функций по адресу
**Решение**: Syscall номер = индекс функции при сортировке по адресу.
**Причина**: Windows kernel назначает номера syscall'ов в порядке адресов Zw* функций в ntdll. Это документированное поведение, используемое SysWhispers.

### 7. C биндинги через custom build.rs (не cbindgen)
**Решение**: Собственный парсер Rust → C header на regex.
**Причина**: cbindgen не справляется с 49K-строчным файлом и специфическим форматом (inline ASM, условная компиляция, SW3_ prefix'ы). Custom build.rs даёт полный контроль над генерацией.

### 8. Dual crate-type: staticlib + cdylib
**Решение**: C биндинги собираются и как статическая библиотека, и как DLL.
**Причина**: Разные сценарии использования -- статическая линковка для standalone tools, DLL для инъекций и динамической загрузки.

### 9. Release profile: opt-level = "z" + strip
**Решение**: Оптимизация размера вместо скорости, удаление символов.
**Причина**: Offensive-инструменты должны быть компактными. Syscall'ы не CPU-bound -- размер важнее скорости. Strip убирает debug info, усложняя reverse engineering.

### 10. WoW64: dummy return address + params array
**Решение**: При вызове WoW64 gate (`fs:[0xC0]`) пушить dummy return address (0) перед аргументами. Для функций с >4 параметрами использовать массив параметров на стеке вместо прямых push.
**Причина**: WoW64 gate ожидает стек в формате ntdll stub'а, где `call edx` уже положил return address. Без dummy -- аргументы смещены на 4 байта, что вызывает STATUS_ACCESS_VIOLATION. Params array решает проблему ограниченного количества регистров в inline ASM для функций с большим числом аргументов.

### 11. SW3_ prefix для C типов
**Решение**: Все типы в C header имеют префикс `SW3_` (SW3_HANDLE, SW3_NTSTATUS).
**Причина**: Избежание конфликтов с Windows SDK headers. Пользователь может подключить и windows.h, и syscalls.h одновременно. Автодетекция `_WINDEF_` позволяет использовать нативные типы когда SDK доступен.

### 12. Rust edition 2024 + MSRV 1.96 (2026-07-02)
**Решение**: Оба крейта (`syscalls`, `syscalls-c`) переведены с edition 2021 на 2024. `rust-version = "1.96"`.
**Причина**: `#[unsafe(naked)]`, `naked_asm!`, `#[unsafe(link_section)]`, `#[unsafe(no_mangle)]` -- все эти атрибуты в edition 2024 становятся "родными" (не warning-worthy на новых версиях rustc). Плюс `if let` chains, precise closure captures и `let-else` -- потенциальные упрощения в неавтогенерируемой части (error.rs, build.rs).
**Компромисс**: MSRV поднимается с ~1.85 до 1.96 -- обрезает старые toolchain'ы. Приемлемо: все downstream consumers работают на stable 1.96.

### 13. `#![allow(unsafe_op_in_unsafe_fn)]` на уровне crate (2026-07-02)
**Решение**: В `lib.rs` и `c-bindings/src/lib.rs` добавлен crate-level allow. Suppression'ы warn'ов новой edition-2024 семантики "each unsafe op in unsafe fn needs explicit unsafe block".
**Причина**: Весь `lib.rs` (57K строк) -- inherently unsafe: inline ASM, raw pointer walks over PEB/PE, static mut syscall table, FFI. Обёртывание каждого из 83+ warning'ов в `unsafe { }` добавляет шум без safety-value -- safety invariants задокументированы per-function (`// SAFETY:`), а не per-operation. Auto-generated wrappers в `c-bindings` -- та же категория.
**Компромисс**: Новый код в этих крейтах не будет получать warn'ы. Митигация -- в CLAUDE.md явно прописано: **разработчик всё равно должен ставить `unsafe { }` вокруг non-obvious operations** (когда указатель вычисляется в одном месте, а разыменовывается через много строк) и писать `// SAFETY:` комментарий. Это дисциплина, а не lint.

### 14. Un-archive syscalls-rust, отказ от «преемника» SysCalls (2026-07-02)
**Решение**: LEGACY-баннер снят, `syscalls-rust` снова primary крейт. Сиблинг-репо `SysCalls (RSC)` удалён локально (git-история сохранена на github.com/Platon7788/SysCalls).
**Причина**: RSC был попыткой заменить SW3-генератор на pipeline «phnt + ntdll.pdb». В теории он покрывает 509 функций (Win10+Win11 union) и не требует Python-генератора. На практике его подхватил только `useful-lib`, а IMGUI_NXT, IMGUI_SPF, NX_DRV-MOD, PE-Protect, Vex0r, Auth-Workspace -- все остались на `syscalls-rust`. При аудите обнаружено, что RSC не даёт достаточно value чтобы оправдать поддержку двух параллельных крейтов, `useful-lib` мигрирован обратно.
**Компромисс**: Теряем более чистый API RSC (модульные `constants::`, `syscalls::`, `types::`, `error::`) в пользу flat-namespace SW3. Но модульная разбивка -- отдельный TODO (см. TODO.md высокий приоритет), можно сделать поверх SW3-генератора если понадобится.
