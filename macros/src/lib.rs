extern crate proc_macro;

mod parse;

use {
    parse::{parse, parse_attr, parse_fields, Field, FieldType, NamedField, TypeComplexity},
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

#[proc_macro_derive(ModelData, attributes(table))]
pub fn derive_model_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let model = match parse(&input) {
        Ok(model) => model,
        Err(err) => return err.into_compile_error().into(),
    };

    match model.table {
        Some(table) => {
            let ident = &model.ident;
            let table_with_point = table.to_owned() + ".";
            quote! {
                impl mysql_connector::model::ModelData for #ident {
                    const TABLE: &'static str = #table;
                    const TABLE_WITH_POINT: &'static str = #table_with_point;
                }
            }
            .into()
        }
        None => syn::Error::new(
            input.ident.span(),
            "missing `table` attribute (`#[table(table_name)]`",
        )
        .into_compile_error()
        .into(),
    }
}

#[proc_macro_derive(FromQueryResult, attributes(mysql_connector))]
pub fn derive_from_query_result(input: TokenStream) -> TokenStream {
    let mut error = Error::empty();
    let input = parse_macro_input!(input as DeriveInput);

    let (_, _, types) = parse_attr(&mut error, input.ident.span(), &input.attrs);
    let fields = parse_fields(&mut error, input.ident.span(), &input.data, &types);

    if let Some(error) = error.error() {
        return error.into_compile_error().into();
    }

    let ident = &input.ident;
    let visibility = &input.vis;
    let mapping_ident = format_ident!("{ident}Mapping");

    let simple_field_names: &Vec<&Ident> = &fields
        .iter()
        .filter(TypeComplexity::simple_ref)
        .map(|x| &x.ident)
        .collect();
    let mut struct_field_names = Vec::new();
    let mut set_struct_fields = proc_macro2::TokenStream::new();
    for field in &fields {
        if let TypeComplexity::Struct(r#struct) = &field.complexity {
            let field_ident = &field.ident;
            let struct_path = &r#struct.path;
            let mapping_names = r#struct
                .fields
                .iter()
                .map(|x| format_ident!("{}_{}", field.ident, x.1));
            struct_field_names.extend(mapping_names.clone());
            let struct_names = r#struct.fields.iter().map(|x| &x.0);
            set_struct_fields = quote! {
                #set_struct_fields
                #field_ident: #struct_path {
                    #(#struct_names: row[mapping.#mapping_names.ok_or(mysql_connector::error::ParseError::MissingField(
                        concat!(stringify!(#ident), ".", stringify!(#mapping_names))
                    ))?].take().try_into()?,)*
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
            let column: &mut Option<usize> = match column.org_name() {
                #(stringify!(#simple_field_names) => &mut self.#simple_field_names,)*
                #(stringify!(#struct_field_names) => &mut self.#struct_field_names,)*
                _ => return,
            };
            *column = Some(index);
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
                        #(#simple_field_names: row[mapping.#simple_field_names.ok_or(mysql_connector::error::ParseError::MissingField(
                            concat!(stringify!(#ident), ".", stringify!(#simple_field_names))
                        ))?].take().try_into()?,)*
                        #set_struct_fields
                        #(#complex_field_names: <#complex_field_types>::from_mapping_and_row(&mapping.#complex_field_names, row)?,)*
                    })
                }
            }
        };
    }.into()
}

#[proc_macro_derive(ActiveModel, attributes(primary, simple_struct, relation))]
pub fn derive_active_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let model = match parse(&input) {
        Ok(model) => model,
        Err(err) => return err.into_compile_error().into(),
    };

    let mut struct_fields = proc_macro2::TokenStream::new();
    let mut into_values = quote! { let mut values = Vec::new(); };
    let mut get_primary = quote! { None };
    let mut into_active_model = proc_macro2::TokenStream::new();
    for Field {
        ident,
        path,
        r#type,
    } in model.fields
    {
        let value_type = match r#type {
            FieldType::Simple | FieldType::Primary(_) | FieldType::Struct(_) => "ActiveValue",
            FieldType::Complex => "ActiveReference",
        };
        let value_type = Ident::new(value_type, Span::call_site());

        struct_fields.extend(Some(quote! {
            pub #ident: mysql_connector::model::#value_type<#path>,
        }));

        let insert_value = match &r#type {
            FieldType::Simple | FieldType::Primary(_) => {
                quote! { self.#ident.insert_named_value(&mut values, stringify!(#ident))?; }
            }
            FieldType::Struct(fields) => {
                let mut set = proc_macro2::TokenStream::new();
                for (field_ident, name) in fields {
                    let field_name = format_ident!("{ident}_{name}");
                    set.extend(Some(quote! {
                        values.push(mysql_connector::model::NamedValue(
                            stringify!(#field_name),
                            value.#field_ident.try_into().map_err(Into::<mysql_connector::error::SerializeError>::into)?,
                        ));
                    }));
                }
                quote! {
                    match self.#ident {
                        mysql_connector::model::ActiveValue::Unset => (),
                        mysql_connector::model::ActiveValue::Set(value) => {
                            #set
                        }
                    }
                }
            }
            FieldType::Complex => {
                quote! { self.#ident.insert_named_value(&mut values, stringify!(#ident), conn).await?; }
            }
        };
        into_values.extend(Some(insert_value));

        if let FieldType::Primary(_) = r#type {
            get_primary = quote! {
                match self.#ident {
                    mysql_connector::model::ActiveValue::Set(x) => Some(x.into()),
                    mysql_connector::model::ActiveValue::Unset => None,
                }
            };
        }

        let into_active_model_field = match r#type {
            FieldType::Primary(_) => quote! { #ident: mysql_connector::model::ActiveValue::Unset, },
            FieldType::Simple | FieldType::Struct(_) => {
                quote! { #ident: mysql_connector::model::ActiveValue::Set(self.#ident), }
            }
            FieldType::Complex => quote! {
                #ident: mysql_connector::model::ActiveReference::Insert(
                    <#path as mysql_connector::model::HasActiveModel>::into_active_model(self.#ident)
                ),
            },
        };
        into_active_model.extend(Some(into_active_model_field));
    }
    into_values.extend(Some(quote! { Ok(values) }));

    let ident = &model.ident;
    let active_ident = format_ident!("{ident}ActiveModel");

    quote! {
        const _: () = {
            #[derive(Debug, Default)]
            pub struct #active_ident {
                #struct_fields
            }

            impl mysql_connector::model::ActiveModel<#ident> for #active_ident {
                async fn into_values(self, conn: &mut mysql_connector::Connection) ->
                    Result<Vec<mysql_connector::model::NamedValue>, mysql_connector::error::Error>
                {
                    #into_values
                }

                fn primary(&self) -> Option<mysql_connector::types::Value> {
                    #get_primary
                }
            }

            impl mysql_connector::model::HasActiveModel for #ident {
                type ActiveModel = #active_ident;

                fn into_active_model(self) -> Self::ActiveModel {
                    #active_ident {
                        #into_active_model
                    }
                }
            }
        };
    }
    .into()
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
