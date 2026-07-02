# CHANGELOG.md -- syscalls-rust

## [Unreleased]

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
