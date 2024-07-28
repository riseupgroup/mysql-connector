use {
    crate::Error,
    proc_macro::TokenStream,
    proc_macro2::Span,
    syn::{
        spanned::Spanned, Data, DeriveInput, Expr, Fields, Ident, Member, Meta, Type, TypePath,
        Visibility,
    },
};

pub enum FieldType {
    Simple,
    /// (auto_increment)
    Primary(bool),
    Struct(Vec<(Ident, Ident)>),
    Complex,
}

pub struct Field {
    pub ident: Ident,
    pub path: TypePath,
    pub r#type: FieldType,
}

pub struct Model {
    pub vis: Visibility,
    pub ident: Ident,
    pub table: Option<String>,
    pub fields: Vec<Field>,
}

pub(crate) fn parse(input: &DeriveInput) -> Result<Model, syn::Error> {
    match &input.data {
        Data::Enum(_) => Err(syn::Error::new(
            input.ident.span(),
            "mysql_connector does not support derive for enums",
        )),
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields_named) => {
                    let mut error = Error::empty();
                    let mut table = None;
                    for attr in &input.attrs {
                        if attr.path().is_ident("table") {
                            if table.is_some() {
                                error.add(
                                    attr.span(),
                                    "you can't specify multiple `table` attributes",
                                );
                            } else {
                                match &attr.meta {
                                    Meta::List(list) => table = Some(list.tokens.to_string()),
                                    _ => error.add(attr.meta.span(), "expected table name"),
                                }
                            }
                        }
                    }

                    let mut fields = Vec::with_capacity(fields_named.named.len());
                    let mut primary_found = false;

                    'fields: for field in &fields_named.named {
                        match &field.ty {
                            Type::Path(path) => {
                                let mut primary = (false, false); // primary, auto_increment
                                let mut r#struct: Option<(Span, Vec<(Ident, Ident)>)> = None;
                                let mut relation: Option<Span> = None;

                                for attr in &field.attrs {
                                    if attr.path().is_ident("primary") {
                                        if primary_found {
                                            error.add(attr.path().span(), "mysql_connector does not support composite primary keys");
                                        } else {
                                            primary_found = true;
                                            primary.0 = true;
                                            match &attr.meta {
                                            Meta::Path(_) => (),
                                            Meta::List(list) if list.tokens.to_string() == "AutoIncrement" => primary.1 = true,
                                            Meta::List(list) => error.add(list.tokens.span(), "expected identifier `AutoIncrement` or nothing"),
                                            _ => error.add(
                                                attr.meta.span(),
                                                "expected identifier or nothing",
                                            ),
                                        }
                                        }
                                    } else if attr.path().is_ident("simple_struct") {
                                        if r#struct.is_some() {
                                            error.add(
                                            attr.path().span(),
                                            "you can't specify multiple `simple_struct` attributes",
                                        );
                                        } else {
                                            match &attr.meta {
                                                Meta::List(list) => {
                                                    match parse_struct(
                                                        list.span(),
                                                        list.tokens.clone().into(),
                                                    ) {
                                                        Ok(fields) => {
                                                            r#struct = Some((attr.span(), fields))
                                                        }
                                                        Err(err) => error.add_err(err),
                                                    }
                                                }
                                                _ => error.add(
                                                    attr.meta.span(),
                                                    "expected type definition",
                                                ), // TODO: Example
                                            }
                                        }
                                    } else if attr.path().is_ident("relation") {
                                        if relation.is_some() {
                                            error.add(
                                                attr.path().span(),
                                                "you can't specify multiple `relation` attributes",
                                            );
                                        } else {
                                            relation = Some(attr.span());
                                        }
                                    }
                                }

                                let field_type = if let Some((span, fields)) = r#struct {
                                    if primary.0 {
                                        error.add(span, "primary key can't be a simple_struct");
                                        continue 'fields;
                                    }
                                    if relation.is_some() {
                                        error.add(span, "relation can't be a simple_struct");
                                        continue 'fields;
                                    }
                                    FieldType::Struct(fields)
                                } else if let Some(span) = relation {
                                    if primary.0 {
                                        error.add(span, "primary key can't be a relation");
                                        continue 'fields;
                                    }
                                    FieldType::Complex
                                } else {
                                    match primary.0 {
                                        true => FieldType::Primary(primary.1),
                                        false => FieldType::Simple,
                                    }
                                };

                                fields.push(Field {
                                    ident: field.ident.clone().unwrap(),
                                    path: path.clone(),
                                    r#type: field_type,
                                });
                            }
                            _ => error.add(
                                field.ty.span(),
                                "mysql_connector does not support this type",
                            ),
                        }
                    }
                    match error.error() {
                        Some(err) => Err(err),
                        None => Ok(Model {
                            vis: input.vis.clone(),
                            ident: input.ident.clone(),
                            table,
                            fields,
                        }),
                    }
                }
                Fields::Unnamed(_) => Err(syn::Error::new(
                    input.ident.span(),
                    "mysql_connector does not support derive for unnamed fields",
                )),
                Fields::Unit => Err(syn::Error::new(
                    input.ident.span(),
                    "mysql_connector does not support derive for unit fields",
                )),
            }
        }
        Data::Union(_) => Err(syn::Error::new(
            input.ident.span(),
            "mysql_connector does not support derive for unions",
        )),
    }
}

fn parse_struct(span: Span, tokens: TokenStream) -> Result<Vec<(Ident, Ident)>, syn::Error> {
    let simple_struct = match syn::parse::<Expr>(tokens) {
        Ok(Expr::Struct(x)) => x,
        Ok(_) => {
            return Err(syn::Error::new(span, "expected struct"));
        }
        Err(err) => {
            return Err(err);
        }
    };

    let mut error = Error::empty();
    let mut fields = Vec::new();

    'struct_fields: for field in simple_struct.fields {
        let member = match field.member {
            Member::Named(x) => x,
            Member::Unnamed(_) => unreachable!("there are no unnamed fields in structs"),
        };
        let ident = match field.colon_token {
            Some(_) => {
                if let Expr::Path(path) = &field.expr {
                    if path.path.segments.len() != 1 {
                        error.add(field.expr.span(), "expected single identifier");
                        continue 'struct_fields;
                    }
                    path.path.segments[0].ident.clone()
                } else {
                    error.add(field.expr.span(), "expected identifier");
                    continue 'struct_fields;
                }
            }
            None => member.clone(),
        };
        fields.push((member, ident));
    }

    match error.error() {
        Some(err) => Err(err),
        None => Ok(fields),
    }
}
