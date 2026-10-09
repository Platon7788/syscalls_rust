//! Validate declaration references and generated C identifiers before emission.
use super::Parsed;
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::{GenericArgument, PathArguments, ReturnType, Type};

pub(super) fn validate(p: &Parsed) -> Result<(), String> {
    let cursor = std::cell::Cell::new(proc_macro2::Span::call_site());
    let at = |name: &str| {
        cursor.set(
            p.locations
                .get(name)
                .copied()
                .unwrap_or_else(proc_macro2::Span::call_site),
        )
    };
    let result = (|| -> Result<(), String> {
        let mut symbols = HashMap::new();
        for reserved in [
            "X_STDCALL",
            "X_SYSCALLS_H",
            "X_NT_SUCCESS",
            "X_NT_ERROR",
            "X_NT_WARNING",
            "X_NT_INFORMATION",
            "X_NtCurrentProcess",
            "X_NtCurrentThread",
            "X_InitializeObjectAttributes",
            "X_GetSyscallNumber",
            "X_GetSyscallAddress",
            "X_GetRandomSyscallAddress",
            "X_DebugGetCount",
            "X_DebugGetHash",
            "X_DebugGetSyscallAddr",
        ] {
            symbols.insert(reserved.to_string(), "generated helper".to_string());
        }
        let mut types = HashSet::new();
        for name in p
            .types
            .iter()
            .map(|t| &t.0)
            .chain(p.structs.iter().map(|s| &s.0))
        {
            at(name);
            identifier(name, "type")?;
            register(&mut symbols, format!("X_{name}"), format!("type {name}"))?;
            types.insert(name.as_str());
        }
        let constants: HashSet<_> = p.constants.iter().map(|c| c.0.as_str()).collect();
        for (name, _, _) in &p.constants {
            at(name);
            identifier(name, "constant")?;
            register(
                &mut symbols,
                format!("X_{name}"),
                format!("constant {name}"),
            )?;
        }
        let mut alias_graph = HashMap::new();
        for (name, ty) in &p.types {
            at(name);
            let refs = check_type(ty, &types, &format!("type {name}"))?;
            alias_graph.insert(name.clone(), refs);
        }
        acyclic(&alias_graph, "type alias").map_err(|(name, message)| {
            at(&name);
            message
        })?;
        let mut available: HashSet<_> = p.structs.iter().map(|s| s.0.as_str()).collect();
        for (name, _) in &p.types {
            at(name);
            for dependency in &alias_graph[name] {
                if !available.contains(dependency.as_str()) {
                    return Err(format!(
                        "type {name}: alias {dependency} must be declared earlier for C emission"
                    ));
                }
            }
            available.insert(name.as_str());
        }
        let aliases: HashMap<_, _> = p.types.iter().cloned().collect();
        let structs: HashSet<_> = p.structs.iter().map(|s| s.0.as_str()).collect();
        let mut complete = HashSet::new();
        for (name, fields) in &p.structs {
            let mut names = HashSet::new();
            for field in fields {
                at(&format!("{name}.{}", field.name));
                let context = format!("field {name}.{}", field.name);
                identifier(&field.name, &context)?;
                if !names.insert(&field.name) {
                    return Err(format!("{context}: duplicate field name"));
                }
                check_type(&field.typ, &types, &context)?;
                complete_value(&field.typ, &aliases, &structs, &complete)
                    .map_err(|e| format!("{context}: {e}"))?;
            }
            complete.insert(name.as_str());
        }
        let mut constant_graph = HashMap::new();
        for (name, ty, value) in &p.constants {
            at(name);
            check_type(ty, &types, &format!("constant {name}"))?;
            let mut dependencies = Vec::new();
            // Constant expressions have already been restricted and translated by the AST parser.
            for token in value.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                if let Some(reference) = token.strip_prefix("X_") {
                    if constants.contains(reference) {
                        dependencies.push(reference.to_string());
                    } else if !types.contains(reference) {
                        return Err(format!("constant {name}: unknown reference {reference}"));
                    }
                }
            }
            constant_graph.insert(name.clone(), dependencies);
        }
        acyclic(&constant_graph, "constant").map_err(|(name, message)| {
            at(&name);
            message
        })?;
        for f in &p.functions {
            at(&f.name);
            identifier(&format!("X{}", f.pascal), "function")?;
            register(
                &mut symbols,
                format!("X{}", f.pascal),
                format!("function {}", f.name),
            )?;
            let mut params = HashSet::new();
            for param in &f.params {
                at(&format!("{}.{}", f.name, param.name));
                let context = format!("function {} parameter {}", f.name, param.name);
                identifier(&param.name, &context)?;
                if !params.insert(&param.name) {
                    return Err(format!("{context}: duplicate generated parameter name"));
                }
                check_type(&param.typ, &types, &context)?;
            }
            at(&f.name);
            check_type(
                &f.return_type,
                &types,
                &format!("function {} return type", f.name),
            )?;
        }
        Ok(())
    })();
    result.map_err(|error| super::located(cursor.get(), error))
}

