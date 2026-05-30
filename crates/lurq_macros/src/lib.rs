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

#[proc_macro_derive(DevtoolsInspectable, attributes(devtools_ignore))]
pub fn derive_devtools_inspectable(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  let struct_name = input.ident;
  let generics = input.generics;
  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

  let write_info = match input.data {
    Data::Struct(data_struct) => devtools_inspectable_struct_entries(data_struct.fields),
    Data::Enum(data_enum) => {
      let arms = data_enum.variants.into_iter().map(|variant| {
        let variant_name = variant.ident;
        let variant_label = variant_name.to_string();
        let pattern = match variant.fields {
          Fields::Named(_) => quote! { Self::#variant_name { .. } },
          Fields::Unnamed(_) => quote! { Self::#variant_name(..) },
          Fields::Unit => quote! { Self::#variant_name },
        };
        quote! {
          #pattern => {
            buffer.push(::lurq::app::component::ComponentInfo::new("variant", #variant_label));
          }
        }
      });
      quote! {
        match self {
          #(#arms),*
        }
      }
    }
    Data::Union(_) => {
      return quote! {
        compile_error!("DevtoolsInspectable can only be derived for structs and enums.");
      }
      .into();
    }
  };

  quote! {
    impl #impl_generics ::lurq::app::component::DevtoolsInspectable for #struct_name #ty_generics #where_clause {
      fn write_info(&self, buffer: &mut ::std::vec::Vec<::lurq::app::component::ComponentInfo>) {
        #write_info
      }
    }
  }
  .into()
}

fn devtools_inspectable_struct_entries(fields: Fields) -> proc_macro2::TokenStream {
  match fields {
    Fields::Named(fields) => {
      let entries = fields
        .named
        .into_iter()
        .filter(|field| !has_devtools_ignore(field))
        .map(|field| {
          let field_name = field.ident.expect("named field should have an ident");
          let field_label = field_name.to_string();
          let field_ty = field.ty;
          quote! {
            buffer.push(::lurq::app::component::ComponentInfo::new(
              #field_label,
              ::std::any::type_name::<#field_ty>(),
            ));
          }
        });
      quote! {
        #(#entries)*
      }
    }
    Fields::Unnamed(fields) => {
      let entries = fields
        .unnamed
        .into_iter()
        .enumerate()
        .filter(|(_, field)| !has_devtools_ignore(field))
        .map(|(index, field)| {
          let field_label = index.to_string();
          let field_ty = field.ty;
          quote! {
            buffer.push(::lurq::app::component::ComponentInfo::new(
              #field_label,
              ::std::any::type_name::<#field_ty>(),
            ));
          }
        });
      quote! {
        #(#entries)*
      }
    }
    Fields::Unit => quote! {},
  }
}

fn has_devtools_ignore(field: &syn::Field) -> bool {
  field.attrs.iter().any(|attr| attr.path().is_ident("devtools_ignore"))
}
