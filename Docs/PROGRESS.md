# Журнал прогресса syscalls-rust

## 2026-10-09 — локальное ревью перед публикацией

- По запросу владельца GitHub Actions workflow удалён до публикации. Все проверки запускаются локально через `tests/verify-quality.ps1 -Offline`: fmt, Clippy, workspace tests, воспроизводимость шести файлов и C11/C++20 × x86/x64.
- Ревью выявило и исправило восстановление после частичной записи служебных маркеров; маркеры теперь публикуются через временный файл и rename. Зарезервированные имена метаданных проверяются без учёта регистра.
- До генерации отклоняются неподдерживаемые C-деклараторы массивов, нулевые массивы, void по значению и поля с ещё не определённой структурой. Добавлены регрессионные тесты. Двоичные/восьмеричные литералы переводятся в десятичные; заголовок подключает stdbool.h.
- Изменения ограничены диагностикой, разбором деклараций, заголовком, записью файлов и локальными проверками. Runtime и syscall stubs не изменены.
- Итоговый локальный прогон PASS: 57 исполнений workspace-тестов, два дополнительных layout-теста для x86/x64, Clippy с `-D warnings`, fmt, повторная генерация и все четыре C/C++ проверки. Тест восстановления самостоятельно запускает аварийно завершаемый дочерний процесс; отдельный helper и пример doctest штатно ignored. Лог: `target/local-quality-review.log` (не коммитится).

## 2026-10-09 — диагностика, полнота экспорта, C/C++ и восстановление

- Диагностика AST/валидации содержит line:column из proc-macro2 spans; CLI добавляет путь. Для полей и параметров сохраняются собственные позиции. Тесты фиксируют позиции синтаксических ошибок и неизвестных типов.
- Генератор сообщает о невыбранных/неподдерживаемых top-level декларациях, модулях, re-export, макросах и impl. В текущем исходнике явно виден пропуск LARGE_INTEGER_UNION; предупреждения не означают его поддержку.
- Добавлен Rust layout contract для всех 26 экспортируемых структур: sizeof, alignof и offsets всех полей. C11/C++20 × x64/x86 компилируют эти проверки; 12 структур дополнительно сравниваются с Windows SDK. Неофициальные структуры этим не признаются корректными для Windows ABI.
- Автоматизированы fmt, Clippy, workspace tests, повторная генерация со сравнением шести файлов, C/C++ contracts. Cargo.lock больше не игнорируется. Итоговый способ запуска — локальный PowerShell-скрипт, без GitHub Actions.
- Запись использует OS file lock, версионный manifest и сохраняемые old/new snapshots. `--recover <dir>` откатывает незавершённую установку или завершает cleanup committed-транзакции. При пользовательском изменении destination, неверном manifest, legacy staging или активном writer восстановление отказывает и сохраняет данные.
- Проверены выход дочернего процесса после первой/второй установки файла, восстановление после подготовки/commit cleanup, повторное восстановление, конфликт с пользовательской правкой, блокировка активного writer, traversal/device-name в manifest. Рекурсивного удаления нет.
- Верификация: workspace 54 исполнения тестов PASS; helper interruption_child отдельно запускается родительским тестом, doctest остаётся ignored. Clippy -D warnings, fmt, diff-check PASS. Шесть файлов bundle побайтово совпадают с предыдущим результатом и повторной генерацией. Все четыре C/C++/architecture-комбинации PASS.
- Ограничения: видимость для concurrent readers не атомарная; гарантии power-loss durability и защиты от намеренной подмены snapshots не заявлены. Существующие замечания к syscall-обёрткам этим этапом не закрываются.

## 2026-10-09 — согласованность деклараций перед генерацией

