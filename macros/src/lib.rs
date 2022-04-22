use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStruct, NestedMeta};

struct StructPrototype {
    name:  String,
    fields: Vec<Field>
}

impl StructPrototype {
    fn from_tok_stream(tokens: TokenStream) -> Option<StructPrototype> {
        let struct_tokens: ItemStruct = syn::parse(tokens).ok()?;

        let name = struct_tokens.ident.to_string();

        let mut fields: Vec<Field> = vec![];

        for f in &struct_tokens.fields {
            if let Some(ff) = Field::from_tok(f) {
                fields.push(ff);
            }
        }

        Some(StructPrototype { name, fields })
    }
}

struct Field {
    name: String,
    tpe: syn::Type,
    attributes: Vec<Attribute>
}

impl Field {
    fn from_tok(field: &syn::Field) -> Option<Field>
    {
        let attributes = collect_attrs(field);
        let name = field.ident.as_ref().map(|s| s.to_string())?;
        let tpe  = field.ty.clone();
        Some(Field { name, tpe, attributes })
    }

    fn as_int_primitive(&self) -> Option<(usize, String, String)> {
        endian_spec(self.tpe.clone(), &self.attributes)
    }
}

#[allow(dead_code)]
fn typename(tpe: &syn::Type) -> Option<String> {
    if let syn::Type::Path(tp) = tpe {
        tp.path.get_ident().map(|i| i.to_string())
    } else {
        None
    }
}

#[allow(dead_code)]
fn is_int(tpe: &syn::Type) -> bool {
    typename(tpe).map(|t| 
    match t.as_str() {
        "i16" | "u16" | "u32" | "i32" | "u64" | "i64" | "i128" | "u128" =>
            true,
        _ => 
            false
    }).unwrap_or(false)
}


#[derive(Debug, Eq, PartialEq, Clone)]
enum Attribute {
    Endian(String)
}

fn endian_spec(tpe: syn::Type, attrs: &[Attribute]) 
    -> Option<(usize, String, String)> /* (size, typename, endian */ 
{
    /* deal with endian attribute first to avoid huge nested scope */
    let endian = {
        let mut res = None;
        'next: for attr in attrs {
            #[allow(irrefutable_let_patterns)]
            if let Attribute::Endian(value) = attr {
                res = Some(value);
                break 'next;
            }
        }
        res
    }?;

    if let syn::Type::Path(tp) = tpe {
        let ident = tp.path.get_ident().map(|i| i.to_string())?;
        match ident.as_str() {
            "u16" | "i16" => Some((2, ident, endian.clone())),
            "u32" | "i32" => Some((4, ident, endian.clone())),
            "u64" | "i64" => Some((8, ident, endian.clone())),
            _ => None
        }
    } else {
        None
    }
}

fn collect_attrs(field: &syn::Field) -> Vec<Attribute> {
    let mut v = Vec::new();

    /* THE GOLDEN TRIANGLE !!!! */
    for attr in &field.attrs {
        if let Ok(syn::Meta::List(meta_list)) = attr.parse_meta() {
            for attribute in meta_list.nested.iter() {
                if let NestedMeta::Meta(syn::Meta::NameValue(nv)) = attribute {
                    if let Some(key) = nv.path.get_ident().map(|s| s.to_string()) {
                        if let syn::Lit::Str(litv) = &nv.lit {
                            let val = litv.value();
                            match key.as_str() {
                                "endian" => {
                                    match val.as_str() {
                                        "le" | "be" => { 
                                            v.push(Attribute::Endian(val));
                                        },
                                        _    => ()
                                    }
                                },
                                _ => ()
                            }
                        }
                    }
                }
            }
        }
    }

    v
}

#[proc_macro_derive(BytesSerializationSized)]
pub fn derive_serialization_sized(tokens: TokenStream) -> TokenStream {
    let s = StructPrototype::from_tok_stream(tokens).expect("can only apply to struct");

    let sizes = s.fields.iter().map(|field| {
        let name = format_ident!("{}", &field.name);
        match field.as_int_primitive() {
            None => quote! { self.#name.size() },
            Some((size, _t, _e)) => quote ! { #size }
        }
    });

    let struct_name = format_ident!("{}", s.name);

    (quote! {
        impl BytesSerializationSized for #struct_name {
            fn size(&self) -> usize {
                0 #(+ #sizes)*
            }
        }
    }).into()
}

#[proc_macro_derive(BytesDeserializable, attributes(bytes_serialize))]
pub fn derive_deserializable(tokens: TokenStream) -> TokenStream {
    let s = StructPrototype::from_tok_stream(tokens).expect("can only apply to struct");

    let field_names = s.fields.iter().map(|f| format_ident!("{}", f.name));

    let read = s.fields.iter().map(|field| {
        let name = format_ident!("{}", &field.name);
        let tpe  = &field.tpe;
        match field.as_int_primitive() {
            None => quote! {
                let #name = summon_from_bytes::<#tpe>(bytes, strict)?;
                bytes = &bytes[#name.size()..];
            },
            Some((size, tpe, e)) => {
                let func = format_ident!("from_{}_bytes", e);
                let tpei = format_ident!("{}", tpe);

                quote! {
                    if bytes.len() < #size {
                        return Err(Error::OutBufferTooSmall);
                    }
                    let #name = #tpei::#func(bytes.try_into().unwrap());
                    bytes = &bytes[#size..];
                }
            }
        }
    });

    let struct_name = format_ident!("{}", s.name);

    let ret = (quote! {
        impl<'a> BytesDeserializable<'a> for #struct_name {
            fn from_bytes(slice: &'a [u8], strict: bool) -> Result<#struct_name, Error>
            {
                let mut bytes = slice;

                #(#read ;)*

                Ok(#struct_name {
                    #(#field_names ,)*
                })
            }
        }
        
    }).into();

    ret
}


#[proc_macro_derive(BytesSerializable, attributes(bytes_serialize))]
pub fn derive_serializable(tokens: TokenStream) -> TokenStream
{
    let s = StructPrototype::from_tok_stream(tokens).expect("can only apply to struct");
    let write = s.fields.iter().map(|field| {
        let name = format_ident!("{}", &field.name);
        match field.as_int_primitive() {
            None => quote! {
                    self.#name.write_to_slice(bytes, strict)?;
                    bytes = &mut bytes[self.#name.size()..];
                },
            Some((size, _t, endian)) => {
                    let func = format_ident!("to_{}_bytes", endian);
                    quote! {
                        bytes[..#size].copy_from_slice(&self.#name.#func());
                        bytes = &mut bytes[#size..];
                    }
            }
        }
    });
    let struct_name = format_ident!("{}", s.name);

    let ret = (quote! {
        impl BytesSerializable for #struct_name {
            fn write_to_slice(&self, slice: &mut [u8], strict: bool) -> Result<(), Error>
            {
                if slice.len() < self.size() {
                    return Err(Error::OutBufferTooSmall);
                }

                let mut bytes = slice;
                #(#write ;)*

                Ok(())
            }
        }
    }).into();

    ret
}

