# Longboard: The easy way to [surf](https://github.com/http-rs/surf)

```bash
$ longboard get https://async.rs
```

## Installation

```sh
$ cargo install longboard
```

## Usage

```sh
longboard 0.0.2
the easy way to surf

USAGE:
    longboard [OPTIONS] <method> <url>

FLAGS:
        --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
    -b, --body <body>
            provide a request body on the command line

            example:
            longboard post http://httpbin.org/post -b '{"hello": "world"}'
    -c, --client <client>
            http backend for surf. options: h1, curl, hyper

            caveat: h1 currently does not support chunked request bodies,
            so do not use that backend yet if you need to stream bodies [default: h1]
    -f, --file <file>
            provide a file system path to a file to use as the request body

            alternatively, you can use an operating system pipe to pass a file in

            three equivalent examples:

            longboard post http://httpbin.org/anything -f ./body.json
            longboard post http://httpbin.org/anything < ./body.json
            cat ./body.json | longboard post http://httpbin.org/anything
    -h, --headers <headers>...
            provide headers in the form -h KEY1=VALUE1 KEY2=VALUE2

            example:
            longboard get http://httpbin.org/headers -h Accept=application/json Authorization="Basic u:p"
    -j, --jar <jar>
            a filesystem path to a cookie jar in ndjson format

            note: this currently only persists "persistent cookies," which
            either have a max-age or expires.

            if the file does not yet exist, it will be created

            example:
            longboard get "https://httpbin.org/response-headers?Set-Cookie=USER_ID=10;+Max-Age=100" -j ~/.longboard.ndjson

ARGS:
    <method>
    <url>
```

Examples to try:
```
$ longboard get http://httpin.org/get
$ longboard get https://httpbin.org/headers -h Some-Header="header value" User-Agent=longboard
$ longboard post https://httpbin.org/post -b "this is a request body"
$ longboard post http://httpbin.org/anything -b "a=b&c=d" -h content-type=application/x-www-form-urlencoded
$ longboard put https://httpbin.org/put -f ./Cargo.toml -h content-type=application/toml
$ longboard patch https://httpbin.org/patch < ./some-file
$ cat /dev/random | head -c1000 | base64 | longboard post https://httpbin.org/anything
$ longboard get https://httpbin.org/stream | cat
```

## TODO for this to be useful
- [x] request bodies
  - [x] from filesystem
  - [x] as a cli arg
  - [x] from stdin (pipe)
- [x] request headers
- [ ] more emoji, custom formatters
- [ ] opt into / out of displaying different parts of request and response
- [ ] http status -> exit code

## Some future ideas:
- [x] sessions/stored cookie jars
- interactive repl mode for an easy way to make consecutive requests with the same client
- sse mode that pretty-prints sse messages
- render images to ascii for fun?

<!-- * [CI ![CI][ci-badge]][ci] -->
* [Releases][releases] [![crates.io version][version-badge]][lib-rs]
* [Contributing][contributing]

<!-- [ci]: https://github.com/jbr/longboard/actions?query=workflow%3ACI -->
<!-- [ci-badge]: https://github.com/jbr/longboard/workflows/CI/badge.svg -->
[releases]: https://github.com/jbr/longboard/releases
[contributing]: https://github.com/jbr/longboard/blob/master/.github/CONTRIBUTING.md
[lib-rs]: https://lib.rs/longboard
[version-badge]: https://img.shields.io/crates/v/longboard.svg?style=flat-square

## Safety
This crate uses ``#![forbid(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
