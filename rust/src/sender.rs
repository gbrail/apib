use crate::collector::{Collector, LocalCollector};
use crate::error::Error;
use std::time::{Duration, SystemTime};
use url::Url;

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Sender {
    url: Url,
    verbose: bool,
}

impl Sender {
    pub fn new(url: Url) -> Result<Self, Error> {
        Ok(Self {
            url,
            verbose: false,
        })
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    pub async fn send(&mut self) -> Result<(), Error> {
        // Check if connection is saved, if not, open a new one
        // Send using hyper library directly
        //   Be sure to set Host and User-Agent headers
        // Read result
        // Close connection if it's important that we do that
        /*let request = self.client.get(&self.url);
        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(Error::HTTPError(response.status().as_u16()));
        }
        if self.verbose {
            for (key, value) in response.headers().iter() {
                println!("{}: {}", key, value.to_str().unwrap());
            }
            println!("\n{}", response.text().await?);
        } else {
            response.bytes().await?;
        }
        Ok(())
        */
        todo!()
    }

    pub async fn do_loop(&mut self, collector: &Collector) {
        let mut local_stats = LocalCollector::new();
        loop {
            let start = SystemTime::now();
            match self.send().await {
                Ok(_) => {
                    local_stats.success(start, 0, 0);
                    if collector.success() {
                        break;
                    }
                }
                Err(e) => {
                    if self.verbose {
                        println!("Error: {}", e);
                    }
                    local_stats.failure();
                    if collector.failure(e) {
                        break;
                    }
                }
            }
        }
        collector.collect(local_stats);
    }
}
