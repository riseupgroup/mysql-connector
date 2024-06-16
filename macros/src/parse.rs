use {
    crate::Error,
    proc_macro2::Span,
    std::collections::HashMap,
    syn::{
        punctuated::Punctuated, spanned::Spanned, token, Attribute, Data, Expr, ExprLit,
        GenericArgument, Ident, Lit, Member, MetaNameValue, Path, PathArguments, PathSegment,
        Token, Type, TypePath,
    },
};

pub struct NamedField {
    pub complexity: TypeComplexity,
    //pub vis: Visibility,
    pub ident: Ident,
    pub ty: Type,
}

pub fn parse_fields(
    error: &mut Error,
    span: Span,
    data: &Data,
    types: &[SimpleStruct],
) -> Vec<NamedField> {
    let mut fields: Vec<NamedField> = Vec::new();
    match data {
        Data::Enum(_) => error.add(span, "mysql_connector does not support derive for enums"),
        Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields_named) => {
                for field in &fields_named.named {
                    match &field.ty {
                        Type::Path(path) => {
                            fields.push(NamedField {
                                complexity: TypeComplexity::from_path(path, types),
                                //vis: field.vis.clone(),
                                ident: field.ident.clone().unwrap(),
                                ty: field.ty.clone(),
                            });
                        }
                        _ => error.add(
                            field.ty.span(),
                            "mysql_connector does not support this type",
                        ),
                    }
                }
            }
            syn::Fields::Unnamed(_) => error.add(
                span,
                "mysql_connector does not support derive for unnamed fields",
            ),
            syn::Fields::Unit => error.add(
                span,
                "mysql_connector does not support derive for unit fields",
            ),
        },
        Data::Union(_) => error.add(span, "mysql_connector does not support derive for unions"),
    }
    fields
}

#[derive(Clone)]
pub struct SimpleStruct {
    pub path: Path,
    pub fields: Vec<(Ident, Ident)>,
}

pub fn parse_attr(
    error: &mut Error,
    span: Span,
    attrs: &[Attribute],
) -> (Option<Span>, HashMap<String, String>, Vec<SimpleStruct>) {
    let mut map = HashMap::new();
    let mut types = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("mysql_connector") {
            match attr.parse_args_with(Punctuated::<MetaNameValue, token::Comma>::parse_terminated)
            {
                Ok(values) => {
                    for value in values {
                        let ident = match value.path.get_ident() {
                            Some(x) => x.to_string(),
                            None => {
                                error.add(value.path.span(), "expected ident");
                                continue;
                            }
                        };
                        if let Expr::Lit(ExprLit {
                            attrs: _,
                            lit: Lit::Str(x),
                        }) = value.value
                        {
                            map.insert(ident, x.value());
                        } else if ident == "ty" {
                            if let Expr::Struct(r#struct) = value.value {
                                let mut fields = Vec::new();
                                for field in r#struct.fields {
                                    let member = match field.member {
                                        Member::Named(x) => x,
                                        Member::Unnamed(_) => {
                                            unreachable!("there are no unnamed fields in structs")
                                        }
                                    };
                                    let ident = match field.colon_token {
                                        Some(_) => {
                                            if let Expr::Path(path) = &field.expr {
                                                if path.path.segments.len() != 1 {
                                                    error.add(
                                                        field.expr.span(),
                                                        "expected single identifier",
                                                    );
                                                    continue;
                                                }
                                                path.path.segments[0].ident.clone()
                                            } else {
                                                error.add(field.expr.span(), "expected identifier");
                                                continue;
                                            }
                                        }
                                        None => member.clone(),
                                    };
                                    fields.push((member, ident));
                                }
                                types.push(SimpleStruct {
                                    path: r#struct.path,
                                    fields,
                                })
                            } else {
                                error.add(value.value.span(), "expected struct");
                            }
                        } else {
                            error.add(value.value.span(), "expected string literal");
                        }
                    }
                }
                Err(err) => error.add_err(err),
            }
            return (Some(attr.span()), map, types);
        }
    }
    error.add(span, "expected attribute #[mysql_connector()]");
    (None, map, types)
}

#[derive(Clone)]
pub enum TypeComplexity {
    Simple,
    Struct(SimpleStruct),
    Complex,
}

impl TypeComplexity {
    pub fn from_path(path: &TypePath, types: &[SimpleStruct]) -> Self {
        fn path_eq(a: &Path, b: &Path) -> bool {
            if a.segments.len() != b.segments.len() {
                return false;
            }
            for i in 0..a.segments.len() {
                if a.segments[i].ident != b.segments[i].ident {
                    return false;
                }
            }
            true
        }

        fn path_matches(a: &Path, b: &[&'static str]) -> bool {
            let mut i = a.segments.len();
            let mut j = b.len();
            while i > 0 && j > 0 {
                i -= 1;
                j -= 1;
                if a.segments[i].ident != b[j] {
                    return false;
                }
            }
            true
        }

        fn get_last_arguments_path(
            segments: &Punctuated<PathSegment, Token![::]>,
        ) -> Option<&Path> {
            if let Some(segment) = segments.last() {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() == 1 {
                        if let GenericArgument::Type(Type::Path(path)) = args.args.first().unwrap()
                        {
                            return Some(&path.path);
                        }
                    }
                }
            }
            None
        }

        fn is_simple(path: &Path, option: bool) -> bool {
            const SIMPLE: &[&[&str]] = &[
                &["i8"],
                &["i16"],
                &["i32"],
                &["i64"],
                &["u8"],
                &["u16"],
                &["u32"],
                &["u64"],
                &["f32"],
                &["f64"],
                &["bool"],
                &["std", "string", "String"],
                &["chrono", "NaiveDate"],
                &["chrono", "NaiveDateTime"],
                &["chrono", "Duration"],
                &["mysql_connector", "types", "Hex"],
            ];
            for simple_path in SIMPLE {
                if path_matches(path, simple_path) {
                    return true;
                }
            }
            if path_matches(path, &["std", "vec", "Vec"]) {
                if let Some(path) = get_last_arguments_path(&path.segments) {
                    return path_matches(path, &["u8"]);
                }
            }
            if option && path_matches(path, &["std", "option", "Option"]) {
                if let Some(path) = get_last_arguments_path(&path.segments) {
                    return is_simple(path, false);
                }
            }
            false
        }

        if is_simple(&path.path, true) {
            Self::Simple
        } else if let Some(r#type) = types.iter().find(|x| path_eq(&x.path, &path.path)) {
            Self::Struct(r#type.clone())
        } else {
            Self::Complex
        }
    }
}

#[allow(dead_code)]
impl TypeComplexity {
    pub fn struct_type(&self) -> Option<&SimpleStruct> {
        match self {
            Self::Struct(x) => Some(x),
            _ => None,
        }
    }

    pub fn simple(this: &NamedField) -> bool {
        matches!(this.complexity, Self::Simple)
    }

    pub fn r#struct(this: &NamedField) -> bool {
        matches!(this.complexity, Self::Struct(_))
    }

    pub fn complex(this: &NamedField) -> bool {
        matches!(this.complexity, Self::Complex)
    }

    pub fn simple_ref(this: &&NamedField) -> bool {
        matches!(this.complexity, Self::Simple)
    }

    pub fn struct_ref(this: &&NamedField) -> bool {
        matches!(this.complexity, Self::Struct(_))
    }

    pub fn complex_ref(this: &&NamedField) -> bool {
        matches!(this.complexity, Self::Complex)
    }
}
