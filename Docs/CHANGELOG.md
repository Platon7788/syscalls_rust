# CHANGELOG.md -- syscalls-rust

## [Unreleased]

## [0.3.0] - 2026-07-28

### Добавлено
- **Cargo workspace** в корне: `syscalls` + `syscalls-c` + новый `syscalls-standalone`.
  Общий `target/`, общий `Cargo.lock`, `cargo build --workspace` собирает всё сразу
  (см. `DECISIONS.md`, ADR-15).
- **`syscalls-standalone` крейт** -- генератор self-contained drop-in bundle для
  MSVC C/C++ проектов (target consumer: xhook). Emits `syscalls.h` + `syscalls.c` +
  `syscallsstubs.x64.asm` + `syscallsstubs.x86.c` + `syscalls.props` + `README.md`.
  `X`-префикс на всех symbol'ах, CRT-free, ничего не линкует.
  Запуск: `cargo run -p syscalls-standalone -- --out <path>`.
- **Return-Address Spoofing (RAS)** на x64 в generated stubs -- kernel-side
  stack walk видит caller = ntdll, не calling module. Всегда on, per-stub
  argument-shift (mov r10, [rsp+src]; mov [rsp+dst], r10) компенсирует
  `sub rsp, 8` для gadget-слота (см. `DECISIONS.md`, ADR-16).
- **Атомарная инициализация SW3_SYSCALL_LIST** в `lib.rs` и в generated `syscalls.c`:
  `AtomicU32` count + `AtomicBool` init-gate + sentinel `X_COUNT_FAILED (-1)`.
  Устраняет formally-UB гонку и deadlock спинящихся losers при init-failure.

### Изменено
- **Deps** (c-bindings): `regex 1.12.2 → 1.13.1`, `memchr 2.7.6 → 2.8.3`,
  `regex-automata 0.4.13 → 0.4.16`, `regex-syntax 0.8.8 → 0.8.11`.
- **MSRV**: `rust-version = "1.97"` (edition 2024 + `sort_unstable_by_key` на слайсе).
- **Bubble sort → `sort_unstable_by_key`** в `sw3_populate_syscall_list`
  (`lib.rs`) -- `no_std`-совместимо, one-liner.
- **`#![no_std]` → `#![cfg_attr(not(test), no_std)]`** в `lib.rs` -- `cargo test`
  теперь работает, все 3 unit-теста в `error::tests` проходят.
- `[profile.release]` из `c-bindings/Cargo.toml` удалён -- наследуется из workspace root.

### Удалено
- **`output-wow64/`** -- стейл снапшот старой сборки (edition 2021, ~61 493 строк
  устаревшего `lib.rs`, без C-bindings, без ссылок ниоткуда).
- **18 `core::mem::transmute::<_, u32>(routine)`** в x86/WoW64-стабах заменены на
  типизированные касты (`routine as u32` для `PVOID`, `.map_or(0u32, |f| f as usize as u32)`
  для `Option<fn>`). `#![allow(clippy::missing_transmute_annotations)]` снят.
- **Duplicate STATUS_\* constants** в `error.rs` (22 переопределения `NtStatus`,
  заслонённые i32-версиями из lib.rs через glob-shadowing).

### Исправлено
- XML-комментарий с `--` в `syscalls.props` -- MSBuild MSB4024 при первом же импорте.
- `X_c_void` мусор-typedef в generated header (дублировал `void` alias).
- MSBuild передавал C-компиляторные флаги (`/permissive-`, `/Zc:*`, `/utf-8`)
  в `ml64.exe` при добавлении `.asm` через `target_sources()` -- MASM
  игнорировал их и **не создавал .obj**. Исправлено изоляцией стабов в
  OBJECT library в `syscalls.cmake` (см. интеграцию с xhook).

## [0.2.0] - 2026-07-02

### Добавлено
- **Rust edition 2024** (`syscalls`, `syscalls-c` крейты)
- MSRV повышен до **1.96** (`rust-version = "1.96"`)
- Crate-level `#![allow(unsafe_op_in_unsafe_fn)]` -- сознательное решение
  для inherently-unsafe кодовой базы (см. ADR-9 в `DECISIONS.md`)
- `#[unsafe(link_section = ".text")]` на всех 513 x64 naked-стабах --
  гарантия размещения в .text при любом toolchain
- `#[unsafe(no_mangle)]` вместо `#[no_mangle]` во всех 519 генерируемых
  C wrappers (`c-bindings/build.rs` codegen)
- `.gitattributes` -- нормализация line endings (LF в repo, native
  checkout, .bat/.cmd/.ps1 остаются CRLF)
- Обновлённый downstream consumer list в README.md

### Изменено
- CLAUDE.md полностью актуализирован под новую реальность (снят LEGACY
  баннер, добавлены edition-2024 conventions, обновлён FFI section)
- README.md переписан: убран «Moved to SysCalls» баннер, поправлен
  toolchain requirement (stable, не nightly), включена WoW64 в конфиг

### Удалено
- `lib.rs.bloat_baseline` (57740 строк снапшота старого lib.rs) --
  доо репозитория; для tracking размера рекомендуется `cargo bloat` в CI
- Легаси-статус: `syscalls-rust` больше не архив, снова primary крейт

### Инфраструктура
- Пре-existing WIP `SysCalls/` (сиблинг-репо, попытка «преемника»)
  удалён локально; git-история сохранена на github.com/Platon7788/SysCalls
- `useful-lib` мигрирован с `rsc-runtime` (SysCalls) обратно на
  `syscalls` (syscalls-rust)

## [0.1.1] - 2026-03-14

### Добавлено
- Полная WoW64 поддержка для всех 513 x86 syscall функций
  - Runtime-определение WoW64 через `fs:[0xC0]`
  - Dummy return address для корректного stack layout
  - Две стратегии: прямые push (≤4 params) и params array (>4 params)
  - `inlateout("eax")` для закрепления SSN в eax
- 19 новых C тестовых примеров (WoW64, ASM, debug, comparison и др.)

### Исправлено
- Clippy warnings в c-bindings/build.rs
- Trailing whitespace в error.rs

## [0.1.0] - 2026-02-07

### Добавлено
- Инициализация документации проекта (CLAUDE.md, Docs/)
- 513 NT syscall функций с хеш-обфускацией
- SysWhispers3 runtime (PEB parsing, hash lookup, JUMPER_RANDOMIZED)
- NtStatus обёртка с 120+ именованными кодами
- C/C++ биндинги с автогенерацией syscalls.h
- 200+ type aliases для Windows NT типов
- 198 констант (STATUS_*, PAGE_*, MEM_*, PROCESS_*, ...)
- 25+ repr(C) структур
- Rust тестовый пример (memory alloc/free/protect/query)
- 7 C примеров
- Собранные артефакты (syscalls.dll, .lib, _mingw.a)