fn register(
    symbols: &mut HashMap<String, String>,
    c_name: String,
    owner: String,
) -> Result<(), String> {
    if let Some(previous) = symbols.insert(c_name.clone(), owner.clone()) {
        return Err(format!("C name collision {c_name}: {previous} and {owner}"));
    }
    Ok(())
}

fn identifier(name: &str, context: &str) -> Result<(), String> {
    let valid = name
        .as_bytes()
        .first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_');
    const KEYWORDS: &str = "alignas alignof and and_eq asm auto bitand bitor bool break case catch char char8_t char16_t char32_t class compl concept const consteval constexpr constinit const_cast continue co_await co_return co_yield decltype default delete do double dynamic_cast else enum explicit export extern false float for friend goto if inline int long mutable namespace new noexcept not not_eq nullptr operator or or_eq private protected public register reinterpret_cast requires restrict return short signed sizeof static static_assert static_cast struct switch template this thread_local throw true try typedef typeid typename union unsigned using virtual void volatile wchar_t while xor xor_eq _Alignas _Alignof _Atomic _Bool _Complex _Generic _Imaginary _Noreturn _Static_assert _Thread_local";
    if !valid || KEYWORDS.split_whitespace().any(|keyword| keyword == name) {
        return Err(format!("{context}: invalid C/C++ identifier {name}"));
    }
    Ok(())
}

fn check_type(text: &str, known: &HashSet<&str>, context: &str) -> Result<Vec<String>, String> {
    let ty: Type = syn::parse_str(text).map_err(|e| format!("{context}: invalid type: {e}"))?;
    declaration_shape(&ty, context.starts_with("field "), false)
        .map_err(|e| format!("{context}: {e}"))?;
    let mut refs = Vec::new();
    type_references(&ty, &mut refs);
    for name in &refs {
        if !known.contains(name.as_str()) {
            return Err(format!(
                "{context}: unknown type {name}; declare it explicitly"
            ));
        }
    }
    Ok(refs)
}

// The current emitter supports arrays only as one-dimensional struct fields.
fn declaration_shape(ty: &Type, array_allowed: bool, behind_pointer: bool) -> Result<(), String> {
    match ty {
        Type::Ptr(p) => declaration_shape(&p.elem, false, true),
        Type::Array(a) if array_allowed => {
            if !matches!(&a.len, syn::Expr::Lit(l) if matches!(&l.lit, syn::Lit::Int(n) if n.base10_parse::<usize>().is_ok_and(|n| n > 0)))
            {
                return Err("array length must be a positive supported integer".into());
            }
            declaration_shape(&a.elem, false, false)
        }
        Type::Array(_) => Err("arrays are supported only as one-dimensional struct fields".into()),
        Type::Path(p)
            if !behind_pointer
                && (p.path.is_ident("c_void")
                    || p.path.to_token_stream().to_string().replace(' ', "")
                        == "core::ffi::c_void") =>
        {
            Err("void is supported only behind a pointer".into())
        }
        _ => Ok(()),
    }
}

fn complete_value(
    text: &str,
    aliases: &HashMap<String, String>,
    structs: &HashSet<&str>,
    complete: &HashSet<&str>,
) -> Result<(), String> {
    let ty: Type = syn::parse_str(text).map_err(|e| e.to_string())?;
    match ty {
        Type::Path(p) if p.path.segments.len() == 1 => {
            let name = p.path.segments[0].ident.to_string();
            if let Some(target) = aliases.get(&name) {
                complete_value(target, aliases, structs, complete)?;
            } else if structs.contains(name.as_str()) && !complete.contains(name.as_str()) {
                return Err(format!(
                    "by-value struct {name} must be defined earlier for C emission"
                ));
            }
        }
        Type::Array(a) => complete_value(
            &a.elem.to_token_stream().to_string(),
            aliases,
            structs,
            complete,
        )?,
        _ => {}
    }
    Ok(())
}

