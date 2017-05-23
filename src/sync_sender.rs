use std::io::Read;
use std::{thread, time};

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
use error::Result;

#[derive(Debug)]
pub struct SyncSender {
    client: Client,
    endpoint: String,
    max_retry: Option<u32>,
    retry_timeout: Option<time::Duration>,
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
            max_retry: config.max_retry,
            retry_timeout: config.retry_timeout,
        }
    }

    fn send_once(&self, notice: &Notice) -> Result<Value> {
        let uri = try!(Url::parse(&self.endpoint));

        let payload = notice.to_json();
        let bytes = payload.as_bytes();

        debug!("**Airbrake: sending {}", payload);

        let mut response = try!(self.client.post(uri)
            .header(ContentType::json())
            .body(Body::BufBody(bytes, bytes.len()))
            .send());

        let mut buffer = String::new();
        try!(response.read_to_string(&mut buffer));
        let res = try!(serde_json::from_str(&buffer));
        Ok(res)
    }

    pub fn send(&self, notice: &Notice) -> Result<Value> {
        let mut last_res = self.send_once(notice);
        let max_retry = self.max_retry.unwrap_or(0);
        
        if max_retry != 0 {
            let mut retry_num = 0;
            let timeout = self.retry_timeout.unwrap_or(time::Duration::from_secs(1));
            
            while last_res.is_err() && retry_num < max_retry {
                error!("Airbrake Notify failed {} time(s), retrying after {:?}", retry_num+1, timeout);
                thread::sleep(timeout);
                retry_num += 1;
                last_res =  self.send_once(notice);
            }
        }
        
        last_res
    }
}
