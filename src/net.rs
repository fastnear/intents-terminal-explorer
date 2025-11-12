//! Small helpers for rate-limit friendly networking on native targets.
//! WASM builds should keep using fetch in JS (auth + CORS handled by browser).

#[cfg(not(target_arch = "wasm32"))]
use rand::{thread_rng, Rng};

#[cfg(not(target_arch = "wasm32"))]
pub async fn send_with_backoff(
    rb: reqwest::RequestBuilder,
    label: &str,
    max_retries: u8,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut attempt = 0u8;
    loop {
        let res = rb.try_clone().expect("cloneable request").send().await;
        match res {
            Ok(r) => {
                if r.status().as_u16() == 429 && attempt < max_retries {
                    attempt += 1;
                    let back_ms = backoff_delay_ms(attempt);
                    eprintln!(
                        "[nearx][net] 429 {} retry={} backoff={}ms",
                        label, attempt, back_ms
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(back_ms)).await;
                    continue;
                }
                return Ok(r);
            }
            Err(e) => {
                if attempt < max_retries {
                    attempt += 1;
                    let back_ms = backoff_delay_ms(attempt);
                    eprintln!(
                        "[nearx][net] err {} retry={} backoff={}ms : {}",
                        label, attempt, back_ms, e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(back_ms)).await;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn backoff_delay_ms(attempt: u8) -> u64 {
    let base = 300u64.saturating_mul(1u64 << (attempt.min(5) - 1)); // 300,600,1200,2400,4800,9600
    let jitter: u64 = thread_rng().gen_range(0..=250);
    base + jitter
}

#[cfg(target_arch = "wasm32")]
pub async fn send_with_backoff<T>(_rb: T, _label: &str, _max_retries: u8) -> Result<(), ()> {
    Err(()) // stub: use JS fetch with retry logic on the web side
}
