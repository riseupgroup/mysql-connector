extern crate proc_macro;

mod parse;

use {
    parse::{parse_attr, parse_fields, NamedField, TypeComplexity},
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::{format_ident, quote},
    std::fmt,
    syn::{parse_macro_input, DeriveInput, Ident, LitStr, Type},
};

struct Error(Option<syn::Error>);

impl Error {
    pub fn empty() -> Self {
        Self(None)
    }

    pub fn add<T: fmt::Display>(&mut self, span: Span, message: T) {
        let error = syn::Error::new(span, message);
        match &mut self.0 {
            Some(e) => e.combine(error),
            None => self.0 = Some(error),
        }
    }

    pub fn add_err(&mut self, error: syn::Error) {
        match &mut self.0 {
            Some(e) => e.combine(error),
            None => self.0 = Some(error),
        }
    }

    pub fn error(&mut self) -> Option<syn::Error> {
        self.0.take()
    }
}

#[proc_macro_derive(ModelData, attributes(mysql_connector))]
pub fn derive_model_data(input: TokenStream) -> TokenStream {
    let mut error = Error::empty();
    let input = parse_macro_input!(input as DeriveInput);

    let (attr_span, attrs, _) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    if let Some(span) = attr_span {
        if !attrs.contains_key("table") {
            error.add(span, "table needed (#[mysql_connector(table = \"...\")]");
        }
    }

    if let Some(error) = error.error() {
        return error.into_compile_error().into();
    }

    let ident = &input.ident;
    let table = attrs.get("table").unwrap();
    let table_with_point = table.to_owned() + ".";

    quote! {
        impl mysql_connector::model::ModelData for #ident {
            const TABLE: &'static str = #table;
            const TABLE_WITH_POINT: &'static str = #table_with_point;
        }
    }
    .into()
}