fn type_references(ty: &Type, refs: &mut Vec<String>) {
    match ty {
        Type::Ptr(p) => type_references(&p.elem, refs),
        Type::Array(a) => type_references(&a.elem, refs),
        Type::Paren(p) => type_references(&p.elem, refs),
        Type::Group(g) => type_references(&g.elem, refs),
        Type::Reference(r) => type_references(&r.elem, refs),
        Type::Slice(s) => type_references(&s.elem, refs),
        Type::Tuple(t) => {
            for element in &t.elems {
                type_references(element, refs);
            }
        }
        Type::BareFn(f) => {
            for arg in &f.inputs {
                type_references(&arg.ty, refs);
            }
            if let ReturnType::Type(_, ty) = &f.output {
                type_references(ty, refs);
            }
        }
        Type::Path(path) => {
            let name = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            if name == "Option" {
                if let PathArguments::AngleBracketed(args) = &path.path.segments[0].arguments {
                    for arg in &args.args {
                        if let GenericArgument::Type(ty) = arg {
                            type_references(ty, refs);
                        }
                    }
                }
            } else if ![
                "u8",
                "u16",
                "u32",
                "u64",
                "usize",
                "i8",
                "i16",
                "i32",
                "i64",
                "isize",
                "bool",
                "c_void",
                "core::ffi::c_void",
            ]
            .contains(&name.as_str())
            {
                refs.push(name);
            }
        }
        _ => {}
    }
}

