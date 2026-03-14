# Журнал прогресса syscalls-rust

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
