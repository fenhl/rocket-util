// This module contains code that was copied and modified from https://github.com/Stebalien/horrorshow-rs
// Copyright (c) 2015-2016 Horrorshow Authors (see AUTHORS)
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use {
    proc_macro2::TokenStream,
    quote::{
        quote,
        quote_spanned,
    },
    syn::{
        *,
        ext::IdentExt as _,
        parse::{
            Parse,
            ParseStream,
        },
    },
};

enum AttrValue {
    Empty,
    Simple(Expr),
    Optional(Expr),
}

impl Parse for AttrValue {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(if input.peek(Token![=]) {
            let _ = input.parse::<Token![=]>()?;
            Self::Simple(input.parse()?)
        } else if input.peek(Token![?]) {
            let _ = input.parse::<Token![?]>()?;
            let _ = input.parse::<Token![=]>()?;
            Self::Optional(input.parse()?)
        } else {
            Self::Empty
        })
    }
}

struct Attr {
    name: Ident,
    value: AttrValue,
}

impl Parse for Attr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            name: Ident::parse_any(input)?,
            value: input.parse()?,
        })
    }
}

enum Content {
    Empty,
    Flat(Expr),
    Nested(Input),
}

impl Parse for Content {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        Ok(if lookahead.peek(Token![;]) {
            let _ = input.parse::<Token![;]>()?;
            Self::Empty
        } else if lookahead.peek(Token![:]) {
            let _ = input.parse::<Token![:]>()?;
            let expr = input.parse()?;
            let _ = input.parse::<Token![;]>()?;
            Self::Flat(expr)
        } else if lookahead.peek(token::Brace) {
            let content;
            braced!(content in input);
            Self::Nested(content.parse()?)
        } else {
            return Err(lookahead.error())
        })
    }
}

struct MatchArm {
    pat: Pat,
    guard: Option<Box<Expr>>,
    body: Entry,
}

impl Parse for MatchArm {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let pat = Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(Token![if]) {
            let _ = input.parse::<Token![if]>()?;
            Some(Box::new(input.parse()?))
        } else {
            None
        };
        let _ = input.parse::<Token![=>]>()?;
        Ok(Self { pat, guard, body: input.parse()? })
    }
}

enum Entry {
    For {
        pat: Pat,
        expr: Expr,
        body: Input,
    },
    If {
        cond: Expr,
        then_branch: Input,
        else_branch: Option<Box<Entry>>,
    },
    Let {
        pat: Pat,
        init: Expr,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },
    Unimplemented,
    Unreachable,
    While {
        cond: Expr,
        body: Input,
    },
    Simple {
        tag: Option<Ident>,
        attrs: Vec<Attr>,
        content: Content,
    },
}

impl Entry {
    fn parse_if(input: ParseStream<'_>) -> Result<Self> {
        let _ = input.parse::<Token![if]>()?;
        let cond = Expr::parse_without_eager_brace(input)?;
        let content;
        braced!(content in input);
        Ok(Self::If {
            cond,
            then_branch: content.parse()?,
            else_branch: if input.peek(Token![else]) {
                let _ = input.parse::<Token![else]>()?;
                let lookahead = input.lookahead1();
                if lookahead.peek(Token![if]) {
                    Some(Box::new(Self::parse_if(input)?))
                } else if lookahead.peek(token::Brace) {
                    Some(Box::new(input.parse()?))
                } else {
                    return Err(lookahead.error())
                }
            } else {
                None
            },
        })
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        Ok(if lookahead.peek(Token![@]) {
            let _ = input.parse::<Token![@]>()?;
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![for]) {
                let _ = input.parse::<Token![for]>()?;
                let pat = Pat::parse_multi(input)?;
                let _ = input.parse::<Token![in]>()?;
                let expr = Expr::parse_without_eager_brace(input)?;
                let content;
                braced!(content in input);
                Self::For { pat, expr, body: content.parse()? }
            } else if lookahead.peek(Token![if]) {
                Self::parse_if(input)?
            } else if lookahead.peek(Token![let]) {
                let _ = input.parse::<Token![let]>()?;
                let pat = Pat::parse_multi(input)?;
                let _ = input.parse::<Token![=]>()?;
                let init = input.parse()?;
                let _ = input.parse::<Token![;]>()?;
                Self::Let { pat, init }
            } else if lookahead.peek(Token![match]) {
                let _ = input.parse::<Token![match]>()?;
                let expr = Expr::parse_without_eager_brace(input)?;
                let content;
                braced!(content in input);
                let mut arms = Vec::default();
                while !content.is_empty() {
                    arms.push(content.parse()?);
                }
                Self::Match { expr, arms }
            } else if lookahead.peek(Token![while]) {
                let _ = input.parse::<Token![while]>()?;
                let cond = Expr::parse_without_eager_brace(input)?;
                let content;
                braced!(content in input);
                Self::While { cond, body: content.parse()? }
            } else if lookahead.peek(Ident) {
                let ident = input.parse::<Ident>()?;
                match &*ident.to_string() {
                    "unimplemented" => Self::Unimplemented,
                    "unreachable" => Self::Unreachable,
                    _ => return Err(Error::new(ident.span(), "unexpected keyword")),
                }
            } else {
                return Err(lookahead.error())
            }
        } else if lookahead.peek(Ident::peek_any) {
            let tag = Some(Ident::parse_any(input)?);
            let attrs = if input.peek(token::Paren) {
                let content;
                parenthesized!(content in input);
                content.parse_terminated(Attr::parse, Token![,])?.into_iter().collect()
            } else {
                Vec::default()
            };
            Self::Simple { tag, attrs, content: input.parse()? }
        } else if lookahead.peek(Token![;]) || lookahead.peek(Token![:]) || lookahead.peek(token::Brace) {
            Self::Simple { tag: None, attrs: Vec::default(), content: input.parse()? }
        } else {
            return Err(lookahead.error())
        })
    }
}

