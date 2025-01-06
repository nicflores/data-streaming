use actix_web::{
    get,
    http::header,
    post,
    web::{Bytes, Query, ServiceConfig},
    HttpResponse,
};
use azure_app_config::azure_app_config::Client;
use azure_identity::{
    AppServiceManagedIdentityCredential, DefaultAzureCredentialBuilder, TokenCredentialOptions,
};
use serde::{Deserialize, Serialize};
use shuttle_actix_web::ShuttleActixWeb;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

#[derive(serde::Deserialize)]
struct FromIPv4 {
    from: Ipv4Addr,
}

#[derive(serde::Deserialize)]
struct KeyIPv4 {
    key: Ipv4Addr,
}

#[derive(serde::Deserialize)]
struct ToIPv4 {
    to: Ipv4Addr,
}

// ipv6
#[derive(serde::Deserialize)]
struct FromIPv6 {
    from: Ipv6Addr,
}

#[derive(serde::Deserialize)]
struct ToIPv6 {
    to: Ipv6Addr,
}

#[derive(serde::Deserialize)]
struct KeyIPv6 {
    key: Ipv6Addr,
}

#[derive(Deserialize, Serialize, Debug)]
struct Gifts {
    package: Package,
}

#[derive(Deserialize, Serialize, Debug)]
struct Package {
    name: String,
    authors: Vec<String>,
    keywords: Vec<String>,
    metadata: Option<Metadata>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Metadata {
    orders: Vec<Order>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Order {
    item: String,
    quantity: u32,
}

// Day 5
#[post("/5/manifest")]
async fn day5(payload: Bytes) -> HttpResponse {
    match std::str::from_utf8(&payload) {
        Ok(toml_str) => match toml::from_str::<Gifts>(toml_str) {
            Ok(cfg) => HttpResponse::Ok().json(cfg),
            Err(e) => HttpResponse::NotFound().body(()),
        },
        Err(e) => HttpResponse::BadRequest().body(format!("Invalid UTF-8: {}", e)),
    }
}

// Day 2
#[get("/2/v6/key")]
async fn keyv6(f: Query<FromIPv6>, k: Query<ToIPv6>) -> HttpResponse {
    let from_ip = f.from.octets();
    let key_ip = k.to.octets();
    let result_vec: Vec<u8> = from_ip
        .iter()
        .zip(key_ip.iter())
        .map(|(x, y)| x ^ y)
        .collect();

    let result: [u8; 16] = result_vec.try_into().expect("slice with incorrect length");

    HttpResponse::Ok().body(Ipv6Addr::from(result).to_string())
}

#[get("/2/v6/dest")]
async fn destv6(f: Query<FromIPv6>, k: Query<KeyIPv6>) -> HttpResponse {
    let from_ip = f.from.octets();
    let key_ip = k.key.octets();
    let result_vec: Vec<u8> = from_ip
        .iter()
        .zip(key_ip.iter())
        .map(|(x, y)| x ^ y)
        .collect();

    let result: [u8; 16] = result_vec.try_into().expect("slice with incorrect length");

    HttpResponse::Ok().body(Ipv6Addr::from(result).to_string())
}

#[get("/2/key")]
async fn key(f: Query<FromIPv4>, t: Query<ToIPv4>) -> HttpResponse {
    let from_ip = f.from.octets();
    let to_ip = t.to.octets();
    let result: Vec<u8> = from_ip
        .iter()
        .zip(to_ip.iter())
        .map(|(a, b)| b.wrapping_sub(*a))
        .collect();

    let result: [u8; 4] = result.try_into().expect("slice with incorrect length");
    HttpResponse::Ok().body(Ipv4Addr::from(result).to_string())
}

#[get("/2/dest")]
async fn dest(f: Query<FromIPv4>, k: Query<KeyIPv4>) -> HttpResponse {
    let from_ip = f.from.octets();
    let key_ip = k.key.octets();
    let result: Vec<u8> = from_ip
        .iter()
        .zip(key_ip.iter())
        .map(|(a, b)| a.wrapping_add(*b))
        .collect();

    let result: [u8; 4] = result.try_into().expect("slice with incorrect length");
    HttpResponse::Ok().body(Ipv4Addr::from(result).to_string())
}

// Day 1 bonus
#[get("/-1/seek")]
async fn seek() -> HttpResponse {
    let redirect_url: &str = "https://www.youtube.com/watch?v=9Gc4QTqslN4";
    HttpResponse::Found()
        .append_header((header::LOCATION, redirect_url))
        .body("")
}

// Day 1
#[get("/")]
async fn hello_world() -> &'static str {
    "Hello bird!"
}

#[shuttle_runtime::main]
async fn main() -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    let config = move |cfg: &mut ServiceConfig| {
        cfg.service(hello_world)
            .service(seek)
            .service(dest)
            .service(key)
            .service(destv6)
            .service(keyv6)
            .service(day5);
    };

    Ok(config.into())
}
