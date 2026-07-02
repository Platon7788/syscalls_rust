//! Build script for auto-generating C headers from syscalls library
//!
//! Parses syscalls/lib.rs and generates:
//! - Type definitions
//! - Constants
//! - Function declarations
//!
//! Works on stable Rust without cbindgen's parse.expand

use std::collections::HashSet;
use std::fs;
use std::io::Write;

fn main() {
    let syscalls_lib = "../lib.rs";
    let output_header = "include/syscalls.h";
    let output_wrappers = format!("{}/c_wrappers.rs", std::env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", syscalls_lib);
    println!("cargo:rerun-if-changed=build.rs");

    let content = fs::read_to_string(syscalls_lib).expect("Failed to read syscalls/lib.rs");

    let types = extract_type_aliases(&content);
    let structs = extract_structs(&content);
    let constants = extract_constants(&content);
    let functions = extract_functions(&content);

    let header = generate_header(&types, &structs, &constants, &functions);
    let wrappers = generate_c_wrappers(&functions);

    fs::create_dir_all("include").ok();
    let mut file = fs::File::create(output_header).expect("Failed to create header file");
    file.write_all(header.as_bytes())
        .expect("Failed to write header");

    let mut wrapper_file =
        fs::File::create(&output_wrappers).expect("Failed to create wrapper file");
    wrapper_file
        .write_all(wrappers.as_bytes())
        .expect("Failed to write wrappers");

    eprintln!(
        "Generated {} with {} functions",
        output_header,
        functions.len()
    );
}

/// Rust to C type mapping with SW3_ prefixes
fn rust_to_c_type(rust_type: &str) -> String {
    let t = rust_type.trim();
    match t {
        // Primitives
        "i8" => "int8_t".to_string(),
        "i16" => "int16_t".to_string(),
        "i32" => "int32_t".to_string(),
        "i64" => "int64_t".to_string(),
        "u8" => "uint8_t".to_string(),
        "u16" => "uint16_t".to_string(),
        "u32" => "uint32_t".to_string(),
        "u64" => "uint64_t".to_string(),
        "usize" => "size_t".to_string(),
        "isize" => "intptr_t".to_string(),
        "bool" => "bool".to_string(),
        "c_void" | "core::ffi::c_void" => "void".to_string(),

        // Pointer types
        s if s.starts_with("*mut ") => {
            let inner = &s[5..];
            format!("{}*", rust_to_c_type(inner))
        }
        s if s.starts_with("*const ") => {
            let inner = &s[7..];
            format!("const {}*", rust_to_c_type(inner))
        }

        // Option<fn> -> function pointer (simplified to void*)
        s if s.starts_with("Option<") => "void*".to_string(),

        // Handle array types [Type; count] -> Type[count]
        s if s.starts_with("[") && s.contains(";") && s.ends_with("]") => {
            let inner = &s[1..s.len() - 1];
            let parts: Vec<&str> = inner.split(';').collect();
            if parts.len() == 2 {
                let typ = parts[0].trim();
                let count = parts[1].trim();
                format!("{}[{}]", rust_to_c_type(typ), count)
            } else {
                "void*".to_string()
            }
        }

        // Handle enum types that cause problems
        s if s.contains("enum ") => "uint32_t".to_string(),

        // Handle struct types that cause problems
        s if s.contains("struct ") => "void*".to_string(),

        // Map NT types to SW3_ prefixed types
        "HANDLE" => "SW3_HANDLE".to_string(),
        "PHANDLE" => "SW3_PHANDLE".to_string(),
        "PVOID" => "SW3_PVOID".to_string(),
        "PPVOID" => "SW3_PPVOID".to_string(),
        "LPCVOID" => "SW3_LPCVOID".to_string(),
        "NTSTATUS" => "SW3_NTSTATUS".to_string(),
        "BOOL" => "SW3_BOOL".to_string(),
        "BOOLEAN" => "SW3_BOOLEAN".to_string(),
        "PBOOLEAN" => "SW3_PBOOLEAN".to_string(),
        "UCHAR" => "SW3_UCHAR".to_string(),
        "PUCHAR" => "SW3_PUCHAR".to_string(),
        "CHAR" => "SW3_CHAR".to_string(),
        "PCHAR" => "SW3_PCHAR".to_string(),
        "WCHAR" => "SW3_WCHAR".to_string(),
        "PWCHAR" => "SW3_PWCHAR".to_string(),
        "PWSTR" => "SW3_PWSTR".to_string(),
        "PCWSTR" => "SW3_PCWSTR".to_string(),
        "PSTR" => "SW3_PSTR".to_string(),
        "PCSTR" => "SW3_PCSTR".to_string(),
        "USHORT" => "SW3_USHORT".to_string(),
        "PUSHORT" => "SW3_PUSHORT".to_string(),
        "ULONG" => "SW3_ULONG".to_string(),
        "PULONG" => "SW3_PULONG".to_string(),
        "ULONG64" => "SW3_ULONG64".to_string(),
        "ULONGLONG" => "SW3_ULONGLONG".to_string(),
        "DWORD" => "SW3_DWORD".to_string(),
        "PDWORD" => "SW3_PDWORD".to_string(),
        "WORD" => "SW3_WORD".to_string(),
        "PWORD" => "SW3_PWORD".to_string(),
        "SHORT" => "SW3_SHORT".to_string(),
        "PSHORT" => "SW3_PSHORT".to_string(),
        "LONG" => "SW3_LONG".to_string(),
        "PLONG" => "SW3_PLONG".to_string(),
        "LONGLONG" => "SW3_LONGLONG".to_string(),
        "SIZE_T" => "SW3_SIZE_T".to_string(),
        "PSIZE_T" => "SW3_PSIZE_T".to_string(),
        "SSIZE_T" => "SW3_SSIZE_T".to_string(),
        "ULONG_PTR" => "SW3_ULONG_PTR".to_string(),
        "PULONG_PTR" => "SW3_PULONG_PTR".to_string(),
        "LONG_PTR" => "SW3_LONG_PTR".to_string(),
        "DWORD_PTR" => "SW3_DWORD_PTR".to_string(),
        "ACCESS_MASK" => "SW3_ACCESS_MASK".to_string(),
        "PACCESS_MASK" => "SW3_PACCESS_MASK".to_string(),
        "LARGE_INTEGER" => "SW3_LARGE_INTEGER".to_string(),
        "PLARGE_INTEGER" => "SW3_PLARGE_INTEGER".to_string(),
        "ULARGE_INTEGER" => "SW3_ULARGE_INTEGER".to_string(),
        "PULARGE_INTEGER" => "SW3_PULARGE_INTEGER".to_string(),
        "LCID" => "SW3_LCID".to_string(),
        "LANGID" => "SW3_LANGID".to_string(),
        "KAFFINITY" => "SW3_KAFFINITY".to_string(),
        "KPRIORITY" => "SW3_KPRIORITY".to_string(),
        "KIRQL" => "SW3_KIRQL".to_string(),
        "CCHAR" => "SW3_CCHAR".to_string(),
        "BYTE" => "SW3_BYTE".to_string(),
        "PBYTE" => "SW3_PBYTE".to_string(),
        "LUID" => "SW3_LUID".to_string(),
        "PLUID" => "SW3_PLUID".to_string(),

        // Map structure types to SW3_ prefixed types
        "UNICODE_STRING" => "SW3_UNICODE_STRING".to_string(),
        "PUNICODE_STRING" => "SW3_PUNICODE_STRING".to_string(),
        "OBJECT_ATTRIBUTES" => "SW3_OBJECT_ATTRIBUTES".to_string(),
        "POBJECT_ATTRIBUTES" => "SW3_POBJECT_ATTRIBUTES".to_string(),
        "IO_STATUS_BLOCK" => "SW3_IO_STATUS_BLOCK".to_string(),
        "PIO_STATUS_BLOCK" => "SW3_PIO_STATUS_BLOCK".to_string(),
        "CLIENT_ID" => "SW3_CLIENT_ID".to_string(),
        "PCLIENT_ID" => "SW3_PCLIENT_ID".to_string(),
        "GENERIC_MAPPING" => "SW3_GENERIC_MAPPING".to_string(),
        "PGENERIC_MAPPING" => "SW3_PGENERIC_MAPPING".to_string(),
        "LUID_AND_ATTRIBUTES" => "SW3_LUID_AND_ATTRIBUTES".to_string(),
        "PLUID_AND_ATTRIBUTES" => "SW3_PLUID_AND_ATTRIBUTES".to_string(),
        "PRIVILEGE_SET" => "SW3_PRIVILEGE_SET".to_string(),
        "PPRIVILEGE_SET" => "SW3_PPRIVILEGE_SET".to_string(),
        "TOKEN_PRIVILEGES" => "SW3_TOKEN_PRIVILEGES".to_string(),
        "PTOKEN_PRIVILEGES" => "SW3_PTOKEN_PRIVILEGES".to_string(),
        "SID_AND_ATTRIBUTES" => "SW3_SID_AND_ATTRIBUTES".to_string(),
        "PSID_AND_ATTRIBUTES" => "SW3_PSID_AND_ATTRIBUTES".to_string(),
        "TOKEN_GROUPS" => "SW3_TOKEN_GROUPS".to_string(),
        "PTOKEN_GROUPS" => "SW3_PTOKEN_GROUPS".to_string(),
        "SECURITY_QUALITY_OF_SERVICE" => "SW3_SECURITY_QUALITY_OF_SERVICE".to_string(),
        "PSECURITY_QUALITY_OF_SERVICE" => "SW3_PSECURITY_QUALITY_OF_SERVICE".to_string(),
        "INITIAL_TEB" => "SW3_INITIAL_TEB".to_string(),
        "PINITIAL_TEB" => "SW3_PINITIAL_TEB".to_string(),
        "PS_ATTRIBUTE" => "SW3_PS_ATTRIBUTE".to_string(),
        "PPS_ATTRIBUTE" => "SW3_PPS_ATTRIBUTE".to_string(),
        "PS_ATTRIBUTE_LIST" => "SW3_PS_ATTRIBUTE_LIST".to_string(),
        "PPS_ATTRIBUTE_LIST" => "SW3_PPS_ATTRIBUTE_LIST".to_string(),
        "PS_CREATE_INFO" => "SW3_PS_CREATE_INFO".to_string(),
        "PPS_CREATE_INFO" => "SW3_PPS_CREATE_INFO".to_string(),
        "FILE_BASIC_INFORMATION" => "SW3_FILE_BASIC_INFORMATION".to_string(),
        "PFILE_BASIC_INFORMATION" => "SW3_PFILE_BASIC_INFORMATION".to_string(),
        "FILE_NETWORK_OPEN_INFORMATION" => "SW3_FILE_NETWORK_OPEN_INFORMATION".to_string(),
        "PFILE_NETWORK_OPEN_INFORMATION" => "SW3_PFILE_NETWORK_OPEN_INFORMATION".to_string(),
        "MEMORY_BASIC_INFORMATION" => "SW3_MEMORY_BASIC_INFORMATION".to_string(),
        "PMEMORY_BASIC_INFORMATION" => "SW3_PMEMORY_BASIC_INFORMATION".to_string(),
        "MEMORY_RANGE_ENTRY" => "SW3_MEMORY_RANGE_ENTRY".to_string(),
        "PMEMORY_RANGE_ENTRY" => "SW3_PMEMORY_RANGE_ENTRY".to_string(),
        "T2_SET_PARAMETERS" => "SW3_T2_SET_PARAMETERS".to_string(),
        "PT2_SET_PARAMETERS" => "SW3_PT2_SET_PARAMETERS".to_string(),
        "PORT_MESSAGE" => "SW3_PORT_MESSAGE".to_string(),
        "PPORT_MESSAGE" => "SW3_PPORT_MESSAGE".to_string(),
        "KEY_VALUE_ENTRY" => "SW3_KEY_VALUE".to_string(),
        "PKEY_VALUE_ENTRY" => "SW3_PKEY_VALUE_ENTRY".to_string(),
        "WNF_STATE_NAME" => "SW3_WNF_STATE_NAME".to_string(),
        "PWNF_STATE_NAME" => "SW3_PWNF_STATE_NAME".to_string(),
        "PCWNF_STATE_NAME" => "SW3_PWNF_STATE_NAME".to_string(),
        "WNF_TYPE_ID" => "SW3_WNF_TYPE_ID".to_string(),
        "PWNF_TYPE_ID" => "SW3_PWNF_TYPE_ID".to_string(),
        "PCWNF_TYPE_ID" => "SW3_PWNF_TYPE_ID".to_string(),
        "ALPC_PORT_ATTRIBUTES" => "SW3_ALPC_PORT_ATTRIBUTES".to_string(),
        "PALPC_PORT_ATTRIBUTES" => "SW3_PALPC_PORT_ATTRIBUTES".to_string(),
        "THREAD_BASIC_INFORMATION" => "SW3_THREAD_BASIC_INFORMATION".to_string(),
        "PTHREAD_BASIC_INFORMATION" => "SW3_PTHREAD_BASIC_INFORMATION".to_string(),
        "PROCESS_BASIC_INFORMATION" => "SW3_PROCESS_BASIC_INFORMATION".to_string(),
        "PPROCESS_BASIC_INFORMATION" => "SW3_PPROCESS_BASIC_INFORMATION".to_string(),
        "TOKEN_STATISTICS" => "SW3_TOKEN_STATISTICS".to_string(),
        "PTOKEN_STATISTICS" => "SW3_PTOKEN_STATISTICS".to_string(),
        "PSID" => "SW3_PSID".to_string(),
        "PSECURITY_DESCRIPTOR" => "SW3_PSECURITY_DESCRIPTOR".to_string(),

        // Map opaque pointer types
        "PPORT_SECTION_WRITE" => "SW3_PPORT_SECTION_WRITE".to_string(),
        "PPORT_SECTION_READ" => "SW3_PPORT_SECTION_READ".to_string(),
        "PRTL_ATOM" => "SW3_PRTL_ATOM".to_string(),
        "RTL_ATOM" => "SW3_RTL_ATOM".to_string(), // uint16_t, not void*
        "PALPC_MESSAGE_ATTRIBUTES" => "SW3_PALPC_MESSAGE_ATTRIBUTES".to_string(),
        "PALPC_CONTEXT_ATTR" => "SW3_PALPC_CONTEXT_ATTR".to_string(),
        "PALPC_DATA_VIEW_ATTR" => "SW3_PALPC_DATA_VIEW_ATTR".to_string(),
        "PALPC_SECURITY_ATTR" => "SW3_PALPC_SECURITY_ATTR".to_string(),
        "PKNORMAL_ROUTINE" => "SW3_PKNORMAL_ROUTINE".to_string(),
        "PTIMER_APC_ROUTINE" => "SW3_PTIMER_APC_ROUTINE".to_string(),
        "PBOOT_OPTIONS" => "SW3_PBOOT_OPTIONS".to_string(),
        "PFILE_PATH" => "SW3_PFILE_PATH".to_string(),
        "PDBGUI_WAIT_STATE_CHANGE" => "SW3_PDBGUI_WAIT_STATE_CHANGE".to_string(),
        "PWNF_DELIVERY_DESCRIPTOR" => "SW3_PWNF_DELIVERY_DESCRIPTOR".to_string(),
        "PPLUGPLAY_EVENT_BLOCK" => "SW3_PPLUGPLAY_EVENT_BLOCK".to_string(),
        "PFILE_SEGMENT_ELEMENT" => "SW3_PFILE_SEGMENT_ELEMENT".to_string(),
        "PTOKEN_USER" => "SW3_PTOKEN_USER".to_string(),
        "PTOKEN_OWNER" => "SW3_PTOKEN_OWNER".to_string(),
        "PTOKEN_PRIMARY_GROUP" => "SW3_PTOKEN_PRIMARY_GROUP".to_string(),
        "PTOKEN_DEFAULT_DACL" => "SW3_PTOKEN_DEFAULT_DACL".to_string(),
        "PTOKEN_SOURCE" => "SW3_PTOKEN_SOURCE".to_string(),
        "PFILE_FULL_EA_INFORMATION" => "SW3_PFILE_FULL_EA_INFORMATION".to_string(),
        "PFILE_GET_EA_INFORMATION" => "SW3_PFILE_GET_EA_INFORMATION".to_string(),
        "PFILE_USER_QUOTA_INFORMATION" => "SW3_PFILE_USER_QUOTA_INFORMATION".to_string(),
        "PFILE_QUOTA_LIST_INFORMATION" => "SW3_PFILE_QUOTA_LIST_INFORMATION".to_string(),
        "PFILE_IO_COMPLETION_INFORMATION" => "SW3_PFILE_IO_COMPLETION_INFORMATION".to_string(),
        "PIO_APC_ROUTINE" => "SW3_PIO_APC_ROUTINE".to_string(),

        // Keep other types as-is
        _ => t.to_string(),
    }
}

/// Extract type aliases: pub type NAME = TYPE;
fn extract_type_aliases(content: &str) -> Vec<(String, String)> {
    let mut types = Vec::new();
    let re = regex::Regex::new(r"pub type (\w+)\s*=\s*([^;]+);").unwrap();

    for cap in re.captures_iter(content) {
        let name = cap[1].to_string();
        let rust_type = cap[2].trim().to_string();
        types.push((name, rust_type));
    }
    types
}

/// Extract pub const declarations
fn extract_constants(content: &str) -> Vec<(String, String, String)> {
    let mut constants = Vec::new();
    let re = regex::Regex::new(r"pub const (\w+):\s*(\w+)\s*=\s*([^;]+);").unwrap();

    for cap in re.captures_iter(content) {
        let name = cap[1].to_string();
        let typ = cap[2].to_string();
        let value = cap[3].trim().to_string();
        constants.push((name, typ, value));
    }
    constants
}

/// Extract struct definitions
fn extract_structs(content: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut structs = Vec::new();

    // Match #[repr(C)] pub struct NAME { fields }
    let struct_re = regex::Regex::new(
        r#"#\[repr\(C\)\]\s*(?:#\[derive[^\]]*\]\s*)*pub struct (\w+)\s*\{([^}]*)\}"#,
    )
    .unwrap();

    let field_re = regex::Regex::new(r"pub (\w+):\s*([^,\n]+)").unwrap();

    for cap in struct_re.captures_iter(content) {
        let name = cap[1].to_string();
        let body = &cap[2];

        let mut fields = Vec::new();
        for field_cap in field_re.captures_iter(body) {
            let field_name = field_cap[1].to_string();
            let field_type = field_cap[2].trim().trim_end_matches(',').to_string();
            fields.push((field_name, field_type));
        }

        if !fields.is_empty() {
            structs.push((name, fields));
        }
    }
    structs
}

/// Parameter info
#[derive(Debug)]
struct Param {
    name: String,
    typ: String,
}

/// Function info
#[derive(Debug)]
struct Function {
    name: String,
    params: Vec<Param>,
    return_type: String,
}

/// Extract function declarations
fn extract_functions(content: &str) -> Vec<Function> {
    let mut functions = Vec::new();
    let mut seen = HashSet::new();

    // Match: pub unsafe extern "C" fn name(params) -> RetType {
    // or: pub unsafe fn name(params) -> RetType {
    let fn_re =
        regex::Regex::new(r#"pub unsafe (?:extern "C" )?fn (\w+)\s*\(([^)]*)\)\s*->\s*(\w+)"#)
            .unwrap();

    for cap in fn_re.captures_iter(content) {
        let name = cap[1].to_string();

        // Skip duplicates (x86 and x86_64 versions)
        if seen.contains(&name) {
            continue;
        }
        seen.insert(name.clone());

        let params_str = &cap[2];
        let return_type = cap[3].to_string();

        let params = parse_params(params_str);

        functions.push(Function {
            name,
            params,
            return_type,
        });
    }

    functions
}

/// Parse function parameters
fn parse_params(params_str: &str) -> Vec<Param> {
    let mut params = Vec::new();

    if params_str.trim().is_empty() {
        return params;
    }

    // Split by comma, handling nested generics
    let mut current = String::new();
    let mut depth = 0;

    for ch in params_str.chars() {
        match ch {
            '<' | '(' => {
                depth += 1;
                current.push(ch);
            }
            '>' | ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                if let Some(p) = parse_single_param(&current) {
                    params.push(p);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    // Last param
    if let Some(p) = parse_single_param(&current) {
        params.push(p);
    }

    params
}

/// Parse single parameter "name: Type"
fn parse_single_param(s: &str) -> Option<Param> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    let name = parts[0].trim();
    let typ = parts[1].trim();

    // Skip parameters with problematic names or types
    if name.is_empty() || typ.is_empty() {
        return None;
    }

    // Skip parameters that cause "anonymous class" errors
    if name.starts_with("_") || typ.contains("enum ") || typ.contains("struct ") {
        return None;
    }

    Some(Param {
        name: sanitize_param_name(name),
        typ: typ.to_string(),
    })
}

/// Sanitize parameter names to avoid C++ keyword conflicts
fn sanitize_param_name(name: &str) -> String {
    match name {
        // C++ keywords that cause compilation errors
        "class" => "class_name".to_string(),
        "namespace" => "namespace_name".to_string(),
        "template" => "template_name".to_string(),
        "typename" => "typename_name".to_string(),
        "operator" => "operator_name".to_string(),
        "public" => "public_access".to_string(),
        "private" => "private_access".to_string(),
        "protected" => "protected_access".to_string(),
        "virtual" => "virtual_flag".to_string(),
        "static" => "static_flag".to_string(),
        "const" => "const_flag".to_string(),
        "volatile" => "volatile_flag".to_string(),
        "mutable" => "mutable_flag".to_string(),
        "explicit" => "explicit_flag".to_string(),
        "inline" => "inline_flag".to_string(),
        "friend" => "friend_access".to_string(),
        "using" => "using_directive".to_string(),
        "try" => "try_block".to_string(),
        "catch" => "catch_block".to_string(),
        "throw" => "throw_exception".to_string(),
        "new" => "new_operator".to_string(),
        "delete" => "delete_operator".to_string(),
        "this" => "this_pointer".to_string(),
        "true" => "true_value".to_string(),
        "false" => "false_value".to_string(),
        // Additional problematic names
        "and" => "and_operator".to_string(),
        "or" => "or_operator".to_string(),
        "not" => "not_operator".to_string(),
        "xor" => "xor_operator".to_string(),
        "bitand" => "bitand_operator".to_string(),
        "bitor" => "bitor_operator".to_string(),
        "compl" => "compl_operator".to_string(),
        "and_eq" => "and_eq_operator".to_string(),
        "or_eq" => "or_eq_operator".to_string(),
        "xor_eq" => "xor_eq_operator".to_string(),
        "not_eq" => "not_eq_operator".to_string(),
        _ => name.to_string(),
    }
}

/// Convert snake_case to PascalCase (nt_allocate_virtual_memory -> NtAllocateVirtualMemory)
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Generate the complete C header
fn generate_header(
    _types: &[(String, String)],
    structs: &[(String, Vec<(String, String)>)],
    _constants: &[(String, String, String)],
    functions: &[Function],
) -> String {
    let mut h = String::new();

    // Header guard and includes
    h.push_str(HEADER_PREAMBLE);

    // Base types - always available
    h.push_str("\n/* ==================== Base Type Definitions ==================== */\n\n");
    generate_base_types(&mut h);

    // Opaque pointer types - always available
    h.push_str("\n/* ==================== Opaque Pointer Types ==================== */\n\n");
    generate_opaque_types(&mut h);

    // Structures - conditional
    h.push_str("\n/* ==================== Structures ==================== */\n\n");
    generate_structs(&mut h, structs);

    // Constants and NTSTATUS codes - always available
    h.push_str("\n/* ==================== Constants and Status Codes ==================== */\n\n");
    generate_all_constants(&mut h, _constants);

    // Functions - all 519 syscalls
    h.push_str("\n/* ==================== Syscall Functions ==================== */\n\n");
    generate_functions(&mut h, functions);

    // Footer with helper macros
    h.push_str(HEADER_FOOTER);

    h
}

const HEADER_PREAMBLE: &str = r#"/**
 * SysWhispers3 Direct Syscalls - C/C++ Header
 *
 * Auto-generated by build.rs - DO NOT EDIT MANUALLY
 *
 * Direct NT syscalls without going through ntdll.dll
 * Bypasses usermode hooks and EDR/AV monitoring.
 *
 * Build: cargo build --release
 * Output: syscalls.lib (static) + syscalls.dll (dynamic)
 *
 * USAGE:
 *   Option 1 (Recommended): Simple include
 *     #include "syscalls.h"  // Auto-detects Windows SDK
 *
 *   Option 2: Manual Windows SDK integration
 *     #define SYSCALLS_FORCE_WINDOWS_SDK
 *     #include <windows.h>
 *     #include <winternl.h>
 *     #include "syscalls.h"
 *
 *   Option 3: Standalone (no Windows SDK)
 *     #define SYSCALLS_FORCE_STANDALONE
 *     #include "syscalls.h"
 */

#ifndef SYSCALLS_H
#define SYSCALLS_H

/* Auto-detect compatibility mode */
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* Architecture detection */
#if defined(_M_X64) || defined(__x86_64__) || defined(__x86_64) || defined(__amd64__) || defined(__amd64)
    #define SYSCALLS_X64 1
    #define SYSCALLS_X86 0
#elif defined(_M_IX86) || defined(__i386__) || defined(__i386) || defined(_X86_)
    #define SYSCALLS_X64 0
    #define SYSCALLS_X86 1
#else
    #error "Unsupported architecture - only x86 and x64 are supported"
#endif

/* WOW64 detection - compile-time flag only.
 * WOW64 (x86 process on x64 OS) CANNOT be detected at compile time,
 * because _WIN64 is never defined in x86 compilation.
 * Define SYSCALLS_FORCE_WOW64 manually if building for WOW64 target.
 * For runtime detection use IsWow64Process() or NtQueryInformationProcess(). */
#ifdef SYSCALLS_FORCE_WOW64
    #define SYSCALLS_WOW64 1
#else
    #define SYSCALLS_WOW64 0
#endif

/* Detect Windows SDK headers */
#if defined(_WINDOWS_) || defined(_WINDEF_) || defined(_WINNT_) || defined(_NTDEF_) || defined(WINAPI)
    #define SYSCALLS_WINDOWS_SDK_DETECTED 1
#else
    #define SYSCALLS_WINDOWS_SDK_DETECTED 0
#endif

/* Override detection if forced */
#ifdef SYSCALLS_FORCE_WINDOWS_SDK
    #undef SYSCALLS_WINDOWS_SDK_DETECTED
    #define SYSCALLS_WINDOWS_SDK_DETECTED 1
    #ifndef WIN32_LEAN_AND_MEAN
        #define WIN32_LEAN_AND_MEAN
    #endif
    #include <windows.h>
    #include <winternl.h>
#endif

#ifdef SYSCALLS_FORCE_STANDALONE
    #undef SYSCALLS_WINDOWS_SDK_DETECTED
    #define SYSCALLS_WINDOWS_SDK_DETECTED 0
#endif

#ifdef __cplusplus
extern "C" {
#endif

"#;

const HEADER_FOOTER: &str = r#"
/* ==================== SW3 Helper Macros ==================== */

/* SW3 NT status checking macros */
#define SW3_NT_SUCCESS(Status) ((SW3_NTSTATUS)(Status) >= 0)
#define SW3_NT_INFORMATION(Status) (((SW3_NTSTATUS)(Status) >> 30) == 1)
#define SW3_NT_WARNING(Status) (((SW3_NTSTATUS)(Status) >> 30) == 2)
#define SW3_NT_ERROR(Status) (((SW3_NTSTATUS)(Status) >> 30) == 3)

/* SW3 Object attributes initialization macro */
#define SW3_InitializeObjectAttributes(p, n, a, r, s) { \
    (p)->Length = sizeof(SW3_OBJECT_ATTRIBUTES); \
    (p)->RootDirectory = r; \
    (p)->Attributes = a; \
    (p)->ObjectName = n; \
    (p)->SecurityDescriptor = s; \
    (p)->SecurityQualityOfService = NULL; \
}

/* SW3 Pseudo-handles */
#define SW3_NtCurrentProcess() ((SW3_HANDLE)(intptr_t)-1)
#define SW3_NtCurrentThread() ((SW3_HANDLE)(intptr_t)-2)

/* Compatibility aliases when Windows SDK not detected */
#if !SYSCALLS_WINDOWS_SDK_DETECTED

#ifndef NT_SUCCESS
#define NT_SUCCESS(Status) SW3_NT_SUCCESS(Status)
#endif

#ifndef NT_INFORMATION
#define NT_INFORMATION(Status) SW3_NT_INFORMATION(Status)
#endif

#ifndef NT_WARNING
#define NT_WARNING(Status) SW3_NT_WARNING(Status)
#endif

#ifndef NT_ERROR
#define NT_ERROR(Status) SW3_NT_ERROR(Status)
#endif

#ifndef InitializeObjectAttributes
#define InitializeObjectAttributes(p, n, a, r, s) SW3_InitializeObjectAttributes(p, n, a, r, s)
#endif

#ifndef NtCurrentProcess
#define NtCurrentProcess() SW3_NtCurrentProcess()
#endif

#ifndef NtCurrentThread
#define NtCurrentThread() SW3_NtCurrentThread()
#endif

#else

/* Pseudo-handles - always available even with Windows SDK */
#ifndef NtCurrentProcess
#define NtCurrentProcess() ((HANDLE)(intptr_t)-1)
#endif

#ifndef NtCurrentThread
#define NtCurrentThread() ((HANDLE)(intptr_t)-2)
#endif

#endif /* !SYSCALLS_WINDOWS_SDK_DETECTED */

/* ==================== Architecture Information ==================== */

#if SYSCALLS_X64
    #define SYSCALLS_ARCH_STRING "x64"
    #define SYSCALLS_POINTER_SIZE 8
#elif SYSCALLS_X86
    #define SYSCALLS_ARCH_STRING "x86"
    #define SYSCALLS_POINTER_SIZE 4
#endif

#if SYSCALLS_WOW64
    #define SYSCALLS_WOW64_STRING " (WOW64)"
#else
    #define SYSCALLS_WOW64_STRING ""
#endif

/* Architecture info macro */
#define SYSCALLS_GET_ARCH_INFO() (SYSCALLS_ARCH_STRING SYSCALLS_WOW64_STRING)

/* Compile-time architecture checks */
#define SYSCALLS_IS_X64() (SYSCALLS_X64)
#define SYSCALLS_IS_X86() (SYSCALLS_X86)
#define SYSCALLS_IS_WOW64() (SYSCALLS_WOW64)

#ifdef __cplusplus
}
#endif

#endif /* SYSCALLS_H */
"#;

/// Generate base type definitions with SW3_ prefixes - always available
fn generate_base_types(h: &mut String) {
    h.push_str("/* SysWhispers3 base types - always available with SW3_ prefix */\n");
    h.push_str("/* Using SW3_ prefix to avoid conflicts with Windows SDK */\n\n");

    // SW3 prefixed base types - always available
    let sw3_base_types = [
        ("SW3_HANDLE", "HANDLE", "void*"),
        ("SW3_PHANDLE", "PHANDLE", "SW3_HANDLE*"),
        ("SW3_PVOID", "PVOID", "void*"),
        ("SW3_PPVOID", "PPVOID", "void**"),
        ("SW3_LPCVOID", "LPCVOID", "const void*"),
        ("SW3_NTSTATUS", "NTSTATUS", "int32_t"),
        ("SW3_BOOL", "BOOL", "int32_t"),
        ("SW3_BOOLEAN", "BOOLEAN", "uint8_t"),
        ("SW3_PBOOLEAN", "PBOOLEAN", "SW3_BOOLEAN*"),
        ("SW3_UCHAR", "UCHAR", "uint8_t"),
        ("SW3_PUCHAR", "PUCHAR", "uint8_t*"),
        ("SW3_CHAR", "CHAR", "int8_t"),
        ("SW3_PCHAR", "PCHAR", "int8_t*"),
        ("SW3_WCHAR", "WCHAR", "uint16_t"),
        ("SW3_PWCHAR", "PWCHAR", "uint16_t*"),
        ("SW3_PWSTR", "PWSTR", "uint16_t*"),
        ("SW3_PCWSTR", "PCWSTR", "const uint16_t*"),
        ("SW3_PSTR", "PSTR", "int8_t*"),
        ("SW3_PCSTR", "PCSTR", "const int8_t*"),
        ("SW3_USHORT", "USHORT", "uint16_t"),
        ("SW3_PUSHORT", "PUSHORT", "uint16_t*"),
        ("SW3_ULONG", "ULONG", "uint32_t"),
        ("SW3_PULONG", "PULONG", "uint32_t*"),
        ("SW3_ULONG64", "ULONG64", "uint64_t"),
        ("SW3_ULONGLONG", "ULONGLONG", "uint64_t"),
        ("SW3_DWORD", "DWORD", "uint32_t"),
        ("SW3_PDWORD", "PDWORD", "uint32_t*"),
        ("SW3_WORD", "WORD", "uint16_t"),
        ("SW3_PWORD", "PWORD", "uint16_t*"),
        ("SW3_SHORT", "SHORT", "int16_t"),
        ("SW3_PSHORT", "PSHORT", "int16_t*"),
        ("SW3_LONG", "LONG", "int32_t"),
        ("SW3_PLONG", "PLONG", "int32_t*"),
        ("SW3_LONGLONG", "LONGLONG", "int64_t"),
        ("SW3_SIZE_T", "SIZE_T", "size_t"),
        ("SW3_PSIZE_T", "PSIZE_T", "size_t*"),
        ("SW3_SSIZE_T", "SSIZE_T", "intptr_t"),
        ("SW3_ULONG_PTR", "ULONG_PTR", "size_t"),
        ("SW3_PULONG_PTR", "PULONG_PTR", "size_t*"),
        ("SW3_LONG_PTR", "LONG_PTR", "intptr_t"),
        ("SW3_DWORD_PTR", "DWORD_PTR", "size_t"),
        ("SW3_ACCESS_MASK", "ACCESS_MASK", "uint32_t"),
        ("SW3_PACCESS_MASK", "PACCESS_MASK", "uint32_t*"),
        ("SW3_LARGE_INTEGER", "LARGE_INTEGER", "int64_t"),
        ("SW3_PLARGE_INTEGER", "PLARGE_INTEGER", "int64_t*"),
        ("SW3_ULARGE_INTEGER", "ULARGE_INTEGER", "uint64_t"),
        ("SW3_PULARGE_INTEGER", "PULARGE_INTEGER", "uint64_t*"),
        ("SW3_LCID", "LCID", "uint32_t"),
        ("SW3_LANGID", "LANGID", "uint16_t"),
        ("SW3_KAFFINITY", "KAFFINITY", "size_t"),
        ("SW3_KPRIORITY", "KPRIORITY", "int32_t"),
        ("SW3_KIRQL", "KIRQL", "uint8_t"),
        ("SW3_CCHAR", "CCHAR", "int8_t"),
        ("SW3_BYTE", "BYTE", "uint8_t"),
        ("SW3_PBYTE", "PBYTE", "uint8_t*"),
        // Note: SW3_LUID and SW3_PLUID will be defined as structures, not simple types
    ];

    for (sw3_name, original_name, c_type) in &sw3_base_types {
        h.push_str(&format!("typedef {} {};\n", c_type, sw3_name));
        // Add compatibility aliases when Windows SDK not detected
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push('\n');
}

/// Generate opaque pointer types - always available with SW3 prefix
fn generate_opaque_types(h: &mut String) {
    h.push_str("/* SysWhispers3 opaque pointer types - always available */\n");
    h.push_str("/* Using SW3_ prefix to avoid conflicts with Windows SDK */\n\n");

    let sw3_opaque_types = [
        ("SW3_PTIMER_APC_ROUTINE", "PTIMER_APC_ROUTINE"),
        ("SW3_PKNORMAL_ROUTINE", "PKNORMAL_ROUTINE"),
        ("SW3_PBOOT_OPTIONS", "PBOOT_OPTIONS"),
        ("SW3_PFILE_PATH", "PFILE_PATH"),
        ("SW3_PDBGUI_WAIT_STATE_CHANGE", "PDBGUI_WAIT_STATE_CHANGE"),
        ("SW3_PWNF_DELIVERY_DESCRIPTOR", "PWNF_DELIVERY_DESCRIPTOR"),
        ("SW3_PPLUGPLAY_EVENT_BLOCK", "PPLUGPLAY_EVENT_BLOCK"),
        ("SW3_PPORT_SECTION_WRITE", "PPORT_SECTION_WRITE"),
        ("SW3_PPORT_SECTION_READ", "PPORT_SECTION_READ"),
        ("SW3_PALPC_CONTEXT_ATTR", "PALPC_CONTEXT_ATTR"),
        ("SW3_PALPC_DATA_VIEW_ATTR", "PALPC_DATA_VIEW_ATTR"),
        ("SW3_PALPC_SECURITY_ATTR", "PALPC_SECURITY_ATTR"),
        ("SW3_PTOKEN_USER", "PTOKEN_USER"),
        ("SW3_PTOKEN_OWNER", "PTOKEN_OWNER"),
        ("SW3_PTOKEN_PRIMARY_GROUP", "PTOKEN_PRIMARY_GROUP"),
        ("SW3_PTOKEN_DEFAULT_DACL", "PTOKEN_DEFAULT_DACL"),
        ("SW3_PTOKEN_SOURCE", "PTOKEN_SOURCE"),
        ("SW3_PFILE_SEGMENT_ELEMENT", "PFILE_SEGMENT_ELEMENT"),
        ("SW3_PALPC_MESSAGE_ATTRIBUTES", "PALPC_MESSAGE_ATTRIBUTES"),
        ("SW3_PFILE_FULL_EA_INFORMATION", "PFILE_FULL_EA_INFORMATION"),
        ("SW3_PFILE_GET_EA_INFORMATION", "PFILE_GET_EA_INFORMATION"),
        (
            "SW3_PFILE_USER_QUOTA_INFORMATION",
            "PFILE_USER_QUOTA_INFORMATION",
        ),
        (
            "SW3_PFILE_QUOTA_LIST_INFORMATION",
            "PFILE_QUOTA_LIST_INFORMATION",
        ),
        (
            "SW3_PFILE_IO_COMPLETION_INFORMATION",
            "PFILE_IO_COMPLETION_INFORMATION",
        ),
        // Note: These types will be defined as structures, not opaque pointers:
        // SW3_PCLIENT_ID, SW3_PPORT_MESSAGE, SW3_PFILE_BASIC_INFORMATION,
        // SW3_PFILE_NETWORK_OPEN_INFORMATION, SW3_PMEMORY_RANGE_ENTRY,
        // SW3_PINITIAL_TEB, SW3_PPS_ATTRIBUTE_LIST, SW3_PPS_CREATE_INFO,
        // SW3_PT2_SET_PARAMETERS, SW3_PWNF_STATE_NAME, SW3_PWNF_TYPE_ID
    ];

    for (sw3_name, original_name) in &sw3_opaque_types {
        h.push_str(&format!("typedef void* {};\n", sw3_name));
        // Also provide compatibility aliases
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    // RTL_ATOM is uint16_t, not a pointer -- define it separately
    h.push_str("\n/* RTL_ATOM is a 16-bit value, not a pointer */\n");
    h.push_str("typedef uint16_t SW3_RTL_ATOM;\n");
    h.push_str("typedef uint16_t* SW3_PRTL_ATOM;\n");
    h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
    h.push_str("#ifndef RTL_ATOM\n");
    h.push_str("#define RTL_ATOM SW3_RTL_ATOM\n");
    h.push_str("#endif\n");
    h.push_str("#ifndef PRTL_ATOM\n");
    h.push_str("#define PRTL_ATOM SW3_PRTL_ATOM\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n");

    h.push('\n');
}

/// Generate struct definitions with SW3_ prefixes
fn generate_structs(h: &mut String, structs: &[(String, Vec<(String, String)>)]) {
    h.push_str("/* SysWhispers3 structure definitions - always available with SW3_ prefix */\n");
    h.push_str("/* Using SW3_ prefix to avoid conflicts with Windows SDK */\n\n");

    // Add missing pointer types that are always needed
    // NOTE: types already defined in opaque_types are NOT repeated here
    h.push_str("/* Additional SW3 pointer types - always available */\n");
    let missing_types = [
        ("SW3_PSID", "PSID"),
        ("SW3_PSECURITY_DESCRIPTOR", "PSECURITY_DESCRIPTOR"),
        ("SW3_PACL", "PACL"),
        ("SW3_PCONTEXT", "PCONTEXT"),
        ("SW3_PEXCEPTION_RECORD", "PEXCEPTION_RECORD"),
        ("SW3_PIO_APC_ROUTINE", "PIO_APC_ROUTINE"),
        ("SW3_PGROUP_AFFINITY", "PGROUP_AFFINITY"),
        ("SW3_PJOB_SET_ARRAY", "PJOB_SET_ARRAY"),
        ("SW3_PMEM_EXTENDED_PARAMETER", "PMEM_EXTENDED_PARAMETER"),
        ("SW3_PENCLAVE_ROUTINE", "PENCLAVE_ROUTINE"),
    ];

    for (sw3_name, original_name) in &missing_types {
        h.push_str(&format!("typedef void* {};\n", sw3_name));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push('\n');

    // Generate ALL structures with SW3_ prefix
    for (name, fields) in structs {
        let sw3_name = format!("SW3_{}", name);
        let sw3_ptr_name = format!("SW3_P{}", name);

        h.push_str(&format!("typedef struct _{} {{\n", sw3_name));

        // Handle empty structures to avoid C/C++ compatibility issues
        if fields.is_empty()
            || fields
                .iter()
                .all(|(field_name, _)| field_name == "PartitionId" || field_name.starts_with("["))
        {
            // Add placeholder field for empty structures
            match name.as_str() {
                "WNF_STATE_NAME" => {
                    h.push_str("    SW3_ULONG Data[2];  /* Placeholder to avoid empty struct */\n");
                }
                "WNF_TYPE_ID" => {
                    h.push_str(
                        "    SW3_UCHAR TypeId[16];  /* Placeholder to avoid empty struct */\n",
                    );
                }
                _ => {
                    h.push_str(
                        "    SW3_ULONG Reserved;  /* Placeholder to avoid empty struct */\n",
                    );
                }
            }
        } else {
            for (field_name, field_type) in fields {
                // Skip cfg-gated fields but handle arrays properly
                if field_name == "PartitionId" {
                    continue;
                }

                // Handle array types specially for field declarations
                if field_type.starts_with("[")
                    && field_type.contains(";")
                    && field_type.ends_with("]")
                {
                    let inner = &field_type[1..field_type.len() - 1];
                    let parts: Vec<&str> = inner.split(';').collect();
                    if parts.len() == 2 {
                        let typ = parts[0].trim();
                        let count = parts[1].trim();
                        let c_type = rust_to_c_type(typ);
                        h.push_str(&format!("    {} {}[{}];\n", c_type, field_name, count));
                    } else {
                        let c_type = rust_to_c_type(field_type);
                        h.push_str(&format!("    {} {};\n", c_type, field_name));
                    }
                } else {
                    let c_type = rust_to_c_type(field_type);
                    h.push_str(&format!("    {} {};\n", c_type, field_name));
                }
            }
        }

        h.push_str(&format!("}} {}, *{};\n\n", sw3_name, sw3_ptr_name));

        // Add compatibility aliases when Windows SDK not detected
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", name));
        h.push_str(&format!("#define {} {}\n", name, sw3_name));
        h.push_str(&format!("#define P{} {}\n", name, sw3_ptr_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n\n");
    }

    // Add essential structures that are ALWAYS needed (even with Windows SDK)
    h.push_str("/* Essential SW3 structures - always available */\n");
    h.push_str(
        "/* These structures are needed by syscalls but may not be in all SDK versions */\n\n",
    );

    h.push_str("#ifndef _SW3_ALPC_PORT_ATTRIBUTES_DEFINED\n");
    h.push_str("#define _SW3_ALPC_PORT_ATTRIBUTES_DEFINED\n");
    h.push_str("typedef struct _SW3_ALPC_PORT_ATTRIBUTES {\n");
    h.push_str("    SW3_ULONG Flags;\n");
    h.push_str("    SW3_SECURITY_QUALITY_OF_SERVICE SecurityQos;\n");
    h.push_str("    SW3_SIZE_T MaxMessageLength;\n");
    h.push_str("    SW3_SIZE_T MemoryBandwidth;\n");
    h.push_str("    SW3_SIZE_T MaxPoolUsage;\n");
    h.push_str("    SW3_SIZE_T MaxSectionSize;\n");
    h.push_str("    SW3_SIZE_T MaxViewSize;\n");
    h.push_str("    SW3_SIZE_T MaxTotalSectionSize;\n");
    h.push_str("} SW3_ALPC_PORT_ATTRIBUTES, *SW3_PALPC_PORT_ATTRIBUTES;\n");
    h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
    h.push_str("#ifndef ALPC_PORT_ATTRIBUTES\n");
    h.push_str("#define ALPC_PORT_ATTRIBUTES SW3_ALPC_PORT_ATTRIBUTES\n");
    h.push_str("#define PALPC_PORT_ATTRIBUTES SW3_PALPC_PORT_ATTRIBUTES\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n\n");

    h.push_str("#ifndef _SW3_THREAD_BASIC_INFORMATION_DEFINED\n");
    h.push_str("#define _SW3_THREAD_BASIC_INFORMATION_DEFINED\n");
    h.push_str("typedef struct _SW3_THREAD_BASIC_INFORMATION {\n");
    h.push_str("    SW3_NTSTATUS ExitStatus;\n");
    h.push_str("    SW3_PVOID TebBaseAddress;\n");
    h.push_str("    SW3_CLIENT_ID ClientId;\n");
    h.push_str("    SW3_KAFFINITY AffinityMask;\n");
    h.push_str("    SW3_KPRIORITY Priority;\n");
    h.push_str("    SW3_KPRIORITY BasePriority;\n");
    h.push_str("} SW3_THREAD_BASIC_INFORMATION, *SW3_PTHREAD_BASIC_INFORMATION;\n");
    h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
    h.push_str("#ifndef THREAD_BASIC_INFORMATION\n");
    h.push_str("#define THREAD_BASIC_INFORMATION SW3_THREAD_BASIC_INFORMATION\n");
    h.push_str("#define PTHREAD_BASIC_INFORMATION SW3_PTHREAD_BASIC_INFORMATION\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n\n");

    h.push_str("#ifndef _SW3_PROCESS_BASIC_INFORMATION_DEFINED\n");
    h.push_str("#define _SW3_PROCESS_BASIC_INFORMATION_DEFINED\n");
    h.push_str("typedef struct _SW3_PROCESS_BASIC_INFORMATION {\n");
    h.push_str("    SW3_NTSTATUS ExitStatus;\n");
    h.push_str("    SW3_PVOID PebBaseAddress;\n");
    h.push_str("    SW3_ULONG_PTR AffinityMask;\n");
    h.push_str("    SW3_KPRIORITY BasePriority;\n");
    h.push_str("    SW3_HANDLE UniqueProcessId;\n");
    h.push_str("    SW3_HANDLE InheritedFromUniqueProcessId;\n");
    h.push_str("} SW3_PROCESS_BASIC_INFORMATION, *SW3_PPROCESS_BASIC_INFORMATION;\n");
    h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
    h.push_str("#ifndef PROCESS_BASIC_INFORMATION\n");
    h.push_str("#define PROCESS_BASIC_INFORMATION SW3_PROCESS_BASIC_INFORMATION\n");
    h.push_str("#define PPROCESS_BASIC_INFORMATION SW3_PPROCESS_BASIC_INFORMATION\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n\n");

    h.push_str("#ifndef _SW3_TOKEN_STATISTICS_DEFINED\n");
    h.push_str("#define _SW3_TOKEN_STATISTICS_DEFINED\n");
    h.push_str("typedef struct _SW3_TOKEN_STATISTICS {\n");
    h.push_str("    SW3_LUID TokenId;\n");
    h.push_str("    SW3_LUID AuthenticationId;\n");
    h.push_str("    SW3_LARGE_INTEGER ExpirationTime;\n");
    h.push_str("    uint32_t TokenType;\n");
    h.push_str("    uint32_t ImpersonationLevel;\n");
    h.push_str("    SW3_ULONG DynamicCharged;\n");
    h.push_str("    SW3_ULONG DynamicAvailable;\n");
    h.push_str("    SW3_ULONG GroupCount;\n");
    h.push_str("    SW3_ULONG PrivilegeCount;\n");
    h.push_str("    SW3_LUID ModifiedId;\n");
    h.push_str("} SW3_TOKEN_STATISTICS, *SW3_PTOKEN_STATISTICS;\n");
    h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
    h.push_str("#ifndef TOKEN_STATISTICS\n");
    h.push_str("#define TOKEN_STATISTICS SW3_TOKEN_STATISTICS\n");
    h.push_str("#define PTOKEN_STATISTICS SW3_PTOKEN_STATISTICS\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n");
    h.push_str("#endif\n\n");
}

/// Generate all constants and NTSTATUS codes with SW3_ prefixes
fn generate_all_constants(h: &mut String, _constants: &[(String, String, String)]) {
    h.push_str("/* SysWhispers3 constants - always available with SW3_ prefix */\n");
    h.push_str("/* Using SW3_ prefix to avoid conflicts with Windows SDK */\n\n");

    // Core SW3 NTSTATUS codes - always needed
    let sw3_core_status = [
        ("SW3_STATUS_SUCCESS", "STATUS_SUCCESS", "0x00000000L"),
        (
            "SW3_STATUS_UNSUCCESSFUL",
            "STATUS_UNSUCCESSFUL",
            "0xC0000001L",
        ),
        (
            "SW3_STATUS_ACCESS_DENIED",
            "STATUS_ACCESS_DENIED",
            "0xC0000022L",
        ),
        (
            "SW3_STATUS_INVALID_HANDLE",
            "STATUS_INVALID_HANDLE",
            "0xC0000008L",
        ),
        (
            "SW3_STATUS_INVALID_PARAMETER",
            "STATUS_INVALID_PARAMETER",
            "0xC000000DL",
        ),
        ("SW3_STATUS_NO_MEMORY", "STATUS_NO_MEMORY", "0xC0000017L"),
        (
            "SW3_STATUS_BUFFER_TOO_SMALL",
            "STATUS_BUFFER_TOO_SMALL",
            "0xC0000023L",
        ),
        (
            "SW3_STATUS_OBJECT_NAME_NOT_FOUND",
            "STATUS_OBJECT_NAME_NOT_FOUND",
            "0xC0000034L",
        ),
        (
            "SW3_STATUS_OBJECT_PATH_NOT_FOUND",
            "STATUS_OBJECT_PATH_NOT_FOUND",
            "0xC000003AL",
        ),
        (
            "SW3_STATUS_SHARING_VIOLATION",
            "STATUS_SHARING_VIOLATION",
            "0xC0000043L",
        ),
        (
            "SW3_STATUS_INSUFFICIENT_RESOURCES",
            "STATUS_INSUFFICIENT_RESOURCES",
            "0xC000009AL",
        ),
        (
            "SW3_STATUS_NOT_SUPPORTED",
            "STATUS_NOT_SUPPORTED",
            "0xC00000BBL",
        ),
        (
            "SW3_STATUS_INVALID_PARAMETER_1",
            "STATUS_INVALID_PARAMETER_1",
            "0xC00000EFL",
        ),
        (
            "SW3_STATUS_INVALID_PARAMETER_2",
            "STATUS_INVALID_PARAMETER_2",
            "0xC00000F0L",
        ),
        (
            "SW3_STATUS_INVALID_PARAMETER_3",
            "STATUS_INVALID_PARAMETER_3",
            "0xC00000F1L",
        ),
        (
            "SW3_STATUS_INVALID_PARAMETER_4",
            "STATUS_INVALID_PARAMETER_4",
            "0xC00000F2L",
        ),
        (
            "SW3_STATUS_INVALID_PARAMETER_5",
            "STATUS_INVALID_PARAMETER_5",
            "0xC00000F3L",
        ),
        (
            "SW3_STATUS_INVALID_PARAMETER_6",
            "STATUS_INVALID_PARAMETER_6",
            "0xC00000F4L",
        ),
        (
            "SW3_STATUS_PROCEDURE_NOT_FOUND",
            "STATUS_PROCEDURE_NOT_FOUND",
            "0xC000007AL",
        ),
        (
            "SW3_STATUS_INVALID_IMAGE_FORMAT",
            "STATUS_INVALID_IMAGE_FORMAT",
            "0xC000007BL",
        ),
        ("SW3_STATUS_NO_TOKEN", "STATUS_NO_TOKEN", "0xC000007CL"),
        (
            "SW3_STATUS_PRIVILEGE_NOT_HELD",
            "STATUS_PRIVILEGE_NOT_HELD",
            "0xC0000061L",
        ),
        (
            "SW3_STATUS_ENTRYPOINT_NOT_FOUND",
            "STATUS_ENTRYPOINT_NOT_FOUND",
            "0xC0000139L",
        ),
        (
            "SW3_STATUS_DLL_NOT_FOUND",
            "STATUS_DLL_NOT_FOUND",
            "0xC0000135L",
        ),
        ("SW3_STATUS_WAIT_0", "STATUS_WAIT_0", "0x00000000L"),
        (
            "SW3_STATUS_ABANDONED_WAIT_0",
            "STATUS_ABANDONED_WAIT_0",
            "0x00000080L",
        ),
        ("SW3_STATUS_USER_APC", "STATUS_USER_APC", "0x000000C0L"),
        ("SW3_STATUS_TIMEOUT", "STATUS_TIMEOUT", "0x00000102L"),
        ("SW3_STATUS_PENDING", "STATUS_PENDING", "0x00000103L"),
        (
            "SW3_STATUS_BUFFER_OVERFLOW",
            "STATUS_BUFFER_OVERFLOW",
            "0x80000005L",
        ),
        (
            "SW3_STATUS_NO_MORE_FILES",
            "STATUS_NO_MORE_FILES",
            "0x80000006L",
        ),
        (
            "SW3_STATUS_NO_MORE_ENTRIES",
            "STATUS_NO_MORE_ENTRIES",
            "0x8000001AL",
        ),
    ];

    for (sw3_name, original_name, value) in &sw3_core_status {
        h.push_str(&format!("#define {} ((SW3_NTSTATUS){})\n", sw3_name, value));
        // Add compatibility aliases when Windows SDK not detected
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Memory protection constants */\n");
    let sw3_memory_constants = [
        ("SW3_PAGE_NOACCESS", "PAGE_NOACCESS", "0x01"),
        ("SW3_PAGE_READONLY", "PAGE_READONLY", "0x02"),
        ("SW3_PAGE_READWRITE", "PAGE_READWRITE", "0x04"),
        ("SW3_PAGE_WRITECOPY", "PAGE_WRITECOPY", "0x08"),
        ("SW3_PAGE_EXECUTE", "PAGE_EXECUTE", "0x10"),
        ("SW3_PAGE_EXECUTE_READ", "PAGE_EXECUTE_READ", "0x20"),
        (
            "SW3_PAGE_EXECUTE_READWRITE",
            "PAGE_EXECUTE_READWRITE",
            "0x40",
        ),
        (
            "SW3_PAGE_EXECUTE_WRITECOPY",
            "PAGE_EXECUTE_WRITECOPY",
            "0x80",
        ),
        ("SW3_PAGE_GUARD", "PAGE_GUARD", "0x100"),
        ("SW3_PAGE_NOCACHE", "PAGE_NOCACHE", "0x200"),
        ("SW3_PAGE_WRITECOMBINE", "PAGE_WRITECOMBINE", "0x400"),
        ("SW3_MEM_COMMIT", "MEM_COMMIT", "0x00001000"),
        ("SW3_MEM_RESERVE", "MEM_RESERVE", "0x00002000"),
        ("SW3_MEM_DECOMMIT", "MEM_DECOMMIT", "0x00004000"),
        ("SW3_MEM_RELEASE", "MEM_RELEASE", "0x00008000"),
        ("SW3_MEM_FREE", "MEM_FREE", "0x00010000"),
        ("SW3_MEM_PRIVATE", "MEM_PRIVATE", "0x00020000"),
        ("SW3_MEM_MAPPED", "MEM_MAPPED", "0x00040000"),
        ("SW3_MEM_RESET", "MEM_RESET", "0x00080000"),
        ("SW3_MEM_TOP_DOWN", "MEM_TOP_DOWN", "0x00100000"),
        ("SW3_MEM_WRITE_WATCH", "MEM_WRITE_WATCH", "0x00200000"),
        ("SW3_MEM_PHYSICAL", "MEM_PHYSICAL", "0x00400000"),
        ("SW3_MEM_ROTATE", "MEM_ROTATE", "0x00800000"),
        ("SW3_MEM_LARGE_PAGES", "MEM_LARGE_PAGES", "0x20000000"),
        ("SW3_MEM_4MB_PAGES", "MEM_4MB_PAGES", "0x80000000"),
    ];

    for (sw3_name, original_name, value) in &sw3_memory_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Access rights constants */\n");
    let sw3_access_constants = [
        ("SW3_GENERIC_READ", "GENERIC_READ", "0x80000000"),
        ("SW3_GENERIC_WRITE", "GENERIC_WRITE", "0x40000000"),
        ("SW3_GENERIC_EXECUTE", "GENERIC_EXECUTE", "0x20000000"),
        ("SW3_GENERIC_ALL", "GENERIC_ALL", "0x10000000"),
        ("SW3_MAXIMUM_ALLOWED", "MAXIMUM_ALLOWED", "0x02000000"),
        ("SW3_DELETE", "DELETE", "0x00010000"),
        ("SW3_READ_CONTROL", "READ_CONTROL", "0x00020000"),
        ("SW3_WRITE_DAC", "WRITE_DAC", "0x00040000"),
        ("SW3_WRITE_OWNER", "WRITE_OWNER", "0x00080000"),
        ("SW3_SYNCHRONIZE", "SYNCHRONIZE", "0x00100000"),
        (
            "SW3_STANDARD_RIGHTS_REQUIRED",
            "STANDARD_RIGHTS_REQUIRED",
            "0x000F0000",
        ),
        (
            "SW3_STANDARD_RIGHTS_ALL",
            "STANDARD_RIGHTS_ALL",
            "0x001F0000",
        ),
        (
            "SW3_SPECIFIC_RIGHTS_ALL",
            "SPECIFIC_RIGHTS_ALL",
            "0x0000FFFF",
        ),
        ("SW3_PROCESS_TERMINATE", "PROCESS_TERMINATE", "0x0001"),
        (
            "SW3_PROCESS_CREATE_THREAD",
            "PROCESS_CREATE_THREAD",
            "0x0002",
        ),
        ("SW3_PROCESS_VM_OPERATION", "PROCESS_VM_OPERATION", "0x0008"),
        ("SW3_PROCESS_VM_READ", "PROCESS_VM_READ", "0x0010"),
        ("SW3_PROCESS_VM_WRITE", "PROCESS_VM_WRITE", "0x0020"),
        ("SW3_PROCESS_DUP_HANDLE", "PROCESS_DUP_HANDLE", "0x0040"),
        (
            "SW3_PROCESS_CREATE_PROCESS",
            "PROCESS_CREATE_PROCESS",
            "0x0080",
        ),
        ("SW3_PROCESS_SET_QUOTA", "PROCESS_SET_QUOTA", "0x0100"),
        (
            "SW3_PROCESS_SET_INFORMATION",
            "PROCESS_SET_INFORMATION",
            "0x0200",
        ),
        (
            "SW3_PROCESS_QUERY_INFORMATION",
            "PROCESS_QUERY_INFORMATION",
            "0x0400",
        ),
        (
            "SW3_PROCESS_SUSPEND_RESUME",
            "PROCESS_SUSPEND_RESUME",
            "0x0800",
        ),
        (
            "SW3_PROCESS_QUERY_LIMITED_INFORMATION",
            "PROCESS_QUERY_LIMITED_INFORMATION",
            "0x1000",
        ),
        ("SW3_PROCESS_ALL_ACCESS", "PROCESS_ALL_ACCESS", "0x1FFFFF"),
        ("SW3_THREAD_TERMINATE", "THREAD_TERMINATE", "0x0001"),
        (
            "SW3_THREAD_SUSPEND_RESUME",
            "THREAD_SUSPEND_RESUME",
            "0x0002",
        ),
        ("SW3_THREAD_GET_CONTEXT", "THREAD_GET_CONTEXT", "0x0008"),
        ("SW3_THREAD_SET_CONTEXT", "THREAD_SET_CONTEXT", "0x0010"),
        (
            "SW3_THREAD_SET_INFORMATION",
            "THREAD_SET_INFORMATION",
            "0x0020",
        ),
        (
            "SW3_THREAD_QUERY_INFORMATION",
            "THREAD_QUERY_INFORMATION",
            "0x0040",
        ),
        (
            "SW3_THREAD_SET_THREAD_TOKEN",
            "THREAD_SET_THREAD_TOKEN",
            "0x0080",
        ),
        ("SW3_THREAD_IMPERSONATE", "THREAD_IMPERSONATE", "0x0100"),
        (
            "SW3_THREAD_DIRECT_IMPERSONATION",
            "THREAD_DIRECT_IMPERSONATION",
            "0x0200",
        ),
        ("SW3_THREAD_ALL_ACCESS", "THREAD_ALL_ACCESS", "0x1FFFFF"),
    ];

    for (sw3_name, original_name, value) in &sw3_access_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 File and registry constants */\n");
    let sw3_file_constants = [
        ("SW3_FILE_READ_DATA", "FILE_READ_DATA", "0x0001"),
        ("SW3_FILE_WRITE_DATA", "FILE_WRITE_DATA", "0x0002"),
        ("SW3_FILE_APPEND_DATA", "FILE_APPEND_DATA", "0x0004"),
        ("SW3_FILE_READ_EA", "FILE_READ_EA", "0x0008"),
        ("SW3_FILE_WRITE_EA", "FILE_WRITE_EA", "0x0010"),
        ("SW3_FILE_EXECUTE", "FILE_EXECUTE", "0x0020"),
        ("SW3_FILE_READ_ATTRIBUTES", "FILE_READ_ATTRIBUTES", "0x0080"),
        (
            "SW3_FILE_WRITE_ATTRIBUTES",
            "FILE_WRITE_ATTRIBUTES",
            "0x0100",
        ),
        ("SW3_FILE_ALL_ACCESS", "FILE_ALL_ACCESS", "0x1F01FF"),
        ("SW3_FILE_SHARE_READ", "FILE_SHARE_READ", "0x00000001"),
        ("SW3_FILE_SHARE_WRITE", "FILE_SHARE_WRITE", "0x00000002"),
        ("SW3_FILE_SHARE_DELETE", "FILE_SHARE_DELETE", "0x00000004"),
        ("SW3_FILE_SUPERSEDE", "FILE_SUPERSEDE", "0x00000000"),
        ("SW3_FILE_OPEN", "FILE_OPEN", "0x00000001"),
        ("SW3_FILE_CREATE", "FILE_CREATE", "0x00000002"),
        ("SW3_FILE_OPEN_IF", "FILE_OPEN_IF", "0x00000003"),
        ("SW3_FILE_OVERWRITE", "FILE_OVERWRITE", "0x00000004"),
        ("SW3_FILE_OVERWRITE_IF", "FILE_OVERWRITE_IF", "0x00000005"),
        (
            "SW3_FILE_DIRECTORY_FILE",
            "FILE_DIRECTORY_FILE",
            "0x00000001",
        ),
        (
            "SW3_FILE_NON_DIRECTORY_FILE",
            "FILE_NON_DIRECTORY_FILE",
            "0x00000040",
        ),
        (
            "SW3_FILE_SYNCHRONOUS_IO_NONALERT",
            "FILE_SYNCHRONOUS_IO_NONALERT",
            "0x00000020",
        ),
        (
            "SW3_FILE_DELETE_ON_CLOSE",
            "FILE_DELETE_ON_CLOSE",
            "0x00001000",
        ),
        ("SW3_REG_SZ", "REG_SZ", "1"),
        (
            "SW3_REG_OPTION_NON_VOLATILE",
            "REG_OPTION_NON_VOLATILE",
            "0x00000000L",
        ),
        ("SW3_KEY_ALL_ACCESS", "KEY_ALL_ACCESS", "0xF003F"),
        ("SW3_TOKEN_QUERY", "TOKEN_QUERY", "0x0008"),
        ("SW3_EVENT_ALL_ACCESS", "EVENT_ALL_ACCESS", "0x1F0003"),
    ];

    for (sw3_name, original_name, value) in &sw3_file_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Registry access rights constants */\n");
    let sw3_registry_constants = [
        ("SW3_KEY_QUERY_VALUE", "KEY_QUERY_VALUE", "0x0001"),
        ("SW3_KEY_SET_VALUE", "KEY_SET_VALUE", "0x0002"),
        ("SW3_KEY_CREATE_SUB_KEY", "KEY_CREATE_SUB_KEY", "0x0004"),
        (
            "SW3_KEY_ENUMERATE_SUB_KEYS",
            "KEY_ENUMERATE_SUB_KEYS",
            "0x0008",
        ),
        ("SW3_KEY_NOTIFY", "KEY_NOTIFY", "0x0010"),
        ("SW3_KEY_CREATE_LINK", "KEY_CREATE_LINK", "0x0020"),
        ("SW3_KEY_WOW64_64KEY", "KEY_WOW64_64KEY", "0x0100"),
        ("SW3_KEY_WOW64_32KEY", "KEY_WOW64_32KEY", "0x0200"),
        ("SW3_KEY_READ", "KEY_READ", "0x20019"),
        ("SW3_KEY_WRITE", "KEY_WRITE", "0x20006"),
        ("SW3_KEY_EXECUTE", "KEY_EXECUTE", "0x20019"),
    ];

    for (sw3_name, original_name, value) in &sw3_registry_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Section access rights constants */\n");
    let sw3_section_constants = [
        ("SW3_SECTION_QUERY", "SECTION_QUERY", "0x0001"),
        ("SW3_SECTION_MAP_WRITE", "SECTION_MAP_WRITE", "0x0002"),
        ("SW3_SECTION_MAP_READ", "SECTION_MAP_READ", "0x0004"),
        ("SW3_SECTION_MAP_EXECUTE", "SECTION_MAP_EXECUTE", "0x0008"),
        ("SW3_SECTION_EXTEND_SIZE", "SECTION_EXTEND_SIZE", "0x0010"),
        (
            "SW3_SECTION_MAP_EXECUTE_EXPLICIT",
            "SECTION_MAP_EXECUTE_EXPLICIT",
            "0x0020",
        ),
        ("SW3_SECTION_ALL_ACCESS", "SECTION_ALL_ACCESS", "0xF001F"),
    ];

    for (sw3_name, original_name, value) in &sw3_section_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Token access rights constants */\n");
    let sw3_token_constants = [
        ("SW3_TOKEN_ASSIGN_PRIMARY", "TOKEN_ASSIGN_PRIMARY", "0x0001"),
        ("SW3_TOKEN_DUPLICATE", "TOKEN_DUPLICATE", "0x0002"),
        ("SW3_TOKEN_IMPERSONATE", "TOKEN_IMPERSONATE", "0x0004"),
        ("SW3_TOKEN_QUERY_SOURCE", "TOKEN_QUERY_SOURCE", "0x0010"),
        (
            "SW3_TOKEN_ADJUST_PRIVILEGES",
            "TOKEN_ADJUST_PRIVILEGES",
            "0x0020",
        ),
        ("SW3_TOKEN_ADJUST_GROUPS", "TOKEN_ADJUST_GROUPS", "0x0040"),
        ("SW3_TOKEN_ADJUST_DEFAULT", "TOKEN_ADJUST_DEFAULT", "0x0080"),
        (
            "SW3_TOKEN_ADJUST_SESSIONID",
            "TOKEN_ADJUST_SESSIONID",
            "0x0100",
        ),
        ("SW3_TOKEN_ALL_ACCESS", "TOKEN_ALL_ACCESS", "0xF01FF"),
        ("SW3_TOKEN_READ", "TOKEN_READ", "0x20008"),
        ("SW3_TOKEN_WRITE", "TOKEN_WRITE", "0x200E0"),
        ("SW3_TOKEN_EXECUTE", "TOKEN_EXECUTE", "0x20000"),
    ];

    for (sw3_name, original_name, value) in &sw3_token_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Additional important constants */\n");
    let sw3_additional_constants = [
        // Synchronization object constants
        ("SW3_EVENT_MODIFY_STATE", "EVENT_MODIFY_STATE", "0x0002"),
        ("SW3_MUTEX_MODIFY_STATE", "MUTEX_MODIFY_STATE", "0x0001"),
        (
            "SW3_SEMAPHORE_MODIFY_STATE",
            "SEMAPHORE_MODIFY_STATE",
            "0x0002",
        ),
        ("SW3_TIMER_MODIFY_STATE", "TIMER_MODIFY_STATE", "0x0002"),
        ("SW3_TIMER_QUERY_STATE", "TIMER_QUERY_STATE", "0x0001"),
        // Job object constants
        (
            "SW3_JOB_OBJECT_ASSIGN_PROCESS",
            "JOB_OBJECT_ASSIGN_PROCESS",
            "0x0001",
        ),
        (
            "SW3_JOB_OBJECT_SET_ATTRIBUTES",
            "JOB_OBJECT_SET_ATTRIBUTES",
            "0x0002",
        ),
        ("SW3_JOB_OBJECT_QUERY", "JOB_OBJECT_QUERY", "0x0004"),
        ("SW3_JOB_OBJECT_TERMINATE", "JOB_OBJECT_TERMINATE", "0x0008"),
        (
            "SW3_JOB_OBJECT_SET_SECURITY_ATTRIBUTES",
            "JOB_OBJECT_SET_SECURITY_ATTRIBUTES",
            "0x0010",
        ),
        (
            "SW3_JOB_OBJECT_ALL_ACCESS",
            "JOB_OBJECT_ALL_ACCESS",
            "0x1F001F",
        ),
        // File attribute constants
        (
            "SW3_FILE_ATTRIBUTE_READONLY",
            "FILE_ATTRIBUTE_READONLY",
            "0x00000001",
        ),
        (
            "SW3_FILE_ATTRIBUTE_HIDDEN",
            "FILE_ATTRIBUTE_HIDDEN",
            "0x00000002",
        ),
        (
            "SW3_FILE_ATTRIBUTE_SYSTEM",
            "FILE_ATTRIBUTE_SYSTEM",
            "0x00000004",
        ),
        (
            "SW3_FILE_ATTRIBUTE_DIRECTORY",
            "FILE_ATTRIBUTE_DIRECTORY",
            "0x00000010",
        ),
        (
            "SW3_FILE_ATTRIBUTE_ARCHIVE",
            "FILE_ATTRIBUTE_ARCHIVE",
            "0x00000020",
        ),
        (
            "SW3_FILE_ATTRIBUTE_DEVICE",
            "FILE_ATTRIBUTE_DEVICE",
            "0x00000040",
        ),
        (
            "SW3_FILE_ATTRIBUTE_NORMAL",
            "FILE_ATTRIBUTE_NORMAL",
            "0x00000080",
        ),
        (
            "SW3_FILE_ATTRIBUTE_TEMPORARY",
            "FILE_ATTRIBUTE_TEMPORARY",
            "0x00000100",
        ),
        (
            "SW3_FILE_ATTRIBUTE_SPARSE_FILE",
            "FILE_ATTRIBUTE_SPARSE_FILE",
            "0x00000200",
        ),
        (
            "SW3_FILE_ATTRIBUTE_REPARSE_POINT",
            "FILE_ATTRIBUTE_REPARSE_POINT",
            "0x00000400",
        ),
        (
            "SW3_FILE_ATTRIBUTE_COMPRESSED",
            "FILE_ATTRIBUTE_COMPRESSED",
            "0x00000800",
        ),
        (
            "SW3_FILE_ATTRIBUTE_OFFLINE",
            "FILE_ATTRIBUTE_OFFLINE",
            "0x00001000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_NOT_CONTENT_INDEXED",
            "FILE_ATTRIBUTE_NOT_CONTENT_INDEXED",
            "0x00002000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_ENCRYPTED",
            "FILE_ATTRIBUTE_ENCRYPTED",
            "0x00004000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_INTEGRITY_STREAM",
            "FILE_ATTRIBUTE_INTEGRITY_STREAM",
            "0x00008000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_VIRTUAL",
            "FILE_ATTRIBUTE_VIRTUAL",
            "0x00010000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_NO_SCRUB_DATA",
            "FILE_ATTRIBUTE_NO_SCRUB_DATA",
            "0x00020000",
        ),
        ("SW3_FILE_ATTRIBUTE_EA", "FILE_ATTRIBUTE_EA", "0x00040000"),
        (
            "SW3_FILE_ATTRIBUTE_PINNED",
            "FILE_ATTRIBUTE_PINNED",
            "0x00080000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_UNPINNED",
            "FILE_ATTRIBUTE_UNPINNED",
            "0x00100000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_RECALL_ON_OPEN",
            "FILE_ATTRIBUTE_RECALL_ON_OPEN",
            "0x00040000",
        ),
        (
            "SW3_FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS",
            "FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS",
            "0x00400000",
        ),
        // Directory access constants
        ("SW3_FILE_ADD_FILE", "FILE_ADD_FILE", "0x0002"),
        (
            "SW3_FILE_ADD_SUBDIRECTORY",
            "FILE_ADD_SUBDIRECTORY",
            "0x0004",
        ),
        (
            "SW3_FILE_CREATE_PIPE_INSTANCE",
            "FILE_CREATE_PIPE_INSTANCE",
            "0x0004",
        ),
        ("SW3_FILE_DELETE_CHILD", "FILE_DELETE_CHILD", "0x0040"),
        ("SW3_FILE_LIST_DIRECTORY", "FILE_LIST_DIRECTORY", "0x0001"),
        ("SW3_FILE_TRAVERSE", "FILE_TRAVERSE", "0x0020"),
        // Named pipe constants
        (
            "SW3_PIPE_ACCESS_INBOUND",
            "PIPE_ACCESS_INBOUND",
            "0x00000001",
        ),
        (
            "SW3_PIPE_ACCESS_OUTBOUND",
            "PIPE_ACCESS_OUTBOUND",
            "0x00000002",
        ),
        ("SW3_PIPE_ACCESS_DUPLEX", "PIPE_ACCESS_DUPLEX", "0x00000003"),
        // Additional registry constants
        ("SW3_REG_BINARY", "REG_BINARY", "3"),
        ("SW3_REG_DWORD", "REG_DWORD", "4"),
        (
            "SW3_REG_DWORD_LITTLE_ENDIAN",
            "REG_DWORD_LITTLE_ENDIAN",
            "4",
        ),
        ("SW3_REG_DWORD_BIG_ENDIAN", "REG_DWORD_BIG_ENDIAN", "5"),
        ("SW3_REG_EXPAND_SZ", "REG_EXPAND_SZ", "2"),
        ("SW3_REG_LINK", "REG_LINK", "6"),
        ("SW3_REG_MULTI_SZ", "REG_MULTI_SZ", "7"),
        ("SW3_REG_NONE", "REG_NONE", "0"),
        ("SW3_REG_QWORD", "REG_QWORD", "11"),
        (
            "SW3_REG_QWORD_LITTLE_ENDIAN",
            "REG_QWORD_LITTLE_ENDIAN",
            "11",
        ),
        // Security constants
        ("SW3_SECURITY_ANONYMOUS", "SECURITY_ANONYMOUS", "0"),
        (
            "SW3_SECURITY_IDENTIFICATION",
            "SECURITY_IDENTIFICATION",
            "1",
        ),
        ("SW3_SECURITY_IMPERSONATION", "SECURITY_IMPERSONATION", "2"),
        ("SW3_SECURITY_DELEGATION", "SECURITY_DELEGATION", "3"),
        // Additional access rights
        (
            "SW3_ACCESS_SYSTEM_SECURITY",
            "ACCESS_SYSTEM_SECURITY",
            "0x01000000",
        ),
        ("SW3_MAXIMUM_ALLOWED", "MAXIMUM_ALLOWED", "0x02000000"),
        // Wait constants
        ("SW3_WAIT_FAILED", "WAIT_FAILED", "0xFFFFFFFF"),
        ("SW3_WAIT_OBJECT_0", "WAIT_OBJECT_0", "0x00000000"),
        ("SW3_WAIT_ABANDONED", "WAIT_ABANDONED", "0x00000080"),
        ("SW3_WAIT_TIMEOUT", "WAIT_TIMEOUT", "0x00000102"),
        ("SW3_INFINITE", "INFINITE", "0xFFFFFFFF"),
        // Error constants
        ("SW3_ERROR_SUCCESS", "ERROR_SUCCESS", "0"),
        ("SW3_ERROR_INVALID_FUNCTION", "ERROR_INVALID_FUNCTION", "1"),
        ("SW3_ERROR_FILE_NOT_FOUND", "ERROR_FILE_NOT_FOUND", "2"),
        ("SW3_ERROR_PATH_NOT_FOUND", "ERROR_PATH_NOT_FOUND", "3"),
        (
            "SW3_ERROR_TOO_MANY_OPEN_FILES",
            "ERROR_TOO_MANY_OPEN_FILES",
            "4",
        ),
        ("SW3_ERROR_ACCESS_DENIED", "ERROR_ACCESS_DENIED", "5"),
        ("SW3_ERROR_INVALID_HANDLE", "ERROR_INVALID_HANDLE", "6"),
        (
            "SW3_ERROR_NOT_ENOUGH_MEMORY",
            "ERROR_NOT_ENOUGH_MEMORY",
            "8",
        ),
        ("SW3_ERROR_INVALID_DATA", "ERROR_INVALID_DATA", "13"),
        (
            "SW3_ERROR_INVALID_PARAMETER",
            "ERROR_INVALID_PARAMETER",
            "87",
        ),
        (
            "SW3_ERROR_INSUFFICIENT_BUFFER",
            "ERROR_INSUFFICIENT_BUFFER",
            "122",
        ),
        ("SW3_ERROR_MORE_DATA", "ERROR_MORE_DATA", "234"),
        ("SW3_ERROR_NO_MORE_ITEMS", "ERROR_NO_MORE_ITEMS", "259"),
    ];

    for (sw3_name, original_name, value) in &sw3_additional_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Privilege name constants */\n");
    let sw3_privilege_constants = [
        (
            "SW3_SE_ASSIGNPRIMARYTOKEN_NAME",
            "SE_ASSIGNPRIMARYTOKEN_NAME",
            "\"SeAssignPrimaryTokenPrivilege\"",
        ),
        ("SW3_SE_AUDIT_NAME", "SE_AUDIT_NAME", "\"SeAuditPrivilege\""),
        (
            "SW3_SE_BACKUP_NAME",
            "SE_BACKUP_NAME",
            "\"SeBackupPrivilege\"",
        ),
        (
            "SW3_SE_CHANGE_NOTIFY_NAME",
            "SE_CHANGE_NOTIFY_NAME",
            "\"SeChangeNotifyPrivilege\"",
        ),
        (
            "SW3_SE_CREATE_GLOBAL_NAME",
            "SE_CREATE_GLOBAL_NAME",
            "\"SeCreateGlobalPrivilege\"",
        ),
        (
            "SW3_SE_CREATE_PAGEFILE_NAME",
            "SE_CREATE_PAGEFILE_NAME",
            "\"SeCreatePagefilePrivilege\"",
        ),
        (
            "SW3_SE_CREATE_PERMANENT_NAME",
            "SE_CREATE_PERMANENT_NAME",
            "\"SeCreatePermanentPrivilege\"",
        ),
        (
            "SW3_SE_CREATE_SYMBOLIC_LINK_NAME",
            "SE_CREATE_SYMBOLIC_LINK_NAME",
            "\"SeCreateSymbolicLinkPrivilege\"",
        ),
        (
            "SW3_SE_CREATE_TOKEN_NAME",
            "SE_CREATE_TOKEN_NAME",
            "\"SeCreateTokenPrivilege\"",
        ),
        ("SW3_SE_DEBUG_NAME", "SE_DEBUG_NAME", "\"SeDebugPrivilege\""),
        (
            "SW3_SE_DELEGATE_SESSION_USER_IMPERSONATE_NAME",
            "SE_DELEGATE_SESSION_USER_IMPERSONATE_NAME",
            "\"SeDelegateSessionUserImpersonatePrivilege\"",
        ),
        (
            "SW3_SE_ENABLE_DELEGATION_NAME",
            "SE_ENABLE_DELEGATION_NAME",
            "\"SeEnableDelegationPrivilege\"",
        ),
        (
            "SW3_SE_IMPERSONATE_NAME",
            "SE_IMPERSONATE_NAME",
            "\"SeImpersonatePrivilege\"",
        ),
        (
            "SW3_SE_INC_BASE_PRIORITY_NAME",
            "SE_INC_BASE_PRIORITY_NAME",
            "\"SeIncreaseBasePriorityPrivilege\"",
        ),
        (
            "SW3_SE_INCREASE_QUOTA_NAME",
            "SE_INCREASE_QUOTA_NAME",
            "\"SeIncreaseQuotaPrivilege\"",
        ),
        (
            "SW3_SE_INC_WORKING_SET_NAME",
            "SE_INC_WORKING_SET_NAME",
            "\"SeIncreaseWorkingSetPrivilege\"",
        ),
        (
            "SW3_SE_LOAD_DRIVER_NAME",
            "SE_LOAD_DRIVER_NAME",
            "\"SeLoadDriverPrivilege\"",
        ),
        (
            "SW3_SE_LOCK_MEMORY_NAME",
            "SE_LOCK_MEMORY_NAME",
            "\"SeLockMemoryPrivilege\"",
        ),
        (
            "SW3_SE_MACHINE_ACCOUNT_NAME",
            "SE_MACHINE_ACCOUNT_NAME",
            "\"SeMachineAccountPrivilege\"",
        ),
        (
            "SW3_SE_MANAGE_VOLUME_NAME",
            "SE_MANAGE_VOLUME_NAME",
            "\"SeManageVolumePrivilege\"",
        ),
        (
            "SW3_SE_PROF_SINGLE_PROCESS_NAME",
            "SE_PROF_SINGLE_PROCESS_NAME",
            "\"SeProfileSingleProcessPrivilege\"",
        ),
        (
            "SW3_SE_RELABEL_NAME",
            "SE_RELABEL_NAME",
            "\"SeRelabelPrivilege\"",
        ),
        (
            "SW3_SE_REMOTE_SHUTDOWN_NAME",
            "SE_REMOTE_SHUTDOWN_NAME",
            "\"SeRemoteShutdownPrivilege\"",
        ),
        (
            "SW3_SE_RESTORE_NAME",
            "SE_RESTORE_NAME",
            "\"SeRestorePrivilege\"",
        ),
        (
            "SW3_SE_SECURITY_NAME",
            "SE_SECURITY_NAME",
            "\"SeSecurityPrivilege\"",
        ),
        (
            "SW3_SE_SHUTDOWN_NAME",
            "SE_SHUTDOWN_NAME",
            "\"SeShutdownPrivilege\"",
        ),
        (
            "SW3_SE_SYNC_AGENT_NAME",
            "SE_SYNC_AGENT_NAME",
            "\"SeSyncAgentPrivilege\"",
        ),
        (
            "SW3_SE_SYSTEM_ENVIRONMENT_NAME",
            "SE_SYSTEM_ENVIRONMENT_NAME",
            "\"SeSystemEnvironmentPrivilege\"",
        ),
        (
            "SW3_SE_SYSTEM_PROFILE_NAME",
            "SE_SYSTEM_PROFILE_NAME",
            "\"SeSystemProfilePrivilege\"",
        ),
        (
            "SW3_SE_SYSTEMTIME_NAME",
            "SE_SYSTEMTIME_NAME",
            "\"SeSystemtimePrivilege\"",
        ),
        (
            "SW3_SE_TAKE_OWNERSHIP_NAME",
            "SE_TAKE_OWNERSHIP_NAME",
            "\"SeTakeOwnershipPrivilege\"",
        ),
        ("SW3_SE_TCB_NAME", "SE_TCB_NAME", "\"SeTcbPrivilege\""),
        (
            "SW3_SE_TIME_ZONE_NAME",
            "SE_TIME_ZONE_NAME",
            "\"SeTimeZonePrivilege\"",
        ),
        (
            "SW3_SE_TRUSTED_CREDMAN_ACCESS_NAME",
            "SE_TRUSTED_CREDMAN_ACCESS_NAME",
            "\"SeTrustedCredManAccessPrivilege\"",
        ),
        (
            "SW3_SE_UNDOCK_NAME",
            "SE_UNDOCK_NAME",
            "\"SeUndockPrivilege\"",
        ),
        (
            "SW3_SE_UNSOLICITED_INPUT_NAME",
            "SE_UNSOLICITED_INPUT_NAME",
            "\"SeUnsolicitedInputPrivilege\"",
        ),
    ];

    for (sw3_name, original_name, value) in &sw3_privilege_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 System Information Class constants */\n");
    let sw3_system_info_constants = [
        ("SW3_SystemBasicInformation", "SystemBasicInformation", "0"),
        (
            "SW3_SystemProcessorInformation",
            "SystemProcessorInformation",
            "1",
        ),
        (
            "SW3_SystemPerformanceInformation",
            "SystemPerformanceInformation",
            "2",
        ),
        (
            "SW3_SystemTimeOfDayInformation",
            "SystemTimeOfDayInformation",
            "3",
        ),
        ("SW3_SystemPathInformation", "SystemPathInformation", "4"),
        (
            "SW3_SystemProcessInformation",
            "SystemProcessInformation",
            "5",
        ),
        (
            "SW3_SystemCallCountInformation",
            "SystemCallCountInformation",
            "6",
        ),
        (
            "SW3_SystemDeviceInformation",
            "SystemDeviceInformation",
            "7",
        ),
        (
            "SW3_SystemProcessorPerformanceInformation",
            "SystemProcessorPerformanceInformation",
            "8",
        ),
        ("SW3_SystemFlagsInformation", "SystemFlagsInformation", "9"),
        (
            "SW3_SystemCallTimeInformation",
            "SystemCallTimeInformation",
            "10",
        ),
        (
            "SW3_SystemModuleInformation",
            "SystemModuleInformation",
            "11",
        ),
        ("SW3_SystemLocksInformation", "SystemLocksInformation", "12"),
        (
            "SW3_SystemStackTraceInformation",
            "SystemStackTraceInformation",
            "13",
        ),
        (
            "SW3_SystemPagedPoolInformation",
            "SystemPagedPoolInformation",
            "14",
        ),
        (
            "SW3_SystemNonPagedPoolInformation",
            "SystemNonPagedPoolInformation",
            "15",
        ),
        (
            "SW3_SystemHandleInformation",
            "SystemHandleInformation",
            "16",
        ),
        (
            "SW3_SystemObjectInformation",
            "SystemObjectInformation",
            "17",
        ),
        (
            "SW3_SystemPageFileInformation",
            "SystemPageFileInformation",
            "18",
        ),
        (
            "SW3_SystemVdmInstemulInformation",
            "SystemVdmInstemulInformation",
            "19",
        ),
        (
            "SW3_SystemVdmBopInformation",
            "SystemVdmBopInformation",
            "20",
        ),
        (
            "SW3_SystemFileCacheInformation",
            "SystemFileCacheInformation",
            "21",
        ),
        (
            "SW3_SystemPoolTagInformation",
            "SystemPoolTagInformation",
            "22",
        ),
        (
            "SW3_SystemInterruptInformation",
            "SystemInterruptInformation",
            "23",
        ),
        (
            "SW3_SystemDpcBehaviorInformation",
            "SystemDpcBehaviorInformation",
            "24",
        ),
        (
            "SW3_SystemFullMemoryInformation",
            "SystemFullMemoryInformation",
            "25",
        ),
        (
            "SW3_SystemLoadGdiDriverInformation",
            "SystemLoadGdiDriverInformation",
            "26",
        ),
        (
            "SW3_SystemUnloadGdiDriverInformation",
            "SystemUnloadGdiDriverInformation",
            "27",
        ),
        (
            "SW3_SystemTimeAdjustmentInformation",
            "SystemTimeAdjustmentInformation",
            "28",
        ),
        (
            "SW3_SystemSummaryMemoryInformation",
            "SystemSummaryMemoryInformation",
            "29",
        ),
        (
            "SW3_SystemMirrorMemoryInformation",
            "SystemMirrorMemoryInformation",
            "30",
        ),
        (
            "SW3_SystemPerformanceTraceInformation",
            "SystemPerformanceTraceInformation",
            "31",
        ),
        ("SW3_SystemObsolete0", "SystemObsolete0", "32"),
        (
            "SW3_SystemExceptionInformation",
            "SystemExceptionInformation",
            "33",
        ),
        (
            "SW3_SystemCrashDumpStateInformation",
            "SystemCrashDumpStateInformation",
            "34",
        ),
        (
            "SW3_SystemKernelDebuggerInformation",
            "SystemKernelDebuggerInformation",
            "35",
        ),
        (
            "SW3_SystemContextSwitchInformation",
            "SystemContextSwitchInformation",
            "36",
        ),
        (
            "SW3_SystemRegistryQuotaInformation",
            "SystemRegistryQuotaInformation",
            "37",
        ),
        (
            "SW3_SystemExtendServiceTableInformation",
            "SystemExtendServiceTableInformation",
            "38",
        ),
        (
            "SW3_SystemPrioritySeperation",
            "SystemPrioritySeperation",
            "39",
        ),
        (
            "SW3_SystemVerifierAddDriverInformation",
            "SystemVerifierAddDriverInformation",
            "40",
        ),
        (
            "SW3_SystemVerifierRemoveDriverInformation",
            "SystemVerifierRemoveDriverInformation",
            "41",
        ),
        (
            "SW3_SystemProcessorIdleInformation",
            "SystemProcessorIdleInformation",
            "42",
        ),
        (
            "SW3_SystemLegacyDriverInformation",
            "SystemLegacyDriverInformation",
            "43",
        ),
        (
            "SW3_SystemCurrentTimeZoneInformation",
            "SystemCurrentTimeZoneInformation",
            "44",
        ),
        (
            "SW3_SystemLookasideInformation",
            "SystemLookasideInformation",
            "45",
        ),
        (
            "SW3_SystemTimeSlipNotification",
            "SystemTimeSlipNotification",
            "46",
        ),
        ("SW3_SystemSessionCreate", "SystemSessionCreate", "47"),
        ("SW3_SystemSessionDetach", "SystemSessionDetach", "48"),
        (
            "SW3_SystemSessionInformation",
            "SystemSessionInformation",
            "49",
        ),
        (
            "SW3_SystemRangeStartInformation",
            "SystemRangeStartInformation",
            "50",
        ),
        (
            "SW3_SystemVerifierInformation",
            "SystemVerifierInformation",
            "51",
        ),
        (
            "SW3_SystemVerifierThunkExtend",
            "SystemVerifierThunkExtend",
            "52",
        ),
        (
            "SW3_SystemSessionProcessInformation",
            "SystemSessionProcessInformation",
            "53",
        ),
        (
            "SW3_SystemLoadGdiDriverInSystemSpace",
            "SystemLoadGdiDriverInSystemSpace",
            "54",
        ),
        ("SW3_SystemNumaProcessorMap", "SystemNumaProcessorMap", "55"),
        (
            "SW3_SystemPrefetcherInformation",
            "SystemPrefetcherInformation",
            "56",
        ),
        (
            "SW3_SystemExtendedProcessInformation",
            "SystemExtendedProcessInformation",
            "57",
        ),
        (
            "SW3_SystemRecommendedSharedDataAlignment",
            "SystemRecommendedSharedDataAlignment",
            "58",
        ),
        ("SW3_SystemComPlusPackage", "SystemComPlusPackage", "59"),
        (
            "SW3_SystemNumaAvailableMemory",
            "SystemNumaAvailableMemory",
            "60",
        ),
        (
            "SW3_SystemProcessorPowerInformation",
            "SystemProcessorPowerInformation",
            "61",
        ),
        (
            "SW3_SystemEmulationBasicInformation",
            "SystemEmulationBasicInformation",
            "62",
        ),
        (
            "SW3_SystemEmulationProcessorInformation",
            "SystemEmulationProcessorInformation",
            "63",
        ),
        (
            "SW3_SystemExtendedHandleInformation",
            "SystemExtendedHandleInformation",
            "64",
        ),
        (
            "SW3_SystemLostDelayedWriteInformation",
            "SystemLostDelayedWriteInformation",
            "65",
        ),
        (
            "SW3_SystemBigPoolInformation",
            "SystemBigPoolInformation",
            "66",
        ),
        (
            "SW3_SystemSessionPoolTagInformation",
            "SystemSessionPoolTagInformation",
            "67",
        ),
        (
            "SW3_SystemSessionMappedViewInformation",
            "SystemSessionMappedViewInformation",
            "68",
        ),
        (
            "SW3_SystemHotpatchInformation",
            "SystemHotpatchInformation",
            "69",
        ),
        (
            "SW3_SystemObjectSecurityMode",
            "SystemObjectSecurityMode",
            "70",
        ),
        (
            "SW3_SystemWatchdogTimerHandler",
            "SystemWatchdogTimerHandler",
            "71",
        ),
        (
            "SW3_SystemWatchdogTimerInformation",
            "SystemWatchdogTimerInformation",
            "72",
        ),
        (
            "SW3_SystemLogicalProcessorInformation",
            "SystemLogicalProcessorInformation",
            "73",
        ),
        (
            "SW3_SystemWow64SharedInformationObsolete",
            "SystemWow64SharedInformationObsolete",
            "74",
        ),
        (
            "SW3_SystemRegisterFirmwareTableInformationHandler",
            "SystemRegisterFirmwareTableInformationHandler",
            "75",
        ),
        (
            "SW3_SystemFirmwareTableInformation",
            "SystemFirmwareTableInformation",
            "76",
        ),
        (
            "SW3_SystemModuleInformationEx",
            "SystemModuleInformationEx",
            "77",
        ),
        (
            "SW3_SystemVerifierTriageInformation",
            "SystemVerifierTriageInformation",
            "78",
        ),
        (
            "SW3_SystemSuperfetchInformation",
            "SystemSuperfetchInformation",
            "79",
        ),
        (
            "SW3_SystemMemoryListInformation",
            "SystemMemoryListInformation",
            "80",
        ),
        (
            "SW3_SystemFileCacheInformationEx",
            "SystemFileCacheInformationEx",
            "81",
        ),
        (
            "SW3_SystemThreadPriorityClientIdInformation",
            "SystemThreadPriorityClientIdInformation",
            "82",
        ),
        (
            "SW3_SystemProcessorIdleCycleTimeInformation",
            "SystemProcessorIdleCycleTimeInformation",
            "83",
        ),
        (
            "SW3_SystemVerifierCancellationInformation",
            "SystemVerifierCancellationInformation",
            "84",
        ),
        (
            "SW3_SystemProcessorPowerInformationEx",
            "SystemProcessorPowerInformationEx",
            "85",
        ),
        (
            "SW3_SystemRefTraceInformation",
            "SystemRefTraceInformation",
            "86",
        ),
        (
            "SW3_SystemSpecialPoolInformation",
            "SystemSpecialPoolInformation",
            "87",
        ),
        (
            "SW3_SystemProcessIdInformation",
            "SystemProcessIdInformation",
            "88",
        ),
        (
            "SW3_SystemErrorPortInformation",
            "SystemErrorPortInformation",
            "89",
        ),
        (
            "SW3_SystemBootEnvironmentInformation",
            "SystemBootEnvironmentInformation",
            "90",
        ),
        (
            "SW3_SystemHypervisorInformation",
            "SystemHypervisorInformation",
            "91",
        ),
        (
            "SW3_SystemVerifierInformationEx",
            "SystemVerifierInformationEx",
            "92",
        ),
        (
            "SW3_SystemTimeZoneInformation",
            "SystemTimeZoneInformation",
            "93",
        ),
        (
            "SW3_SystemImageFileExecutionOptionsInformation",
            "SystemImageFileExecutionOptionsInformation",
            "94",
        ),
        (
            "SW3_SystemCoverageInformation",
            "SystemCoverageInformation",
            "95",
        ),
        (
            "SW3_SystemPrefetchPatchInformation",
            "SystemPrefetchPatchInformation",
            "96",
        ),
        (
            "SW3_SystemVerifierFaultsInformation",
            "SystemVerifierFaultsInformation",
            "97",
        ),
        (
            "SW3_SystemSystemPartitionInformation",
            "SystemSystemPartitionInformation",
            "98",
        ),
        (
            "SW3_SystemSystemDiskInformation",
            "SystemSystemDiskInformation",
            "99",
        ),
        (
            "SW3_SystemProcessorPerformanceDistribution",
            "SystemProcessorPerformanceDistribution",
            "100",
        ),
        (
            "SW3_SystemNumaProximityNodeInformation",
            "SystemNumaProximityNodeInformation",
            "101",
        ),
        (
            "SW3_SystemDynamicTimeZoneInformation",
            "SystemDynamicTimeZoneInformation",
            "102",
        ),
        (
            "SW3_SystemCodeIntegrityInformation",
            "SystemCodeIntegrityInformation",
            "103",
        ),
        (
            "SW3_SystemProcessorMicrocodeUpdateInformation",
            "SystemProcessorMicrocodeUpdateInformation",
            "104",
        ),
        (
            "SW3_SystemProcessorBrandString",
            "SystemProcessorBrandString",
            "105",
        ),
        (
            "SW3_SystemVirtualAddressInformation",
            "SystemVirtualAddressInformation",
            "106",
        ),
        (
            "SW3_SystemLogicalProcessorAndGroupInformation",
            "SystemLogicalProcessorAndGroupInformation",
            "107",
        ),
        (
            "SW3_SystemProcessorCycleTimeInformation",
            "SystemProcessorCycleTimeInformation",
            "108",
        ),
        (
            "SW3_SystemStoreInformation",
            "SystemStoreInformation",
            "109",
        ),
        (
            "SW3_SystemRegistryAppendString",
            "SystemRegistryAppendString",
            "110",
        ),
        (
            "SW3_SystemAitSamplingValue",
            "SystemAitSamplingValue",
            "111",
        ),
        (
            "SW3_SystemVhdBootInformation",
            "SystemVhdBootInformation",
            "112",
        ),
        (
            "SW3_SystemCpuQuotaInformation",
            "SystemCpuQuotaInformation",
            "113",
        ),
        (
            "SW3_SystemNativeBasicInformation",
            "SystemNativeBasicInformation",
            "114",
        ),
        (
            "SW3_SystemErrorPortTimeouts",
            "SystemErrorPortTimeouts",
            "115",
        ),
        (
            "SW3_SystemLowPriorityIoInformation",
            "SystemLowPriorityIoInformation",
            "116",
        ),
        (
            "SW3_SystemTpmBootEntropyInformation",
            "SystemTpmBootEntropyInformation",
            "117",
        ),
        (
            "SW3_SystemVerifierCountersInformation",
            "SystemVerifierCountersInformation",
            "118",
        ),
        (
            "SW3_SystemPagedPoolInformationEx",
            "SystemPagedPoolInformationEx",
            "119",
        ),
        (
            "SW3_SystemSystemPtesInformationEx",
            "SystemSystemPtesInformationEx",
            "120",
        ),
        (
            "SW3_SystemNodeDistanceInformation",
            "SystemNodeDistanceInformation",
            "121",
        ),
        (
            "SW3_SystemAcpiAuditInformation",
            "SystemAcpiAuditInformation",
            "122",
        ),
        (
            "SW3_SystemBasicPerformanceInformation",
            "SystemBasicPerformanceInformation",
            "123",
        ),
        (
            "SW3_SystemQueryPerformanceCounterInformation",
            "SystemQueryPerformanceCounterInformation",
            "124",
        ),
        (
            "SW3_SystemSessionBigPoolInformation",
            "SystemSessionBigPoolInformation",
            "125",
        ),
        (
            "SW3_SystemBootGraphicsInformation",
            "SystemBootGraphicsInformation",
            "126",
        ),
        (
            "SW3_SystemScrubPhysicalMemoryInformation",
            "SystemScrubPhysicalMemoryInformation",
            "127",
        ),
        (
            "SW3_SystemBadPageInformation",
            "SystemBadPageInformation",
            "128",
        ),
        (
            "SW3_SystemProcessorProfileControlArea",
            "SystemProcessorProfileControlArea",
            "129",
        ),
        (
            "SW3_SystemCombinePhysicalMemoryInformation",
            "SystemCombinePhysicalMemoryInformation",
            "130",
        ),
        (
            "SW3_SystemEntropyInterruptTimingInformation",
            "SystemEntropyInterruptTimingInformation",
            "131",
        ),
        (
            "SW3_SystemConsoleInformation",
            "SystemConsoleInformation",
            "132",
        ),
        (
            "SW3_SystemPlatformBinaryInformation",
            "SystemPlatformBinaryInformation",
            "133",
        ),
        (
            "SW3_SystemPolicyInformation",
            "SystemPolicyInformation",
            "134",
        ),
        (
            "SW3_SystemHypervisorProcessorCountInformation",
            "SystemHypervisorProcessorCountInformation",
            "135",
        ),
        (
            "SW3_SystemDeviceDataInformation",
            "SystemDeviceDataInformation",
            "136",
        ),
        (
            "SW3_SystemDeviceDataEnumerationInformation",
            "SystemDeviceDataEnumerationInformation",
            "137",
        ),
        (
            "SW3_SystemMemoryTopologyInformation",
            "SystemMemoryTopologyInformation",
            "138",
        ),
        (
            "SW3_SystemMemoryChannelInformation",
            "SystemMemoryChannelInformation",
            "139",
        ),
        (
            "SW3_SystemBootLogoInformation",
            "SystemBootLogoInformation",
            "140",
        ),
        (
            "SW3_SystemProcessorPerformanceInformationEx",
            "SystemProcessorPerformanceInformationEx",
            "141",
        ),
        (
            "SW3_SystemCriticalProcessErrorLogInformation",
            "SystemCriticalProcessErrorLogInformation",
            "142",
        ),
        (
            "SW3_SystemSecureBootPolicyInformation",
            "SystemSecureBootPolicyInformation",
            "143",
        ),
        (
            "SW3_SystemPageFileInformationEx",
            "SystemPageFileInformationEx",
            "144",
        ),
        (
            "SW3_SystemSecureBootInformation",
            "SystemSecureBootInformation",
            "145",
        ),
        (
            "SW3_SystemEntropyInterruptTimingRawInformation",
            "SystemEntropyInterruptTimingRawInformation",
            "146",
        ),
        (
            "SW3_SystemPortableWorkspaceEfiLauncherInformation",
            "SystemPortableWorkspaceEfiLauncherInformation",
            "147",
        ),
        (
            "SW3_SystemFullProcessInformation",
            "SystemFullProcessInformation",
            "148",
        ),
        (
            "SW3_SystemKernelDebuggerInformationEx",
            "SystemKernelDebuggerInformationEx",
            "149",
        ),
        (
            "SW3_SystemBootMetadataInformation",
            "SystemBootMetadataInformation",
            "150",
        ),
        (
            "SW3_SystemSoftRebootInformation",
            "SystemSoftRebootInformation",
            "151",
        ),
        (
            "SW3_SystemElamCertificateInformation",
            "SystemElamCertificateInformation",
            "152",
        ),
        (
            "SW3_SystemOfflineDumpConfigInformation",
            "SystemOfflineDumpConfigInformation",
            "153",
        ),
        (
            "SW3_SystemProcessorFeaturesInformation",
            "SystemProcessorFeaturesInformation",
            "154",
        ),
        (
            "SW3_SystemRegistryReconciliationInformation",
            "SystemRegistryReconciliationInformation",
            "155",
        ),
        ("SW3_SystemEdidInformation", "SystemEdidInformation", "156"),
        (
            "SW3_SystemManufacturingInformation",
            "SystemManufacturingInformation",
            "157",
        ),
        (
            "SW3_SystemEnergyEstimationConfigInformation",
            "SystemEnergyEstimationConfigInformation",
            "158",
        ),
        (
            "SW3_SystemHypervisorDetailInformation",
            "SystemHypervisorDetailInformation",
            "159",
        ),
        (
            "SW3_SystemProcessorCycleStatsInformation",
            "SystemProcessorCycleStatsInformation",
            "160",
        ),
        (
            "SW3_SystemVmGenerationCountInformation",
            "SystemVmGenerationCountInformation",
            "161",
        ),
        (
            "SW3_SystemTrustedPlatformModuleInformation",
            "SystemTrustedPlatformModuleInformation",
            "162",
        ),
        (
            "SW3_SystemKernelDebuggerFlags",
            "SystemKernelDebuggerFlags",
            "163",
        ),
        (
            "SW3_SystemCodeIntegrityPolicyInformation",
            "SystemCodeIntegrityPolicyInformation",
            "164",
        ),
        (
            "SW3_SystemIsolatedUserModeInformation",
            "SystemIsolatedUserModeInformation",
            "165",
        ),
        (
            "SW3_SystemHardwareSecurityTestInterfaceResultsInformation",
            "SystemHardwareSecurityTestInterfaceResultsInformation",
            "166",
        ),
        (
            "SW3_SystemSingleModuleInformation",
            "SystemSingleModuleInformation",
            "167",
        ),
        (
            "SW3_SystemAllowedCpuSetsInformation",
            "SystemAllowedCpuSetsInformation",
            "168",
        ),
        (
            "SW3_SystemVsmProtectionInformation",
            "SystemVsmProtectionInformation",
            "169",
        ),
        (
            "SW3_SystemInterruptCpuSetsInformation",
            "SystemInterruptCpuSetsInformation",
            "170",
        ),
        (
            "SW3_SystemSecureBootPolicyFullInformation",
            "SystemSecureBootPolicyFullInformation",
            "171",
        ),
        (
            "SW3_SystemCodeIntegrityPolicyFullInformation",
            "SystemCodeIntegrityPolicyFullInformation",
            "172",
        ),
        (
            "SW3_SystemAffinitizedInterruptProcessorInformation",
            "SystemAffinitizedInterruptProcessorInformation",
            "173",
        ),
        (
            "SW3_SystemRootSiloInformation",
            "SystemRootSiloInformation",
            "174",
        ),
        (
            "SW3_SystemCpuSetInformation",
            "SystemCpuSetInformation",
            "175",
        ),
        (
            "SW3_SystemCpuSetTagInformation",
            "SystemCpuSetTagInformation",
            "176",
        ),
        (
            "SW3_SystemWin32WerStartCallout",
            "SystemWin32WerStartCallout",
            "177",
        ),
        (
            "SW3_SystemSecureKernelProfileInformation",
            "SystemSecureKernelProfileInformation",
            "178",
        ),
        (
            "SW3_SystemCodeIntegrityPlatformManifestInformation",
            "SystemCodeIntegrityPlatformManifestInformation",
            "179",
        ),
        (
            "SW3_SystemInterruptSteeringInformation",
            "SystemInterruptSteeringInformation",
            "180",
        ),
        (
            "SW3_SystemSupportedProcessorArchitectures",
            "SystemSupportedProcessorArchitectures",
            "181",
        ),
        (
            "SW3_SystemMemoryUsageInformation",
            "SystemMemoryUsageInformation",
            "182",
        ),
        (
            "SW3_SystemCodeIntegrityCertificateInformation",
            "SystemCodeIntegrityCertificateInformation",
            "183",
        ),
        (
            "SW3_SystemPhysicalMemoryInformation",
            "SystemPhysicalMemoryInformation",
            "184",
        ),
        (
            "SW3_SystemControlFlowTransition",
            "SystemControlFlowTransition",
            "185",
        ),
        (
            "SW3_SystemKernelDebuggingAllowed",
            "SystemKernelDebuggingAllowed",
            "186",
        ),
        (
            "SW3_SystemActivityModerationExeState",
            "SystemActivityModerationExeState",
            "187",
        ),
        (
            "SW3_SystemActivityModerationUserSettings",
            "SystemActivityModerationUserSettings",
            "188",
        ),
        (
            "SW3_SystemCodeIntegrityPoliciesFullInformation",
            "SystemCodeIntegrityPoliciesFullInformation",
            "189",
        ),
        (
            "SW3_SystemCodeIntegrityUnlockInformation",
            "SystemCodeIntegrityUnlockInformation",
            "190",
        ),
        (
            "SW3_SystemIntegrityQuotaInformation",
            "SystemIntegrityQuotaInformation",
            "191",
        ),
        (
            "SW3_SystemFlushInformation",
            "SystemFlushInformation",
            "192",
        ),
        (
            "SW3_SystemProcessorIdleMaskInformation",
            "SystemProcessorIdleMaskInformation",
            "193",
        ),
        (
            "SW3_SystemSecureDumpEncryptionInformation",
            "SystemSecureDumpEncryptionInformation",
            "194",
        ),
        (
            "SW3_SystemWriteConstraintInformation",
            "SystemWriteConstraintInformation",
            "195",
        ),
        (
            "SW3_SystemKernelVaShadowInformation",
            "SystemKernelVaShadowInformation",
            "196",
        ),
        (
            "SW3_SystemHypervisorSharedPageInformation",
            "SystemHypervisorSharedPageInformation",
            "197",
        ),
        (
            "SW3_SystemFirmwareBootPerformanceInformation",
            "SystemFirmwareBootPerformanceInformation",
            "198",
        ),
        (
            "SW3_SystemCodeIntegrityVerificationInformation",
            "SystemCodeIntegrityVerificationInformation",
            "199",
        ),
        (
            "SW3_SystemFirmwarePartitionInformation",
            "SystemFirmwarePartitionInformation",
            "200",
        ),
        (
            "SW3_SystemSpeculationControlInformation",
            "SystemSpeculationControlInformation",
            "201",
        ),
        (
            "SW3_SystemDmaGuardPolicyInformation",
            "SystemDmaGuardPolicyInformation",
            "202",
        ),
        (
            "SW3_SystemEnclaveLaunchControlInformation",
            "SystemEnclaveLaunchControlInformation",
            "203",
        ),
        (
            "SW3_SystemWorkloadAllowedCpuSetsInformation",
            "SystemWorkloadAllowedCpuSetsInformation",
            "204",
        ),
        (
            "SW3_SystemCodeIntegrityUnlockModeInformation",
            "SystemCodeIntegrityUnlockModeInformation",
            "205",
        ),
        (
            "SW3_SystemLeapSecondInformation",
            "SystemLeapSecondInformation",
            "206",
        ),
        (
            "SW3_SystemFlags2Information",
            "SystemFlags2Information",
            "207",
        ),
        (
            "SW3_SystemSecurityModelInformation",
            "SystemSecurityModelInformation",
            "208",
        ),
        (
            "SW3_SystemCodeIntegritySyntheticCacheInformation",
            "SystemCodeIntegritySyntheticCacheInformation",
            "209",
        ),
        ("SW3_MaxSystemInfoClass", "MaxSystemInfoClass", "210"),
    ];

    for (sw3_name, original_name, value) in &sw3_system_info_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Information class constants */\n");
    let sw3_info_constants = [
        ("SW3_NotificationEvent", "NotificationEvent", "0"),
        ("SW3_SynchronizationEvent", "SynchronizationEvent", "1"),
        ("SW3_ViewShare", "ViewShare", "1"),
        ("SW3_ViewUnmap", "ViewUnmap", "2"),
        (
            "SW3_KeyValueFullInformation",
            "KeyValueFullInformation",
            "1",
        ),
        (
            "SW3_ProcessBasicInformation",
            "ProcessBasicInformation",
            "0",
        ),
        ("SW3_ThreadBasicInformation", "ThreadBasicInformation", "0"),
        ("SW3_TokenStatistics", "TokenStatistics", "10"),
        ("SW3_FileBasicInformation", "FileBasicInformation", "4"),
        ("SW3_FALSE", "FALSE", "0"),
        ("SW3_TRUE", "TRUE", "1"),
    ];

    for (sw3_name, original_name, value) in &sw3_info_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push_str("\n/* SW3 Object attributes constants */\n");
    let sw3_obj_constants = [
        ("SW3_OBJ_INHERIT", "OBJ_INHERIT", "0x00000002"),
        ("SW3_OBJ_PERMANENT", "OBJ_PERMANENT", "0x00000010"),
        ("SW3_OBJ_EXCLUSIVE", "OBJ_EXCLUSIVE", "0x00000020"),
        (
            "SW3_OBJ_CASE_INSENSITIVE",
            "OBJ_CASE_INSENSITIVE",
            "0x00000040",
        ),
        ("SW3_OBJ_OPENIF", "OBJ_OPENIF", "0x00000080"),
        ("SW3_OBJ_OPENLINK", "OBJ_OPENLINK", "0x00000100"),
        ("SW3_OBJ_KERNEL_HANDLE", "OBJ_KERNEL_HANDLE", "0x00000200"),
        (
            "SW3_OBJ_FORCE_ACCESS_CHECK",
            "OBJ_FORCE_ACCESS_CHECK",
            "0x00000400",
        ),
        (
            "SW3_OBJ_VALID_ATTRIBUTES",
            "OBJ_VALID_ATTRIBUTES",
            "0x000007F2",
        ),
    ];

    for (sw3_name, original_name, value) in &sw3_obj_constants {
        h.push_str(&format!("#define {} {}\n", sw3_name, value));
        h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
        h.push_str(&format!("#ifndef {}\n", original_name));
        h.push_str(&format!("#define {} {}\n", original_name, sw3_name));
        h.push_str("#endif\n");
        h.push_str("#endif\n");
    }

    h.push('\n');
}

/// Generate function declarations with SW3 prefixes
fn generate_functions(h: &mut String, functions: &[Function]) {
    h.push_str("/* ==================== ALL SYSCALL FUNCTIONS ==================== */\n");
    h.push_str("/* Architecture: Supports both x86 and x64 */\n");
    h.push_str("/* WOW64: Automatically detected and handled */\n");
    h.push_str("/* Function names: All prefixed with SW3 for maximum compatibility */\n");

    // Since all our functions have SW3_ prefix, they don't conflict with Windows SDK
    // Generate ALL functions in the main block

    h.push_str(&format!(
        "/* Total functions: {} (all with SW3_ prefix for compatibility) */\n\n",
        functions.len()
    ));

    // Generate ALL functions with SW3 prefix - no conflicts possible
    for func in functions {
        // Convert nt_xxx to NtXxx, then add SW3 prefix
        let c_name = to_pascal_case(&func.name);
        let sw3_func_name = format!("SW3{}", c_name);
        let ret_type = rust_to_c_type(&func.return_type);

        // Build parameter list with special handling for enum-like parameters
        let params: Vec<String> = func
            .params
            .iter()
            .filter_map(|p| {
                // Skip parameters that cause anonymous class errors
                if p.name.starts_with("_") || p.typ.contains("enum ") || p.typ.contains("struct ") {
                    return None;
                }

                let mut c_type = rust_to_c_type(&p.typ);

                // Convert enum-like u32 parameters to proper types
                if p.typ == "u32" && p.name.ends_with("_class") {
                    c_type = "uint32_t".to_string();
                }

                Some(format!("{} {}", c_type, p.name))
            })
            .collect();

        let params_str = if params.is_empty() {
            "void".to_string()
        } else {
            params.join(", ")
        };

        h.push_str(&format!(
            "{} {}({});\n",
            ret_type, sw3_func_name, params_str
        ));
    }

    // Add compatibility aliases for all functions
    h.push_str(
        "\n/* ==================== FUNCTION COMPATIBILITY ALIASES ==================== */\n",
    );
    h.push_str(
        "/* Aliases for backward compatibility - map original names to SW3 prefixed names */\n\n",
    );

    h.push_str("#if !SYSCALLS_WINDOWS_SDK_DETECTED\n");
    h.push_str("/* Function aliases - only when Windows SDK not detected */\n\n");

    // Generate aliases for all functions
    for func in functions {
        let c_name = to_pascal_case(&func.name);
        let sw3_func_name = format!("SW3{}", c_name);

        h.push_str(&format!("#ifndef {}\n", c_name));
        h.push_str(&format!("#define {} {}\n", c_name, sw3_func_name));
        h.push_str("#endif\n");
    }

    h.push_str("\n#endif /* !SYSCALLS_WINDOWS_SDK_DETECTED */\n");

    h.push_str(&format!(
        "\n/* End of {} total syscall functions with SW3 prefixes */\n",
        functions.len()
    ));
    h.push_str("/* Note: All function names are prefixed with SW3 for maximum compatibility */\n");
    h.push_str(
        "/* Note: All functions are always available regardless of Windows SDK detection */\n",
    );
    h.push_str("/* Note: Compatibility aliases are provided when Windows SDK is not detected */\n");
    h.push_str("/* Note: Syscall numbers are automatically selected based on architecture */\n");
    h.push_str("/* x64: Uses x64 syscall numbers */\n");
    h.push_str("/* x86: Uses x86 syscall numbers */\n");
    h.push_str("/* WOW64: Uses x86 syscall numbers with special handling */\n");
}

/// Generate C-compatible Rust wrappers for all functions
fn generate_c_wrappers(functions: &[Function]) -> String {
    let mut w = String::new();

    w.push_str("// Auto-generated C wrappers for syscall functions\n");
    w.push_str("// DO NOT EDIT MANUALLY - Generated by build.rs\n\n");

    w.push_str("#[allow(unused_imports)]\n");
    w.push_str("use syscalls::*;\n\n");

    // Generate helper functions first (only once, not in the loop)
    w.push_str("// Helper functions for SysWhispers3 runtime\n");
    w.push_str("/// Debug function to get syscall list count\n");
    w.push_str("#[unsafe(no_mangle)]\n");
    w.push_str("pub unsafe extern \"C\" fn SW3Sw3DebugGetCount() -> u32 {\n");
    w.push_str("    sw3_debug_get_count()\n");
    w.push_str("}\n\n");

    w.push_str("/// Debug function to get hash at index\n");
    w.push_str("#[unsafe(no_mangle)]\n");
    w.push_str("pub unsafe extern \"C\" fn SW3Sw3DebugGetHash(index: usize) -> u32 {\n");
    w.push_str("    sw3_debug_get_hash(index)\n");
    w.push_str("}\n\n");

    w.push_str("/// Debug function to get syscall address at index\n");
    w.push_str("#[unsafe(no_mangle)]\n");
    w.push_str("pub unsafe extern \"C\" fn SW3Sw3DebugGetSyscallAddr(index: usize) -> *mut core::ffi::c_void {\n");
    w.push_str("    sw3_debug_get_syscall_addr(index)\n");
    w.push_str("}\n\n");

    w.push_str("/// Get syscall address for jumper mode\n");
    w.push_str("#[unsafe(no_mangle)]\n");
    w.push_str("pub unsafe extern \"C\" fn SW3Sw3GetSyscallAddress(function_hash: u32) -> *mut core::ffi::c_void {\n");
    w.push_str("    sw3_get_syscall_address(function_hash)\n");
    w.push_str("}\n\n");

    w.push_str("/// Get random syscall address for return address spoofing\n");
    w.push_str("#[unsafe(no_mangle)]\n");
    w.push_str("pub unsafe extern \"C\" fn SW3Sw3GetRandomSyscallAddress(function_hash: u32) -> *mut core::ffi::c_void {\n");
    w.push_str("    sw3_get_random_syscall_address(function_hash)\n");
    w.push_str("}\n\n");

    w.push_str("/// Get syscall number for function hash\n");
    w.push_str("#[unsafe(no_mangle)]\n");
    w.push_str("pub unsafe extern \"C\" fn SW3Sw3GetSyscallNumber(function_hash: u32) -> u32 {\n");
    w.push_str("    sw3_get_syscall_number(function_hash)\n");
    w.push_str("}\n\n");

    // Filter out helper functions from syscall functions to avoid duplication
    let helper_functions = [
        "sw3_debug_get_count",
        "sw3_debug_get_hash",
        "sw3_debug_get_syscall_addr",
        "sw3_get_syscall_address",
        "sw3_get_random_syscall_address",
        "sw3_get_syscall_number",
        "sw3_populate_syscall_list",
        "sw3_hash_syscall",
        "sw3_resolve_syscalls_from_ldr_data_table_entry",
        "sw3_resolve_syscalls_from_export_table",
    ];

    // Generate wrappers only for actual NT syscall functions (not helper functions)
    let mut syscall_count = 0;
    for func in functions {
        let rust_func_name = &func.name;

        // Skip helper functions to avoid duplication
        if helper_functions.contains(&rust_func_name.as_str()) {
            continue;
        }

        // Only wrap NT syscalls (functions starting with "nt_")
        if !rust_func_name.starts_with("nt_") {
            continue;
        }

        let c_name = to_pascal_case(&func.name);
        let sw3_func_name = format!("SW3{}", c_name);

        // Build parameter list for wrapper
        let params: Vec<String> = func
            .params
            .iter()
            .filter_map(|p| {
                // Skip parameters that cause issues
                if p.name.starts_with("_") || p.typ.contains("enum ") || p.typ.contains("struct ") {
                    return None;
                }

                let rust_type = &p.typ;
                Some(format!("{}: {}", p.name, rust_type))
            })
            .collect();

        let param_names: Vec<String> = func
            .params
            .iter()
            .filter_map(|p| {
                if p.name.starts_with("_") || p.typ.contains("enum ") || p.typ.contains("struct ") {
                    return None;
                }
                Some(p.name.clone())
            })
            .collect();

        let params_str = if params.is_empty() {
            String::new()
        } else {
            params.join(", ")
        };

        let call_params = param_names.join(", ");
        let return_type = &func.return_type;

        w.push_str(&format!("/// C wrapper for {}\n", rust_func_name));
        w.push_str("#[unsafe(no_mangle)]\n");
        w.push_str(&format!(
            "pub unsafe extern \"C\" fn {}({}) -> {} {{\n",
            sw3_func_name, params_str, return_type
        ));
        w.push_str(&format!("    {}({})\n", rust_func_name, call_params));
        w.push_str("}\n\n");

        syscall_count += 1;
    }

    w.push_str(&format!(
        "// Generated {} C wrappers for NT syscall functions\n",
        syscall_count
    ));
    w.push_str("// Helper functions: 6 (SW3Sw3DebugGetCount, SW3Sw3DebugGetHash, SW3Sw3DebugGetSyscallAddr, SW3Sw3GetSyscallAddress, SW3Sw3GetRandomSyscallAddress, SW3Sw3GetSyscallNumber)\n");
    w.push_str(&format!(
        "// Total exported functions: {}\n",
        syscall_count + 6
    ));

    w
}