impl Entry {
    fn to_tokens(self, internal: bool) -> TokenStream {
        let rocket_util = if internal { quote!(crate) } else { quote!(::rocket_util) };
        match self {
            Self::For { pat, expr, body } => {
                let body = body.0.into_iter().map(|entry| entry.to_tokens(internal));
                quote!(for #pat in #expr { #(#body)* })
            }
            Self::If { cond, then_branch, else_branch: Some(else_branch) } => {
                let then_branch = then_branch.0.into_iter().map(|entry| entry.to_tokens(internal));
                let else_branch = else_branch.to_tokens(internal);
                quote!(if #cond { #(#then_branch)* } else { #else_branch })
            }
            Self::If { cond, then_branch, else_branch: None } => {
                let then_branch = then_branch.0.into_iter().map(|entry| entry.to_tokens(internal));
                quote!(if #cond { #(#then_branch)* })
            }
            Self::Let { pat, init } => quote!(let #pat = #init;),
            Self::Match { expr, arms } => {
                let arms = arms.into_iter().map(|MatchArm { pat, guard, body }| {
                    let guard = guard.map(|guard| quote!(if #guard));
                    let body = body.to_tokens(internal);
                    quote!(#pat #guard => { #body })
                });
                quote!(match #expr { #(#arms),* })
            }
            Self::Unimplemented => quote!(unimplemented!();), //TODO stop generating code after this
            Self::Unreachable => quote!(unreachable!();), //TODO stop generating code after this
            Self::While { cond, body } => {
                let body = body.0.into_iter().map(|entry| entry.to_tokens(internal));
                quote!(while #cond { #(#body)* })
            }
            Self::Simple { tag: Some(tag), attrs, content } => {
                let is_void = matches!(
                    &*tag.unraw().to_string().to_ascii_lowercase(),
                    "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr"
                );
                if is_void && !matches!(content, Content::Empty) {
                    return quote_spanned!(tag.span()=> compile_error!("this HTML tag must be empty");)
                }
                let content = match content {
                    Content::Empty => quote!(),
                    Content::Flat(expr) => match expr {
                        Expr::Lit(ExprLit { attrs, lit: Lit::Str(s) }) if attrs.is_empty() => {
                            // special-case string literals to ensure HTML escaping is done at compile time
                            let mut escaped = Vec::with_capacity(s.value().len());
                            for b in s.value().bytes() {
                                match b {
                                    b'"' => escaped.extend_from_slice(b"&quot;"),
                                    b'&' => escaped.extend_from_slice(b"&amp;"),
                                    b'<' => escaped.extend_from_slice(b"&lt;"),
                                    b'>' => escaped.extend_from_slice(b"&gt;"),
                                    _ => escaped.push(b),
                                }
                            }
                            //SAFETY: `escaped` is derived from a valid UTF-8 string, with only ASCII characters replaced with other ASCII characters. Since UTF-8 is self-synchronizing, `escaped` remains valid UTF-8.
                            let escaped = unsafe { String::from_utf8_unchecked(escaped) };
                            quote!(__rocket_util_buf.push_str(#escaped);)
                        }
                        _ => quote!(__rocket_util_buf.push_str(&#rocket_util::ToHtml::to_html(&(#expr)).0);),
                    },
                    Content::Nested(Input(entries)) => {
                        let body = entries.into_iter().map(|entry| entry.to_tokens(internal));
                        quote! {{ #(#body)* }}
                    }
                };
                let open_tag = format!("<{}", tag.unraw());
                let attrs = attrs.into_iter().map(|Attr { name, value }| match value {
                    AttrValue::Empty => {
                        let attr = format!(" {}", name.unraw().to_string().replace('_', "-"));
                        quote!(__rocket_util_buf.push_str(#attr);)
                    }
                    AttrValue::Simple(value) => {
                        let attr = format!(" {}=\"", name.unraw().to_string().replace('_', "-"));
                        quote! {
                            __rocket_util_buf.push_str(#attr);
                            __rocket_util_buf.push_str(&#rocket_util::ToHtml::to_html(&(#value)).0);
                            __rocket_util_buf.push('"');
                        }
                    }
                    AttrValue::Optional(value) => {
                        let attr_no_value = format!(" {}", name.unraw().to_string().replace('_', "-"));
                        let attr_with_value = format!(" {}=\"", name.unraw().to_string().replace('_', "-"));
                        quote! {
                            match #rocket_util::OptionalAttr::attr_value(#value) {
                                ::core::option::Option::None => {}
                                ::core::option::Option::Some(::core::option::Option::None) => __rocket_util_buf.push_str(#attr_no_value),
                                ::core::option::Option::Some(::core::option::Option::Some(__rocket_util_value)) => {
                                    __rocket_util_buf.push_str(#attr_with_value);
                                    __rocket_util_buf.push_str(&#rocket_util::ToHtml::to_html(&__rocket_util_value).0);
                                    __rocket_util_buf.push('"');
                                }
                            }
                        }
                    }
                });
                let close_tag = (!is_void).then(|| {
                    let close_tag = format!("</{}>", tag.unraw());
                    quote!(__rocket_util_buf.push_str(#close_tag);)
                });
                quote! {
                    __rocket_util_buf.push_str(#open_tag);
                    #(#attrs)*
                    __rocket_util_buf.push('>');
                    #content
                    #close_tag
                }
            }
            Self::Simple { tag: None, attrs, content } => {
                assert!(attrs.is_empty());
                match content {
                    Content::Empty => quote!(),
                    Content::Flat(expr) => match expr {
                        Expr::Lit(ExprLit { attrs, lit: Lit::Str(s) }) if attrs.is_empty() => {
                            // special-case string literals to ensure HTML escaping is done at compile time
                            let mut escaped = Vec::with_capacity(s.value().len());
                            for b in s.value().bytes() {
                                match b {
                                    b'"' => escaped.extend_from_slice(b"&quot;"),
                                    b'&' => escaped.extend_from_slice(b"&amp;"),
                                    b'<' => escaped.extend_from_slice(b"&lt;"),
                                    b'>' => escaped.extend_from_slice(b"&gt;"),
                                    _ => escaped.push(b),
                                }
                            }
                            //SAFETY: `escaped` is derived from a valid UTF-8 string, with only ASCII characters replaced with other ASCII characters. Since UTF-8 is self-synchronizing, `escaped` remains valid UTF-8.
                            let escaped = unsafe { String::from_utf8_unchecked(escaped) };
                            quote!(__rocket_util_buf.push_str(#escaped);)
                        }
                        _ => quote!(__rocket_util_buf.push_str(&#rocket_util::ToHtml::to_html(&(#expr)).0);),
                    },
                    Content::Nested(Input(entries)) => {
                        let body = entries.into_iter().map(|entry| entry.to_tokens(internal));
                        quote! {{ #(#body)* }}
                    }
                }
            }
        }
    }
}

struct Input(Vec<Entry>);

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut entries = Vec::default();
        while !input.is_empty() {
            entries.push(input.parse()?);
        }
        Ok(Self(entries))
    }
}

impl Input {
    fn to_tokens(self, internal: bool) -> TokenStream {
        let rocket_util = if internal { quote!(crate) } else { quote!(::rocket_util) };
        let entries = self.0.into_iter().map(|entry| entry.to_tokens(internal));
        quote! {{
            let mut __rocket_util_buf = ::std::string::String::new();
            #(#entries)*
            #rocket_util::rocket::response::content::RawHtml(__rocket_util_buf)
        }}
    }
}

pub(crate) fn mac(input: proc_macro::TokenStream, internal: bool) -> proc_macro::TokenStream {
    let tokens = parse_macro_input!(input as Input).to_tokens(internal);
    proc_macro::TokenStream::from(quote! {{ #[allow(unused)] #tokens }})
}
