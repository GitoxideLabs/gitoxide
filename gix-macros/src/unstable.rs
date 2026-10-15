use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ImplItem, Item, ItemStruct, LitStr, Visibility, parse::Parse, parse_quote, spanned::Spanned};

struct Reason(LitStr);

impl Parse for Reason {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        if name != "reason" {
            return Err(syn::Error::new(name.span(), "expected `reason = \"...\"`"));
        }
        input.parse::<syn::Token![=]>()?;
        let reason: LitStr = input.parse()?;
        if reason.value().trim().is_empty() {
            return Err(syn::Error::new(
                reason.span(),
                "the instability reason must not be empty",
            ));
        }
        Ok(Self(reason))
    }
}

impl Reason {
    fn document(&self, attrs: &mut Vec<Attribute>) {
        let note = format!("\n\n**Unstable:** {}", self.0.value());
        attrs.push(parse_quote!(#[doc = #note]));
    }
}

fn restrict(vis: &mut Visibility) -> syn::Result<()> {
    if !matches!(vis, Visibility::Public(_)) {
        return Err(syn::Error::new(vis.span(), "expected an unrestricted `pub` item"));
    }
    *vis = parse_quote!(pub(crate));
    Ok(())
}

pub(crate) fn module(input: TokenStream) -> syn::Result<TokenStream> {
    struct Module {
        reason: Reason,
        item: syn::ItemMod,
    }
    impl Parse for Module {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let reason = input.parse()?;
            input.parse::<syn::Token![;]>()?;
            Ok(Self {
                reason,
                item: input.parse()?,
            })
        }
    }
    let Module { reason, item: module } = syn::parse2(input)?;
    let reason = reason.0;
    item(quote!(reason = #reason), quote!(#module))
}

pub(crate) fn item(attrs: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let reason: Reason = syn::parse2(attrs)?;
    let mut stable: Item = syn::parse2(input.clone())?;
    if let Item::Impl(item) = &mut stable {
        if item.trait_.is_some() {
            reason.document(&mut item.attrs);
            return Ok(quote!(#[cfg(feature = "unstable")] #stable));
        }
        let mut unstable = item.clone();
        for (stable, unstable) in item.items.iter_mut().zip(&mut unstable.items) {
            let (vis, attrs) = match (stable, unstable) {
                (ImplItem::Fn(stable), ImplItem::Fn(unstable)) => (&mut stable.vis, &mut unstable.attrs),
                (ImplItem::Const(stable), ImplItem::Const(unstable)) => (&mut stable.vis, &mut unstable.attrs),
                (ImplItem::Type(stable), ImplItem::Type(unstable)) => (&mut stable.vis, &mut unstable.attrs),
                _ => continue,
            };
            if matches!(vis, Visibility::Public(_)) {
                restrict(vis)?;
                reason.document(attrs);
            }
        }
        return Ok(quote! {
            #[cfg(feature = "unstable")]
            #unstable
            #[cfg(not(feature = "unstable"))]
            #[allow(dead_code)]
            #stable
        });
    }
    let vis = match &mut stable {
        Item::Const(item) => &mut item.vis,
        Item::Enum(item) => &mut item.vis,
        Item::ExternCrate(item) => &mut item.vis,
        Item::Fn(item) => &mut item.vis,
        Item::Mod(item) => &mut item.vis,
        Item::Static(item) => &mut item.vis,
        Item::Struct(item) => &mut item.vis,
        Item::Trait(item) => &mut item.vis,
        Item::TraitAlias(item) => &mut item.vis,
        Item::Type(item) => &mut item.vis,
        Item::Union(item) => &mut item.vis,
        Item::Use(item) => &mut item.vis,
        _ => {
            return Err(syn::Error::new(
                stable.span(),
                "expected a public item or implementation",
            ));
        }
    };
    restrict(vis)?;
    let mut docs = Vec::new();
    reason.document(&mut docs);
    Ok(quote! {
        #[cfg(feature = "unstable")]
        #(#docs)*
        #input
        #[cfg(not(feature = "unstable"))]
        #[allow(dead_code, unused_imports)]
        #stable
    })
}

pub(crate) fn fields(attrs: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if !attrs.is_empty() {
        return Err(syn::Error::new_spanned(
            attrs,
            "put `#[unstable(reason = \"...\")]` on each gated field",
        ));
    }
    let mut stable: ItemStruct = syn::parse2(input)?;
    let mut unstable = stable.clone();
    let mut annotated = false;
    for (stable, unstable) in stable.fields.iter_mut().zip(&mut unstable.fields) {
        let mut reason = None;
        for attr in &stable.attrs {
            if attr.path().is_ident("unstable") {
                if reason.is_some() {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "the field already has an instability reason",
                    ));
                }
                reason = Some(attr.parse_args::<Reason>()?);
            }
        }
        if let Some(reason) = reason {
            annotated = true;
            restrict(&mut stable.vis)?;
            stable.attrs.retain(|attr| !attr.path().is_ident("unstable"));
            unstable.attrs.retain(|attr| !attr.path().is_ident("unstable"));
            reason.document(&mut unstable.attrs);
        }
    }
    if !annotated {
        return Err(syn::Error::new_spanned(
            stable,
            "expected at least one field with `#[unstable(reason = \"...\")]`",
        ));
    }
    Ok(quote! {
        #[cfg(feature = "unstable")]
        #unstable
        #[cfg(not(feature = "unstable"))]
        #[allow(dead_code)]
        #stable
    })
}
