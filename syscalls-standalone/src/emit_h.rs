//! Emit standalone `syscalls.h` — X-prefixed types and function declarations.
//!
//! Zero dependency on `<windows.h>`. All names use the `X` / `X_` prefix so
//! the bundle drops into a project that already includes the Windows SDK.

use crate::parse::{Function, Parsed, rust_to_c_type};

use std::fmt::Write as _;

pub fn emit(p: &Parsed) -> String {
    let mut h = String::new();

    preamble(&mut h);
    forward_struct_decls(&mut h, p);
    type_aliases(&mut h, p);

    struct_bodies(&mut h, p);
    constants(&mut h, p);
    runtime_helpers(&mut h);
    function_decls(&mut h, &p.functions);
    helper_macros(&mut h);
    footer(&mut h);

    h
}

fn preamble(h: &mut String) {
    h.push_str(
        r#"/*
 * X-Syscalls -- direct NT syscall bundle (drop-in for MSVC, CRT-free)
 *
 * Auto-generated -- DO NOT EDIT MANUALLY.
 *
 * Ships with:
 *   syscalls.h              -- this header
 *   syscalls.c              -- runtime (PEB walk, hash table)
 *   syscallsstubs.x64.asm   -- MASM stubs for x64
 *   syscallsstubs.x86.c     -- MSVC naked stubs for x86 (+ WoW64 gate)
 *
 * Every symbol uses an `X` prefix so nothing collides with <windows.h>.
 */

#ifndef X_SYSCALLS_H
#define X_SYSCALLS_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* Calling convention for NT syscall stubs:
 * on x86 the syscall wrappers exposed by ntdll are __stdcall (callee cleans);
 * on x64 there's only one C convention so the qualifier is empty. */
#if defined(_M_X64) || defined(__x86_64__)
#  define X_STDCALL
#elif defined(_M_IX86) || defined(__i386__)
#  define X_STDCALL __stdcall
#else
#  define X_STDCALL
#endif

#ifdef __cplusplus
extern "C" {
#endif

"#,
    );
}

/// Every parsed struct gets a forward declaration so pointer aliases defined
/// before the body still compile.
fn forward_struct_decls(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Forward struct decls ==================== */\n\n");
    for (name, _) in &p.structs {
        writeln!(h, "typedef struct _X_{n} X_{n};", n = name).unwrap();
    }
    h.push('\n');
}

fn type_aliases(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Type aliases ==================== */\n\n");
    for (name, rust_type) in &p.types {
        let c = rust_to_c_type(rust_type);
        writeln!(h, "typedef {} X_{};", c, name).unwrap();
    }
    h.push('\n');
}

fn struct_bodies(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Struct definitions ==================== */\n\n");
    for (name, fields) in &p.structs {
        writeln!(h, "struct _X_{} {{", name).unwrap();
        if fields.is_empty() {
            writeln!(h, "    uint32_t X_reserved;").unwrap();
        } else {
            for field in fields {
                let fname = &field.name;
                let ftype = &field.typ;
                if let Some(condition) = field.condition {
                    writeln!(h, "#if {}", condition).unwrap();
                }
                if let Some((elem, count)) = parse_array(ftype) {
                    let c = rust_to_c_type(&elem);
                    writeln!(h, "    {} {}[{}];", c, fname, count).unwrap();
                } else {
                    let c = rust_to_c_type(ftype);
                    writeln!(h, "    {} {};", c, fname).unwrap();
                }
                if field.condition.is_some() {
                    writeln!(h, "#endif").unwrap();
                }
            }
        }
        writeln!(h, "}};\n").unwrap();
    }
}

fn parse_array(t: &str) -> Option<(String, String)> {
    let t = t.trim();
    if !(t.starts_with('[') && t.ends_with(']') && t.contains(';')) {
        return None;
    }
    let inner = &t[1..t.len() - 1];
    let (elem, count) = inner.split_once(';')?;
    Some((elem.trim().to_string(), count.trim().to_string()))
}

fn constants(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Constants (#define) ==================== */\n\n");
    for (name, _typ, value) in &p.constants {
        writeln!(h, "#define X_{} {}", name, value).unwrap();
    }
    h.push('\n');
}

fn runtime_helpers(h: &mut String) {
    h.push_str(r#"/* ==================== Runtime helpers (implemented in syscalls.c) ==================== */
/* NB: helpers use the default calling convention (cdecl on x86, single conv on
 * x64) -- the stubs call them with matching signatures. */

uint32_t X_GetSyscallNumber(uint32_t function_hash);
void*    X_GetSyscallAddress(uint32_t function_hash);
void*    X_GetRandomSyscallAddress(uint32_t function_hash);
uint32_t X_DebugGetCount(void);
uint32_t X_DebugGetHash(size_t index);
void*    X_DebugGetSyscallAddr(size_t index);

"#);
}

fn function_decls(h: &mut String, functions: &[Function]) {
    h.push_str("/* ==================== NT syscall declarations ==================== */\n\n");
    for f in functions {
        let ret = rust_to_c_type(&f.return_type);
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{} {}", rust_to_c_type(&p.typ), p.name))
            .collect();
        let params_str = if params.is_empty() {
            "void".to_string()
        } else {
            params.join(", ")
        };
        writeln!(h, "{} X_STDCALL X{}({});", ret, f.pascal, params_str).unwrap();
    }
    h.push('\n');
}

fn helper_macros(h: &mut String) {
    h.push_str(
        r#"/* ==================== Helper macros ==================== */

#define X_NT_SUCCESS(s)     ((X_NTSTATUS)(s) >= 0)
#define X_NT_INFORMATION(s) ((((uint32_t)(s)) >> 30) == 1u)
#define X_NT_WARNING(s)     ((((uint32_t)(s)) >> 30) == 2u)
#define X_NT_ERROR(s)       ((((uint32_t)(s)) >> 30) == 3u)

#define X_NtCurrentProcess() ((X_HANDLE)(intptr_t)-1)
#define X_NtCurrentThread()  ((X_HANDLE)(intptr_t)-2)

#define X_InitializeObjectAttributes(p, n, a, r, s) do { \
    (p)->Length = sizeof(X_OBJECT_ATTRIBUTES); \
    (p)->RootDirectory = (r); \
    (p)->Attributes = (a); \
    (p)->ObjectName = (n); \
    (p)->SecurityDescriptor = (s); \
    (p)->SecurityQualityOfService = NULL; \
} while (0)

"#,
    );
}

fn footer(h: &mut String) {
    h.push_str(
        r#"#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* X_SYSCALLS_H */
"#,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_information_preserves_architecture_condition() {
        let parsed = crate::parse::parse(include_str!("../../lib.rs")).unwrap();
        let header = emit(&parsed);
        assert!(header.contains(
            "#if defined(_M_X64) || defined(__x86_64__)\n    X_USHORT PartitionId;\n#endif\n    X_SIZE_T RegionSize;"
        ));
    }

    #[test]
    fn conditional_arrays_and_unconditional_fields_are_preserved() {
        let parsed = crate::parse::parse(
            r#"
            #[repr(C)]
            pub struct Example {
                pub common: u32,
                #[cfg(target_arch = "x86")]
                pub words: [u16; 2],
                pub tail: u8,
            }
        "#,
        )
        .unwrap();
        let header = emit(&parsed);
        assert!(header.contains("    uint32_t common;\n#if defined(_M_IX86) || defined(__i386__)\n    uint16_t words[2];\n#endif\n    uint8_t tail;"));
    }

    #[test]
    fn unsupported_field_condition_is_not_silently_erased() {
        let result = crate::parse::parse(
            r#"
            #[repr(C)]
            pub struct Example {
                #[cfg(feature = "optional")]
                pub value: u32,
            }
        "#,
        );
        assert!(
            matches!(result, Err(error) if error.contains("unsupported attributes on field Example.value"))
        );
    }
}
