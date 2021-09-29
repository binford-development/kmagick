use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{self, Expr, ReturnType};
use quote::{ToTokens, quote};

mod utils;

#[proc_macro_attribute]
pub fn jni_method(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);
    let attrs = syn::parse_macro_input!(attr as syn::AttributeArgs);

    let args = match utils::get_args(attrs) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into()
    };

    let fn_inputs = utils::fn_full_args(&item_fn.sig.inputs);
    let (fn_call, fn_inputs) = match fn_inputs {
        Err(e) => return e.to_compile_error().into(),
        Ok(v) => v
    };
    let res = utils::validate_fn_args(&item_fn.sig.inputs, false, &vec![]);
    match res {
        Err(e) => return e.to_compile_error().into(),
        Ok(v) => v
    }

    let name = &item_fn.sig.ident;
    let name_str = name.to_string();
    let (java_return, is_result) = match utils::extract_return(&item_fn.sig.output, &name, None, &utils::top_attrs(&item_fn.attrs)) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into()
    };

    let is_returning = match java_return {
        ReturnType::Default => false,
        ReturnType::Type(_, _) => true
    };

    // cls is required
    let cls = args.get("cls");
    let java_fn;

    match cls {
        Some(v) => {
            java_fn = utils::class_to_ident(v, &name.to_string());
        }

        None => return syn::Error::new(Span::call_site(), "cls is a required attribute").to_compile_error().into()
    }

    let exc = args.get("exc");
    let exc = match exc {
        Some(v) => format!("\"{}\"", v),
        _ => String::from("\"java/lang/RuntimeException\"")
    };
    let exc = syn::parse_str::<Expr>(&exc);
    let exc = match exc {
        Ok(v) => v,
        Err(e) => return syn::Error::new(Span::call_site(), e.to_string()).to_compile_error().into()
    };

    // change the function output depending on whether it's a result type or not
    let match_res = if is_result {
        quote! {
            match c_res {
                Ok(v) => v,
                Err(e) => {
                    env.throw_new(#exc, e.to_string()).ok();
                    ::std::ptr::null_mut()
                }
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let res_binding = if is_result {
        quote! {
            let c_res =
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let res_semicolon = if is_returning {
        if is_result {
            quote! { ; }
        } else {
            proc_macro2::TokenStream::new()
        }
    } else {
        quote! { ; }
    };

    let null_mut = if is_returning {
        quote! {
            ::std::ptr::null_mut()
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let v_or_underscore = if is_returning {
        quote! { v }
    } else {
        quote! { _ }
    };

    let v_or_unit = if is_returning {
        quote! { v }
    } else {
        quote! { () }
    };

    let new_tokens = quote! {
        #item_fn

        #[no_mangle]
        pub extern "system" fn #java_fn(env: JNIEnv#fn_inputs) #java_return {
            let p_res = ::std::panic::catch_unwind(|| {
                #res_binding #name(#fn_call)#res_semicolon

                #match_res
            });

            match p_res {
                Ok(#v_or_underscore) => #v_or_unit,
                Err(e) => {
                    env.throw_new("java/lang/RuntimeException", &format!("`{}()` panicked", #name_str)).ok();
                    #null_mut
                }
            }
        }
    };

    new_tokens.into()
}

/// Don't generate an implementation for a method in an impl
#[proc_macro_attribute]
pub fn jni_ignore(_: TokenStream, item: TokenStream) -> TokenStream {
    // even though this is a no-op, this validates that it is an ItemFn and not something else
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);
    item_fn.to_token_stream().into()
}

#[proc_macro_attribute]
pub fn jni_static(_: TokenStream, item: TokenStream) -> TokenStream {
    // even though this is a no-op, this validates that it is an ItemFn and not something else
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);
    item_fn.to_token_stream().into()
}

#[proc_macro_attribute]
pub fn jni_destroy(_: TokenStream, item: TokenStream) -> TokenStream {
    // even though this is a no-op, this validates that it is an ItemFn and not something else
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);
    item_fn.to_token_stream().into()
}

#[proc_macro_attribute]
pub fn jni_new(_: TokenStream, item: TokenStream) -> TokenStream {
    // even though this is a no-op, this validates that it is an ItemFn and not something else
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);
    item_fn.to_token_stream().into()
}

#[proc_macro_attribute]
pub fn jni_class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = syn::parse_macro_input!(item as syn::ItemImpl);
    let mut item_impl_mod = item_impl.clone();
    let attrs = syn::parse_macro_input!(attr as syn::AttributeArgs);

    let args = match utils::get_args(attrs) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into()
    };

    let mut pkg = args.get("pkg");
    let mut cls = args.get("cls");
    if let Some(_) = pkg {
        if let Some(_) = cls {
            return syn::Error::new(Span::call_site(), "Can't use both pkg and cls attributes at same time").to_compile_error().into();
        }
    }
    if let None = pkg {
        if let None = cls {
            return syn::Error::new(Span::call_site(), "Must specify either pkg or cls attributes").to_compile_error().into();
        }
    }

    let f: String;
    let c: String;
    if let Some(v) = pkg {
        f = utils::fix_class_path(v, false);
        pkg = Some(&f);
    }
    if let Some(v) = cls {
        c = utils::fix_class_path(v, false);
        cls = Some(&c);
    }

    let name = utils::extract_impl_name(&*item_impl_mod.self_ty);
    let name = match name {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into()
    };

    // (data, is_pkg *, ident)
    // * as opposed to is_cls
    let namespace = if pkg.is_some() {
        (pkg.unwrap(), true, &name)
    } else {
        (cls.unwrap(), false, &name)
    };

    // filter out ignored methods
    utils::filter_out_ignored(&mut item_impl_mod);

    let vl = utils::validate_impl_args(&item_impl_mod.items);
    match vl {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into()
    };

    let impl_returns = utils::validate_impl_returns(&item_impl_mod.items, &name);
    let impl_returns = match impl_returns {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into()
    };
    
    let exc = match args.get("exc") {
        Some(v) => utils::fix_class_path(&*v, true),
        None => "java/lang/RuntimeException".to_owned()
    };

    let funcs = utils::generate_impl_functions(&item_impl_mod.items, &impl_returns, namespace, &exc);
    let funcs = match funcs {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into()
    };

    let mut stream = item_impl.to_token_stream();

    for f in funcs {
        stream.extend(f);
    }

    stream.into()
}
