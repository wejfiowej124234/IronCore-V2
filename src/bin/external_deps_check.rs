use anyhow::{Context, Result};
use reqwest::StatusCode;
use std::time::Duration;

#[derive(Debug)]
struct CheckResult {
    name: String,
    ok: bool,
    status: Option<StatusCode>,
    detail: String,
}

fn base_url_from_args() -> String {
    let mut base_url = "https://oxidevault-ironcore-v2.fly.dev".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--base-url" {
            if let Some(v) = args.next() {
                base_url = v;
            }
        }
    }

    base_url.trim_end_matches('/').to_string()
}

async fn check_get_json(client: &reqwest::Client, name: &str, url: String) -> CheckResult {
    match client.get(url.clone()).send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let ok = status.is_success();

            let detail = if ok {
                "ok".to_string()
            } else {
                // Try to extract {message} if backend uses an envelope
                let extracted = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| {
                        v.get("message")
                            .and_then(|m| m.as_str())
                            .map(|s| s.to_string())
                    });

                if let Some(m) = extracted {
                    m
                } else {
                    let trimmed = body.trim();
                    if trimmed.is_empty() {
                        format!("http {}", status.as_u16())
                    } else {
                        trimmed
                            .chars()
                            .take(240)
                            .collect::<String>()
                            .replace('\n', " ")
                    }
                }
            };

            CheckResult {
                name: name.to_string(),
                ok,
                status: Some(status),
                detail,
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            ok: false,
            status: None,
            detail: format!("request failed: {e}"),
        },
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let base_url = base_url_from_args();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("build reqwest client")?;

    let chains = [
        "ethereum",
        "bsc",
        "polygon",
        "arbitrum",
        "optimism",
        "avalanche",
    ];

    let mut results: Vec<CheckResult> = Vec::new();

    results.push(
        check_get_json(
            &client,
            "backend health",
            format!("{}/api/health", base_url),
        )
        .await,
    );

    for chain in chains {
        results.push(
            check_get_json(
                &client,
                &format!("gas estimate-all ({chain})"),
                format!("{}/api/v1/gas/estimate-all?chain={}", base_url, chain),
            )
            .await,
        );
    }

    results.push(
        check_get_json(
            &client,
            "tokens list (ethereum)",
            format!("{}/api/v1/tokens/list?chain=ethereum", base_url),
        )
        .await,
    );

    results.push(
        check_get_json(
            &client,
            "prices (ETH,BTC,SOL)",
            format!("{}/api/v1/prices?symbols=ETH,BTC,SOL", base_url),
        )
        .await,
    );

    results.push(
        check_get_json(
            &client,
            "prices alias (assets/prices)",
            format!("{}/api/v1/assets/prices?symbols=ETH,BTC,SOL", base_url),
        )
        .await,
    );

    println!("External dependency smoke-check against: {}", base_url);
    let mut failures = 0usize;
    for r in &results {
        let status = r
            .status
            .map(|s| s.as_u16().to_string())
            .unwrap_or_else(|| "-".to_string());
        if r.ok {
            println!("[OK]   {:28} status={} detail={}", r.name, status, r.detail);
        } else {
            failures += 1;
            println!("[FAIL] {:28} status={} detail={}", r.name, status, r.detail);
        }
    }

    if failures > 0 {
        anyhow::bail!("{} checks failed", failures);
    }

    Ok(())
}
