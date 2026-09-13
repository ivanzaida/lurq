use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
  parse::Parser, punctuated::Punctuated, FnArg, GenericArgument, ItemFn, MetaNameValue, Pat, ReturnType, Token, Type,
};

pub fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
  let options = Punctuated::<MetaNameValue, Token![,]>::parse_terminated.parse2(args)?;
  let mut stale_time = quote!(None);
  let mut gc_time = quote!(None);
  let mut seen = std::collections::HashSet::new();
  for option in options {
    let name = option.path.get_ident().map(ToString::to_string).unwrap_or_default();
    if !matches!(name.as_str(), "stale_time" | "gc_time") || !seen.insert(name.clone()) {
      return Err(syn::Error::new_spanned(
        option.path,
        "expected a unique stale_time or gc_time option",
      ));
    }
    let syn::Expr::Lit(syn::ExprLit {
      lit: syn::Lit::Str(value),
      ..
    }) = &option.value
    else {
      return Err(syn::Error::new_spanned(
        option.value,
        "expected a duration string, e.g. \"30s\" or \"5m\"",
      ));
    };
    let nanos = duration_nanos(value)?;
    let expression = quote!(Some(::std::time::Duration::from_nanos(#nanos)));
    if name == "stale_time" {
      stale_time = expression;
    } else {
      gc_time = expression;
    }
  }

  let function: ItemFn = syn::parse2(input)?;
  let signature = &function.sig;
  if signature.asyncness.is_none()
    || signature.constness.is_some()
    || signature.unsafety.is_some()
    || signature.abi.is_some()
    || signature.variadic.is_some()
    || !signature.generics.params.is_empty()
    || signature.generics.where_clause.is_some()
  {
    return Err(syn::Error::new_spanned(
      signature,
      "query requires a safe, non-generic async free function",
    ));
  }
  let mut names = Vec::new();
  let mut patterns = Vec::new();
  let mut types = Vec::new();
  for argument in &signature.inputs {
    let FnArg::Typed(argument) = argument else {
      return Err(syn::Error::new_spanned(
        argument,
        "query methods are not supported; use a free function",
      ));
    };
    let Pat::Ident(pattern) = &*argument.pat else {
      return Err(syn::Error::new_spanned(
        &argument.pat,
        "query arguments must have simple names",
      ));
    };
    if pattern.by_ref.is_some() || pattern.subpat.is_some() {
      return Err(syn::Error::new_spanned(
        pattern,
        "query arguments must be owned values with simple names",
      ));
    }
    names.push(&pattern.ident);
    patterns.push(pattern);
    types.push(&argument.ty);
  }
  let ReturnType::Type(_, output) = &signature.output else {
    return Err(syn::Error::new_spanned(signature, "query must return Result<T, E>"));
  };
  let Type::Path(output) = &**output else {
    return Err(syn::Error::new_spanned(output, "query must return Result<T, E>"));
  };
  let segment = output.path.segments.last().unwrap();
  let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
    return Err(syn::Error::new_spanned(output, "query must return Result<T, E>"));
  };
  let arguments: Vec<_> = arguments.args.iter().collect();
  let [GenericArgument::Type(data), GenericArgument::Type(error)] = arguments.as_slice() else {
    return Err(syn::Error::new_spanned(output, "query must return Result<T, E>"));
  };
  if segment.ident != "Result" {
    return Err(syn::Error::new_spanned(output, "query must return Result<T, E>"));
  }
  let name = &signature.ident;
  let definition = format_ident!("__LurqQuery_{}", name);
  let visibility = &function.vis;
  let attributes = &function.attrs;
  let cfg_attributes: Vec<_> = attributes
    .iter()
    .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
    .collect();
  let statements = &function.block.stmts;
  Ok(quote! {
    #(#attributes)*
    #visibility fn #name(#(#names: #types),*) -> ::lurq::query::Query<
      impl ::lurq::query::QueryDefinition<Args = (#(#types,)*), Data = #data, Error = #error>
    > {
      ::lurq::query::Query::<#definition>::new((#(#names,)*))
    }

    #(#cfg_attributes)*
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    struct #definition;
    #(#cfg_attributes)*
    impl ::lurq::query::QueryDefinition for #definition {
        type Args = (#(#types,)*);
        type Data = #data;
        type Error = #error;
        fn name() -> &'static str { concat!(module_path!(), "::", stringify!(#name)) }
        fn options() -> ::lurq::query::QueryOptions {
          ::lurq::query::QueryOptions { stale_time: #stale_time, gc_time: #gc_time }
        }
        fn run(args: Self::Args) -> ::lurq::query::QueryFuture<Self::Data, Self::Error> {
          Box::pin(async move {
            let (#(#patterns,)*) = args;
            #(#statements)*
          })
        }
    }
    #(#cfg_attributes)*
    #visibility mod #name {
      /// Selects every cached argument combination of this query in a client.
      pub fn all() -> impl ::lurq::query::QuerySelector {
        ::lurq::query::QueryFamily::<super::#definition>::new()
      }
    }
  })
}

fn duration_nanos(value: &syn::LitStr) -> syn::Result<u64> {
  let text = value.value();
  let split = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
  let multiplier = match &text[split..] {
    "ns" => 1,
    "us" => 1_000,
    "ms" => 1_000_000,
    "s" => 1_000_000_000,
    "m" => 60_000_000_000,
    "h" => 3_600_000_000_000,
    _ => {
      return Err(syn::Error::new_spanned(
        value,
        "duration requires an integer and ns, us, ms, s, m, or h",
      ))
    }
  };
  text[..split]
    .parse::<u64>()
    .ok()
    .and_then(|n| n.checked_mul(multiplier))
    .ok_or_else(|| syn::Error::new_spanned(value, "invalid or overflowing duration"))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_unsupported_definitions_and_options() {
    for source in [
      "fn q() -> Result<(), ()> { Ok(()) }",
      "async fn q<T>() -> Result<T, ()> { todo!() }",
      "async fn q() -> Option<()> { None }",
    ] {
      assert!(expand(quote!(), source.parse().unwrap()).is_err());
    }
    for options in [
      quote!(stale_time = "1.5s"),
      quote!(retry = "3"),
      quote!(gc_time = "1m", gc_time = "2m"),
    ] {
      assert!(expand(
        options,
        quote!(
          async fn q() -> Result<(), ()> {
            Ok(())
          }
        )
      )
      .is_err());
    }
  }

  #[test]
  fn accepts_zero_and_multiple_arguments_and_checked_durations() {
    for source in [
      "async fn q() -> Result<(), ()> { Ok(()) }",
      "pub async fn q(mut id: u64, name: String) -> Result<String, String> { Ok(name) }",
    ] {
      syn::parse2::<syn::File>(expand(quote!(stale_time = "0s", gc_time = "5m"), source.parse().unwrap()).unwrap())
        .unwrap();
    }
    assert_eq!(duration_nanos(&syn::parse_quote!("30s")).unwrap(), 30_000_000_000);
    assert!(duration_nanos(&syn::parse_quote!("18446744073709551615h")).is_err());
  }
}
