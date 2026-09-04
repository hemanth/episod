use reqwest::Client;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

use super::ConsistentHashRouter;

pub fn spawn_health_checker(router: ConsistentHashRouter, interval: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_default();

        loop {
            tokio::time::sleep(interval).await;

            let replicas = router.list_replicas();
            for replica in replicas {
                let health_url = if replica.url.ends_with("/v1") {
                    format!("{}/models", replica.url)
                } else if replica.url.ends_with('/') {
                    format!("{}models", replica.url)
                } else {
                    format!("{}/v1/models", replica.url)
                };

                match client.get(&health_url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            router.record_success(&replica.id);
                            debug!(replica_id = %replica.id, "Health check passed");
                        } else if resp.status().is_server_error() {
                            let tripped = router.record_failure(&replica.id);
                            warn!(
                                replica_id = %replica.id,
                                status = %resp.status(),
                                tripped = tripped,
                                "Health check returned server error"
                            );
                        }
                    }
                    Err(e) => {
                        let tripped = router.record_failure(&replica.id);
                        warn!(
                            replica_id = %replica.id,
                            error = %e,
                            tripped = tripped,
                            "Health check network failure"
                        );
                    }
                }
            }
        }
    })
}