#[proc_macro_derive(FromQueryResult)]
pub fn derive_from_query_result(input: TokenStream) -> TokenStream {
    let mut error = Error::empty();
    let input = parse_macro_input!(input as DeriveInput);

    let (_, _, types) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    let fields = parse_fields(&mut error, input.ident.span(), &input.data, &types);

    if let Some(error) = error.error() {
        return error.into_compile_error().into();
    }

    let simple_field_names: &Vec<&Ident> = &fields
        .iter()
        .filter(TypeComplexity::simple_ref)
        .map(|x| &x.ident)
        .collect();
    let mut struct_field_names = Vec::new();
    let mut set_struct_fields = proc_macro2::TokenStream::new();
    for field in &fields {
        if let TypeComplexity::Struct(r#struct) = &field.complexity {
            let ident = &field.ident;
            let struct_path = &r#struct.path;
            let mapping_names = r#struct
                .fields
                .iter()
                .map(|x| format_ident!("{}_{}", field.ident, x.1));
            struct_field_names.extend(mapping_names.clone());
            let struct_names = r#struct.fields.iter().map(|x| &x.0);
            set_struct_fields = quote! {
                #set_struct_fields
                #ident: #struct_path {
                    #(#struct_names: row[mapping.#mapping_names.ok_or(mysql_connector::error::ParseError::MissingField(stringify!(#mapping_names)))?].take().try_into()?,)*
                },
            }
        }
    }
    let struct_field_names = &struct_field_names;

    let complex_field_names: &Vec<&Ident> = &fields
        .iter()
        .filter(TypeComplexity::complex_ref)
        .map(|x: &parse::NamedField| &x.ident)
        .collect();
    let complex_field_types: &Vec<&Type> = &fields
        .iter()
        .filter(TypeComplexity::complex_ref)
        .map(|x| &x.ty)
        .collect();

    let ident = &input.ident;
    let visibility = &input.vis;
    let mapping_ident = format_ident!("{ident}Mapping");

    let set_mapping = {
        let mut set_child_mapping = proc_macro2::TokenStream::new();

        for (
            i,
            NamedField {
                complexity: _,
                //vis: _,
                ident,
                ty: _,
            },
        ) in fields
            .iter()
            .filter(TypeComplexity::complex_ref)
            .enumerate()
        {
            let name = ident.to_string();
            let name_with_point = name.clone() + ".";
            let len = name_with_point.as_bytes().len();
            let maybe_else = if i == 0 { None } else { Some(quote!(else)) };

            set_child_mapping = quote! {
                #set_child_mapping
                #maybe_else if table == #name {
                    self.#ident.set_mapping(column, "", index);
                } else if table.starts_with(#name_with_point) {
                    self.#ident.set_mapping(column, &table[#len..], index);
                }
            };
        }

        let set_own_mapping = quote! {
            *match column.org_name() {
                #(stringify!(#simple_field_names) => &mut self.#simple_field_names,)*
                #(stringify!(#struct_field_names) => &mut self.#struct_field_names,)*
                _ => return,
            } = Some(index);
        };

        if !fields.iter().any(TypeComplexity::complex) {
            set_own_mapping
        } else {
            quote! {
                #set_child_mapping
                else {
                    #set_own_mapping
                }
            }
        }
    };

    quote! {
        const _: () = {
            #[derive(Default)]
            #visibility struct #mapping_ident {
                #(#simple_field_names: Option<usize>,)*
                #(#struct_field_names: Option<usize>,)*
                #(#complex_field_names: <#complex_field_types as mysql_connector::model::FromQueryResult>::Mapping,)*
            }

            impl mysql_connector::model::FromQueryResultMapping<#ident> for #mapping_ident {
                fn set_mapping_inner(&mut self, column: &mysql_connector::types::Column, table: &str, index: usize) {
                    #set_mapping
                }
            }

            impl mysql_connector::model::FromQueryResult for #ident {
                type Mapping = #mapping_ident;

                fn from_mapping_and_row(mapping: &Self::Mapping, row: &mut std::vec::Vec<mysql_connector::types::Value>) -> std::result::Result<Self, mysql_connector::error::ParseError> {
                    Ok(Self {
                        #(#simple_field_names: row[mapping.#simple_field_names.ok_or(mysql_connector::error::ParseError::MissingField(stringify!(#simple_field_names)))?].take().try_into()?,)*
                        #set_struct_fields
                        #(#complex_field_names: <#complex_field_types>::from_mapping_and_row(&mapping.#complex_field_names, row)?,)*
                    })
                }
            }
        };
    }.into()
}