- Добавлен `validate.rs`: ссылки на типы/константы, коллизии C-имён с учётом служебных символов, дубликаты полей/параметров, C/C++ identifiers, циклы aliases/констант и порядок aliases для C.
- Удалена неявная подстановка `void*` для неизвестных типов; opaque-типы должны быть явно объявлены в исходнике.
- Две версии одной функции допускаются только для разных x86/x64 cfg с совпадающими типами параметров/результата; обычные дубликаты и третья версия отклоняются.
- Добавлены негативные unit-тесты и проверки CLI: ошибочные ссылки/дубликаты возвращают exit code 2 до создания output. Существующие объявления репозитория проходят валидацию.
- Проверки относятся к выбранным декларациям, не заменяют Rust type checker и не подтверждают ABI обёрток.
- Итог: 44 исполнения тестов workspace PASS (включая повторные parser tests в двух binary targets), 1 doctest ignored; Clippy -D warnings, fmt, diff-check PASS. Все шесть файлов bundle побайтово совпадают с результатом предыдущего AST-этапа. MSVC SDK layout/constant checks x64/x86 PASS.

## 2026-10-09 — замена regex на syn AST

- Исходник разбирается через syn::parse_file; объявления извлекаются из AST. Удалены regex и транзитивные зависимости; добавлены syn 2.0.119 и quote, обновлён Cargo.lock.
- Комментарии/строки не создают ложных объявлений; порядок derive/repr и комментарии между cfg и полем не теряют атрибуты. Вложенные callback-типы и именованные параметры с подчёркиванием разбираются структурно.
- Неподдерживаемые типы, generic-декларации, repr, выражения констант и синтаксические ошибки возвращают диагностику. Макросы/внешние модули не раскрываются; это ограниченный экспортёр объявлений, не Rust-to-C компилятор.
- Константы переводятся через AST выражений; исправлены три ссылки STANDARD_RIGHTS_* на X_READ_CONTROL.
- Сравнение с предыдущим результатом: 127 aliases, 26 structs, 189 constants, 513 functions; все файлы кроме трёх исправленных строк заголовка побайтово совпали.
- Проверки: workspace 32 исполнения тестов PASS (5 AST-тестов входят в оба binary target), 1 doctest ignored; Clippy -D warnings, fmt, diff-check PASS; SDK layout/constant checks MSVC x64/x86 PASS.

## 2026-10-09 — CLI и запись файлов генератора

- CLI использует args_os, поддерживает --help/-h и отклоняет неизвестные/повторные параметры, пустые значения и отсутствующие пути. По умолчанию читает lib.rs из текущего каталога; вне корня репозитория нужен --lib.
- Неподдерживаемые атрибуты полей возвращают Result с контекстом вместо panic. Пустой/неподдерживаемый вход без функций отклоняется до создания выходного каталога. Это не полная проверка синтаксиса Rust.
- Запись: staging, блокировка второго генератора, резервирование заменяемых файлов и откат при ошибках I/O. При неудаче отката сохраняются файлы восстановления; рекурсивного удаления каталогов нет. Crash-atomicity не заявляется.
- Добавлены unit-тесты CLI/записи (в том числе искусственный сбой второй установки файла) и интеграционные тесты exit code/диагностики/отсутствия выходных файлов при неверном входе.
- Верификация: workspace 22 теста PASS (1 doctest ignored), clippy workspace/all-targets с -D warnings PASS, fmt и git diff --check PASS.

## 2026-10-09 — диагностика NTSTATUS и layout C-заголовка

- Исправлены 28 неверных соответствий в `NtStatus::name()` по Windows SDK 10.0.28000.0 (`shared/ntstatus.h`). Добавлены имена для STATUS_NOT_SUPPORTED и STATUS_INVALID_PARAMETER_1..6, уже объявленных в lib.rs.
- Добавлены SDK fixture и тесты severity/unknown status. Числовые значения публичных констант не изменены; исправлены отображаемые имена.
- Парсер сохраняет архитектурные условия полей x86/x64; C emitter переносит их в `#if`. Неподдерживаемые атрибуты поля вызывают явную ошибку до записи bundle вместо молчаливого изменения layout.
- `tests/header-layout.c` проверяет размер, выравнивание и смещения MEMORY_BASIC_INFORMATION относительно Windows SDK без линковки runtime. MSVC x64/x86: PASS.
- Проверки: workspace 14 unit tests PASS (1 doctest ignored), x86 error tests 5 PASS, clippy all-targets с `-D warnings` PASS, fmt PASS.
- Изменения ограничены диагностикой, декларациями и их проверками. Остальные замечания исходного аудита остаются открытыми; downstream bundle не обновлялись.

