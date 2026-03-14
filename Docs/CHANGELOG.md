# CHANGELOG.md -- syscalls-rust

## [Unreleased]

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