#[proc_macro_derive(ActiveModel)]
pub fn derive_active_model(input: TokenStream) -> TokenStream {
    let mut error = Error::empty();
    let input = parse_macro_input!(input as DeriveInput);

    let (attr_span, attrs, types) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    let fields = parse_fields(&mut error, input.ident.span(), &input.data, &types);

    let primary = match attr_span {
        Some(span) => match attrs.get("primary") {
            Some(primary) => match attrs.get("auto_increment") {
                Some(ai) => Some((format_ident!("{primary}"), ai == "true")),
                None => {
                    error.add(
                        span,
                        "auto_increment needed (#[mysql_connector(auto_increment = \"...\")]",
                    );
                    None
                }
            },
            None => None,
        },
        None => None,
    };

    if let Some(error) = error.error() {
        return error.into_compile_error().into();
    }

    let mut insert_struct_fields = proc_macro2::TokenStream::new();
    for field in &fields {
        if let TypeComplexity::Struct(r#struct) = &field.complexity {
            let ident = &field.ident;
            let idents = r#struct.fields.iter().map(|(x, _)| x);
            let names = r#struct
                .fields
                .iter()
                .map(|(_, x)| format_ident!("{ident}_{x}"));
            insert_struct_fields = quote! {
                #insert_struct_fields
                match self.#ident {
                    mysql_connector::model::ActiveValue::Unset =>(),
                    mysql_connector::model::ActiveValue::Set(value) => {
                        #(values.push(mysql_connector::model::NamedValue(stringify!(#names), value.#idents.try_into().map_err(Into::<mysql_connector::error::SerializeError>::into)?));)*
                    }
                }
            };
        }
    }

    let simple_field_names: &Vec<&Ident> = &fields
        .iter()
        .filter(TypeComplexity::simple_ref)
        .map(|x| &x.ident)
        .collect();
    let (simple_field_names_without_primary, set_primary) = primary
        .as_ref()
        .and_then(|(primary, auto_increment)| {
            if *auto_increment {
                let field_names = simple_field_names
                    .iter()
                    .filter(|x| **x != primary)
                    .copied()
                    .collect();
                let set_primary = quote! {
                    #primary: mysql_connector::model::ActiveValue::Unset,
                };
                Some((field_names, set_primary))
            } else {
                None
            }
        })
        .unwrap_or_else(|| (simple_field_names.clone(), proc_macro2::TokenStream::new()));
    let get_primary = match primary {
        Some((primary, _)) => quote! {
            match self.#primary {
                mysql_connector::model::ActiveValue::Set(x) => Some(x.into()),
                mysql_connector::model::ActiveValue::Unset => None,
            }
        },
        None => quote! {None},
    };

    let simple_field_types: &Vec<&Type> = &fields
        .iter()
        .filter(TypeComplexity::simple_ref)
        .map(|x| &x.ty)
        .collect();
    let struct_field_names: &Vec<&Ident> = &fields
        .iter()
        .filter(TypeComplexity::struct_ref)
        .map(|x| &x.ident)
        .collect();
    let struct_field_types: &Vec<&Type> = &fields
        .iter()
        .filter(TypeComplexity::struct_ref)
        .map(|x| &x.ty)
        .collect();
    let complex_field_names: &Vec<&Ident> = &fields
        .iter()
        .filter(TypeComplexity::complex_ref)
        .map(|x| &x.ident)
        .collect();
    let complex_field_types: &Vec<&Type> = &fields
        .iter()
        .filter(TypeComplexity::complex_ref)
        .map(|x| &x.ty)
        .collect();

    let ident = &input.ident;
    let model_ident = format_ident!("{ident}ActiveModel");

    quote! {
        const _: () = {
            #[derive(Debug, Default)]
            pub struct #model_ident {
                #(pub #simple_field_names: mysql_connector::model::ActiveValue<#simple_field_types>,)*
                #(pub #struct_field_names: mysql_connector::model::ActiveValue<#struct_field_types>,)*
                #(pub #complex_field_names: mysql_connector::model::ActiveReference<#complex_field_types>,)*
            }

            impl mysql_connector::model::ActiveModel<#ident> for #model_ident {
                async fn into_values<S: mysql_connector::Stream>(self, conn: &mut mysql_connector::Connection<S>) -> Result<Vec<mysql_connector::model::NamedValue>, mysql_connector::error::Error> {
                    let mut values = Vec::new();
                    #(self.#simple_field_names.insert_named_value(&mut values, stringify!(#simple_field_names))?;)*
                    #insert_struct_fields
                    #(self.#complex_field_names.insert_named_value(&mut values, stringify!(#complex_field_names), conn).await?;)*
                    Ok(values)
                }

                fn primary(&self) -> Option<mysql_connector::types::Value> {
                    #get_primary
                }
            }

            impl mysql_connector::model::HasActiveModel for #ident {
                type ActiveModel = #model_ident;

                fn into_active_model(self) -> Self::ActiveModel {
                    #model_ident {
                        #set_primary
                        #(#simple_field_names_without_primary: mysql_connector::model::ActiveValue::Set(self.#simple_field_names_without_primary),)*
                        #(#struct_field_names: mysql_connector::model::ActiveValue::Set(self.#struct_field_names),)*
                        #(#complex_field_names: mysql_connector::model::ActiveReference::Insert(<#complex_field_types as mysql_connector::model::HasActiveModel>::into_active_model(self.#complex_field_names)),)*
                    }
                }
            }
        };
    }.into()
}

