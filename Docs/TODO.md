# TODO.md -- syscalls-rust

## Из анализа кода

- 0 TODO/FIXME/HACK комментариев найдено -- кодовая база чистая

## Потенциальные улучшения

### Высокий приоритет
- [x] Инициализировать git-репозиторий
- [x] Создать .gitignore (target/, *.exe, *.dll, *.lib, *.a, *.pdb)
- [ ] Разделить lib.rs на модули (types.rs, constants.rs, runtime.rs, syscalls/) -- для удобства навигации

### Средний приоритет
- [ ] Добавить thread-safe инициализацию (Once / atomic flag) для SW3_SYSCALL_LIST
- [x] Включить WoW64 поддержку -- реализовано для всех 513 функций (2026-03-14)
- [ ] Добавить feature gate для категорий функций (memory, process, thread, ...) -- уменьшит размер если не все нужны
- [ ] Cargo workspace для основного крейта и c-bindings

### Низкий приоритет
- [ ] CI/CD (GitHub Actions) для автосборки на MSVC и MinGW
- [ ] Бенчмарки: сравнение с прямым вызовом ntdll
- [ ] Документация inline (rustdoc) для публичных функций
- [ ] Crates.io публикация
