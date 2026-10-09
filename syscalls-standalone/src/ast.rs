//! Rust syntax parsing. This does not expand macros or resolve external modules.
use super::{Field, Function, Param, Parsed, hash_syscall, sanitize_param_name, to_pascal_case};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::spanned::Spanned;
use syn::{Expr, Fields, FnArg, Item, Lit, Meta, Pat, ReturnType, Type, Visibility};

pub(super) fn parse(content: &str) -> Result<Parsed, String> {
    let file = syn::parse_file(content)
        .map_err(|e| super::located(e.span(), format!("invalid Rust syntax: {e}")))?;
    let mut result = Parsed {
        locations: HashMap::new(),
        notices: Vec::new(),
        types: vec![],
        structs: vec![],
        constants: vec![],
        functions: vec![],
    };
    let constants: HashSet<_> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Const(item) if public(&item.vis) => Some(item.ident.to_string()),
            _ => None,
        })
        .collect();
    type Signature = (Vec<String>, String);
    let mut functions: HashMap<String, (Option<&'static str>, Signature)> = HashMap::new();
    for item in file.items {
        let mut error_span = item.span();
        let outcome = (|| -> Result<(), String> {
            match item {
                Item::Type(item) if public(&item.vis) => {
                    let name = item.ident.to_string();
                    error_span = item.ident.span();
                    result.locations.insert(name.clone(), error_span);
                    if name == "c_void" {
                        return Ok(());
                    }
                    no_generics(&item.generics, &name)?;
                    no_conditional_item(&item.attrs, &name)?;
                    result.types.push((
                        name.clone(),
                        type_text(&item.ty).map_err(|e| format!("type {name}: {e}"))?,
                    ));
                }
                Item::Const(item) if public(&item.vis) => {
                    let name = item.ident.to_string();
                    error_span = item.ident.span();
                    result.locations.insert(name.clone(), error_span);
                    no_conditional_item(&item.attrs, &name)?;
                    check_constant_references(&item.expr, &constants)
                        .map_err(|e| format!("constant {name}: {e}"))?;
                    result.constants.push((
                        name.clone(),
                        type_text(&item.ty)?,
                        constant(&item.expr).map_err(|e| format!("constant {name}: {e}"))?,
                    ));
                }
                Item::Struct(item) if public(&item.vis) => {
                    let name = item.ident.to_string();
                    error_span = item.ident.span();
                    result.locations.insert(name.clone(), error_span);
                    no_generics(&item.generics, &name)?;
                    no_conditional_item(&item.attrs, &name)?;
                    let reprs: Vec<_> = item
                        .attrs
                        .iter()
                        .filter(|a| a.path().is_ident("repr"))
                        .collect();
                    if reprs.len() != 1
                        || !reprs[0]
                            .parse_args::<syn::Ident>()
                            .is_ok_and(|id| id == "C")
                    {
                        return Err(format!("struct {name}: only repr(C) is supported"));
                    }
                    let Fields::Named(fields) = item.fields else {
                        return Err(format!("struct {name}: named fields required"));
                    };
                    let mut parsed_fields = Vec::new();
                    for field in fields.named {
                        let field_name = field.ident.as_ref().unwrap().to_string();
                        error_span = field.span();
                        result
                            .locations
                            .insert(format!("{name}.{field_name}"), error_span);
                        if !public(&field.vis) {
                            return Err(format!(
                                "struct {name}: private field {field_name} is unsupported"
                            ));
                        }
                        let condition = field_condition(&field.attrs).map_err(|e| {
                            format!("unsupported attributes on field {name}.{field_name}: {e}")
                        })?;
                        parsed_fields.push(Field {
                            name: field_name,
                            typ: type_text(&field.ty)?,
                            condition,
                        });
                    }
                    if parsed_fields.is_empty() {
                        return Err(format!("struct {name}: empty structs are unsupported"));
                    }
                    result.structs.push((name, parsed_fields));
                }
                Item::Fn(item)
                    if public(&item.vis)
                        && item.sig.unsafety.is_some()
                        && item.sig.ident.to_string().starts_with("nt_") =>
                {
                    let name = item.sig.ident.to_string();
                    error_span = item.sig.ident.span();
                    result.locations.insert(name.clone(), error_span);
                    no_generics(&item.sig.generics, &name)?;
                    if item.sig.variadic.is_some() || item.sig.asyncness.is_some() {
                        return Err(format!(
                            "function {name}: variadic/async signatures are unsupported"
                        ));
                    }
                    let mut params = Vec::new();
                    for arg in &item.sig.inputs {
                        let FnArg::Typed(arg) = arg else {
                            return Err(format!("function {name}: receiver unsupported"));
                        };
                        let Pat::Ident(pat) = &*arg.pat else {
                            return Err(format!("function {name}: parameter must be named"));
                        };
                        if pat.by_ref.is_some() || pat.subpat.is_some() || !arg.attrs.is_empty() {
                            return Err(format!(
                                "function {name}: unsupported parameter attributes or pattern"
                            ));
                        }
                        error_span = arg.span();
                        result.locations.insert(
                            format!("{name}.{}", sanitize_param_name(&pat.ident.to_string())),
                            error_span,
                        );
                        params.push(Param {
                            name: sanitize_param_name(&pat.ident.to_string()),
                            typ: type_text(&arg.ty)?,
                        });
                    }
                    let ReturnType::Type(_, ret) = &item.sig.output else {
                        return Err(format!("function {name}: explicit return type required"));
                    };
                    let conditions: Vec<_> = item
                        .attrs
                        .iter()
                        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
                        .cloned()
                        .collect();
                    let architecture = field_condition(&conditions)
                        .map_err(|e| format!("function {name}: {e}"))?;
                    let signature = (
                        params.iter().map(|p| p.typ.clone()).collect::<Vec<_>>(),
                        type_text(ret)?,
                    );
                    if let Some((previous_architecture, previous_signature)) = functions.get(&name)
                    {
                        if architecture.is_none()
                            || *previous_architecture == architecture
                            || previous_architecture.is_none()
                        {
                            return Err(format!(
                                "function {name}: duplicate definition without distinct architecture conditions"
                            ));
                        }
                        if *previous_signature != signature {
                            return Err(format!(
                                "function {name}: conflicting architecture signatures"
                            ));
                        }
                        // Only two supported architectures exist. A third definition is invalid.
                        functions.insert(name, (None, signature));
                        return Ok(());
                    }
                    functions.insert(name.clone(), (architecture, signature));
                    let pascal = to_pascal_case(&name);
                    let zw_hash = hash_syscall(&format!("Zw{}", &pascal[2..]));
                    result.functions.push(Function {
                        name,
                        pascal,
                        params,
                        return_type: type_text(ret)?,
                        zw_hash,
                    });
                }
                Item::Enum(item) if public(&item.vis) => result.notices.push(super::located(
                    item.ident.span(),
                    format!("not exported: enum {}", item.ident),
                )),
                Item::Union(item) if public(&item.vis) => result.notices.push(super::located(
                    item.ident.span(),
                    format!("not exported: union {}", item.ident),
                )),
                Item::Mod(item) => result.notices.push(super::located(
                    item.ident.span(),
                    format!(
                        "not inspected: module {} (module contents are not expanded)",
                        item.ident
                    ),
                )),
                Item::Use(item) if public(&item.vis) => result.notices.push(super::located(
                    item.span(),
                    "not resolved: public re-export",
                )),
                Item::Trait(item) if public(&item.vis) => result.notices.push(super::located(
                    item.ident.span(),
                    format!("not exported: trait {}", item.ident),
                )),
                Item::Static(item) if public(&item.vis) => result.notices.push(super::located(
                    item.ident.span(),
                    format!("not exported: static {}", item.ident),
                )),
                Item::Fn(item) if public(&item.vis) => result.notices.push(super::located(
                    item.sig.ident.span(),
                    format!("not selected: function {}", item.sig.ident),
                )),
                Item::Macro(item) => result.notices.push(super::located(
                    item.span(),
                    "not expanded: top-level macro definition/invocation",
                )),
                Item::Impl(item) => result.notices.push(super::located(
                    item.span(),
                    "not exported: impl methods/associated items",
                )),
                Item::ForeignMod(item) => result.notices.push(super::located(
                    item.span(),
                    "not exported: foreign declarations",
                )),
                _ => {}
            }
            Ok(())
        })();
        outcome.map_err(|error| super::located(error_span, error))?;
    }
    Ok(result)
}

