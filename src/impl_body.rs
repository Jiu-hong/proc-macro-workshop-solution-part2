use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn cycle_path(path_segment: &syn::PathSegment) {
    if let syn::PathSegment {
        arguments:
            syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }),
        ..
    } = &path_segment
    {
        let a = args;
        if args.len() > 0 {
            let generic_argument = &args[0];
            if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                path: syn::Path { segments, .. },
                ..
            })) = generic_argument
            {
                let path_segment = &segments[0];
                let ident = &path_segment.ident; // "T"
                let b = &segments[1];
                let ident_1 = &b.ident; //Value

                if segments.len() > 0 {
                    cycle_path(&segments[0]);
                }
            }
        } else {
            return;
        }
    } else {
        return;
    }
}

pub fn get_path(ty: &syn::Type) -> Option<&syn::PathSegment> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = &ty
    {
        let path_segment = &segments[0];
        let ident = &path_segment.ident; //Vec

        if let syn::PathSegment {
            arguments:
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }),
            ..
        } = &path_segment
        {
            let generic_argument = &args[0];
            if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                path: syn::Path { segments, .. },
                ..
            })) = generic_argument
            {
                let path_segment = &segments[0];
                let ident = &path_segment.ident; // "T"
                let b = &segments[1];
                let ident_1 = &b.ident; //Value
            }
            return Some(&segments[0]);
        }
        return None;
    }
    None
}
pub fn body(ast: &syn::DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let generics = &ast.generics;
    let mut generic_symbols = vec![];
    for x in generics.type_params() {
        generic_symbols.push(&x.ident);
    }
    println!("generic_symbols is {:#?}", generic_symbols);
    let generic_flag = &ast.generics.params.len() > &0usize;
    let mut phantom_only = false;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // if let syn::ImplGenerics(syn::Generics { params, .. }) = impl_generics {}
    let fields = match &ast.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => named,
        _ => panic!("should be struct"),
    };
    let mut fields_names = vec![];
    // let mut fields_names_string = vec![];
    let mut tys = vec![];
    let impl_debug_inner = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        fields_names.push(field_name);
        let field_name_string = field_name.to_string();
        // fields_names_string.push(field_name_string.clone());
        let mut phantom_field_flag = false;
        let mut generic_field_flag = false;
        let ty = f.ty.clone();
        eprint!("ty is {:#?}", ty);
        if let syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        }) = &ty
        {
            let ident = &segments[0].ident; //PhantomData
            if ident == "PhantomData" {
                if let syn::PathSegment {
                    arguments:
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            args,
                            ..
                        }),
                    ..
                } = &segments[0]
                {
                    if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                        path: syn::Path { segments, .. },
                        ..
                    })) = &args[0]
                    {
                        let ident = &segments[0].ident; // "T"
                        if generic_symbols.contains(&ident) {
                            phantom_field_flag = true
                        }
                    }
                }
            }
            // else if type ident is the generic type
            else if generic_symbols.contains(&ident) {
                generic_field_flag = true
            }
        };
        tys.push(ty);
        let attr = f.attrs.clone();
        // println!("attr length is {}", attr.len());
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
        // else no generic symbol at all by default
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
                quote! {
                    impl #impl_generics  std::fmt::Debug for #name #ty_generics where T: std::fmt::Debug  {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                            f.debug_struct(#string_name)
                            #(#inner_token_stream)*
                            .finish()
                        }
                    }
                }
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
    // let org_format = "0b{:08b}";
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
