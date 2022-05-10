#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        borrow::Cow,
        convert::Infallible as Never,
        fmt::{
            self,
            Write as _,
        },
        io::Cursor,
    },
    image::{
        ImageOutputFormat,
        RgbaImage,
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
            Debug,
            Responder,
        },
    },
};
pub use rocket_util_derive::Error;
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
pub(crate) struct Origin<'a>(pub(crate) rocket::http::uri::Origin<'a>);

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
