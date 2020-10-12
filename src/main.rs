#![forbid(unsafe_code, future_incompatible)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    missing_copy_implementations,
    unused_qualifications
)]

use bat::{Input, PagingMode, PrettyPrinter};
use http_client::{h1::H1Client, hyper::HyperClient, isahc::IsahcClient};
use std::borrow::Cow;
use std::str::FromStr;
use structopt::StructOpt;
use surf::http::{self, Headers, Method, Url};
use surf::{Client, Error, Request, Response, Result};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "longboard",
    about = "the easy way to surf",
    setting = structopt::clap::AppSettings::DeriveDisplayOrder
)]
struct Longboard {
    #[structopt(parse(try_from_str = parse_method_case_insensitive))]
    method: Method,
    url: Url,

    /// http backend for surf. options: h1, curl, hyper
    #[structopt(short, long, default_value = "h1")]
    backend: Backend,
}

impl Longboard {
    pub fn client(&self) -> Client {
        match self.backend {
            Backend::H1 => Client::with_http_client(H1Client::new()),
            Backend::Curl => Client::with_http_client(IsahcClient::new()),
            Backend::Hyper => Client::with_http_client(HyperClient::new()),
        }
    }

    pub fn request(&self) -> Request {
        Request::new(self.method.clone(), self.url.clone())
    }

    pub fn url(&self) -> Url {
        self.url.clone()
    }

    pub async fn send(&self) -> Result<Response> {
        self.client().send(self.request()).await
    }
}

#[derive(Debug)]
enum Backend {
    H1,
    Curl,
    Hyper,
}

impl FromStr for Backend {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
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

    Ok(())
}
