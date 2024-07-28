extern crate proc_macro;

mod parse;

use {
    parse::{parse, Field, FieldType},
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::{format_ident, quote},
    std::fmt,
    syn::{parse_macro_input, DeriveInput, Ident},
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

#[proc_macro_derive(FromQueryResult, attributes(simple_struct, relation))]
pub fn derive_from_query_result(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let model = match parse(&input) {
        Ok(model) => model,
        Err(err) => return err.into_compile_error().into(),
    };

    let visibility = &model.vis;
    let ident = &model.ident;
    let mapping_ident = format_ident!("{ident}Mapping");

    let mut struct_fields = proc_macro2::TokenStream::new();
    let mut set_child_mapping = proc_macro2::TokenStream::new();
    let mut set_own_mapping = proc_macro2::TokenStream::new();
    let mut from_mapping_and_row = proc_macro2::TokenStream::new();
    for Field {
        ident: field_ident,
        path,
        r#type,
    } in model.fields
    {
        match &r#type {
            FieldType::Simple
            | FieldType::Primary(_) => struct_fields.extend(Some(quote! { #field_ident: Option<usize>, })),
            FieldType::Struct(simple_struct_fields) => {
                for (_, struct_field_name) in simple_struct_fields {
                    let mapping_name = format_ident!("{}_{}", field_ident, struct_field_name);
                    struct_fields.extend(Some(quote! { #mapping_name: Option<usize>, }));
                }
            }
            FieldType::Complex => struct_fields.extend(Some(quote! { #field_ident: <#path as mysql_connector::model::FromQueryResult>::Mapping,})),
        }

        match &r#type {
            FieldType::Simple | FieldType::Primary(_) => {
                set_own_mapping.extend(Some(quote! {
                    stringify!(#field_ident) => &mut self.#field_ident,
                }));
            }
            FieldType::Struct(struct_fields) => {
                for (_, struct_field_name) in struct_fields {
                    let mapping_name = format_ident!("{}_{}", field_ident, struct_field_name);
                    set_own_mapping.extend(Some(quote! {
                        stringify!(#mapping_name) => &mut self.#mapping_name,
                    }));
                }
            }
            FieldType::Complex => {
                let name = field_ident.to_string();
                let name_with_point = name.clone() + ".";
                let len = name_with_point.as_bytes().len();

                set_child_mapping.extend(Some(quote! {
                    if table == #name {
                        self.#field_ident.set_mapping(column, "", index);
                    } else if table.starts_with(#name_with_point) {
                        self.#field_ident.set_mapping(column, &table[#len..], index);
                    } else
                }));
            }
        }

        let from_mapping_and_row_field = match &r#type {
            FieldType::Simple | FieldType::Primary(_) => quote! {
                #field_ident: row[mapping.#field_ident.ok_or(mysql_connector::error::ParseError::MissingField(
                    concat!(stringify!(#ident), ".", stringify!(#field_ident))
                ))?].take().try_into()?,
            },
            FieldType::Struct(struct_fields) => {
                let mut set_fields = proc_macro2::TokenStream::new();

                for (struct_field_ident, struct_field_name) in struct_fields {
                    let mapping_name = format_ident!("{}_{}", field_ident, struct_field_name);
                    set_fields.extend(Some(quote! {
                        #struct_field_ident: row[mapping.#mapping_name.ok_or(mysql_connector::error::ParseError::MissingField(
                            concat!(stringify!(#ident), ".", stringify!(#mapping_name))
                        ))?].take().try_into()?,
                    }));
                }

                quote! {
                    #field_ident: #path {
                       #set_fields
                    },
                }
            }
            FieldType::Complex => quote! {
                #field_ident: <#path>::from_mapping_and_row(&mapping.#field_ident, row)?,
            },
        };
        from_mapping_and_row.extend(Some(from_mapping_and_row_field));
    }
    set_child_mapping.extend(Some(quote! {
        {
            let column: &mut Option<usize> = match column.org_name() {
                #set_own_mapping
                _ => return,
            };
            *column = Some(index);
        }
    }));
    drop(set_own_mapping);

    quote! {
        const _: () = {
            #[derive(Default)]
            #visibility struct #mapping_ident {
                #struct_fields
            }

            impl mysql_connector::model::FromQueryResultMapping<#ident> for #mapping_ident {
                fn set_mapping_inner(&mut self, column: &mysql_connector::types::Column, table: &str, index: usize) {
                    #set_child_mapping
                }
            }

            impl mysql_connector::model::FromQueryResult for #ident {
                type Mapping = #mapping_ident;

                fn from_mapping_and_row(mapping: &Self::Mapping, row: &mut std::vec::Vec<mysql_connector::types::Value>) -> std::result::Result<Self, mysql_connector::error::ParseError> {
                    Ok(Self {
                        #from_mapping_and_row
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

#[proc_macro_derive(IntoQuery, attributes(simple_struct, relation))]
pub fn derive_into_query(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let model = match parse(&input) {
        Ok(model) => model,
        Err(err) => return err.into_compile_error().into(),
    };

    let mut columns = proc_macro2::TokenStream::new();
    for Field {
        ident: field_ident,
        path,
        r#type,
    } in model.fields
    {
        match r#type {
            FieldType::Simple | FieldType::Primary(_) => {
                let name = field_ident.to_string();
                columns.extend(Some(quote! {
                    mysql_connector::model::QueryColumn::Column(#name),
                }))
            }
            FieldType::Struct(struct_fields) => {
                for (_, struct_field_name) in struct_fields {
                    let mapping_name = format!("{}_{}", field_ident, struct_field_name);
                    columns.extend(Some(
                        quote! { mysql_connector::model::QueryColumn::Column(#mapping_name), },
                    ));
                }
            }
            FieldType::Complex => {
                let name = field_ident.to_string();
                columns.extend(Some(quote! {
                    mysql_connector::model::QueryColumn::Reference(mysql_connector::model::QueryColumnReference {
                        column: #name,
                        table: <#path as mysql_connector::model::ModelData>::TABLE,
                        key: <#path as mysql_connector::model::Model>::PRIMARY,
                        columns: <#path as mysql_connector::model::IntoQuery>::COLUMNS,
                    }),
                }))
            }
        }
    }

    let ident = &model.ident;
    quote! {
        impl mysql_connector::model::IntoQuery for #ident {
            const COLUMNS: &'static [mysql_connector::model::QueryColumn] = &[#columns];
        }
    }
    .into()
}

#[proc_macro_derive(Model, attributes(primary, simple_struct, relation))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let model = match parse(&input) {
        Ok(model) => model,
        Err(err) => return err.into_compile_error().into(),
    };

    let (primary_key, auto_increment, primary_type) =
        match model.fields.iter().find_map(|field| match field.r#type {
            FieldType::Primary(auto_increment) => Some((&field.ident, auto_increment, &field.path)),
            _ => None,
        }) {
            Some(x) => x,
            None => {
                return syn::Error::new(model.ident.span(), "missing primary key (`#[primary]`")
                    .into_compile_error()
                    .into()
            }
        };

    let ident = &model.ident;
    let primary_key_name = primary_key.to_string();

    quote! {
        impl mysql_connector::model::Model for #ident {
            const PRIMARY: &'static str = #primary_key_name;
            const AUTO_INCREMENT: bool = #auto_increment;

            type Primary = #primary_type;

            fn primary(&self) -> Self::Primary {
                self.#primary_key
            }
        }
    }
    .into()
}
