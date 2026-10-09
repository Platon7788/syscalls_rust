/* Compile-only SDK contract check: no runtime or syscall stubs are linked. */
#include <windows.h>
#include <winternl.h>
#include <stddef.h>
#include "syscalls.h"

#ifdef __cplusplus
#define ASSERT static_assert
#define ALIGNOF alignof
#else
#define ASSERT _Static_assert
#define ALIGNOF _Alignof
#endif
#if defined(_M_X64)
#include "layout-x64.h"
#else
#include "layout-x86.h"
#endif

#define SDK_TYPE(type) \
    ASSERT(sizeof(X_##type) == sizeof(type), #type " SDK size"); \
    ASSERT(ALIGNOF(X_##type) == ALIGNOF(type), #type " SDK alignment")
#define SDK_FIELD(type, field) \
    ASSERT(offsetof(X_##type, field) == offsetof(type, field), #type "." #field " SDK offset")
SDK_TYPE(UNICODE_STRING);
SDK_FIELD(UNICODE_STRING, Length);
SDK_FIELD(UNICODE_STRING, MaximumLength);
SDK_FIELD(UNICODE_STRING, Buffer);
SDK_TYPE(OBJECT_ATTRIBUTES);
SDK_FIELD(OBJECT_ATTRIBUTES, Length);
SDK_FIELD(OBJECT_ATTRIBUTES, RootDirectory);
SDK_FIELD(OBJECT_ATTRIBUTES, ObjectName);
SDK_FIELD(OBJECT_ATTRIBUTES, Attributes);
SDK_FIELD(OBJECT_ATTRIBUTES, SecurityDescriptor);
SDK_FIELD(OBJECT_ATTRIBUTES, SecurityQualityOfService);
SDK_TYPE(IO_STATUS_BLOCK);
SDK_FIELD(IO_STATUS_BLOCK, Status);
SDK_FIELD(IO_STATUS_BLOCK, Information);
SDK_TYPE(GENERIC_MAPPING);
SDK_FIELD(GENERIC_MAPPING, GenericRead);
SDK_FIELD(GENERIC_MAPPING, GenericWrite);
SDK_FIELD(GENERIC_MAPPING, GenericExecute);
SDK_FIELD(GENERIC_MAPPING, GenericAll);
SDK_TYPE(LUID);
SDK_FIELD(LUID, LowPart);
SDK_FIELD(LUID, HighPart);
SDK_TYPE(LUID_AND_ATTRIBUTES);
SDK_FIELD(LUID_AND_ATTRIBUTES, Luid);
SDK_FIELD(LUID_AND_ATTRIBUTES, Attributes);
SDK_TYPE(PRIVILEGE_SET);
SDK_FIELD(PRIVILEGE_SET, PrivilegeCount);
SDK_FIELD(PRIVILEGE_SET, Control);
SDK_FIELD(PRIVILEGE_SET, Privilege);
SDK_TYPE(TOKEN_PRIVILEGES);
SDK_FIELD(TOKEN_PRIVILEGES, PrivilegeCount);
SDK_FIELD(TOKEN_PRIVILEGES, Privileges);
SDK_TYPE(SID_AND_ATTRIBUTES);
SDK_FIELD(SID_AND_ATTRIBUTES, Sid);
SDK_FIELD(SID_AND_ATTRIBUTES, Attributes);
SDK_TYPE(TOKEN_GROUPS);
SDK_FIELD(TOKEN_GROUPS, GroupCount);
SDK_FIELD(TOKEN_GROUPS, Groups);
SDK_TYPE(SECURITY_QUALITY_OF_SERVICE);
SDK_FIELD(SECURITY_QUALITY_OF_SERVICE, Length);
SDK_FIELD(SECURITY_QUALITY_OF_SERVICE, ImpersonationLevel);
SDK_FIELD(SECURITY_QUALITY_OF_SERVICE, ContextTrackingMode);
SDK_FIELD(SECURITY_QUALITY_OF_SERVICE, EffectiveOnly);

ASSERT(X_STANDARD_RIGHTS_READ == X_READ_CONTROL, "read rights alias");
ASSERT(X_STANDARD_RIGHTS_WRITE == X_READ_CONTROL, "write rights alias");
ASSERT(X_STANDARD_RIGHTS_EXECUTE == X_READ_CONTROL, "execute rights alias");

#define CHECK_LAYOUT(field) \
    ASSERT(offsetof(X_MEMORY_BASIC_INFORMATION, field) == \
                   offsetof(MEMORY_BASIC_INFORMATION, field), #field " offset")

ASSERT(sizeof(X_MEMORY_BASIC_INFORMATION) ==
               sizeof(MEMORY_BASIC_INFORMATION), "memory information size");
ASSERT(ALIGNOF(X_MEMORY_BASIC_INFORMATION) ==
               ALIGNOF(MEMORY_BASIC_INFORMATION), "memory information alignment");
CHECK_LAYOUT(BaseAddress);
CHECK_LAYOUT(AllocationBase);
CHECK_LAYOUT(AllocationProtect);
#if defined(_M_X64)
CHECK_LAYOUT(PartitionId);
#endif
CHECK_LAYOUT(RegionSize);
CHECK_LAYOUT(State);
CHECK_LAYOUT(Protect);
CHECK_LAYOUT(Type);
