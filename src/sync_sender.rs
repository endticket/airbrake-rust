use std::io::Read;

use serde_json;
use serde_json::Value;
use hyper::Url;
use hyper::header::ContentType;
//use hyper::client::{Client, Body, ProxyConfig};
use hyper::client::{Client, Body};
use hyper::net::HttpsConnector;
use hyper_sync_rustls::TlsClient;

use config::Config;
use notice::Notice;

#[derive(Debug)]
pub struct SyncSender {
    client: Client,
    endpoint: String,
}

impl SyncSender {
    pub fn new(config: &Config) -> SyncSender {
        let ssl = TlsClient::new();
        let connector = HttpsConnector::new(ssl);
        let client = if config.proxy.is_empty() {
            Client::with_connector(connector)
        } else {
            let mut proxy = config.proxy.clone();
            let mut port = 80;

            if let Some(colon) = proxy.rfind(':') {
                port = proxy[colon + 1..].parse().unwrap_or_else(|e| {
                    panic!("proxy is malformed: {:?}, port parse error: {}",
                           proxy, e);
                });
                proxy.truncate(colon);
            }

            Client::with_http_proxy(proxy, port)
            // why is this failing with hyper-sync-rustls?
            // let ssl2 = TlsClient::new();

            //Client::with_proxy_config(
            //    ProxyConfig::new("http", proxy, port, connector, ssl2)
            //)
        };

        SyncSender {
            client: client,
            endpoint: config.endpoint(),
        }
    }

    pub fn send(&self, notice: Notice) -> Value {
        let uri = Url::parse(&self.endpoint).ok().expect("malformed URL");

        let payload = notice.to_json();
        let bytes = payload.as_bytes();

        debug!("**Airbrake: sending {}", payload);

        let response = self.client.post(uri)
            .header(ContentType::json())
            .body(Body::BufBody(bytes, bytes.len()))
            .send();

        let mut buffer = String::new();
        response.unwrap().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    }
}
