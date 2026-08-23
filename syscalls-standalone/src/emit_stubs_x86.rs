//! Emit `syscallsstubs.x86.c` -- MSVC `__declspec(naked)` stubs with inline
//! `__asm`. Runtime-detects WoW64 (fs:[0C0h] != 0) and dispatches through the
//! gate or via a native `sysenter; ret` in ntdll.

use crate::parse::{Function, rust_to_c_type};
use std::fmt::Write as _;

pub fn emit(functions: &[Function]) -> String {
    let mut w = String::with_capacity(functions.len() * 800);
    w.push_str(HEADER);
    for f in functions {
        emit_stub(&mut w, f);
    }
    w.push_str("\n#pragma warning(pop)\n");
    w
}

const HEADER: &str = r####"/*
 * X-Syscalls -- x86 (stdcall) direct-syscall stubs
 *
 * Auto-generated -- DO NOT EDIT MANUALLY.
 *
 * Each stub:
 *   1. fetches the syscall number via X_GetSyscallNumber,
 *   2. detects WoW64 via fs:[0C0h] (WOW64Reserved gate ptr),
 *   3. dispatches through the WoW64 gate or via a native `sysenter; ret`
 *      returned by X_GetRandomSyscallAddress (jumper_randomized).
 *
 * Requires MSVC (uses __declspec(naked) + __asm). Static (in-tree) linkage.
 */

#include "syscalls.h"

extern uint32_t X_GetSyscallNumber(uint32_t function_hash);
extern void*    X_GetRandomSyscallAddress(uint32_t function_hash);

/* Warn: __asm blocks below assume esp is preserved by MSVC across the naked
 * function body (they don't reference local C variables). */
#pragma warning(push)
#pragma warning(disable: 4100) /* unreferenced formal parameter -- args are read via __asm */
#pragma warning(disable: 4189) /* local variable initialized but not referenced */

"####;

fn emit_stub(w: &mut String, f: &Function) {
    let name = format!("X{}", f.pascal);
    // MASM/MSVC __asm hex literal: leading `0` required when the first digit
    // would be A-F, otherwise the tokenizer treats the value as an identifier.
    let hash_hex = {
        let raw = format!("{:X}", f.zw_hash);
        if raw.chars().next().unwrap().is_ascii_digit() {
            format!("{}h", raw)
        } else {
            format!("0{}h", raw)
        }
    };
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
    let arg_count = f.params.len() as u32;
    let bytes = arg_count * 4;

    writeln!(
        w,
        "__declspec(naked) {} X_STDCALL {}({}) {{",
        ret, name, params_str
    )
    .unwrap();
    w.push_str("    __asm {\n");
    w.push_str("        push ebp\n");
    w.push_str("        mov  ebp, esp\n");
    w.push_str("        push ebx\n");
    w.push_str("        push esi\n\n");

    // Get syscall number -> ebx
    writeln!(w, "        push {}", hash_hex).unwrap();
    w.push_str("        call X_GetSyscallNumber\n");
    w.push_str("        add  esp, 4\n");
    w.push_str("        mov  ebx, eax          ; syscall number\n\n");

    // WoW64 detect: esi = gate ptr (or 0)
    w.push_str("        mov  esi, dword ptr fs:[0C0h]\n");
    w.push_str("        test esi, esi\n");
    w.push_str("        jz   x_native\n\n");

    // WoW64 path (fall-through)
    push_args_reverse(w, arg_count);
    w.push_str("        push 0                 ; dummy return address for WoW64 gate\n");
    w.push_str("        mov  eax, ebx\n");
    w.push_str("        call esi               ; WoW64 gate call\n");
    writeln!(w, "        add  esp, {}", bytes + 4).unwrap();
    w.push_str("        jmp  x_done\n\n");

    // Native path
    w.push_str("    x_native:\n");
    writeln!(w, "        push {}", hash_hex).unwrap();
    w.push_str("        call X_GetRandomSyscallAddress\n");
    w.push_str("        add  esp, 4\n");
    w.push_str("        mov  esi, eax          ; addr of `sysenter; ret` in ntdll\n");
    push_args_reverse(w, arg_count);
    w.push_str("        mov  eax, ebx\n");
    w.push_str("        mov  edx, esp          ; syscall ABI: edx = ptr to args\n");
    w.push_str("        call esi\n");
    writeln!(w, "        add  esp, {}", bytes).unwrap();

    // Common epilogue
    w.push_str("    x_done:\n");
    w.push_str("        pop  esi\n");
    w.push_str("        pop  ebx\n");
    w.push_str("        pop  ebp\n");
    writeln!(w, "        ret  {}", bytes).unwrap();
    w.push_str("    }\n}\n\n");
}

/// `push [ebp+8+4*(n-1)] ... push [ebp+8]` -- copy caller args right-to-left.
fn push_args_reverse(w: &mut String, arg_count: u32) {
    for i in (0..arg_count).rev() {
        let off = 8 + i * 4;
        writeln!(w, "        push dword ptr [ebp + {}]", off).unwrap();
    }
}