fn check_constant_references(expr: &Expr, known: &HashSet<String>) -> Result<(), String> {
    match expr {
        Expr::Path(path) if path.path.segments.len() == 1 => {
            let name = path.path.segments[0].ident.to_string();
            if !known.contains(&name) {
                return Err(format!("unknown reference {name}"));
            }
        }
        Expr::Cast(cast) => check_constant_references(&cast.expr, known)?,
        Expr::Paren(paren) => check_constant_references(&paren.expr, known)?,
        _ => {}
    }
    Ok(())
}

fn public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn no_generics(generics: &syn::Generics, name: &str) -> Result<(), String> {
    if !generics.params.is_empty() || generics.where_clause.is_some() {
        Err(format!("{name}: generic declarations are unsupported"))
    } else {
        Ok(())
    }
}

fn no_conditional_item(attrs: &[syn::Attribute], name: &str) -> Result<(), String> {
    if attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
    {
        Err(format!(
            "{name}: conditional type/constant declarations are unsupported"
        ))
    } else {
        Ok(())
    }
}

fn field_condition(attrs: &[syn::Attribute]) -> Result<Option<&'static str>, String> {
    let mut condition = None;
    for attr in attrs {
        if attr.path().is_ident("doc") {
            continue;
        }
        if !attr.path().is_ident("cfg") || condition.is_some() {
            return Err("only one target_arch cfg is supported".into());
        }
        let meta = attr.parse_args::<Meta>().map_err(|e| e.to_string())?;
        if let Meta::NameValue(value) = meta
            && value.path.is_ident("target_arch")
            && let Expr::Lit(expr) = value.value
            && let Lit::Str(arch) = expr.lit
        {
            condition = Some(match arch.value().as_str() {
                "x86" => "defined(_M_IX86) || defined(__i386__)",
                "x86_64" => "defined(_M_X64) || defined(__x86_64__)",
                _ => return Err("unsupported target architecture".into()),
            });
            continue;
        }
        return Err("expected cfg(target_arch = \"x86\"/\"x86_64\")".into());
    }
    Ok(condition)
}

