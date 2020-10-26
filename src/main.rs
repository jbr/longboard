#![forbid(unsafe_code, future_incompatible)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    missing_copy_implementations,
    unused_qualifications
)]

use async_std::io;
use async_std::io::BufReader;
use async_std::prelude::*;
use bat::{Input, PagingMode, PrettyPrinter};
use http_client::{h1::H1Client, hyper::HyperClient, isahc::IsahcClient};
use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use surf::http::{self, Headers, Method, Url};
use surf::Body;
use surf::{Client, Error, Request, Response, Result};
use surf_cookie_middleware::CookieMiddleware;

fn parse_header(s: &str) -> Result<(String, String)> {
    let pos = s
        .find('=')
        .ok_or_else(|| http::format_err!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((String::from(&s[..pos]), String::from(&s[pos + 1..])))
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "longboard",
    about = "the easy way to surf",
    verbatim_doc_comment
)]
struct Longboard {
    #[structopt(parse(try_from_str = parse_method_case_insensitive))]
    method: Method,
    url: Url,

    /// provide a file system path to a file to use as the request body
    ///
    /// alternatively, you can use an operating system pipe to pass a file in
    ///
    /// three equivalent examples:
    ///
    /// longboard post http://httpbin.org/anything -f ./body.json
    /// longboard post http://httpbin.org/anything < ./body.json
    /// cat ./body.json | longboard post http://httpbin.org/anything
    #[structopt(short, long, parse(from_os_str), verbatim_doc_comment)]
    file: Option<PathBuf>,

    /// provide a request body on the command line
    ///
    /// example:
    /// longboard post http://httpbin.org/post -b '{"hello": "world"}'
    #[structopt(short, long, verbatim_doc_comment)]
    body: Option<String>,

    /// provide headers in the form -h KEY1=VALUE1 KEY2=VALUE2
    ///
    /// example:
    /// longboard get http://httpbin.org/headers -h Accept=application/json Authorization="Basic u:p"
    #[structopt(short, long, parse(try_from_str = parse_header), verbatim_doc_comment)]
    headers: Vec<(String, String)>,

    /// http backend for surf. options: h1, curl, hyper
    ///
    /// caveat: h1 currently does not support chunked request bodies,
    /// so do not use that backend yet if you need to stream bodies
    #[structopt(short, long, default_value = "h1", verbatim_doc_comment)]
    client: Backend,

    /// a filesystem path to a cookie jar in ndjson format
    ///
    /// note: this currently only persists "persistent cookies," which
    /// either have a max-age or expires.
    ///
    /// if the file does not yet exist, it will be created
    ///
    /// example:
    /// longboard get "https://httpbin.org/response-headers?Set-Cookie=USER_ID=10;+Max-Age=100" -j ~/.longboard.ndjson
    #[structopt(short, long, parse(from_os_str), verbatim_doc_comment)]
    jar: Option<PathBuf>,
}

impl Longboard {
    pub async fn client(&self) -> Result<Client> {
        use Backend::*;
        let mut client = match self.client {
            H1 => Client::with_http_client(H1Client::new()),
            Curl => Client::with_http_client(IsahcClient::new()),
            Hyper => Client::with_http_client(HyperClient::new()),
        };

        if let Some(ref cookie_path) = self.jar {
            client = client.with(CookieMiddleware::from_path(cookie_path).await?);
        }

        Ok(client)
    }

    pub async fn request(&self) -> Result<Request> {
        let mut request = Request::new(self.method, self.url.clone());

        for (name, value) in &self.headers {
            request.append_header(&name[..], &value[..]);
        }

        if let Some(path) = &self.file {
            request.body_file(path).await?;
        } else if let Some(body) = &self.body {
            request.body_string(body.to_owned());
        } else if atty::isnt(atty::Stream::Stdin) {
            if self.client == Backend::H1 {
                // h1 can't stream
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).await?;
                request.body_string(buffer);
            } else {
                request.set_body(Body::from_reader(BufReader::new(io::stdin()), None));
            }
        }

        Ok(request)
    }

    pub fn url(&self) -> Url {
        self.url.clone()
    }

    pub async fn send(&self) -> Result<Response> {
        let request = self.request().await?;
        let client = self.client().await?;
        client.send(request).await
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Backend {
    H1,
    Curl,
    Hyper,
}

impl FromStr for Backend {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match &*s.to_ascii_lowercase() {
            "h1" | "async-h1" => Ok(Self::H1),
            "curl" | "isahc" => Ok(Self::Curl),
            "hyper" => Ok(Self::Hyper),
            _ => Err(http::format_err!("unrecognized backend {}", s)),
        }
    }
}

fn parse_method_case_insensitive(src: &str) -> Result<Method> {
    src.to_uppercase().parse()
}

#[async_std::main]
async fn main() -> Result<()> {
    let longboard = Longboard::from_args();
    let url = longboard.url();
    let mut response = longboard.send().await?;

    if atty::is(atty::Stream::Stdout) {
        let body = response.body_string().await?;
        let http_res: &http::Response = response.as_ref();
        let headers: &Headers = http_res.as_ref();
        let headers_as_string = format!("{:#?}", headers);
        let content_type = response.content_type().map(|c| c.to_string());
        let filename = match content_type.as_deref() {
            Some("application/json") => "body.json", // bat can't sniff json for some reason
            _ => url.path(),
        };

        let response_status = format!(
            "{}: {}",
            response.status(),
            response.status().canonical_reason()
        );

        PrettyPrinter::new()
            .paging_mode(PagingMode::QuitIfOneScreen)
            .header(true)
            .grid(true)
            .inputs(vec![
                Input::from_bytes(headers_as_string.as_bytes())
                    .name("headers.rs")
                    .title("response headers"),
                Input::from_bytes(response_status.as_bytes())
                    .name("status")
                    .title("status"),
                Input::from_bytes(body.as_bytes()).name(filename).title(
                    if let Some(content_type) = content_type {
                        Cow::Owned(format!("response body ({})", content_type))
                    } else {
                        Cow::Borrowed("response body")
                    },
                ),
            ])
            .print()
            .unwrap();
    } else {
        io::copy(response, io::stdout()).await?;
    }

    Ok(())
}
