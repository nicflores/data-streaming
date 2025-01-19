use actix_web::{
    get,
    http::header,
    post,
    web::{Bytes, Query, ServiceConfig},
    HttpMessage, HttpResponse, Responder,
};
use async_compression::tokio::bufread::GzipDecoder;
use awc::Client;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use shuttle_actix_web::ShuttleActixWeb;
use std::{
    fs,
    net::{Ipv4Addr, Ipv6Addr},
};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader; // This is important for .read()

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

// struct AppState {
//     producer: Arc<FutureProducer>,
//     topic: String,
//     client: Client,
// }

#[derive(Debug, Serialize, Deserialize)]
struct DataItem {
    name: String,
    language: String,
    id: String,
    bio: String,
    version: f64,
}

struct JsonArrayStream {
    depth: usize,
    buffer: String,
    in_string: bool,
    escape_next: bool,
    started: bool,
}

impl JsonArrayStream {
    fn new() -> Self {
        Self {
            depth: 0,
            buffer: String::new(),
            in_string: false,
            escape_next: false,
            started: false,
        }
    }

    // Process a chunk of JSON data and return complete objects
    fn process_chunk(&mut self, chunk: &str) -> Vec<String> {
        let mut complete_objects = Vec::new();

        for c in chunk.chars() {
            // Skip whitespace outside of strings and objects
            if !self.started && !self.in_string && self.depth == 0 && c.is_whitespace() {
                continue;
            }

            // Handle array start
            if !self.started && c == '[' {
                self.started = true;
                continue;
            }

            match c {
                '{' if !self.in_string => {
                    self.depth += 1;
                    self.buffer.push(c);
                }
                '}' if !self.in_string => {
                    self.depth -= 1;
                    self.buffer.push(c);

                    if self.depth == 0 {
                        // Complete object
                        complete_objects.push(self.buffer.clone());
                        self.buffer.clear();
                    }
                }
                '"' if !self.escape_next => {
                    self.in_string = !self.in_string;
                    self.buffer.push(c);
                }
                '\\' if self.in_string => {
                    self.escape_next = true;
                    self.buffer.push(c);
                }
                _ => {
                    if self.escape_next {
                        self.escape_next = false;
                    }
                    if self.depth > 0 {
                        self.buffer.push(c);
                    }
                }
            }
        }

        complete_objects
    }
}

#[get("/process_data")]
async fn process_data() -> actix_web::Result<String> {
    let url2 = "http://localhost:8000/download";
    let client = Client::new();
    let mut response = client
        .get(url2)
        .send()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // Create a stream from the response body
    let body_stream = response.take_payload();
    let stream_reader = StreamReader::new(
        body_stream
            .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))),
    );

    // Create a gzip decoder stream
    let gzip_decoder = GzipDecoder::new(stream_reader);
    let mut reader = tokio::io::BufReader::new(gzip_decoder);

    let mut parser = JsonArrayStream::new();
    let mut processed_count = 0;
    let mut buffer = vec![0; 8192]; // 8KB buffer

    // Process the decompressed stream
    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

        if bytes_read == 0 {
            break;
        }

        let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);

        // Process each complete JSON object in the chunk
        for json_obj in parser.process_chunk(&chunk) {
            match serde_json::from_str::<DataItem>(&json_obj) {
                Ok(item) => {
                    println!("{:?}", item);
                    processed_count += 1;
                    // // Send to Event Hub
                    // let record = FutureRecord::to(&state.topic)
                    //     .key(&item.id)
                    //     .payload(&json_obj);

                    // match state.producer.send(record, Duration::from_secs(5)).await {
                    //     Ok(_) => {
                    //         processed_count += 1;
                    //         if processed_count % 1000 == 0 {
                    //             tracing::info!("Processed {} messages", processed_count);
                    //         }
                    //     }
                    //     Err((e, _)) => {
                    //         tracing::error!("Failed to send message: {:?}", e);
                    //         return Err(actix_web::error::ErrorInternalServerError(e));
                    //     }
                    // }
                }
                Err(e) => {
                    tracing::error!("Failed to parse JSON object: {:?}", e);
                    // Continue processing other objects
                }
            }
        }
    }

    Ok(format!(
        "Successfully processed {} messages",
        processed_count
    ))
}

#[get("/download")]
async fn download_gzip_file() -> impl Responder {
    let file_bytes = match fs::read("gizpdata.gz") {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Error reading file: {}", e));
        }
    };

    // Return the file bytes with the gzip content type and an attachment header.
    HttpResponse::Ok()
        .content_type("application/gzip")
        .insert_header(("Content-Disposition", "attachment; filename=myfile.gz"))
        .body(file_bytes)
}

// Day 5
#[post("/5/manifest")]
async fn day5(payload: Bytes) -> HttpResponse {
    match std::str::from_utf8(&payload) {
        Ok(toml_str) => match toml::from_str::<Gifts>(toml_str) {
            Ok(cfg) => HttpResponse::Ok().json(cfg),
            Err(e) => HttpResponse::NotFound().body(e.to_string()),
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
            .service(day5)
            .service(download_gzip_file)
            .service(process_data);
    };

    Ok(config.into())
}
