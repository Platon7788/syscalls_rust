# Заметки и наблюдения syscalls-rust

## Архитектура
- [2026-02-07] lib.rs ~57K строк -- автогенерирован из SysWhispers3, не предназначен для ручного редактирования
- [2026-02-07] Каждая из 513 функций содержит inline ASM с уникальной хеш-константой
- [2026-02-07] c-bindings/build.rs парсит lib.rs регулярками -- хрупкий подход, но работает

## Безопасность
- [2026-02-07] JUMPER_RANDOMIZED -- самый продвинутый режим, подменяет return address
- [2026-02-07] Хеш-seed `0xB8A54425` -- менять только если нужна новая уникальная сборка
- [2026-02-07] `static mut SW3_SYSCALL_LIST` -- единственная mutable static, потенциальная data race при первом вызове

## Качество кода
- [2026-02-07] 0 TODO/FIXME/HACK -- чистая кодовая база
- [2026-02-07] error.rs -- единственный модуль с unit тестами (3 теста)
- [2026-02-07] Комментарии на английском, хорошо документированы

## WoW64 поддержка
- [2026-03-14] Полная WoW64 поддержка реализована для всех 513 x86 syscall функций
- [2026-03-14] Runtime-определение WoW64 через `fs:[0xC0]` (Wow64SystemServiceCall)
- [2026-03-14] Критичный момент: dummy return address (push 0) обязателен перед аргументами при вызове WoW64 gate
  - Без dummy: аргументы смещены на 4 байта, вызывает STATUS_ACCESS_VIOLATION
  - ntdll stub делает `call edx`, что само создаёт return addr на стеке -- при прямом вызове gate надо эмулировать это
- [2026-03-14] Две стратегии передачи параметров:
  - ≤4 params: прямые `push {pN}` через `in(reg)` операнды
  - >4 params: массив параметров на стеке, push через `[params_ptr + offset]`
- [2026-03-14] Используется `inlateout("eax")` для закрепления номера syscall в eax (предотвращает конфликт регистров)
- [2026-03-14] ntdll stubs формат: `B8 xx xx xx xx BA yy yy yy yy FF D2 C2 zz 00` (mov eax, SSN; mov edx, addr; call edx; ret N)

## Инфраструктура
- [2026-02-07] Git-репозиторий инициализирован, .gitignore создан
- [2026-02-07] Собранные артефакты (dll, lib, a, exe) исключены через .gitignore
- [2026-02-07] Rust stable -- не требует nightly
- [2026-07-02] `.gitattributes` добавлен: LF в repo, native checkout, .bat/.cmd/.ps1 остаются CRLF -- убирает шум CRLF-warning'ов

## Edition 2024 миграция (2026-07-02)
- [2026-07-02] Оба крейта переведены на edition 2024, MSRV `1.96`. Всё собирается на stable.
- [2026-07-02] Основной риск был -- 83 warning'а от `unsafe_op_in_unsafe_fn` (новый default lint в edition 2024). Решено crate-level `#![allow(...)]` -- см. ADR-13.
- [2026-07-02] `#[unsafe(no_mangle)]` заменил `#[no_mangle]` в 519 генерируемых C wrappers. Изменение в `c-bindings/build.rs` codegen.
- [2026-07-02] `#[unsafe(naked)]`, `#[unsafe(link_section)]` уже использовались с 2026-03. С edition 2024 стали "родными" (без warning'ов).
- [2026-07-02] Downstream verify: IMGUI_NXT, IMGUI_SPF, Auth-Workspace/client-sdk-windows -- собираются чисто. NX_DRV-MOD, PE-Protect, Vex0r -- pre-existing dear-imgui-sys конфликты, к нашим изменениям не относится.

## Экосистема (2026-07-02)
- [2026-07-02] `useful-lib` возвращён на syscalls-rust (был кратко на RSC/SysCalls). Единственная нетривиальность миграции -- snake_case vs PascalCase имён (rsc-runtime использовал `NtClose`, syscalls-rust -- `nt_close`). Решено через алиасы `use syscalls::{nt_close as NtClose, ...}` в `ntsys.rs` модулях -- call-sites в `core.rs` не тронуты.
- [2026-07-02] Другие потребители (IMGUI_NXT, IMGUI_SPF, NX_DRV-MOD, PE-Protect, Vex0r, Auth-Workspace) никогда с syscalls-rust не переезжали.
- [2026-07-02] Сиблинг-репо SysCalls (RSC) удалён локально. git-история на github.com/Platon7788/SysCalls.

## Documentation debt закрыт (2026-07-02)
- [2026-07-02] README.md актуализирован: убран LEGACY-баннер, поправлен toolchain requirement (stable вместо nightly), включена WoW64=True в конфиг.
- [2026-07-02] CLAUDE.md обновлён: снят LEGACY, добавлены edition-2024 conventions, обновлён FFI section (`#[unsafe(no_mangle)]`).
- [2026-07-02] Все файлы в Docs/ приведены в соответствие: PROGRESS, CHANGELOG (v0.2.0), CURRENT_STATE, TODO, DECISIONS (ADR-12/13/14), NOTES (эта запись).
