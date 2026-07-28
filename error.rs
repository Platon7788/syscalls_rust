//! NTSTATUS error handling for SysWhispers3 syscalls
//!
//! This module provides ergonomic error handling for NT syscall return values.

use core::fmt;

/// NTSTATUS wrapper with error information
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct NtStatus(pub i32);

impl NtStatus {
    /// Create from raw NTSTATUS value
    #[inline]
    pub const fn from_raw(status: i32) -> Self {
        Self(status)
    }

    /// Get raw NTSTATUS value
    #[inline]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Get as unsigned for comparison with hex constants
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0 as u32
    }

    /// Check if status indicates success (NT_SUCCESS macro)
    #[inline]
    pub const fn is_success(self) -> bool {
        self.0 >= 0
    }

    /// Check if status indicates information (severity = 1)
    #[inline]
    pub const fn is_information(self) -> bool {
        (self.as_u32() >> 30) == 1
    }

    /// Check if status indicates warning (severity = 2)
    #[inline]
    pub const fn is_warning(self) -> bool {
        (self.as_u32() >> 30) == 2
    }

    /// Check if status indicates error (severity = 3)
    #[inline]
    pub const fn is_error(self) -> bool {
        (self.as_u32() >> 30) == 3
    }

    /// Get facility code
    #[inline]
    pub const fn facility(self) -> u16 {
        ((self.as_u32() >> 16) & 0x0FFF) as u16
    }

    /// Get status code
    #[inline]
    pub const fn code(self) -> u16 {
        (self.as_u32() & 0xFFFF) as u16
    }

    /// Get human-readable name if known
    pub const fn name(self) -> &'static str {
        match self.as_u32() {
            // Success codes
            0x00000000 => "STATUS_SUCCESS",
            0x00000001 => "STATUS_WAIT_1",
            0x00000002 => "STATUS_WAIT_2",
            0x00000003 => "STATUS_WAIT_3",
            0x0000003F => "STATUS_WAIT_63",
            0x00000080 => "STATUS_ABANDONED",
            0x000000C0 => "STATUS_USER_APC",
            0x00000101 => "STATUS_ALERTED",
            0x00000102 => "STATUS_TIMEOUT",
            0x00000103 => "STATUS_PENDING",
            0x00000104 => "STATUS_REPARSE",
            0x00000105 => "STATUS_MORE_ENTRIES",
            0x00000106 => "STATUS_NOT_ALL_ASSIGNED",
            0x00000107 => "STATUS_SOME_NOT_MAPPED",
            0x00000108 => "STATUS_OPLOCK_BREAK_IN_PROGRESS",
            0x0000010A => "STATUS_NOTIFY_CLEANUP",
            0x0000010B => "STATUS_NOTIFY_ENUM_DIR",
            0x0000010C => "STATUS_NO_QUOTAS_FOR_ACCOUNT",

            // Information codes (0x40xxxxxx)
            0x40000000 => "STATUS_OBJECT_NAME_EXISTS",
            0x40000001 => "STATUS_THREAD_WAS_SUSPENDED",
            0x40000002 => "STATUS_WORKING_SET_LIMIT_RANGE",
            0x40000003 => "STATUS_IMAGE_NOT_AT_BASE",
            0x40000005 => "STATUS_RXACT_STATE_CREATED",
            0x40000006 => "STATUS_SEGMENT_NOTIFICATION",
            0x40000015 => "STATUS_RECEIVE_PARTIAL",
            0x40000016 => "STATUS_RECEIVE_EXPEDITED",
            0x40000017 => "STATUS_RECEIVE_PARTIAL_EXPEDITED",
            0x40000018 => "STATUS_EVENT_DONE",
            0x40000019 => "STATUS_EVENT_PENDING",

            // Warning codes (0x80xxxxxx)
            0x80000001 => "STATUS_GUARD_PAGE_VIOLATION",
            0x80000002 => "STATUS_DATATYPE_MISALIGNMENT",
            0x80000003 => "STATUS_BREAKPOINT",
            0x80000004 => "STATUS_SINGLE_STEP",
            0x80000005 => "STATUS_BUFFER_OVERFLOW",
            0x80000006 => "STATUS_NO_MORE_FILES",
            0x8000000A => "STATUS_HANDLES_CLOSED",
            0x8000000B => "STATUS_NO_INHERITANCE",
            0x8000000D => "STATUS_PARTIAL_COPY",
            0x8000001A => "STATUS_NO_MORE_ENTRIES",
            0x80000288 => "STATUS_DEVICE_BUSY",

            // Error codes (0xC0xxxxxx)
            0xC0000001 => "STATUS_UNSUCCESSFUL",
            0xC0000002 => "STATUS_NOT_IMPLEMENTED",
            0xC0000003 => "STATUS_INVALID_INFO_CLASS",
            0xC0000004 => "STATUS_INFO_LENGTH_MISMATCH",
            0xC0000005 => "STATUS_ACCESS_VIOLATION",
            0xC0000006 => "STATUS_IN_PAGE_ERROR",
            0xC0000007 => "STATUS_PAGEFILE_QUOTA",
            0xC0000008 => "STATUS_INVALID_HANDLE",
            0xC0000009 => "STATUS_BAD_INITIAL_STACK",
            0xC000000A => "STATUS_BAD_INITIAL_PC",
            0xC000000B => "STATUS_INVALID_CID",
            0xC000000C => "STATUS_TIMER_NOT_CANCELED",
            0xC000000D => "STATUS_INVALID_PARAMETER",
            0xC000000E => "STATUS_NO_SUCH_DEVICE",
            0xC000000F => "STATUS_NO_SUCH_FILE",
            0xC0000010 => "STATUS_INVALID_DEVICE_REQUEST",
            0xC0000011 => "STATUS_END_OF_FILE",
            0xC0000012 => "STATUS_WRONG_VOLUME",
            0xC0000013 => "STATUS_NO_MEDIA_IN_DEVICE",
            0xC0000017 => "STATUS_NO_MEMORY",
            0xC0000018 => "STATUS_CONFLICTING_ADDRESSES",
            0xC0000019 => "STATUS_NOT_MAPPED_VIEW",
            0xC000001A => "STATUS_UNABLE_TO_FREE_VM",
            0xC000001B => "STATUS_UNABLE_TO_DELETE_SECTION",
            0xC000001C => "STATUS_INVALID_SYSTEM_SERVICE",
            0xC000001D => "STATUS_ILLEGAL_INSTRUCTION",
            0xC000001E => "STATUS_INVALID_LOCK_SEQUENCE",
            0xC000001F => "STATUS_INVALID_VIEW_SIZE",
            0xC0000020 => "STATUS_INVALID_FILE_FOR_SECTION",
            0xC0000021 => "STATUS_ALREADY_COMMITTED",
            0xC0000022 => "STATUS_ACCESS_DENIED",
            0xC0000023 => "STATUS_BUFFER_TOO_SMALL",
            0xC0000024 => "STATUS_OBJECT_TYPE_MISMATCH",
            0xC0000025 => "STATUS_NONCONTINUABLE_EXCEPTION",
            0xC0000026 => "STATUS_INVALID_DISPOSITION",
            0xC0000027 => "STATUS_UNWIND",
            0xC0000028 => "STATUS_BAD_STACK",
            0xC0000029 => "STATUS_INVALID_UNWIND_TARGET",
            0xC000002A => "STATUS_NOT_LOCKED",
            0xC000002B => "STATUS_PARITY_ERROR",
            0xC000002C => "STATUS_UNABLE_TO_DECOMMIT_VM",
            0xC000002D => "STATUS_NOT_COMMITTED",
            0xC0000030 => "STATUS_INVALID_PARAMETER_MIX",
            0xC0000033 => "STATUS_OBJECT_NAME_INVALID",
            0xC0000034 => "STATUS_OBJECT_NAME_NOT_FOUND",
            0xC0000035 => "STATUS_OBJECT_NAME_COLLISION",
            0xC0000037 => "STATUS_PORT_DISCONNECTED",
            0xC0000039 => "STATUS_OBJECT_PATH_INVALID",
            0xC000003A => "STATUS_OBJECT_PATH_NOT_FOUND",
            0xC000003B => "STATUS_OBJECT_PATH_SYNTAX_BAD",
            0xC000003C => "STATUS_DATA_OVERRUN",
            0xC000003D => "STATUS_DATA_LATE_ERROR",
            0xC000003E => "STATUS_DATA_ERROR",
            0xC000003F => "STATUS_CRC_ERROR",
            0xC0000040 => "STATUS_SECTION_TOO_BIG",
            0xC0000041 => "STATUS_PORT_CONNECTION_REFUSED",
            0xC0000042 => "STATUS_INVALID_PORT_HANDLE",
            0xC0000043 => "STATUS_SHARING_VIOLATION",
            0xC0000044 => "STATUS_QUOTA_EXCEEDED",
            0xC0000045 => "STATUS_INVALID_PAGE_PROTECTION",
            0xC0000046 => "STATUS_MUTANT_NOT_OWNED",
            0xC0000047 => "STATUS_SEMAPHORE_LIMIT_EXCEEDED",
            0xC0000048 => "STATUS_PORT_ALREADY_SET",
            0xC0000049 => "STATUS_SECTION_NOT_IMAGE",
            0xC000004A => "STATUS_SUSPEND_COUNT_EXCEEDED",
            0xC000004B => "STATUS_THREAD_IS_TERMINATING",
            0xC000004C => "STATUS_BAD_WORKING_SET_LIMIT",
            0xC000004D => "STATUS_INCOMPATIBLE_FILE_MAP",
            0xC000004E => "STATUS_SECTION_PROTECTION",
            0xC0000050 => "STATUS_FILE_LOCK_CONFLICT",
            0xC0000051 => "STATUS_LOCK_NOT_GRANTED",
            0xC0000052 => "STATUS_DELETE_PENDING",
            0xC0000054 => "STATUS_FILE_IS_A_DIRECTORY",
            0xC0000055 => "STATUS_NOT_A_DIRECTORY",
            0xC0000056 => "STATUS_PROCESS_IS_TERMINATING",
            0xC0000061 => "STATUS_PRIVILEGE_NOT_HELD",
            0xC0000062 => "STATUS_INVALID_ACCOUNT_NAME",
            0xC0000063 => "STATUS_USER_EXISTS",
            0xC0000064 => "STATUS_NO_SUCH_USER",
            0xC0000065 => "STATUS_GROUP_EXISTS",
            0xC0000066 => "STATUS_NO_SUCH_GROUP",
            0xC0000067 => "STATUS_MEMBER_IN_GROUP",
            0xC0000068 => "STATUS_MEMBER_NOT_IN_GROUP",
            0xC0000069 => "STATUS_LAST_ADMIN",
            0xC000006A => "STATUS_WRONG_PASSWORD",
            0xC000006B => "STATUS_ILL_FORMED_PASSWORD",
            0xC000006C => "STATUS_PASSWORD_RESTRICTION",
            0xC000006D => "STATUS_LOGON_FAILURE",
            0xC000006E => "STATUS_ACCOUNT_RESTRICTION",
            0xC000006F => "STATUS_INVALID_LOGON_HOURS",
            0xC0000070 => "STATUS_INVALID_WORKSTATION",
            0xC0000071 => "STATUS_PASSWORD_EXPIRED",
            0xC0000072 => "STATUS_ACCOUNT_DISABLED",
            0xC000007A => "STATUS_PROCEDURE_NOT_FOUND",
            0xC000007B => "STATUS_INVALID_IMAGE_FORMAT",
            0xC000007C => "STATUS_NO_TOKEN",
            0xC000007D => "STATUS_BAD_INHERITANCE_ACL",
            0xC000007E => "STATUS_RANGE_NOT_LOCKED",
            0xC000007F => "STATUS_DISK_FULL",
            0xC0000080 => "STATUS_SERVER_DISABLED",
            0xC0000081 => "STATUS_SERVER_NOT_DISABLED",
            0xC0000082 => "STATUS_TOO_MANY_GUIDS_REQUESTED",
            0xC0000083 => "STATUS_GUIDS_EXHAUSTED",
            0xC0000084 => "STATUS_INVALID_ID_AUTHORITY",
            0xC0000085 => "STATUS_AGENTS_EXHAUSTED",
            0xC0000086 => "STATUS_INVALID_VOLUME_LABEL",
            0xC0000087 => "STATUS_SECTION_NOT_EXTENDED",
            0xC0000088 => "STATUS_NOT_MAPPED_DATA",
            0xC0000089 => "STATUS_RESOURCE_DATA_NOT_FOUND",
            0xC000008A => "STATUS_RESOURCE_TYPE_NOT_FOUND",
            0xC000008B => "STATUS_RESOURCE_NAME_NOT_FOUND",
            0xC000009A => "STATUS_INSUFFICIENT_RESOURCES",
            0xC000009B => "STATUS_DFS_EXIT_PATH_FOUND",
            0xC000009C => "STATUS_DEVICE_DATA_ERROR",
            0xC000009D => "STATUS_DEVICE_NOT_CONNECTED",
            0xC00000A2 => "STATUS_MEDIA_WRITE_PROTECTED",
            0xC00000A3 => "STATUS_BAD_IMPERSONATION_LEVEL",
            0xC00000A4 => "STATUS_CANT_OPEN_ANONYMOUS",
            0xC00000A5 => "STATUS_BAD_VALIDATION_CLASS",
            0xC00000A6 => "STATUS_BAD_TOKEN_TYPE",
            0xC00000AB => "STATUS_INSTANCE_NOT_AVAILABLE",
            0xC00000AC => "STATUS_PIPE_NOT_AVAILABLE",
            0xC00000AD => "STATUS_INVALID_PIPE_STATE",
            0xC00000AE => "STATUS_PIPE_BUSY",
            0xC00000AF => "STATUS_ILLEGAL_FUNCTION",
            0xC00000B0 => "STATUS_PIPE_DISCONNECTED",
            0xC00000B1 => "STATUS_PIPE_CLOSING",
            0xC00000B2 => "STATUS_PIPE_CONNECTED",
            0xC00000B3 => "STATUS_PIPE_LISTENING",
            0xC00000B4 => "STATUS_INVALID_READ_MODE",
            0xC00000B5 => "STATUS_IO_TIMEOUT",
            0xC00000BA => "STATUS_FILE_IS_OFFLINE",
            0xC00000BB => "STATUS_NOT_A_REPARSE_POINT",
            0xC00000E5 => "STATUS_INTERNAL_ERROR",
            0xC0000120 => "STATUS_CANCELLED",
            0xC0000121 => "STATUS_CANNOT_DELETE",
            0xC0000128 => "STATUS_FILE_DELETED",
            0xC0000135 => "STATUS_DLL_NOT_FOUND",
            0xC0000138 => "STATUS_ENTRYPOINT_NOT_FOUND",
            0xC0000139 => "STATUS_DLL_INIT_FAILED",
            0xC0000142 => "STATUS_DLL_INIT_FAILED_LOGOFF",
            0xC0000194 => "STATUS_POSSIBLE_DEADLOCK",
            0xC0000225 => "STATUS_NOT_FOUND",
            0xC000022D => "STATUS_RETRY",
            0xC0000263 => "STATUS_USER_MAPPED_FILE",

            _ => "STATUS_UNKNOWN",
        }
    }
}

