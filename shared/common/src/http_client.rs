use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn get<T>(&self, url: &str) -> Result<T, reqwest::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.client
            .get(url)
            .send()
            .await?
            .json::<T>()
            .await
    }

    pub async fn post<T, U>(&self, url: &str, body: &T) -> Result<U, reqwest::Error>
    where
        T: Serialize,
        U: for<'de> Deserialize<'de>,
    {
        self.client
            .post(url)
            .json(body)
            .send()
            .await?
            .json::<U>()
            .await
    }

    pub async fn put<T, U>(&self, url: &str, body: &T) -> Result<U, reqwest::Error>
    where
        T: Serialize,
        U: for<'de> Deserialize<'de>,
    {
        self.client
            .put(url)
            .json(body)
            .send()
            .await?
            .json::<U>()
            .await
    }

    pub async fn delete(&self, url: &str) -> Result<(), reqwest::Error> {
        self.client
            .delete(url)
            .send()
            .await?;
        Ok(())
    }
}
