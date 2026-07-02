# Журнал прогресса syscalls-rust

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
