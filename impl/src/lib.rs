extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::spanned::Spanned;
use syn::parse::{Parse, ParseStream, Result};
use syn::*;

struct Opts {
    name: Option<LitStr>,
    skip: Option<Option<LitStr>>,
    disable: bool,
    no_compile: bool,
    should_fail: bool,
    should_panic: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            name: None,
            skip: None,
            no_compile: false,
            disable: false,
            should_fail: false,
            should_panic: false,
        }
    }
}

impl Opts {
    fn parse_mut(&mut self, input: ParseStream) -> Result<()> {
        loop {
            match &input.parse::<Option<Ident>>()? {
                None => break,
                Some(id) if id == "name" => {
                    input.parse::<Token![=]>()?;
                    self.name = Some(input.parse()?)
                },
                Some(id) if id == "skip" => {
                    self.skip = Some(if let Some(_) = input.parse::<Option<Token![=]>>()? {
                        input.parse()?
                    } else {
                        None
                    });
                },
                Some(id) if id == "disable" => self.disable = true,
                Some(id) if id == "no_compile" => self.no_compile = true,
                Some(id) if id == "should_fail" => self.should_fail = true,
                Some(id) if id == "should_panic" => self.should_panic = true,
                Some(id) =>
                    return Err(Error::new_spanned(id, "unrecognized option")),
            }

            if let None = input.parse::<Option<Token![,]>>()? {
                break
            }
        }

        Ok(())
    }
}

impl Parse for Opts {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut opts = Self::default();
        opts.parse_mut(input)?;

        Ok(opts)
    }
}

struct FnArg {
    ident: Ident,
    ty: Type,
}

impl Parse for FnArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        Ok(FnArg { ident, ty })
    }
}

struct TestFn {
    fn_attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    args: Vec<FnArg>,
    ret_ty: Option<Type>,
    body: TokenStream2,
}

impl Parse for TestFn {
    fn parse(input: ParseStream) -> Result<Self> {
        let fn_attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<Token![fn]>()?;
        let ident: Ident = input.parse()?;

        let argument_list;
        parenthesized!(argument_list in input);

        let mut args = Vec::new();
        loop {
            if argument_list.is_empty() {
                break;
            }
            args.push(argument_list.parse::<FnArg>()?);
            if argument_list.is_empty() {
                break
            } else {
                argument_list.parse::<Token![,]>()?;
            }
        }

        let ret_ty = if input.parse::<Option<Token![->]>>()?.is_some() {
            Some(input.parse::<Type>()?)
        } else {
            None
        };

        let body;
        braced!(body in input);
        let body: TokenStream2 = body.parse()?;

        Ok(TestFn {
            fn_attrs,
            vis,
            ident,
            args,
            ret_ty,
            body,
        })
    }
}

#[proc_macro_attribute]
pub fn test(opts: TokenStream, input: TokenStream) -> TokenStream {
    let mut opts = parse_macro_input!(opts as Opts);
    let TestFn {
        fn_attrs,
        vis,
        ident,
        args,
        ret_ty,
        body,
    } = parse_macro_input!(input as TestFn);

    let fn_attrs: Vec<_> = fn_attrs.into_iter().filter(|attr| match attr.tokens.is_empty() {
        true if attr.path.is_ident("ignore") => {
            if opts.skip.is_none() {
                opts.skip = Some(Some(LitStr::new("ignore", attr.span())))
            }
            false
        },
        true if attr.path.is_ident("should_panic") => {
            opts.should_panic = true;
            false
        },
        true if attr.path.is_ident("test") => {
            // TODO support additional options applied here? the point of combining #[test] and #[mintest] is mostly because of compatibility with built-in tests though, so why would you put extra options here..?
            false
        },
        false if attr.path.is_ident("mintest") => {
            unimplemented!() // TODO support additional options applied here? maybe useless but could be convenient because #[test(opt)] isn't portable if you disable mintest and want to use rust's test crate...
        },
        _ => true,
    }).collect();

    let arg_names = args.iter().map(|fn_arg| &fn_arg.ident).collect::<Vec<_>>();
    let arg_types = args.iter().map(|fn_arg| &fn_arg.ty).collect::<Vec<_>>();
    let ret_ty = ret_ty.map(|ret| quote!(-> #ret));

    let path = quote! {
        ::mintest
    };

    let test_ident = format_ident!("{}__test", ident);
    let test_fn_ident = format_ident!("{}__test_fn", ident);
    let test_status = match (opts.disable, &opts.skip) {
        (true, _) => quote! { #path::TestStatus::Disable },
        (false, Some(None)) => quote! { #path::TestStatus::Skip(#path::internal::core::option::Option::None) },
        (false, Some(Some(reason))) => quote! { #path::TestStatus::Skip(#path::internal::core::option::Option::Some(#reason)) },
        (false, None) => quote! { #path::TestStatus::Enable },
    };
    let test_name = opts.name.map(|n| n.value()).unwrap_or(ident.to_string());
    let context_ident: Ident = parse_quote!(__test_context);
    let context_ident = arg_names.get(0).cloned().unwrap_or(&context_ident);
    let context_args = quote!(#context_ident: #path::TestContext);
    let test_expected = match (opts.should_panic, opts.should_fail) {
        (true, _) => quote! { #path::TestExpected::Panic },
        (false, true) => quote! { #path::TestExpected::Fail },
        (false, false) => quote! { #path::TestExpected::Success },
    };
    let test = quote! {
        #path::Test {
            status: #test_status,
            name: #test_name,
            test: #path::TestFn::Static(#test_fn_ident),
            expected: #test_expected,
        }
    };

    let body = match (opts.no_compile, opts.disable, opts.skip) {
        (true, false, None) => {
            Error::new_spanned(body, "no_compile test not disabled or skipped").to_compile_error()
        },
        (true, _, _) => {
            quote! { #path::internal::core::unimplemented!() }
        },
        (false, _, _) => {
            body
        },
    };

    let (test_attr, test_def) = match () {
        #[cfg(any(not(feature = "unstable-test"), feature = "test"))]
        _ => (quote! { #[#path::internal::distributed_slice(#path::TESTS)] }, quote!(static)),
        #[cfg(all(feature = "unstable-test", not(feature = "test")))]
        _ => (quote! { #[test_case] }, quote!(const)),
    };

    let expanded_test = quote! {
        #[allow(non_snake_case)]
        fn #test_fn_ident(#context_args) -> #path::TestResult {
            #path::IntoTestResult::into_test_result(#ident(#(#arg_names),*))
        }

        #test_attr
        #[allow(non_upper_case_globals)]
        #test_def #test_ident: #path::Test = #test;
    };

    let expanded_fn = quote! {
        #(#fn_attrs)*
        #vis fn #ident(#(#arg_names: #arg_types),*) #ret_ty
    };

    let expanded = match () {
        #[cfg(any(not(feature = "unstable-test"), feature = "test"))]
        _ => quote! {
            #expanded_fn {
                #expanded_test

                #body
            }
        },
        #[cfg(all(feature = "unstable-test", not(feature = "test")))]
        _ => quote! {
            #expanded_fn {
                #body
            }

            #expanded_test
        },
    };

    TokenStream::from(expanded)
}