fn type_text(ty: &Type) -> Result<String, String> {
    match ty {
        Type::Ptr(ptr) => Ok(format!(
            "*{} {}",
            if ptr.mutability.is_some() {
                "mut"
            } else {
                "const"
            },
            type_text(&ptr.elem)?
        )),
        Type::Array(array) => {
            let Expr::Lit(expr) = &array.len else {
                return Err("array length must be an integer literal".into());
            };
            let Lit::Int(length) = &expr.lit else {
                return Err("array length must be an integer literal".into());
            };
            Ok(format!(
                "[{}; {}]",
                type_text(&array.elem)?,
                length.base10_digits()
            ))
        }
        Type::Path(path) if path.qself.is_none() => {
            // Preserve the existing opaque callback representation in the emitter.
            if path.path.segments.len() == 1 && path.path.segments[0].ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &path.path.segments[0].arguments
                    && args.args.len() == 1
                    && matches!(
                        args.args.first(),
                        Some(syn::GenericArgument::Type(Type::BareFn(_)))
                    )
                {
                    return Ok(ty
                        .to_token_stream()
                        .to_string()
                        .replacen("Option <", "Option<", 1));
                }
                return Err("only Option<function pointer> is supported".into());
            }
            if path
                .path
                .segments
                .iter()
                .any(|s| !matches!(s.arguments, syn::PathArguments::None))
            {
                return Err("generic types are unsupported".into());
            }
            Ok(path.path.to_token_stream().to_string().replace(' ', ""))
        }
        Type::Paren(p) => type_text(&p.elem),
        _ => Err(format!("unsupported type: {}", ty.to_token_stream())),
    }
}