fn acyclic(graph: &HashMap<String, Vec<String>>, kind: &str) -> Result<(), (String, String)> {
    fn visit<'a>(
        name: &'a str,
        graph: &'a HashMap<String, Vec<String>>,
        visiting: &mut HashSet<&'a str>,
        done: &mut HashSet<&'a str>,
        kind: &str,
    ) -> Result<(), (String, String)> {
        if done.contains(name) || !graph.contains_key(name) {
            return Ok(());
        }
        if !visiting.insert(name) {
            return Err((
                name.to_string(),
                format!("{kind} dependency cycle at {name}"),
            ));
        }
        for next in &graph[name] {
            visit(next, graph, visiting, done, kind)?;
        }
        visiting.remove(name);
        done.insert(name);
        Ok(())
    }
    let mut done = HashSet::new();
    // Stable ordering keeps diagnostics reproducible.
    let mut names: Vec<_> = graph.keys().collect();
    names.sort();
    for name in names {
        visit(name, graph, &mut HashSet::new(), &mut done, kind)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    fn rejects(source: &str, message: &str) {
        match parse(source) {
            Err(error) => assert!(error.contains(message), "{error}"),
            Ok(_) => panic!("accepted invalid input: {source}"),
        }
    }
    #[test]
    fn unknown_references_are_not_guessed_as_pointers() {
        rejects("pub type Handle = Missing;", "unknown type Missing");
        rejects(
            "#[repr(C)] pub struct S { pub value: Missing }",
            "field S.value: unknown type Missing",
        );
        rejects(
            "pub unsafe fn nt_example(value: Missing) -> i32 { 0 }",
            "unknown type Missing",
        );
        rejects(
            "pub type Callback = Option<unsafe extern \"C\" fn(Missing)>;",
            "unknown type Missing",
        );
        rejects("pub const A: u32 = MISSING;", "unknown reference MISSING");
    }
    #[test]
    fn unsupported_c_declarators_are_rejected() {
        for source in [
            "pub type Array = [u8; 4];",
            "pub type Pointer = *mut [u8; 4];",
            "#[repr(C)] pub struct S { pub nested: [[u8; 2]; 4] }",
            "#[repr(C)] pub struct S { pub empty: [u8; 0] }",
            "#[repr(C)] pub struct S { pub empty: core::ffi::c_void }",
            "#[repr(C)] pub struct S { pub next: S }",
            "pub type Alias = Later; #[repr(C)] pub struct S { pub later: Alias } #[repr(C)] pub struct Later { pub n: u32 }",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
        assert!(
            parse(
                "#[repr(C)] pub struct S { pub bytes: [u8; 4], pub opaque: *mut core::ffi::c_void }"
            )
            .is_ok()
        );
    }
    #[test]
    fn duplicate_names_and_generated_collisions_are_rejected() {
        rejects(
            "pub type A = u32; pub type A = u16;",
            "C name collision X_A",
        );
        rejects(
            "pub type A = u32; pub const A: u32 = 1;",
            "C name collision X_A",
        );
        rejects("pub const STDCALL: u32 = 1;", "C name collision X_STDCALL");
        rejects(
            "pub unsafe fn nt_example(class: u32, class_name: u32) -> i32 { 0 }",
            "duplicate generated parameter name",
        );
        rejects(
            "pub unsafe fn nt_some_name() -> i32 { 0 } pub unsafe fn nt_some__name() -> i32 { 0 }",
            "C name collision XNtSomeName",
        );
    }
    #[test]
    fn cycles_are_rejected_but_recursive_struct_pointers_are_valid() {
        rejects(
            "pub type A = B; pub type B = *mut A;",
            "type alias dependency cycle",
        );
        rejects(
            "pub const A: u32 = B; pub const B: u32 = A;",
            "constant dependency cycle",
        );
        assert!(parse("#[repr(C)] pub struct Node { pub next: *mut Node }").is_ok());
    }
    #[test]
    fn real_repository_declarations_pass() {
        let parsed = parse(include_str!("../../lib.rs")).unwrap();
        let contract = include_str!("../../tests/layout_contract.rs");
        for (name, _) in &parsed.structs {
            assert!(
                contract.contains(&format!("layout!({name};")),
                "missing Rust/C layout contract for {name}"
            );
        }
    }

    #[test]
    fn c_incompatible_names_and_alias_order_are_reported() {
        rejects(
            "#[repr(C)] pub struct S { pub class: u32 }",
            "invalid C/C++ identifier class",
        );
        rejects(
            "pub type A = B; pub type B = u32;",
            "alias B must be declared earlier",
        );
        rejects(
            "pub type A = u32; pub const VALUE: u32 = A;",
            "unknown reference A",
        );
        assert!(parse("pub type B = u32; pub type A = B;").is_ok());
    }

    #[test]
    fn repeated_functions_require_distinct_compatible_architectures() {
        rejects(
            "pub unsafe fn nt_example() -> i32 { 0 } pub unsafe fn nt_example() -> i32 { 0 }",
            "duplicate definition",
        );
        rejects(
            r#"#[cfg(target_arch = "x86")] pub unsafe fn nt_example(a: u32) -> i32 { 0 }
            #[cfg(target_arch = "x86_64")] pub unsafe fn nt_example(a: u64) -> i32 { 0 }"#,
            "conflicting architecture signatures",
        );
        assert!(
            parse(
                r#"#[cfg(target_arch = "x86")] pub unsafe fn nt_example(a: u32) -> i32 { 0 }
            #[cfg(target_arch = "x86_64")] pub unsafe fn nt_example(a: u32) -> i32 { 0 }"#
            )
            .is_ok()
        );
    }

    #[test]
    fn diagnostics_include_source_positions() {
        let error = parse("\n\npub type Example = Missing;").err().unwrap();
        assert!(error.starts_with("3:10:"), "{error}");
        let error = parse("\n#[repr(C)]\npub struct Example {\n    pub field: Missing,\n}")
            .err()
            .unwrap();
        assert!(error.starts_with("4:5:"), "{error}");
        let error = parse("\n\npub type Broken = ;").err().unwrap();
        assert!(error.starts_with("3:"), "{error}");
    }

    #[test]
    fn unexpanded_and_unsupported_exports_are_reported() {
        let parsed = parse("pub mod other; pub use other::*; pub enum Choice { A } pub union Bits { value: u32 } make_items!();").unwrap();
        assert_eq!(parsed.notices.len(), 5);
        assert!(parsed.notices.iter().any(|n| n.contains("union Bits")));
        assert!(parsed.notices.iter().all(|n| n.starts_with("1:")));
    }
}
