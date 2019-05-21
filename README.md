# detone

[![crates.io](https://meritbadge.herokuapp.com/detone)](https://crates.io/crates/detone)
[![docs.rs](https://docs.rs/detone/badge.svg)](https://docs.rs/detone/)
[![Apache 2 / MIT dual-licensed](https://img.shields.io/badge/license-Apache%202%20%2F%20MIT-blue.svg)](https://github.com/hsivonen/detone/blob/master/COPYRIGHT)

An iterator adapter that takes an iterator over `char` yielding a sequence of
`char`s in Normalization Form C (this precondition is not checked!) and
yields `char`s either such that tone marks that wouldn't otherwise fit into
windows-1258 are decomposed or such that text is decomposed into orthographic
units.

Use cases include preprocessing before encoding Vietnamese text into
windows-1258 or converting precomposed Vietnamese text into a form that looks
like it was written with the (non-IME) Vietnamese keyboard layout (e.g. for
machine learning training or benchmarking purposes).

## Licensing

Please see the file named
[COPYRIGHT](https://github.com/hsivonen/detone/blob/master/COPYRIGHT).

## Documentation

Generated [API documentation](https://docs.rs/detone/) is available
online.

## Release Notes

### 1.0.0

* Initial release.
