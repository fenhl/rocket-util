This is a [Rust](https://rust-lang.org/) crate containing utilities for writing web apps with [Rocket](https://rocket.rs/). It includes:

* a derive macro to generate error responses, complementing the derive from the [`thiserror`](https://docs.rs/thiserorr) crate
* a macro to build HTML inspired by the [`horrorshow`](https://docs.rs/horrorshow) crate
* an optional (feature-gated) wrapper type to generate responses containing images from the [`image`](https://docs.rs/image) crate
* a wrapper around `Origin` (relative URLs) that can be parsed from a form field
* a `Suffix` type that can be used to parse a URL part into a filename and extension
