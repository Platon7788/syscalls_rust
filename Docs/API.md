# API.md -- syscalls-rust

## Rust API

### Основные функции (513 штук)

Все функции имеют единообразную сигнатуру:

```rust
pub unsafe fn nt_xxx(...) -> NTSTATUS;
pub unsafe extern "C" fn nt_xxx(...) -> NTSTATUS;
```

### Часто используемые

#### Память
```rust
nt_allocate_virtual_memory(ProcessHandle, BaseAddress, ZeroBits, RegionSize, AllocationType, Protect) -> NTSTATUS
nt_free_virtual_memory(ProcessHandle, BaseAddress, RegionSize, FreeType) -> NTSTATUS
nt_protect_virtual_memory(ProcessHandle, BaseAddress, RegionSize, NewProtect, OldProtect) -> NTSTATUS
nt_read_virtual_memory(ProcessHandle, BaseAddress, Buffer, BufferSize, NumberOfBytesRead) -> NTSTATUS
nt_write_virtual_memory(ProcessHandle, BaseAddress, Buffer, BufferSize, NumberOfBytesWritten) -> NTSTATUS
nt_query_virtual_memory(ProcessHandle, BaseAddress, InfoClass, Buffer, BufferLength, ReturnLength) -> NTSTATUS
nt_map_view_of_section(SectionHandle, ProcessHandle, BaseAddress, ...) -> NTSTATUS
nt_unmap_view_of_section(ProcessHandle, BaseAddress) -> NTSTATUS
```

#### Процессы
```rust
nt_open_process(ProcessHandle, DesiredAccess, ObjectAttributes, ClientId) -> NTSTATUS
nt_terminate_process(ProcessHandle, ExitStatus) -> NTSTATUS
nt_query_information_process(ProcessHandle, InfoClass, Buffer, BufferLength, ReturnLength) -> NTSTATUS
nt_suspend_process(ProcessHandle) -> NTSTATUS
nt_resume_process(ProcessHandle) -> NTSTATUS
```

#### Потоки
```rust
nt_create_thread_ex(ThreadHandle, DesiredAccess, ObjAttr, ProcessHandle, StartRoutine, Argument, ...) -> NTSTATUS
nt_open_thread(ThreadHandle, DesiredAccess, ObjectAttributes, ClientId) -> NTSTATUS
nt_suspend_thread(ThreadHandle, PreviousSuspendCount) -> NTSTATUS
nt_resume_thread(ThreadHandle, PreviousSuspendCount) -> NTSTATUS
nt_get_context_thread(ThreadHandle, ThreadContext) -> NTSTATUS
nt_set_context_thread(ThreadHandle, ThreadContext) -> NTSTATUS
nt_queue_apc_thread(ThreadHandle, ApcRoutine, ApcArgument1, ApcArgument2, ApcArgument3) -> NTSTATUS
```

#### Файлы
```rust
nt_create_file(FileHandle, DesiredAccess, ObjAttr, IoStatusBlock, AllocationSize, FileAttributes, ShareAccess, CreateDisposition, CreateOptions, EaBuffer, EaLength) -> NTSTATUS
nt_read_file(FileHandle, Event, ApcRoutine, ApcContext, IoStatusBlock, Buffer, Length, ByteOffset, Key) -> NTSTATUS
nt_write_file(FileHandle, Event, ApcRoutine, ApcContext, IoStatusBlock, Buffer, Length, ByteOffset, Key) -> NTSTATUS
nt_close(Handle) -> NTSTATUS
```

#### Токены
```rust
nt_open_process_token(ProcessHandle, DesiredAccess, TokenHandle) -> NTSTATUS
nt_open_thread_token(ThreadHandle, DesiredAccess, OpenAsSelf, TokenHandle) -> NTSTATUS
nt_adjust_privileges_token(TokenHandle, DisableAll, NewState, BufferLength, PreviousState, ReturnLength) -> NTSTATUS
nt_duplicate_token(ExistingHandle, DesiredAccess, ObjAttr, ImpersonationLevel, TokenType, NewHandle) -> NTSTATUS
```

#### Синхронизация
```rust
nt_wait_for_single_object(Handle, Alertable, Timeout) -> NTSTATUS
nt_wait_for_multiple_objects(Count, Handles, WaitType, Alertable, Timeout) -> NTSTATUS
nt_create_event(EventHandle, DesiredAccess, ObjAttr, EventType, InitialState) -> NTSTATUS
nt_set_event(EventHandle, PreviousState) -> NTSTATUS
nt_delay_execution(Alertable, DelayInterval) -> NTSTATUS
```

#### Система
```rust
nt_query_system_information(InfoClass, Buffer, BufferLength, ReturnLength) -> NTSTATUS
nt_query_system_time(SystemTime) -> NTSTATUS
```

### SW3 Runtime (отладка)

```rust
sw3_debug_get_count() -> u32           // Количество syscall'ов в таблице
sw3_debug_get_hash(index) -> u32       // Хеш по индексу
sw3_debug_get_syscall_addr(index) -> PVOID  // Адрес syscall по индексу
```

### Error API (error.rs)

```rust
// NtStatus обёртка
let status = NtStatus::from_raw(raw_ntstatus);
status.is_success()     // >= 0
status.is_error()       // severity == 3
status.name()           // "STATUS_ACCESS_DENIED"
status.facility()       // facility code
status.code()           // status code

// Конвертация в Result
raw_ntstatus.to_result()?;                    // NtResult<()>
raw_ntstatus.to_result_with(handle)?;         // NtResult<HANDLE>
```

### Полезные макросы

```rust
NT_SUCCESS(status)     // status >= 0
NT_INFORMATION(status) // severity == 1
NT_WARNING(status)     // severity == 2
NT_ERROR(status)       // severity == 3
```

### Константы-хэндлы

```rust
-1isize as HANDLE      // NtCurrentProcess()
-2isize as HANDLE      // NtCurrentThread()
-3isize as HANDLE      // NtCurrentSession()
```

## C API

Все функции доступны с префиксом `SW3`:

```c
SW3_NTSTATUS SW3NtAllocateVirtualMemory(
    SW3_HANDLE ProcessHandle,
    SW3_PPVOID BaseAddress,
    SW3_ULONG_PTR ZeroBits,
    SW3_PSIZE_T RegionSize,
    SW3_ULONG AllocationType,
    SW3_ULONG Protect
);
```

Макросы:
```c
SW3_NT_SUCCESS(status)
SW3_NtCurrentProcess()     // ((SW3_HANDLE)(LONG_PTR)-1)
SW3_NtCurrentThread()      // ((SW3_HANDLE)(LONG_PTR)-2)
```
