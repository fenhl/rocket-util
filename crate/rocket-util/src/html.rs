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
    std::convert::Infallible as Never,
    rocket::response::content::RawHtml,
};

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