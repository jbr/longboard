#![forbid(unsafe_code, future_incompatible)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    missing_copy_implementations,
    unused_qualifications
)]

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

fn parse_header(s: &str) -> Result<(String, String)> {
    let pos = s
        .find('=')
        .ok_or_else(|| http::format_err!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((String::from(&s[..pos]), String::from(&s[pos + 1..])))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "longboard", about = "the easy way to surf")]
struct Longboard {
    #[structopt(parse(try_from_str = parse_method_case_insensitive))]
    method: Method,
    url: Url,

    /// provide a the path to a file to use as the request body
    #[structopt(short, long, parse(from_os_str))]
    file: Option<PathBuf>,

    /// provide a request body on the command line
    #[structopt(short, long)]
    body: Option<String>,

    /// provide headers in the form -h KEY1=VALUE1 KEY2=VALUE2
    #[structopt(short, long, parse(try_from_str = parse_header))]
    headers: Vec<(String, String)>,

    /// http backend for surf. options: h1, curl, hyper
    #[structopt(short, long, default_value = "h1")]
    client: Backend,
}

impl Longboard {
    pub fn client(&self) -> Client {
        use Backend::*;
        match self.client {
            H1 => Client::with_http_client(H1Client::new()),
            Curl => Client::with_http_client(IsahcClient::new()),
            Hyper => Client::with_http_client(HyperClient::new()),
        }
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
                async_std::io::stdin().read_to_string(&mut buffer).await?;
                request.body_string(buffer);
            } else {
                request.set_body(Body::from_reader(
                    BufReader::new(async_std::io::stdin()),
                    None,
                ));
            }
        }

        Ok(request)
    }

    pub fn url(&self) -> Url {
        self.url.clone()
    }

    pub async fn send(&self) -> Result<Response> {
        let request = self.request().await?;
        self.client().send(request).await
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
        async_std::io::copy(response, async_std::io::stdout()).await?;
    }

    Ok(())
}