## 2026-10-08 — SDK audit ExternalX01: native init failure

- Старый Rust gate сохранял спин ожидающих после failure; пустая таблица считалась success. Наблюдаемый RED: 2 fault теста; full-cap table отдельно RED. Теперь Failed терминален для всех caller, Ready только при `0 < count < cap`.
- UnsafeCell оформляет законную interior mutability таблицы; один writer, Release/Acquire publication. Изменения ограничены runtime/init и C emitter; public signatures/513 hashes/ASM не менялись.
- Настоящий x86 тест выявил и исправил невыровненное чтение export DLL name. WoW64 uses FS gate и не требует native sysenter gadget; тест проверяет реальный read-only NtQuerySystemTime, а не неверный nullable-gadget oracle.
- Проверено: 9 tests PASS на x64 и x86, fmt, strict Clippy обоих targets, release обоих targets, debug-feature build, 513 unique hash audit. Один прежний ignored usage doctest не изменялся.
- Regenerated emitted C: MSVC `/W4 /WX`, x64/x86; по три исполняемых oracle (empty/full table + actual Windows table), все PASS. Текущая Windows: 489/509 entries. Никакой OS corruption/resource exhaustion.
- Isolated xhook verification at 14bc2bbe: regenerated bundle build Debug/Release x64/Win32 PASS, static/default warning policy, install OFF. Его прежний fail-closed cap guard перенесён в common publisher, регенерация его не теряет. Consumer README preserved, header/stubs неизменны. xhook source/main не изменён и не опубликован.
- Ограничение проверок xhook: optional strict /WX выявляет прежний vendor HDE C4701; static INSTALL=ON export-set configure issue также вне runtime change. Проверка bundle build не утверждает успешную установку пакета или исправление всего xhook.

## 2026-10-06 — Rust 1.99 и зависимости

- Toolchain 1.99.0, MSRV 1.99 в обоих манифестах; regex 1.13.1.
- Изменения не затрагивают generated lib.rs, ASM и hash-seed.
- Проверено на Windows x64: fmt, Clippy -D warnings, тесты/doctests с
  all/default/no-default features, rustdoc -D warnings, release-сборка,
  генерация bundle и аудит 513 уникальных хешей без коллизий.


## 2026-08-22 (toolchain bump: Rust 1.98, deps refresh, version align)

- **MSRV `1.97` → `1.98`** во всех Cargo.toml (`syscalls`, `syscalls-standalone`)
  и в docs (CLAUDE.md, README.md, CURRENT_STATE.md, CONVENTIONS.md,
  PROJECT_OVERVIEW.md). Rust 1.98.0 (release 2026-08-20) стабильных фич,
  напрямую применимых к нашему inline-asm/PEB-walk/no_std коду, не принёс --
  см. `CHANGELOG.md` секция «MSRV подняли `1.97` → `1.98`» для конкретики,
  что рассмотрено и почему не задействовано.
- **Cargo package version** `0.1.0` → `0.3.0` в обоих крейтах -- Cargo.toml
  висел на инициальном `0.1.0`, тогда как docs (CURRENT_STATE, PROJECT_OVERVIEW,
  CHANGELOG) уже описывали v0.3.0. Приведено в соответствие.
- **Deps refresh**:
  - `regex` pin `"1"` → `"1.13"` в `syscalls-standalone/Cargo.toml` (последняя
    minor; сам crate уже был на `1.13.1`, но pin теперь явный).
  - `cargo update`: `aho-corasick 1.1.4 → 1.1.5`, `regex-automata 0.4.16 → 0.4.18`.
    `memchr 2.8.3`, `regex-syntax 0.8.11`, `regex 1.13.1` -- уже на latest.
