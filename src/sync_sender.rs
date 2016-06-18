use hyper::Url;
use hyper::header::ContentType;
use hyper::client::{Client, Body};

use config::Config;
use notice::Notice;

#[derive(Debug)]
pub struct SyncSender {
    client: Client,
    endpoint: String,
}

impl SyncSender {
    pub fn new(config: &Config) -> SyncSender {
        SyncSender {
            client: Client::new(),
            endpoint: config.endpoint(),
        }
    }

    pub fn send(&self, notice: Notice) {
        let uri = Url::parse(&self.endpoint).ok().expect("malformed URL");

        let payload = notice.to_json();
        let bytes = payload.as_bytes();

        debug!("**Airbrake: sending {}", payload);

        let response = self.client.post(uri)
            .header(ContentType::json())
            .body(Body::BufBody(bytes, bytes.len()))
            .send();

        debug!("**Airbrake: received response {:?}", response);
    }
}