use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Accessors)]
pub fn derive_accessors(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  let struct_name = input.ident;

  let fields = match input.data {
    Data::Struct(data_struct) => match data_struct.fields {
      Fields::Named(fields) => fields.named,
      _ => {
        return quote! {
          compile_error!("Accessors can only be derived for structs with named fields.");
        }
        .into();
      }
    },
    _ => {
      return quote! {
        compile_error!("Accessors can only be derived for structs.");
      }
      .into();
    }
  };

  let methods = fields.into_iter().map(|field| {
    let field_name = field.ident.expect("named field should have an ident");
    let field_ty = field.ty;

    let getter_name = field_name.clone();
    let with_name = format_ident!("with_{}", field_name);
    let set_name = format_ident!("set_{}", field_name);

    quote! {
      pub fn #getter_name(&self) -> #field_ty
      where
        #field_ty: Copy,
      {
        self.#field_name
      }

      pub fn #with_name(mut self, value: #field_ty) -> Self {
        self.#field_name = value;
        self
      }

      pub fn #set_name(&mut self, value: #field_ty) {
        self.#field_name = value;
      }
    }
  });

  quote! {
    impl #struct_name {
      #(#methods)*
    }
  }
  .into()
}
