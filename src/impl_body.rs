use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Attribute;

pub fn body(ast: &syn::DeriveInput) -> TokenStream2 {
    let name = &ast.ident;
    let generics = &ast.generics;

    eprintln!("a is ->{:#?}", &ast.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    eprintln!("impl_generics is {:#?}", impl_generics);
    eprintln!("ty_generics is {:#?}", ty_generics);
    eprintln!("where_clause is {:#?}", where_clause);

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
    let impl_debug_inner = fields.iter().map(|f| {
        eprintln!("f is {:#?}", f);
        let field_name = f.ident.as_ref().unwrap();
        fields_names.push(field_name);
        let field_name_string = field_name.to_string();
        fields_names_string.push(field_name_string.clone());
        let ty = f.ty.clone();
        tys.push(ty);
        let attr = f.attrs.clone();
        // println!("attr length is {}", attr.len());
        if attr.len() > 0 {
            if let syn::Attribute {
                meta: syn::Meta::NameValue(named_value),
                ..
            } = &attr[0]
            {
                // eprintln!("named_value -> {:#?}", named_value);
                let attr_ident = &named_value.path.segments[0].ident; //debug
                if attr_ident != "debug" {
                    panic!("the attribute should be debug")
                }
                // eprintln!("path in attribute is {:#?}", attr_ident);
                let attr_value = &named_value.value;
                eprintln!("value in attribute is {:#?}", attr_value);
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(litstr),
                    ..
                }) = attr_value
                {
                    let orig_format = litstr.value();
                    println!("origin_format:{}", orig_format);
                    let custom_format = transform_format(orig_format);
                    println!("custom_format here {}", custom_format);
                    // println!("format_method {}", format_method);
                    let output = quote! {
                        .field(#field_name_string, &format_args!(#custom_format, self.#field_name))
                    };
                    // eprintln!("output is {}", output);
                    output
                } else {
                    quote! {}
                }
            } else {
                quote! {}
            }
        } else {
            quote! {
                .field(#field_name_string, &self.#field_name)
            }
        }

        // let syn::Attribute {meta: {}}
    });

    let string_name = name.to_string();
    let impl_debug = impl_debug_func(
        name,
        &string_name,
        impl_debug_inner,
        impl_generics,
        ty_generics,
        where_clause,
    );

    quote! {
        #impl_debug
    }
}

fn impl_debug_func<T>(
    name: &syn::Ident,
    string_name: &str,
    impl_debug_inner: T,
    impl_generics: syn::ImplGenerics<'_>,
    ty_generics: syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream2
where
    T: Iterator<Item = TokenStream2>,
{
    quote! {
        impl #impl_generics  std::fmt::Debug for #name #ty_generics #where_clause  {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                f.debug_struct(#string_name)
                #(#impl_debug_inner)*
                .finish()
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
        println!("custom_format {}", custom_format);
        custom_format
    } else {
        panic!("format incorrect")
    }
}
