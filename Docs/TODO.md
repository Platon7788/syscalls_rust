# TODO.md -- syscalls-rust

## Из анализа кода

- 0 TODO/FIXME/HACK комментариев найдено -- кодовая база чистая

## Потенциальные улучшения

### Высокий приоритет
- [x] Инициализировать git-репозиторий
- [x] Создать .gitignore (target/, *.exe, *.dll, *.lib, *.a, *.pdb)
- [x] Мигрировать на Rust edition 2024 + stable Rust 1.96 (2026-07-02)
- [x] `.gitattributes` для нормализации line endings (2026-07-02)
- [ ] Разделить lib.rs на модули (types.rs, constants.rs, runtime.rs, syscalls/) -- для удобства навигации. **Blocker**: SW3-генератор не поддерживает multi-file вывод; либо форкать генератор, либо переехать на другой источник (phnt/pdb).

### Средний приоритет
- [x] Thread-safe инициализация SW3_SYSCALL_LIST -- `AtomicU32` count + CAS init-gate (2026-07-28)
- [x] Включить WoW64 поддержку -- реализовано для всех 513 функций (2026-03-14)
- [x] `#[unsafe(link_section = ".text")]` на naked-стабах (2026-07-02)
- [ ] Добавить feature gate для категорий функций (memory, process, thread, ...) -- уменьшит размер если не все нужны
- [x] Cargo workspace для основного крейта и c-bindings -- сделан 2026-07-28
  вместе с новым `syscalls-standalone` крейтом (ADR-15)
- [ ] **Bundle stealth**: signature diversification, x86 RAS, HalosGate fallback --
  осознанно отложено, триггер = первое столкновение с реальным EDR (см.
  `NOTES.md`, раздел «Отложенные улучшения bundle»)

### Низкий приоритет
- [ ] CI/CD (GitHub Actions) для автосборки на MSVC и MinGW + smoke-test downstream consumers
- [ ] Бенчмарки: сравнение с прямым вызовом ntdll
- [ ] Документация inline (rustdoc) для публичных функций
- [ ] Crates.io публикация -- **осмысленно только если API стабилизирован** и хеш-seed политика продумана (сейчас seed compile-time hardcoded во всех 513 функциях)
