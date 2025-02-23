use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn body(ast: &syn::DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let generics = &ast.generics;
    eprintln!("generics -> {:#?}", generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut generics_flag = false;

    if generics.params.len() > 0 {
        generics_flag = true;
    }

    let fields = match &ast.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => named,
        _ => panic!("should be struct"),
    };
    let mut fields_names = vec![];
    let mut fields_names_string = vec![];
    let mut tys = vec![];
    let mut generic_apply_to_phantom = false;
    let mut generic_apply_to_others = false;

    let impl_debug_inner = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        fields_names.push(field_name);
        let field_name_string = field_name.to_string();
        fields_names_string.push(field_name_string.clone());
        let ty = &f.ty;
        tys.push(f.ty.clone());
        let mut phontom_data = false;

        if let syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        }) = ty
        {
            let a = &segments[0];
            if a.ident == "PhantomData" {
                phontom_data = true;
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) = &a.arguments
                {
                    if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                        path: syn::Path { segments, .. },
                        ..
                    })) = &args[0]
                    {
                        let c = &segments[0].ident;
                        if c == "T" {
                            generic_apply_to_phantom = true
                        }
                    }
                };
            } else if a.ident == "T" {
                phontom_data = false;
                generic_apply_to_others = true
            }
        };

        let attr = f.attrs.clone();

        if attr.len() > 0 {
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
                    let output = quote! {
                        .field(#field_name_string, &format_args!(#custom_format, self.#field_name))
                    };
                    if phontom_data {
                        (quote! {}, generic_apply_to_phantom, generic_apply_to_others)
                    } else {
                        (output, generic_apply_to_phantom, generic_apply_to_others)
                    }
                } else {
                    (quote! {}, generic_apply_to_phantom, generic_apply_to_others)
                }
            } else {
                (quote! {}, generic_apply_to_phantom, generic_apply_to_others)
            }
        } else {
            if phontom_data {
                (quote! {}, generic_apply_to_phantom, generic_apply_to_others)
            } else {
                (
                    quote! {
                        .field(#field_name_string, &self.#field_name)
                    },
                    generic_apply_to_phantom,
                    generic_apply_to_others,
                )
            }
        }
    });

    let string_name = name.to_string();

    let mut token_stream = vec![];
    let mut phantom_flags = vec![]; //generic_apply_to_phantom
    let mut generic_other_flags = vec![]; //generic_apply_to_others

    for f in impl_debug_inner {
        token_stream.push(f.0);
        phantom_flags.push(f.1);
        generic_other_flags.push(f.2);
    }

    let impl_debug = impl_debug_func(
        name,
        &string_name,
        token_stream.into_iter(),
        impl_generics.clone(),
        ty_generics.clone(),
        where_clause,
        generic_other_flags.contains(&true),
        phantom_flags.contains(&true),
        generics_flag,
    );

    quote! {
        #impl_debug
    }
}

fn impl_debug_func<T>(
    name: &syn::Ident,
    string_name: &str,
    token_stream_inner: T,
    impl_generics: syn::ImplGenerics<'_>,
    ty_generics: syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
    generic_other_flags: bool,
    phantom_flags: bool,
    generics_flag: bool,
) -> TokenStream2
where
    T: Iterator<Item = TokenStream2>,
{
    if !generic_other_flags {
        if phantom_flags && !generic_other_flags {
            quote! {
                impl #impl_generics  std::fmt::Debug for #name #ty_generics  {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                        f.debug_struct(#string_name)
                        #(#token_stream_inner)*
                        .finish()
                    }
                }
            }
        } else {
            if where_clause.is_none() {
                quote! {
                    impl #impl_generics  std::fmt::Debug for #name #ty_generics where T: std::fmt::Debug    {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                            f.debug_struct(#string_name)
                            #(#token_stream_inner)*
                            .finish()
                        }
                    }
                }
            } else {
                quote! {
                    impl #impl_generics  std::fmt::Debug for #name #ty_generics #where_clause {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                            f.debug_struct(#string_name)
                            #(#token_stream_inner)*
                            .finish()
                        }
                    }
                }
            }
        }
    } else {
        quote! {
            impl std::fmt::Debug for #name  {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                    f.debug_struct(#string_name)
                    #(#token_stream_inner)*
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
