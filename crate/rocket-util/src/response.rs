use rocket::{
    request::Request,
    response::Responder,
};
#[cfg(any(feature = "ics", feature = "image", feature = "reqwest"))] use crate::Error;
#[cfg(any(feature = "ics", feature = "image"))] use rocket::http::ContentType;
#[cfg(feature = "ics")] use ics::ICalendar;
#[cfg(feature = "image")] use {
    std::io::Cursor,
    image::{
        ImageFormat,
        RgbaImage,
    },
};
#[cfg(feature = "reqwest")] use {
    std::io,
    futures::stream::TryStreamExt as _,
    tokio_util::io::StreamReader,
};

pub trait WrappedResponder {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'static>;
}

/// Implements [`rocket::response::Responder`] for a type implementing [`WrappedResponder`].
pub struct Response<T: WrappedResponder>(pub T);

impl<'r, T: WrappedResponder> Responder<'r, 'static> for Response<T> {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        WrappedResponder::respond_to(self.0, request)
    }
}

#[cfg(feature = "ics")]
impl WrappedResponder for ICalendar<'_> {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'static> {
        let mut buf = Vec::default();
        match self.write(&mut buf) {
            Ok(()) => (ContentType::Calendar, buf).respond_to(request),
            Err(e) => Error(e).respond_to(request),
        }
    }
}

#[cfg(feature = "image")]
impl WrappedResponder for RgbaImage {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'static> {
        let mut buf = Cursor::new(Vec::default());
        match self.write_to(&mut buf, ImageFormat::Png) {
            Ok(()) => (ContentType::PNG, buf.into_inner()).respond_to(request),
            Err(e) => Error(e).respond_to(request),
        }
    }
}

#[cfg(feature = "reqwest")]
fn io_error_from_reqwest(e: reqwest::Error) -> io::Error {
    io::Error::new(if e.is_timeout() {
        io::ErrorKind::TimedOut
    } else {
        io::ErrorKind::Other
    }, e)
}

#[cfg(feature = "reqwest")]
impl WrappedResponder for reqwest::Response {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'static> {
        let mut builder = rocket::response::Response::build();
        builder.status(rocket::http::Status::new(self.status().as_u16()));
        for (name, value) in self.headers() {
            match std::str::from_utf8(value.as_bytes()) {
                Ok(value) => { builder.raw_header_adjoin(name.as_str().to_owned(), value.to_owned()); }
                Err(e) => return Error(e).respond_to(request),
            }
        }
        builder
            .streamed_body(StreamReader::new(self.bytes_stream().map_err(io_error_from_reqwest)))
            .ok()
    }
}