- **Verify** (все зелёные):
  - `cargo build --workspace` (dev + `--features debug` + `--release`).
  - `cargo test --workspace`: 3 unit-теста в `error::tests`, 0 fail.
  - `cargo run -p syscalls-standalone -- --out <tmp>`: 513 syscalls, 6 файлов
    bundle сгенерированы без ошибок.
  - `cargo run -p syscalls-standalone --bin audit`: 513/513 уникальных
    хешей, 0 коллизий.

## 2026-07-28 (cleanup: c-bindings удалён, MinGW-таргеты сняты)

- **`c-bindings/` крейт удалён** -- 550 MB артефактов и ~3200 строк generator
  build.rs. Роль (staticlib + cdylib + генерация syscalls.h) полностью покрыл
  `syscalls-standalone`, но с лучшей моделью: consumer'у не нужен Rust
  toolchain, только запуск генератора один раз. Никакой downstream Rust-крейт
  на `syscalls-c` path-dep не ссылался (все пользуются основным `syscalls`).
- **MinGW-таргеты сняты** из `.cargo/config.toml` -- `x86_64-pc-windows-gnu`
  и `i686-pc-windows-gnu` блоки удалены. MSVC-only.
- **Docs actualization**: CLAUDE.md (карта файлов, зависимости, FFI-раздел
  переписан под standalone), CURRENT_STATE (workspace = 2 крейта, платформы
  без gnu), CHANGELOG (Unreleased breaking-change entry), DECISIONS ADR-17.
- **`Docs/modules/c_bindings.md`** удалён.
- Verify: `cargo build --workspace` + `cargo test --workspace` -- зелёные.

## 2026-07-28 (v0.3.0: standalone bundle + workspace + RAS + quality audit)

- **Cargo workspace** в корне (`syscalls`, `syscalls-c`, `syscalls-standalone`).
  Единый target/ + Cargo.lock. Удалён duplicated `[profile.release]` из c-bindings.
- **Новый крейт `syscalls-standalone`**: генератор self-contained C/H/MASM bundle
  с `X`-префиксом для drop-in в MSVC-проекты. Consumer: xhook. Файлы:
  `syscalls-standalone/src/{parse,emit_h,emit_c,emit_asm_x64,emit_stubs_x86,emit_props}.rs`.
- **Return-Address Spoofing (RAS) на x64** в generated stubs -- kernel-side stack walk
  показывает caller = ntdll. Per-stub argument-shift компенсирует `sub rsp, 8`.
  Всегда on, без опций.
- **Атомарная инициализация** SW3_SYSCALL_LIST и в lib.rs (Rust runtime), и в
  emitted syscalls.c: `AtomicU32` count + CAS gate + sentinel `-1` для failure.
  Losers больше не спинятся вечно при неудачной инициализации.
- **Аудит + фиксы**:
  - `regex 1.12.2 → 1.13.1`, `memchr 2.7.6 → 2.8.3` (+ regex-{automata,syntax})
  - `rust-version = "1.97"` (MSRV минимум, не патч)
  - `#![cfg_attr(not(test), no_std)]` → `cargo test` работает (3/3 в error::tests)
  - Bubble sort → `sort_unstable_by_key` в populate
  - 18 `transmute::<_, u32>` → типизированные касты; allow снят
  - 22 duplicate `STATUS_*` в error.rs удалены (glob-shadowing артефакт)
  - Стейл `output-wow64/` (61K строк) удалён
- **Интеграция с xhook (`D:/GitHub/VsProjects/xhook`)**:
  - Bundle сгенерирован в `xhook/syscalls/`
  - `syscalls.cmake` с функцией `xsyscalls_attach(target)` -- один include в
    CMakeLists.txt подключает всё
  - Верифицировано: `cmake --build --target xhook` для x64 и Win32 -- 0 errors,
    0 warnings from our code. `xhook.dll` собран для обеих архитектур
  - Critical fix: MSBuild forward'ил C compile-options в `ml64.exe` при добавлении
    .asm через `target_sources` -- MASM игнорировал их и не создавал .obj.
    Решено изоляцией стабов в OBJECT library
