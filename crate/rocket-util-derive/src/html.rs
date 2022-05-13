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
        let pat = input.parse()?;
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
    ForLoop {
        pat: Pat,
        expr: Expr,
        body: Input,
    },
    If {
        cond: Expr,
        then_branch: Input,
        else_branch: Option<Box<Entry>>,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },
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

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        Ok(if lookahead.peek(Token![@]) {
            let _ = input.parse::<Token![@]>()?;
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![for]) {
                let _ = input.parse::<Token![for]>()?;
                let pat = input.parse()?;
                let _ = input.parse::<Token![in]>()?;
                let expr = Expr::parse_without_eager_brace(input)?;
                let content;
                braced!(content in input);
                Self::ForLoop { pat, expr, body: content.parse()? }
            } else if lookahead.peek(Token![if]) {
                let _ = input.parse::<Token![if]>()?;
                let cond = Expr::parse_without_eager_brace(input)?;
                let content;
                braced!(content in input);
                Self::If {
                    cond,
                    then_branch: content.parse()?,
                    else_branch: if input.peek(Token![else]) {
                        let _ = input.parse::<Token![else]>()?;
                        let lookahead = input.lookahead1();
                        if lookahead.peek(Token![if]) || lookahead.peek(token::Brace) {
                            Some(Box::new(input.parse()?))
                        } else {
                            return Err(lookahead.error())
                        }
                    } else {
                        None
                    },
                }
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
            } else {
                return Err(lookahead.error())
            }
        } else if lookahead.peek(Ident::peek_any) {
            let tag = Some(Ident::parse_any(input)?);
            let attrs = if input.peek(token::Paren) {
                let content;
                parenthesized!(content in input);
                content.parse_terminated::<_, Token![,]>(Attr::parse)?.into_iter().collect()
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
    fn to_tokens(self) -> TokenStream {
        match self {
            Self::ForLoop { pat, expr, body } => {
                let body = body.to_tokens();
                quote! {{
                    let mut buf = ::std::string::String::new();
                    for #pat in #expr { buf.push_str(&#body.0) }
                    ::rocket_util::rocket::response::content::RawHtml(buf)
                }}
            }
            Self::If { cond, then_branch, else_branch: Some(else_branch) } => {
                let then_branch = then_branch.to_tokens();
                let else_branch = else_branch.to_tokens();
                quote!((if #cond { #then_branch } else { #else_branch }))
            }
            Self::If { cond, then_branch, else_branch: None } => {
                let then_branch = then_branch.to_tokens();
                quote!((if #cond { #then_branch } else { ::rocket_util::rocket::response::content::RawHtml(::std::string::String::new()) }))
            }
            Self::Match { expr, arms } => {
                let arms = arms.into_iter().map(|MatchArm { pat, guard, body }| {
                    let guard = guard.map(|guard| quote!(if #guard));
                    let body = body.to_tokens();
                    quote!(#pat #guard => #body)
                });
                quote!((match #expr { #(#arms),* }))
            }
            Self::While { cond, body } => {
                let body = body.to_tokens();
                quote! {{
                    let mut buf = ::std::string::String::new();
                    while #cond { buf.push_str(&#body.0) }
                    ::rocket_util::rocket::response::content::RawHtml(buf)
                }}
            }
            Self::Simple { tag: Some(tag), attrs, content } => {
                let is_void = matches!(
                    &*tag.unraw().to_string().to_ascii_lowercase(),
                    "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr"
                );
                if is_void && !matches!(content, Content::Empty) {
                    return quote_spanned!(tag.span()=> { compile_error!("this HTML tag must be empty"); })
                }
                let content = match content {
                    Content::Empty => quote!(),
                    Content::Flat(expr) => quote!(buf.push_str(&::rocket_util::ToHtml::to_html(&(#expr)).0);),
                    Content::Nested(Input(entries)) => {
                        let entries = entries.into_iter().map(Entry::to_tokens);
                        quote!(#(buf.push_str(&#entries.0);)*)
                    }
                };
                let open_tag = format!("<{}", tag.unraw());
                let attrs = attrs.into_iter().map(|Attr { name, value }| match value {
                    AttrValue::Empty => {
                        let attr = format!(" {}", name.unraw());
                        quote!(buf.push_str(#attr);)
                    }
                    AttrValue::Simple(value) => {
                        let attr = format!(" {}=\"", name.unraw());
                        quote! {
                            buf.push_str(#attr);
                            buf.push_str(&::rocket_util::ToHtml::to_html(&(#value)).0);
                            buf.push('"');
                        }
                    }
                    AttrValue::Optional(value) => {
                        let attr_no_value = format!(" {}", name.unraw());
                        let attr_with_value = format!(" {}=\"", name.unraw());
                        quote! {
                            match ::rocket_util::OptionalAttr::attr_value(#value) {
                                ::core::Option::None => {}
                                ::core::Option::Some(::core::Option::None) => buf.push_str(#attr_no_value),
                                ::core::Option::Some(::core::Option::Some(value)) => {
                                    buf.push_str(#attr_with_value);
                                    buf.push_str(&::rocket_util::ToHtml::to_html(&(#value)).0);
                                    buf.push('"');
                                }
                            }
                        }
                    }
                });
                let close_tag = (!is_void).then(|| {
                    let close_tag = format!("</{}>", tag.unraw());
                    quote!(buf.push_str(#close_tag);)
                });
                quote! {{
                    let mut buf = ::std::string::ToString::to_string(#open_tag);
                    #(#attrs)*
                    buf.push('>');
                    #content
                    #close_tag
                    ::rocket_util::rocket::response::content::RawHtml(buf)
                }}
            }
            Self::Simple { tag: None, attrs, content } => {
                assert!(attrs.is_empty());
                match content {
                    Content::Empty => quote!((::rocket_util::rocket::response::content::RawHtml(::std::string::String::new()))),
                    Content::Flat(expr) => quote!((::rocket_util::ToHtml::to_html(&(#expr)))),
                    Content::Nested(input) => input.to_tokens(),
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
    fn to_tokens(self) -> TokenStream {
        let entries = self.0.into_iter().map(Entry::to_tokens);
        quote! {{
            let mut buf = ::std::string::String::new();
            #(buf.push_str(&#entries.0);)*
            ::rocket_util::rocket::response::content::RawHtml(buf)
        }}
    }
}

pub(crate) fn mac(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parse_macro_input!(input as Input).to_tokens().into()
}
