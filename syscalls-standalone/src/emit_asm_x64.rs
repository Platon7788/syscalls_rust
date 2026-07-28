//! Emit `syscallsstubs.x64.asm` — MASM (ml64) PROC per syscall, jumper_randomized.

use crate::parse::Function;
use std::fmt::Write as _;

pub fn emit(functions: &[Function]) -> String {
    let mut w = String::with_capacity(functions.len() * 400);

    w.push_str(HEADER);
    for f in functions {
        emit_stub(&mut w, f);
    }
    w.push_str("END\n");
    w
}

const HEADER: &str = r#";
; X-Syscalls -- MASM (x64) direct-syscall stubs (jumper_randomized + RAS)
;
; Auto-generated -- DO NOT EDIT MANUALLY.
;
; Return-Address Spoofing:
;   Before the final `jmp r11` (which lands on `syscall; ret` in a random
;   ntdll function), each stub rewrites the on-stack return address to point
;   at a bare `ret` inside ntdll (X_NtdllRetGadget). The caller's real return
;   address is preserved one slot lower so the double-ret sequence
;     [syscall gadget's `ret`] -> [ntdll `ret` gadget] -> user code
;   still lands in user code. Any kernel-side stack walk during the syscall
;   sees the top frame anchored in ntdll, not in the calling module.
;
; Stack shape just before `jmp r11`:
;   [rsp+0]  = X_NtdllRetGadget   (kernel sees this as caller)
;   [rsp+8]  = real user_ret_addr (used after the second ret)

EXTERN X_GetSyscallNumber        : PROC
EXTERN X_GetRandomSyscallAddress : PROC
EXTERN X_NtdllRetGadget          : QWORD

.code

"#;

fn emit_stub(w: &mut String, f: &Function) {
    let name = format!("X{}", f.pascal);
    let hash = format_hex(f.zw_hash);
    // Extra args beyond the 4 that live in registers -- they sit on the
    // caller's stack at [rsp+0x28..]. Our RAS pushes an 8-byte gadget slot,
    // shifting rsp by -8, which desyncs where the kernel reads args 5..N.
    // We copy those slots down by 8 to compensate.
    let stack_args = f.params.len().saturating_sub(4);

    writeln!(w, "{} PROC", name).unwrap();
    // Save arg registers into caller-provided shadow space.
    writeln!(w, "    mov [rsp + 8],  rcx").unwrap();
    writeln!(w, "    mov [rsp + 10h], rdx").unwrap();
    writeln!(w, "    mov [rsp + 18h], r8").unwrap();
    writeln!(w, "    mov [rsp + 20h], r9").unwrap();

    // Allocate our own frame (shadow + retaddr slot for helpers).
    writeln!(w, "    sub rsp, 28h").unwrap();

    // Resolve syscall gadget -> r11 (jmp target).
    writeln!(w, "    mov ecx, {}", hash).unwrap();
    writeln!(w, "    call X_GetRandomSyscallAddress").unwrap();
    writeln!(w, "    mov r11, rax").unwrap();

    // Resolve syscall number -> eax.
    writeln!(w, "    mov ecx, {}", hash).unwrap();
    writeln!(w, "    call X_GetSyscallNumber").unwrap();

    // Unwind our frame; rsp now points at user_ret_addr again.
    writeln!(w, "    add rsp, 28h").unwrap();

    // RAS: reserve a slot; user_ret_addr stays at [rsp+8].
    writeln!(w, "    sub rsp, 8").unwrap();
    writeln!(w, "    mov rcx, [X_NtdllRetGadget]").unwrap();
    writeln!(w, "    mov [rsp], rcx").unwrap();

    // Restore args 1-4 from caller's shadow (offsets shifted by 8).
    writeln!(w, "    mov rcx, [rsp + 10h]").unwrap();
    writeln!(w, "    mov rdx, [rsp + 18h]").unwrap();
    writeln!(w, "    mov r8,  [rsp + 20h]").unwrap();
    writeln!(w, "    mov r9,  [rsp + 28h]").unwrap();

    // Shift args 5..N down 8 bytes so kernel finds them at [rsp+0x28..].
    // r9 has already been restored, so overwriting the shadow[3] slot is safe.
    // Use r10 as scratch -- it's reloaded with rcx just before the syscall
    // and eax (holding syscall_num) must stay untouched.
    for i in 0..stack_args {
        let src = 0x30 + i * 8;
        let dst = 0x28 + i * 8;
        writeln!(w, "    mov r10, [rsp + {:X}h]", src).unwrap();
        writeln!(w, "    mov [rsp + {:X}h], r10", dst).unwrap();
    }

    writeln!(w, "    mov r10, rcx").unwrap();
    writeln!(w, "    jmp r11").unwrap();
    writeln!(w, "{} ENDP\n", name).unwrap();
}

/// MASM hex literal: `<hex>h`. Leading digit needed if starts with A-F.
fn format_hex(n: u32) -> String {
    let h = format!("{:X}", n);
    if h.chars().next().unwrap().is_ascii_digit() {
        format!("{}h", h)
    } else {
        format!("0{}h", h)
    }
}
