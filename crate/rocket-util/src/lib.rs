#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]

use {
    std::{
        borrow::Cow,
        convert::Infallible as Never,
        fmt::{
            self,
            Write as _,
        },
    },
    rocket::{
        form::{
            self,
            FromFormField,
        },
        http::{
            impl_from_uri_param_identity,
            uri::{
                self,
                fmt::{
                    FromUriParam,
                    Path,
                    Query,
                    UriDisplay,
                },
            },
        },
        request::{
            self,
            FromParam,
            FromRequest,
            Request,
        },
        response::{
            content::RawHtml,
            Responder,
        },
    },
};
#[cfg(feature = "image")] use {
    std::io::Cursor,
    image::{
        ImageOutputFormat,
        RgbaImage,
    },
    rocket::response::Debug,
};
pub use rocket_util_derive::{
    Error,
    html,
};
#[doc(hidden)] pub use rocket; // used in proc macro

pub trait WrappedResponder {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'static>;
}

pub struct Response<T: WrappedResponder>(pub T);

impl<'r, T: WrappedResponder> Responder<'r, 'static> for Response<T> {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        WrappedResponder::respond_to(self.0, request)
    }
}

#[cfg(feature = "image")]
impl WrappedResponder for RgbaImage {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'static> {
        let mut buf = Cursor::new(Vec::default());
        match self.write_to(&mut buf, ImageOutputFormat::Png) {
            Ok(()) => buf.into_inner().respond_to(request),
            Err(e) => Debug(e).respond_to(request),
        }
    }
}

/// A URL without a hostname but with an absolute path and optional query.
///
/// Wrapper type used here to allow decoding from URI query
#[derive(Clone)]
pub struct Origin<'a>(pub rocket::http::uri::Origin<'a>);

#[rocket::async_trait]
impl<'a> FromRequest<'a> for Origin<'a> {
    type Error = Never;

    async fn from_request(req: &'a Request<'_>) -> request::Outcome<Self, Never> {
        <&rocket::http::uri::Origin<'_>>::from_request(req).await.map(|origin| Self(origin.clone()))
    }
}

impl<'a> FromFormField<'a> for Origin<'a> {
    fn from_value(field: form::ValueField<'a>) -> form::Result<'a, Self> {
        Ok(Self(rocket::http::uri::Origin::try_from(field.value).map_err(|e| form::Error::validation(e.to_string()))?))
    }
}

impl<'a> UriDisplay<Query> for Origin<'a> {
    fn fmt(&self, f: &mut rocket::http::uri::fmt::Formatter<'_, Query>) -> fmt::Result {
        UriDisplay::fmt(&self.0.to_string(), f)
    }
}

impl<'a> FromUriParam<Query, rocket::http::uri::Origin<'a>> for Origin<'a> {
    type Target = Self;

    fn from_uri_param(param: rocket::http::uri::Origin<'a>) -> Self {
        Self(param)
    }
}

impl_from_uri_param_identity!([Query] ('a) Origin<'a>);

impl From<Origin<'_>> for Cow<'_, str> {
    fn from(Origin(origin): Origin<'_>) -> Self {
        Self::Owned(origin.to_string())
    }
}

pub struct Suffix<'a, T>(pub T, pub &'a str);

#[derive(Debug)]
pub enum SuffixFromParamError<E> {
    Prefix(E),
    Split,
}

impl<'a, T: FromParam<'a>> FromParam<'a> for Suffix<'a, T> {
    type Error = SuffixFromParamError<T::Error>;

    fn from_param(param: &'a str) -> Result<Self, SuffixFromParamError<T::Error>> {
        let (prefix, suffix) = param.rsplit_once('.').ok_or(SuffixFromParamError::Split)?;
        Ok(Self(T::from_param(prefix).map_err(SuffixFromParamError::Prefix)?, suffix))
    }
}

impl<'a, T: UriDisplay<Path>> UriDisplay<Path> for Suffix<'a, T> {
    fn fmt(&self, f: &mut uri::fmt::Formatter<'_, Path>) -> fmt::Result {
        self.0.fmt(f)?;
        write!(f, ".{}", self.1) //TODO ensure URI safety
    }
}

impl_from_uri_param_identity!([Path] ('a, T: UriDisplay<Path>) Suffix<'a, T>);

pub trait ToHtml {
    fn to_html(&self) -> RawHtml<String>;
}

impl<T: ToString> ToHtml for RawHtml<T> {
    fn to_html(&self) -> RawHtml<String> {
        RawHtml(self.0.to_string())
    }
}

impl<'a> ToHtml for &'a str {
    fn to_html(&self) -> RawHtml<String> {
        let mut escaped = Vec::with_capacity(self.len());
        for b in self.bytes() {
            match b {
                b'"' => escaped.extend_from_slice(b"&quot;"),
                b'&' => escaped.extend_from_slice(b"&amp;"),
                b'<' => escaped.extend_from_slice(b"&lt;"),
                b'>' => escaped.extend_from_slice(b"&gt;"),
                _ => escaped.push(b),
            }
        }
        //SAFETY: `escaped` is derived from a valid UTF-8 string, with only ASCII characters replaced with other ASCII characters. Since UTF-8 is self-synchronizing, `escaped` remains valid UTF-8.
        RawHtml(unsafe { String::from_utf8_unchecked(escaped) })
    }
}

impl ToHtml for String {
    fn to_html(&self) -> RawHtml<String> {
        (&**self).to_html()
    }
}

impl<T: ToHtml> ToHtml for Option<T> {
    fn to_html(&self) -> RawHtml<String> {
        if let Some(value) = self {
            value.to_html()
        } else {
            RawHtml(Default::default())
        }
    }
}

impl ToHtml for Never {
    fn to_html(&self) -> RawHtml<String> {
        match *self {}
    }
}

/// Members of this trait can be used as the `value` in a `tag(attr? = value)` expression inside the [`html`] macro.
pub trait OptionalAttr {
    type Value: ToHtml;

    /// * If `Some(Some(value))` is returned, that value is rendered for the attribute.
    /// * If `Some(None)` is returned, the attribute does not have a value.
    /// * If `None` is returned, the attribute is omitted entirely.
    fn attr_value(self) -> Option<Option<Self::Value>>;
}

impl<T: ToHtml> OptionalAttr for Option<T> {
    type Value = T;

    fn attr_value(self) -> Option<Option<Self::Value>> {
        self.map(Some)
    }
}

impl OptionalAttr for bool {
    type Value = Never;

    fn attr_value(self) -> Option<Option<Self::Value>> {
        self.then(|| None)
    }
}
