use crate::collector::{Collector, LocalCollector};
use crate::error::Error;
use std::time::SystemTime;

pub struct Sender {
    url: String,
    client: reqwest::Client,
    verbose: bool,
}

impl Sender {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::new(),
            verbose: false,
        }
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    pub async fn send(&self) -> Result<(), Error> {
        let request = self.client.get(&self.url);
        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(Error::HTTPError(response.status().as_u16()));
        }
        if self.verbose {
            for (key, value) in response.headers().iter() {
                println!("{}: {}", key, value.to_str().unwrap());
            }
            println!("\n{}\n", response.text().await?);
        } else {
            response.bytes().await?;
        }
        Ok(())
    }

    pub async fn do_loop(&self, collector: &Collector) {
        let mut local_stats = LocalCollector::new();
        let mut please_stop = false;
        while !please_stop {
            let start = SystemTime::now();
            match self.send().await {
                Ok(_) => {
                    local_stats.success(start, 0, 0);
                    please_stop = collector.success();
                }
                Err(e) => {
                    local_stats.failure();
                    please_stop = collector.failure(e);
                }
            }
        }
        collector.collect(local_stats);
    }
}
