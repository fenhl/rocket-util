use rocket::{
    request::Request,
    response::Responder,
};
#[cfg(any(feature = "ics", feature = "image"))] use {
    rocket::http::ContentType,
    crate::Error,
};
#[cfg(feature = "ics")] use ics::ICalendar;
#[cfg(feature = "image")] use {
    std::io::Cursor,
    image::{
        ImageOutputFormat,
        RgbaImage,
    },
};

pub trait WrappedResponder {
    fn respond_to(self, request: &Request<'_>) -> rocket::response::Result<'static>;
}

/// Implements [`rocket::response::Responder`] for a type implementing [`rocket_util::WrappedResponder`].
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
        match self.write_to(&mut buf, ImageOutputFormat::Png) {
            Ok(()) => (ContentType::PNG, buf.into_inner()).respond_to(request),
            Err(e) => Error(e).respond_to(request),
        }
    }
}
