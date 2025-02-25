use std::vec;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Token, punctuated::Punctuated};

fn get_innermost_type(
    ty: &syn::Type,
    generic_symbols: &Vec<&syn::Ident>,
) -> (Option<syn::Type>, bool, bool) {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        return get_innermost_type_inner(segments, generic_symbols);
    }
    (None, false, false)
}

fn get_innermost_type_inner(
    segments: &Punctuated<syn::PathSegment, Token![::]>,
    generic_symbols: &Vec<&syn::Ident>,
) -> (Option<syn::Type>, bool, bool) {
    while !segments[0].arguments.is_none() && segments[0].ident != "PhantomData" {
        if let syn::PathSegment {
            arguments:
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }),
            ..
        } = &segments[0]
        {
            let a = &args[0];
            if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                path: syn::Path { segments, .. },
                ..
            })) = a
            {
                return get_innermost_type_inner(&segments, generic_symbols);
            };
        };
    }

    let mut phantomflag = false;
    let mut generic_flag = false;
    if segments[0].ident == "PhantomData" {
        phantomflag = true
    }
    if generic_symbols.contains(&&segments[0].ident) {
        generic_flag = true
    }
    let innermost_type = syn::Type::Path(syn::TypePath {
        path: syn::Path {
            segments: segments.clone(),
            leading_colon: None,
        },
        qself: None,
    });
    (Some(innermost_type), phantomflag, generic_flag)
}

pub fn body(ast: &syn::DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let generics = &ast.generics;
    let mut generic_symbols = vec![];
    let mut generic_type: Option<syn::Type> = None;
    for x in generics.type_params() {
        generic_symbols.push(&x.ident);
    }
    let generic_flag = &ast.generics.params.len() > &0usize;
    let mut phantom_only = false;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    eprintln!("impl_generics is {:#?}", impl_generics);
    eprint!("ty_generics is {:#?}", ty_generics);
    eprintln!("where_clause is {:#?}", where_clause);
    let fields = match &ast.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => named,
        _ => panic!("should be struct"),
    };
    let mut fields_names = vec![];

    let mut tys = vec![];
    let impl_debug_inner = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        fields_names.push(field_name);
        let field_name_string = field_name.to_string();

        let ty = f.ty.clone();
        let (innermost_type, phantom_field_flag, generic_field_flag) =
            get_innermost_type(&ty, &generic_symbols);
        if generic_field_flag {
            generic_type = innermost_type
        }

        tys.push(ty.clone());
        let attr = f.attrs.clone();
        let field_name_1 = if attr.len() > 0 {
            if let syn::Attribute {
                meta: syn::Meta::NameValue(named_value),
                ..
            } = &attr[0]
            {
                let attr_ident = &named_value.path.segments[0].ident; //debug
                if attr_ident != "debug" {
                    panic!("the attribute should be debug")
                }
                let attr_value = &named_value.value;
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(litstr),
                    ..
                }) = attr_value
                {
                    let orig_format = litstr.value();
                    let custom_format = transform_format(orig_format);
                    quote! {&format_args!(#custom_format, self.#field_name)}
                } else {
                    quote! {}
                }
            } else {
                quote! {}
            }
        } else {
            quote! {&self.#field_name}
        };
        if !phantom_field_flag {
            return (
                quote! {.field(#field_name_string, #field_name_1)},
                phantom_field_flag,
                generic_field_flag,
            );
        }
        (quote! {}, phantom_field_flag, generic_field_flag)
    });

    let mut inner_token_stream = vec![];
    let mut phantom_fields_flags = vec![];
    let mut generic_fields_flags = vec![];
    for inner in impl_debug_inner {
        inner_token_stream.push(inner.0);
        phantom_fields_flags.push(inner.1);
        generic_fields_flags.push(inner.2);
    }

    // if generic struct
    if generic_flag {
        //if generic symbol appears in phantom field
        if phantom_fields_flags.contains(&true) {
            // not in other generics
            if !generic_fields_flags.contains(&true) {
                phantom_only = true
            }
        }
    }

    let string_name = name.to_string();

    let impl_debug = impl_debug_func(
        name,
        &string_name,
        inner_token_stream.into_iter(),
        impl_generics,
        ty_generics,
        where_clause,
        generic_flag,
        generic_type,
        phantom_only,
    );

    quote! {
        #impl_debug
    }
}

fn impl_debug_func<T>(
    name: &syn::Ident,
    string_name: &str,
    inner_token_stream: T,
    impl_generics: syn::ImplGenerics<'_>,
    ty_generics: syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
    generic_flag: bool,
    generic_type: Option<syn::Type>,
    phantom_only: bool,
) -> TokenStream2
where
    T: Iterator<Item = TokenStream2>,
{
    // generic struct
    if generic_flag {
        // phantom only
        if phantom_only {
            quote! {
                impl #impl_generics  std::fmt::Debug for #name #ty_generics  {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                        f.debug_struct(#string_name)
                        #(#inner_token_stream)*
                        .finish()
                    }
                }
            }
        // phantom plus other fields contain generic symbol
        } else {
            if where_clause.is_none() {
                let generic_type = generic_type.unwrap();
                let output = quote! {
                    impl #impl_generics  std::fmt::Debug for #name #ty_generics where #generic_type: std::fmt::Debug  {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                            f.debug_struct(#string_name)
                            #(#inner_token_stream)*
                            .finish()
                        }
                    }
                };
                output
            } else {
                quote! {
                    impl #impl_generics  std::fmt::Debug for #name #ty_generics #where_clause  {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                            f.debug_struct(#string_name)
                            #(#inner_token_stream)*
                            .finish()
                        }
                    }
                }
            }
        }
    } else {
        // no generic struct
        quote! {
            impl std::fmt::Debug for #name   {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                    f.debug_struct(#string_name)
                    #(#inner_token_stream)*
                    .finish()
                }
            }

        }
    }
}

fn transform_format(org_format: String) -> String {
    let vec_org_format = org_format
        .split(['{', ':', '}'])
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>();
    if vec_org_format.len() > 1 {
        let mut value = vec_org_format[1].chars();
        let base = value.next_back().unwrap().to_string();

        let length = value.as_str().parse::<usize>().unwrap() + vec_org_format[0].len();

        let prefix = "{:#0";
        let suffix = "}";
        let custom_format = prefix.to_owned() + &length.to_string() + &base + suffix;
        custom_format
    } else {
        panic!("format incorrect")
    }
}