- **msbuild-верификация `.props`** (для не-CMake consumers): все 4 конфига
  (Debug/Release × Win32/x64) собираются, exe запускаются, реальные syscalls
  отдают `STATUS_SUCCESS`. Toolset на VS 2026 preview = `v145` (не `v180`).
- **Docs update**: CHANGELOG v0.3.0, DECISIONS ADR-15/16, NOTES секция
  «Standalone C/H/MASM bundle (2026-07-28)» и «Отложенные улучшения bundle»
  (signature diversification, x86 RAS, HalosGate).

## 2026-07-02 (edition 2024 + un-archive + downstream re-consolidation)

Kоммиты: `3c320d7`, `db3c6fd`, `e17b05b`, `73b790a`.

- **Миграция на Rust edition 2024, MSRV 1.96**
  - `syscalls`, `syscalls-c` -- `edition = "2024"`
  - `#![allow(unsafe_op_in_unsafe_fn)]` в `lib.rs` и `c-bindings/src/lib.rs`
    как сознательное crate-wide решение (см. `DECISIONS.md`, ADR-9)
  - `c-bindings/build.rs` codegen: `#[no_mangle]` → `#[unsafe(no_mangle)]`
    во всех 519 генерируемых C wrappers
- **Un-archive**: снят LEGACY-баннер, `syscalls-rust` снова primary
  крейт всей экосистемы (перенос `useful-lib` c `rsc-runtime` обратно)
- **`#[unsafe(link_section = ".text")]`** добавлен всем 513 x64
  naked-стабам (гарантия размещения в .text-секции при любом toolchain)
- **useful-lib**: инициализирован git, добавлены README + LICENSE-{MIT,APACHE},
  мигрирован на syscalls-rust (`refactor: migrate rsc-runtime dep`)
- **Repo hygiene**: `.gitattributes` в оба репо (LF в repo, native checkout),
  удалён `lib.rs.bloat_baseline` (57K строк снапшот) + добавлен паттерн в
  `.gitignore`
- **Docs**: полная актуализация Docs/, CLAUDE.md обновлён с edition-2024
  conventions, ADR-9 (edition 2024 + allow scope)
- **Верификация downstream**: IMGUI_NXT, IMGUI_SPF, Auth-Workspace/client-sdk-windows
  собраны против нового edition 2024 -- регрессий нет
- **cargo update**: `bitflags 2.11 → 2.13`, `quote 1.0.45 → 1.0.46`,
  `syn 2.0.117 → 2.0.118` в useful-lib

## 2026-03-14 (WoW64 поддержка + актуализация документации)
- Полная WoW64 поддержка для x86: все 513 syscall функций
  - Переписан inline ASM: params array для функций с >4 параметрами
  - Добавлен dummy return address для корректной работы WoW64 gate
  - Runtime-определение WoW64 через fs:[0xC0]
- Исправлены clippy warnings в c-bindings/build.rs
- Добавлено 19 новых C тестовых примеров (итого 26)
- lib.rs вырос с ~49K до ~57K строк (WoW64 ветки в каждой функции)
- c-bindings/build.rs вырос с ~2K до ~3.2K строк
- Актуализирована вся документация Docs/

## 2026-02-07 (инициализация документации)
- Полный анализ проекта (52K+ строк кода)
- Создан CLAUDE.md -- правила для AI-ассистента
- Создана Docs/ со всеми файлами:
  - PROJECT_OVERVIEW.md, ARCHITECTURE.md, CURRENT_STATE.md
  - CONVENTIONS.md, API.md, DECISIONS.md
  - TODO.md, CHANGELOG.md, PROGRESS.md, NOTES.md
  - modules/ (lib_core.md, error.md, c_bindings.md)
