use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, ItemFn, MetaNameValue, Token, parse::Parser, punctuated::Punctuated};

use crate::query::{Parsed, parse_function};

pub fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
  let options = Punctuated::<MetaNameValue, Token![,]>::parse_terminated.parse2(args)?;
  let mut invalidates: Option<Vec<Expr>> = None;
  for option in options {
    if !option.path.is_ident("invalidates") || invalidates.is_some() {
      return Err(syn::Error::new_spanned(
        option.path,
        "expected a unique invalidates option",
      ));
    }
    let Expr::Array(selectors) = option.value else {
      return Err(syn::Error::new_spanned(
        option.value,
        "expected a list of queries, e.g. [get_task::all(), list_tasks::all()]",
      ));
    };
    invalidates = Some(selectors.elems.into_iter().collect());
  }
  let invalidates = invalidates.unwrap_or_default();

  let function: ItemFn = syn::parse2(input)?;
  let Parsed {
    names,
    patterns,
    types,
    data,
    error,
  } = parse_function(&function, "mutation")?;
  let name = &function.sig.ident;
  let definition = format_ident!("__LurqMutation_{}", name);
  let visibility = &function.vis;
  let attributes = &function.attrs;
  let cfg_attributes: Vec<_> = attributes
    .iter()
    .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
    .collect();
  let statements = &function.block.stmts;
  Ok(quote! {
    #(#attributes)*
    #visibility fn #name(#(#names: #types),*) -> ::lurq::query::MutationCall<
      impl ::lurq::query::MutationDefinition<Args = (#(#types,)*), Data = #data, Error = #error>
    > {
      ::lurq::query::MutationCall::<#definition>::new((#(#names,)*))
    }

    #(#cfg_attributes)*
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    struct #definition;
    #(#cfg_attributes)*
    impl ::lurq::query::MutationDefinition for #definition {
        type Args = (#(#types,)*);
        type Data = #data;
        type Error = #error;
        fn name() -> &'static str { concat!(module_path!(), "::", stringify!(#name)) }
        #[allow(unused_variables)]
        fn invalidate(client: &::lurq::query::QueryClient) {
          #(client.invalidate(#invalidates);)*
        }
        fn run(args: Self::Args) -> ::lurq::query::QueryFuture<Self::Data, Self::Error> {
          Box::pin(async move {
            let (#(#patterns,)*) = args;
            #(#statements)*
          })
        }
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_unsupported_definitions_and_options() {
    for source in [
      "fn m() -> Result<(), ()> { Ok(()) }",
      "async fn m<T>() -> Result<T, ()> { todo!() }",
      "async fn m() -> Option<()> { None }",
    ] {
      assert!(expand(quote!(), source.parse().unwrap()).is_err());
    }
    for options in [
      quote!(invalidates = get_task::all()),
      quote!(stale_time = "30s"),
      quote!(invalidates = [a::all()], invalidates = [b::all()]),
    ] {
      let source = quote!(
        async fn m() -> Result<(), ()> {
          Ok(())
        }
      );
      assert!(expand(options, source).is_err());
    }
  }

  #[test]
  fn accepts_arguments_and_the_queries_a_success_invalidates() {
    for source in [
      "async fn m() -> Result<(), ()> { Ok(()) }",
      "pub async fn m(mut id: u64, name: String) -> Result<String, String> { Ok(name) }",
    ] {
      for options in [quote!(), quote!(invalidates = [get_task::all(), get_task(1)])] {
        syn::parse2::<syn::File>(expand(options, source.parse().unwrap()).unwrap()).unwrap();
      }
    }
  }
}
