# Модуль: error.rs

## Путь: `error.rs`
## Размер: 378 строк

## Описание

Эргономичная обёртка над NTSTATUS для удобной обработки ошибок. Работает в `no_std` окружении (использует `core::fmt`).

## Ключевые типы

### NtStatus

```rust
#[repr(transparent)]
pub struct NtStatus(pub i32);
```

**Методы:**
- `from_raw(i32)` / `raw()` / `as_u32()` -- конверсии
- `is_success()` -- status >= 0
- `is_information()` -- severity bits == 1
- `is_warning()` -- severity bits == 2
- `is_error()` -- severity bits == 3
- `facility()` -- extract facility code (bits 16-27)
- `code()` -- extract status code (bits 0-15)
- `name()` -- human-readable имя (120+ кодов, match по u32)

**Traits:** `Clone`, `Copy`, `PartialEq`, `Eq`, `From<i32>`, `Into<i32>`, `Debug`, `Display`

### NtResult\<T\>

```rust
pub type NtResult<T> = Result<T, NtStatus>;
```

### NtStatusExt trait

```rust
pub trait NtStatusExt {
    fn to_result(self) -> NtResult<()>;
    fn to_result_with<T>(self, value: T) -> NtResult<T>;
}
impl NtStatusExt for i32 { ... }
```

### Предопределённые константы (24)

`STATUS_SUCCESS`, `STATUS_TIMEOUT`, `STATUS_PENDING`, `STATUS_ACCESS_VIOLATION`, `STATUS_INVALID_HANDLE`, `STATUS_INVALID_PARAMETER`, `STATUS_NO_MEMORY`, `STATUS_ACCESS_DENIED`, `STATUS_BUFFER_TOO_SMALL`, `STATUS_OBJECT_NAME_NOT_FOUND`, `STATUS_PRIVILEGE_NOT_HELD`, `STATUS_INSUFFICIENT_RESOURCES`, `STATUS_NOT_FOUND` и другие.

## Тесты (3)

| Тест | Проверяет |
|------|-----------|
| `test_status_success` | NtStatus(0) -- is_success, !is_error, name == "STATUS_SUCCESS" |
| `test_status_error` | NtStatus(0xC0000005) -- !is_success, is_error, name == "STATUS_ACCESS_VIOLATION" |
| `test_to_result` | 0.to_result() == Ok, 0xC0000005.to_result() == Err |
