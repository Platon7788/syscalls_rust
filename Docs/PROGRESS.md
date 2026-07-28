# Журнал прогресса syscalls-rust

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
