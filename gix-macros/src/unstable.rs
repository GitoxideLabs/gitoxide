use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ImplItem, Item, ItemStruct, LitStr, Visibility, parse::Parse, parse_quote, spanned::Spanned};

struct Instability {
    since: LitStr,
    reason: Option<LitStr>,
}

impl Parse for Instability {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut since = None;
        let mut reason = None;
        while !input.is_empty() {
            let name: syn::Ident = input.parse()?;
            let field = match name.to_string().as_str() {
                "since" => &mut since,
                "reason" => &mut reason,
                _ => return Err(syn::Error::new(name.span(), "expected `since` or `reason`")),
            };
            if field.is_some() {
                return Err(syn::Error::new(name.span(), format!("duplicate `{name}`")));
            }
            input.parse::<syn::Token![=]>()?;
            let value: LitStr = input.parse()?;
            if name == "since" && !valid_date(&value.value()) {
                return Err(syn::Error::new(
                    value.span(),
                    "expected a valid date in YYYY-MM-DD format",
                ));
            }
            if name == "reason" && value.value().trim().is_empty() {
                return Err(syn::Error::new(
                    value.span(),
                    "the instability reason must not be empty",
                ));
            }
            *field = Some(value);
            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
            }
        }
        let since = since.ok_or_else(|| input.error("missing required `since = \"YYYY-MM-DD\"`"))?;
        Ok(Self { since, reason })
    }
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7) && !byte.is_ascii_digit())
    {
        return false;
    }
    let number = |digits: &[u8]| {
        digits
            .iter()
            .fold(0u16, |value, digit| value * 10 + u16::from(digit - b'0'))
    };
    let year = number(&bytes[..4]);
    let month = number(&bytes[5..7]);
    let day = number(&bytes[8..]);
    let days = match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };
    year != 0 && (1..=days).contains(&day)
}

impl Instability {
    fn document(&self, attrs: &mut Vec<Attribute>) {
        let mut note = format!("\n\n**Unstable:** Since {}.", self.since.value());
        if let Some(reason) = &self.reason {
            note.push(' ');
            note.push_str(&reason.value());
        }
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

pub(crate) fn item(attrs: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let reason: Instability = syn::parse2(attrs)?;
    let mut stable: Item = syn::parse2(input)?;
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
                (ImplItem::Macro(item), _) => {
                    return Err(syn::Error::new_spanned(
                        item,
                        "cannot restrict the visibility of macro-generated impl members",
                    ));
                }
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
    let mut unstable = stable.clone();
    let (vis, attrs) = match (&mut stable, &mut unstable) {
        (Item::Const(stable), Item::Const(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Enum(stable), Item::Enum(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::ExternCrate(stable), Item::ExternCrate(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Fn(stable), Item::Fn(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Mod(stable), Item::Mod(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Static(stable), Item::Static(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Struct(stable), Item::Struct(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Trait(stable), Item::Trait(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::TraitAlias(stable), Item::TraitAlias(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Type(stable), Item::Type(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Union(stable), Item::Union(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        (Item::Use(stable), Item::Use(unstable)) => (&mut stable.vis, &mut unstable.attrs),
        _ => {
            return Err(syn::Error::new(
                stable.span(),
                "expected a public item or implementation",
            ));
        }
    };
    restrict(vis)?;
    reason.document(attrs);
    Ok(quote! {
        #[cfg(feature = "unstable")]
        #unstable
        #[cfg(not(feature = "unstable"))]
        #[allow(dead_code, unused_imports)]
        #stable
    })
}

pub(crate) fn fields(attrs: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if !attrs.is_empty() {
        return Err(syn::Error::new_spanned(attrs, "put `#[unstable]` on each gated field"));
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
                        "the field already has an instability annotation",
                    ));
                }
                reason = Some(if matches!(&attr.meta, syn::Meta::Path(_)) {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "missing required `since = \"YYYY-MM-DD\"`",
                    ));
                } else {
                    attr.parse_args::<Instability>()?
                });
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
            "expected at least one field with `#[unstable]`",
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