#[proc_macro_derive(IntoQuery)]
pub fn derive_into_query(input: TokenStream) -> TokenStream {
    let mut error = Error::empty();
    let input = parse_macro_input!(input as DeriveInput);

    let (_, _, types) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    let fields = parse_fields(&mut error, input.ident.span(), &input.data, &types);

    if let Some(error) = error.error() {
        return error.into_compile_error().into();
    }

    let mut simple_field_names: Vec<LitStr> = fields
        .iter()
        .filter(TypeComplexity::simple_ref)
        .map(|x| LitStr::new(&x.ident.to_string(), x.ident.span()))
        .collect();
    for field in &fields {
        if let TypeComplexity::Struct(r#struct) = &field.complexity {
            let mapping_names = r#struct
                .fields
                .iter()
                .map(|x| LitStr::new(&format!("{}_{}", field.ident, x.1), field.ident.span()));
            simple_field_names.extend(mapping_names);
        }
    }
    let complex_field_names: &Vec<LitStr> = &fields
        .iter()
        .filter(TypeComplexity::complex_ref)
        .map(|x| LitStr::new(&x.ident.to_string(), x.ident.span()))
        .collect();
    let complex_field_types: &Vec<&Type> = &fields
        .iter()
        .filter(TypeComplexity::complex_ref)
        .map(|x| &x.ty)
        .collect();

    let ident = &input.ident;

    quote! {
        impl mysql_connector::model::IntoQuery for #ident {
            const COLUMNS: &'static [mysql_connector::model::QueryColumn] = &[
                #(mysql_connector::model::QueryColumn::Column(#simple_field_names),)*
                #(mysql_connector::model::QueryColumn::Reference(mysql_connector::model::QueryColumnReference {
                    column: #complex_field_names,
                    table: <#complex_field_types as mysql_connector::model::ModelData>::TABLE,
                    key: <#complex_field_types as mysql_connector::model::Model>::PRIMARY,
                    columns: <#complex_field_types as mysql_connector::model::IntoQuery>::COLUMNS,
                }),)*
            ];
        }
    }.into()
}

#[proc_macro_derive(Model)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let mut error = Error::empty();
    let input = parse_macro_input!(input as DeriveInput);

    let (attr_span, attrs, types) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    let fields = parse_fields(&mut error, input.ident.span(), &input.data, &types);

    let mut primary_type = None;
    let mut auto_increment = false;
    if let Some(span) = attr_span {
        match attrs.get("primary") {
            Some(primary) => match fields.iter().find(|field| field.ident == primary) {
                Some(field) => primary_type = Some(&field.ty),
                None => error.add(span, "primary not found in struct"),
            },
            None => error.add(
                span,
                "primary needed (#[mysql_connector(primary = \"...\")]",
            ),
        }
        match attrs.get("auto_increment") {
            Some(ai) => auto_increment = ai == "true",
            None => error.add(
                span,
                "auto_increment needed (#[mysql_connector(auto_increment = \"...\")]",
            ),
        }
    }

    if let Some(error) = error.error() {
        return error.into_compile_error().into();
    }

    let primary = attrs.get("primary").unwrap();
    let primary_type = primary_type.unwrap();
    let primary_ident = Ident::new(primary, Span::call_site());
    let ident = &input.ident;

    quote! {
        impl mysql_connector::model::Model for #ident {
            const PRIMARY: &'static str = #primary;
            const AUTO_INCREMENT: bool = #auto_increment;

            type Primary = #primary_type;

            fn primary(&self) -> Self::Primary {
                self.#primary_ident
            }
        }
    }
    .into()
}
