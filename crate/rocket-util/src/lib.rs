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
            Status,
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
        response::Responder,
    },
};
#[cfg(feature = "rocket_csrf")] use {
    rocket::form::Contextual,
    rocket_csrf::CsrfToken,
};
pub use {
    rocket_util_derive::{
        Error,
        html,
    },
    crate::{
        html::{
            Doctype,
            OptionalAttr,
            ToHtml,
        },
        response::{
            Response,
            WrappedResponder,
        },
    },
};
#[doc(hidden)] pub use rocket; // used in proc macro
#[cfg(feature = "rocket_csrf")] pub use rocket_util_derive::CsrfForm;

mod html;
mod response;

#[cfg(feature = "rocket_csrf")]
pub trait CsrfForm {
    fn csrf(&self) -> &String;
}

#[cfg(feature = "rocket_csrf")]
pub trait ContextualExt {
    fn verify(&mut self, token: &Option<CsrfToken>);
}

#[cfg(feature = "rocket_csrf")]
impl<F: CsrfForm> ContextualExt for Contextual<'_, F> {
    fn verify(&mut self, token: &Option<CsrfToken>) {
        if let Some(ref value) = self.value {
            match token.as_ref().map(|token| token.verify(value.csrf())) {
                Some(Ok(())) => {}
                Some(Err(rocket_csrf::VerificationFailure)) | None => self.context.push_error(form::Error::validation("Please submit the form again to confirm your identity.").with_name("csrf")),
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error<E: std::error::Error>(#[from] pub E);

impl<'r, E: std::error::Error> Responder<'r, 'static> for Error<E> {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        eprintln!("responded with {} to request to {}", Status::InternalServerError, request.uri());
        eprintln!("display: {self}");
        eprintln!("debug: {self:?}");
        Err(Status::InternalServerError)
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

impl fmt::Display for Origin<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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

pub struct OptSuffix<'a, T>(pub T, pub Option<&'a str>);

impl<'a, T: FromParam<'a>> FromParam<'a> for OptSuffix<'a, T> {
    type Error = T::Error;

    fn from_param(param: &'a str) -> Result<Self, T::Error> {
        let (prefix, suffix) = if let Some((prefix, suffix)) = param.rsplit_once('.') {
            (prefix, Some(suffix))
        } else {
            (param, None)
        };
        Ok(Self(T::from_param(prefix)?, suffix))
    }
}

impl<'a, T: UriDisplay<Path>> UriDisplay<Path> for OptSuffix<'a, T> {
    fn fmt(&self, f: &mut uri::fmt::Formatter<'_, Path>) -> fmt::Result {
        self.0.fmt(f)?;
        if let Some(suffix) = self.1 {
            write!(f, ".{suffix}")?; //TODO ensure URI safety
        }
        Ok(())
    }
}

impl_from_uri_param_identity!([Path] ('a, T: UriDisplay<Path>) OptSuffix<'a, T>);
