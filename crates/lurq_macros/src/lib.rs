use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
  parse_macro_input, parse_quote, AngleBracketedGenericArguments, Data, DeriveInput, Fields, GenericArgument,
  PathArguments, Type,
};

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
            buffer.push(::lurq::app::component::ComponentInfo::with_value(
              "variant",
              ::std::any::type_name::<Self>(),
              #variant_label,
            ));
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

#[proc_macro_derive(PersistentValue)]
pub fn derive_persistent_value(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  let struct_name = input.ident;
  let mut generics = input.generics;

  let fields = match input.data {
    Data::Struct(data_struct) => data_struct.fields,
    Data::Enum(_) | Data::Union(_) => {
      return quote! {
        compile_error!("PersistentValue can only be derived for structs.");
      }
      .into();
    }
  };

  let field_tys = fields.iter().map(|field| field.ty.clone()).collect::<Vec<_>>();
  {
    let where_clause = generics.make_where_clause();
    for field_ty in &field_tys {
      where_clause.predicates.push(parse_quote! {
        #field_ty: ::lurq::persistent_storage::PersistentValue + ::lurq::persistent_storage::IntoPersistentValue
      });
    }
  }

  let field_count = field_tys.len();
  let encode_fields = persistent_encode_fields(&fields);
  let decode_body = persistent_decode_body(&fields, &field_tys);

  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

  quote! {
    impl #impl_generics ::lurq::persistent_storage::IntoPersistentValue for #struct_name #ty_generics #where_clause {
      fn encode_persistent_value(self) -> ::std::vec::Vec<u8> {
        let mut bytes = ::lurq::persistent_storage::derive_support::begin_struct(
          ::std::any::type_name::<Self>(),
          #field_count,
        );
        #encode_fields
        bytes
      }
    }

    impl #impl_generics ::lurq::persistent_storage::PersistentValue for #struct_name #ty_generics #where_clause {
      fn decode_persistent_value(bytes: &[u8]) -> ::std::option::Option<Self> {
        let mut cursor = ::lurq::persistent_storage::derive_support::DecodeCursor::new(
          bytes,
          ::std::any::type_name::<Self>(),
          #field_count,
        )?;
        let value = #decode_body;
        cursor.finish()?;
        ::std::option::Option::Some(value)
      }
    }
  }
  .into()
}

fn persistent_encode_fields(fields: &Fields) -> proc_macro2::TokenStream {
  match fields {
    Fields::Named(fields) => {
      let pushes = fields.named.iter().map(|field| {
        let field_name = field.ident.as_ref().expect("named field should have an ident");
        quote! {
          ::lurq::persistent_storage::derive_support::push_field(&mut bytes, self.#field_name);
        }
      });
      quote! {
        #(#pushes)*
      }
    }
    Fields::Unnamed(fields) => {
      let pushes = fields.unnamed.iter().enumerate().map(|(index, _)| {
        let field_index = syn::Index::from(index);
        quote! {
          ::lurq::persistent_storage::derive_support::push_field(&mut bytes, self.#field_index);
        }
      });
      quote! {
        #(#pushes)*
      }
    }
    Fields::Unit => quote! {},
  }
}

fn persistent_decode_body(fields: &Fields, field_tys: &[Type]) -> proc_macro2::TokenStream {
  match fields {
    Fields::Named(fields) => {
      let entries = fields.named.iter().zip(field_tys.iter()).map(|(field, field_ty)| {
        let field_name = field.ident.as_ref().expect("named field should have an ident");
        quote! {
          #field_name: cursor.read_field::<#field_ty>()?
        }
      });
      quote! {
        Self {
          #(#entries),*
        }
      }
    }
    Fields::Unnamed(_) => {
      let entries = field_tys.iter().map(|field_ty| {
        quote! {
          cursor.read_field::<#field_ty>()?
        }
      });
      quote! {
        Self(#(#entries),*)
      }
    }
    Fields::Unit => quote! { Self },
  }
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
          let value = devtools_value_expr(&field_ty, quote! { &self.#field_name });
          if let Some(value) = value {
            quote! {
              buffer.push(::lurq::app::component::ComponentInfo::with_value(
                #field_label,
                ::std::any::type_name::<#field_ty>(),
                #value,
              ));
            }
          } else {
            quote! {
              let mut children = ::std::vec::Vec::new();
              ::lurq::app::component::DevtoolsInspectable::write_info(&self.#field_name, &mut children);
              buffer.push(::lurq::app::component::ComponentInfo::with_children(
                #field_label,
                ::std::any::type_name::<#field_ty>(),
                children,
              ));
            }
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
          let field_index = syn::Index::from(index);
          let value = devtools_value_expr(&field_ty, quote! { &self.#field_index });
          if let Some(value) = value {
            quote! {
              buffer.push(::lurq::app::component::ComponentInfo::with_value(
                #field_label,
                ::std::any::type_name::<#field_ty>(),
                #value,
              ));
            }
          } else {
            quote! {
              let mut children = ::std::vec::Vec::new();
              ::lurq::app::component::DevtoolsInspectable::write_info(&self.#field_index, &mut children);
              buffer.push(::lurq::app::component::ComponentInfo::with_children(
                #field_label,
                ::std::any::type_name::<#field_ty>(),
                children,
              ));
            }
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

fn devtools_value_expr(ty: &Type, access: proc_macro2::TokenStream) -> Option<proc_macro2::TokenStream> {
  if is_str_ref(ty) {
    return Some(quote! { ::std::format!("{:?}", #access) });
  }

  let path = match ty {
    Type::Path(path) => &path.path,
    _ => return None,
  };
  let segment = path.segments.last()?;
  let ident = segment.ident.to_string();

  if ident == "Arc" {
    return Some(quote! { ::std::format!("{:p}", ::std::sync::Arc::as_ptr(#access)) });
  }

  if is_debug_safe_path(&ident, &segment.arguments) {
    return Some(quote! { ::std::format!("{:?}", #access) });
  }

  None
}

fn is_str_ref(ty: &Type) -> bool {
  let Type::Reference(reference) = ty else {
    return false;
  };
  matches!(
    reference.elem.as_ref(),
    Type::Path(path) if path.path.is_ident("str")
  )
}

fn is_debug_safe_type(ty: &Type) -> bool {
  if is_str_ref(ty) {
    return true;
  }

  let Type::Path(path) = ty else {
    return false;
  };
  let Some(segment) = path.path.segments.last() else {
    return false;
  };
  is_debug_safe_path(&segment.ident.to_string(), &segment.arguments)
}

fn is_debug_safe_path(ident: &str, arguments: &PathArguments) -> bool {
  if matches!(
    ident,
    "bool"
      | "i8"
      | "i16"
      | "i32"
      | "i64"
      | "i128"
      | "isize"
      | "u8"
      | "u16"
      | "u32"
      | "u64"
      | "u128"
      | "usize"
      | "f32"
      | "f64"
      | "String"
      | "Signal"
  ) {
    return true;
  }

  if matches!(ident, "Option" | "Vec" | "Box") {
    return first_type_argument(arguments).is_some_and(is_debug_safe_type);
  }

  false
}

fn first_type_argument(arguments: &PathArguments) -> Option<&Type> {
  let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) = arguments else {
    return None;
  };

  args.iter().find_map(|arg| match arg {
    GenericArgument::Type(ty) => Some(ty),
    _ => None,
  })
}
