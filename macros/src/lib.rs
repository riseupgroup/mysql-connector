extern crate proc_macro;

mod parse;

use {
    parse::{parse_attr, parse_fields, NamedField, TypeComplexity},
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::{format_ident, quote},
    std::fmt,
    syn::{parse_macro_input, DeriveInput, Ident, Type},
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
                vis: _,
                ident,
                ty: _,
            },
        ) in fields
            .iter()
            .filter(TypeComplexity::complex_ref)
            .enumerate()
        {
            let name = ident.to_string() + ".";
            let len = name.len();
            let maybe_else = if i == 0 { None } else { Some(quote!(else)) };

            set_child_mapping = quote! {
                #set_child_mapping
                #maybe_else if name.starts_with(#name) {
                    self.#ident.set_mapping(column, &name[#len..], index);
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
                fn set_mapping_inner(&mut self, column: &mysql_connector::types::Column, name: &str, index: usize) {
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

    let (_, _, types) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    let fields = parse_fields(&mut error, input.ident.span(), &input.data, &types);

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
                #(pub #complex_field_names: mysql_connector::model::ActiveValue<<#complex_field_types as mysql_connector::model::Model>::Primary>,)*
            }

            impl mysql_connector::model::ActiveModel<#ident> for #model_ident {
                fn into_values(self) -> Result<Vec<mysql_connector::model::NamedValue>, mysql_connector::error::Error> {
                    let mut values = Vec::new();
                    #(self.#simple_field_names.insert_named_value(&mut values, stringify!(#simple_field_names))?;)*
                    #insert_struct_fields
                    #(self.#complex_field_names.insert_named_value(&mut values, stringify!(#complex_field_names))?;)* // TODO: join with simple_fields
                    Ok(values)
                }
            }

            impl mysql_connector::model::HasActiveModel for #ident {
                type ActiveModel = #model_ident;
            }
        };
    }.into()
}

#[proc_macro_derive(Model)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let mut error = Error::empty();
    let input = parse_macro_input!(input as DeriveInput);

    let (attr_span, attrs, types) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    let fields = parse_fields(&mut error, input.ident.span(), &input.data, &types);

    let mut primary_type = None;
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

            type Primary = #primary_type;

            fn primary(&self) -> mysql_connector::types::Value {
                self.#primary_ident.into()
            }
        }
    }
    .into()
}
