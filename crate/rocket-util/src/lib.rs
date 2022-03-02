use {
    std::{
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
        http::{
            impl_from_uri_param_identity,
            uri::{
                self,
                fmt::{
                    Path,
                    UriDisplay,
                },
            },
        },
        request::{
            FromParam,
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