impl From<i32> for NtStatus {
    #[inline]
    fn from(status: i32) -> Self {
        Self(status)
    }
}

impl From<NtStatus> for i32 {
    #[inline]
    fn from(status: NtStatus) -> Self {
        status.0
    }
}

impl fmt::Debug for NtStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NtStatus(0x{:08X} = {})", self.as_u32(), self.name())
    }
}

impl fmt::Display for NtStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (0x{:08X})", self.name(), self.as_u32())
    }
}

/// Result type for syscall operations
pub type NtResult<T> = Result<T, NtStatus>;

/// Extension trait for converting NTSTATUS to Result
pub trait NtStatusExt {
    /// Convert to Result, treating negative values as errors
    fn to_result(self) -> NtResult<()>;

    /// Convert to Result with a success value
    fn to_result_with<T>(self, value: T) -> NtResult<T>;
}

impl NtStatusExt for i32 {
    #[inline]
    fn to_result(self) -> NtResult<()> {
        let status = NtStatus(self);
        if status.is_success() {
            Ok(())
        } else {
            Err(status)
        }
    }

    #[inline]
    fn to_result_with<T>(self, value: T) -> NtResult<T> {
        let status = NtStatus(self);
        if status.is_success() {
            Ok(value)
        } else {
            Err(status)
        }
    }
}

// NOTE: raw i32 STATUS_* constants live in lib.rs (the NTSTATUS surface).
// Callers wanting the NtStatus wrapper build one via NtStatus(super::STATUS_*).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_success() {
        let status = NtStatus::from_raw(0);
        assert!(status.is_success());
        assert!(!status.is_error());
        assert_eq!(status.name(), "STATUS_SUCCESS");
    }

    #[test]
    fn test_status_error() {
        let status = NtStatus::from_raw(0xC0000005u32 as i32);
        assert!(!status.is_success());
        assert!(status.is_error());
        assert_eq!(status.name(), "STATUS_ACCESS_VIOLATION");
    }

    #[test]
    fn test_to_result() {
        assert!(0i32.to_result().is_ok());
        assert!((0xC0000005u32 as i32).to_result().is_err());
    }
}
