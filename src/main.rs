#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;
mod errors {
    error_chain!{}
}
use errors::*;

use std::collections::HashMap;
use std::io::Read;

extern crate strfmt;
use strfmt::strfmt;

extern crate hyper;
use hyper::Client;
use hyper::net::HttpsConnector;

extern crate hyper_native_tls;
use hyper_native_tls::NativeTlsClient;

extern crate clap;
use clap::{App, Arg};

extern crate serde;
extern crate serde_json;

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let matches = App::new("cryptocurrency-tracker")
        .version("0.1.0")
        .author("Joe Roberts <joe@resin.io>")
        .about("Get cryptocurrency information from coinmarketcap.com")
        .arg(
            Arg::with_name("cryptocurrency")
                .short("c")
                .long("cryptocurrency")
                .help("Enter the cryptocurrency(s) you are interested in")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .help("Enter the format")
                .takes_value(true)
                .default_value(
                    "{name}, {symbol}, {rank}, {price_usd}, {price_btc}, \
                     {24h_volume_usd}, {market_cap_usd}, {available_supply}, \
                     {total_supply}, {percent_change_1h}, {percent_change_24h}, \
                     {percent_change_7d}, {last_updated}",
                ),
        )
        .arg(
            Arg::with_name("separator")
                .short("s")
                .long("separator")
                .help("Enter the separator")
                .takes_value(true)
                .default_value(" | "),
        )
        .get_matches_safe()
        .chain_err(|| "unable to get matches")?;

    let cryptocurrencys: Vec<_> = matches
        .values_of("cryptocurrency")
        .ok_or_else(|| Error::from("unable to get cryptocurrency vector"))?
        .collect();

    let format: &str = matches.value_of("format").ok_or_else(|| {
        Error::from("unable to get format string")
    })?;

    let separator: &str = matches.value_of("separator").ok_or_else(|| {
        Error::from("unable to get separator string")
    })?;

    let ssl = NativeTlsClient::new().chain_err(
        || "unable to create NativeTlsClient",
    )?;
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);
    let mut response = client
        .get("https://api.coinmarketcap.com/v1/ticker")
        .send()
        .chain_err(|| "unable to send GET request")?;

    let mut contents = String::new();
    response.read_to_string(&mut contents).chain_err(
        || "unable to read API response",
    )?;

    let pre_data: Vec<HashMap<String, Option<String>>> =
        serde_json::from_str(&contents).chain_err(
            || "unable to parse API response",
        )?;

    let mut post_data: HashMap<String, HashMap<String, String>> = HashMap::new();
    for d in pre_data {
        let mut buffer: HashMap<String, String> = HashMap::new();
        for (k, v) in d {
            buffer.insert(k, v.unwrap_or_else(|| String::from("not found")));
        }
        post_data.insert(buffer["symbol"].clone(), buffer);
    }

    let mut iter = cryptocurrencys.iter().peekable();
    while let Some(current) = iter.next() {
        let value = post_data.get::<str>(current).ok_or_else(|| {
            Error::from(format!("unable to get {} value", current))
        })?;

        print!("{}", strfmt(format, value).chain_err(|| "unable to format value")?);

        match iter.peek() {
            Some(_) => print!("{}", separator),
            None => println!(""),
        }
    }

    Ok(())
}
