//! URL fetching utilities.

use crate::FetchError;
use reqwest::blocking::Client;
use std::time::Duration;

/// Default timeout for HTTP requests.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// Default user agent.
const USER_AGENT: &str = "neve-fetch/0.1";

/// Fetch content from a URL.
pub fn fetch_url(url: &str) -> Result<Vec<u8>, FetchError> {
    let client = Client::builder()
        .timeout(DEFAULT_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?;
    
    let response = client.get(url).send()?;
    
    if !response.status().is_success() {
        return Err(FetchError::Http(
            response.error_for_status().unwrap_err()
        ));
    }
    
    Ok(response.bytes()?.to_vec())
}

/// Fetch content from a URL with progress callback.
pub fn fetch_url_with_progress<F>(url: &str, mut on_progress: F) -> Result<Vec<u8>, FetchError>
where
    F: FnMut(u64, Option<u64>),
{
    let client = Client::builder()
        .timeout(DEFAULT_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?;
    
    let response = client.get(url).send()?;
    
    if !response.status().is_success() {
        return Err(FetchError::Http(
            response.error_for_status().unwrap_err()
        ));
    }
    
    let total_size = response.content_length();
    let mut downloaded: u64 = 0;
    let mut content = Vec::new();
    
    // Read in chunks
    let bytes = response.bytes()?;
    downloaded += bytes.len() as u64;
    content.extend_from_slice(&bytes);
    on_progress(downloaded, total_size);
    
    Ok(content)
}

/// Check if a URL is reachable.
pub fn check_url(url: &str) -> Result<bool, FetchError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(USER_AGENT)
        .build()?;
    
    let response = client.head(url).send()?;
    Ok(response.status().is_success())
}

/// Get the content length of a URL without downloading.
pub fn get_content_length(url: &str) -> Result<Option<u64>, FetchError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(USER_AGENT)
        .build()?;
    
    let response = client.head(url).send()?;
    
    if !response.status().is_success() {
        return Err(FetchError::Http(
            response.error_for_status().unwrap_err()
        ));
    }
    
    Ok(response.content_length())
}