/// Translate the explicitly supported constant-expression subset to C.
fn constant(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Lit(expr) => match &expr.lit {
            Lit::Int(value) => {
                let token = value.to_string();
                if token.starts_with("0o") || token.starts_with("0b") {
                    return Ok(value.base10_digits().to_string());
                }
                Ok(token
                    .strip_suffix(value.suffix())
                    .unwrap_or(&token)
                    .replace('_', ""))
            }
            _ => Err("only integer literals are supported".into()),
        },
        Expr::Cast(cast) => Ok(format!(
            "({}){}",
            super::rust_to_c_type(&type_text(&cast.ty)?),
            constant(&cast.expr)?
        )),
        Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            Ok(format!("X_{}", path.path.segments[0].ident))
        }
        Expr::Paren(p) => Ok(format!("({})", constant(&p.expr)?)),
        _ => Err(format!(
            "unsupported expression: {}",
            expr.to_token_stream()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_strings_do_not_create_declarations() {
        let parsed = parse(
            r#"
            // pub type Fake = u32;
            /* pub const FAKE: u32 = 1; */
            fn private() { let _text = "pub unsafe fn nt_fake() -> i32 { 0 }"; }
            pub type Real = u32;
        "#,
        )
        .unwrap();
        assert_eq!(parsed.types, vec![("Real".into(), "u32".into())]);
        assert!(parsed.constants.is_empty());
        assert!(parsed.functions.is_empty());
    }

    #[test]
    fn field_docs_and_attribute_order_do_not_change_layout() {
        let parsed = parse(
            r#"
            #[derive(Clone)]
            #[repr(C)]
            pub struct Example {
                /// A brace } and comma , in docs are not syntax.
                pub bytes: [u8; 16],
                #[cfg(target_arch = "x86_64")]
                // A comment between the attribute and field.
                pub pointer: *mut u8,
            }
        "#,
        )
        .unwrap();
        let fields = &parsed.structs[0].1;
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].typ, "[u8; 16]");
        assert_eq!(
            fields[1].condition,
            Some("defined(_M_X64) || defined(__x86_64__)")
        );
    }

    #[test]
    fn nested_callback_and_underscore_parameters_are_not_lost() {
        let parsed = parse(
            r#"
            pub unsafe fn nt_example(
                _reserved: u32,
                callback: Option<unsafe extern "system" fn(*mut u8, u32)>,
            ) -> i32 { 0 }
        "#,
        )
        .unwrap();
        assert_eq!(parsed.functions[0].params.len(), 2);
        assert_eq!(parsed.functions[0].params[0].name, "_reserved");
        assert!(parsed.functions[0].params[1].typ.starts_with("Option<"));
    }

    #[test]
    fn integer_casts_and_constant_references_are_translated() {
        let parsed = parse("pub const A: u32 = 0x12_34u32; pub const B: i32 = 0xC0000001u32 as i32; pub const C: u32 = A;").unwrap();
        let values: Vec<_> = parsed.constants.iter().map(|c| c.2.as_str()).collect();
        assert_eq!(values, ["0x1234", "(int32_t)0xC0000001", "X_A"]);
        let parsed = parse("pub const OCT: u32 = 0o77; pub const BIN: u32 = 0b1010;").unwrap();
        assert_eq!(parsed.constants[0].2, "63");
        assert_eq!(parsed.constants[1].2, "10");
    }

    #[test]
    fn invalid_or_unrepresentable_input_is_rejected() {
        for source in [
            "pub type Broken = ;",
            "pub type Generic<T> = *mut T;",
            "pub type Reference = &'static u32;",
            "pub type Maybe = Option<u32>;",
            "#[repr(C, packed)] pub struct Packed { pub value: u32 }",
            "#[repr(C)] pub struct Hidden { private: u32 }",
            "pub const A: u32 = compute();",
            "#[cfg(feature = \"extra\")] pub type Hidden = u32;",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
    }
}
