mod ant;
mod channels;
mod osc;

use ant::run_ant;
use anyhow::{Context, Result};
use std::thread;
use tokio::signal;
use tokio::sync::broadcast;
use vrchat_osc::VRChatOSC;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .init();

    log::info!("Starting vrchat_ant_hr");

    let (bpm_tx, bpm_rx) = tokio::sync::watch::channel(None::<u8>);
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    let shutdown_rx_ant = shutdown_tx.subscribe();
    let bpm_tx_ant = bpm_tx.clone();

    thread::spawn(move || {
        if let Err(e) = run_ant(bpm_tx_ant, shutdown_rx_ant) {
            log::error!("ANT+ thread failed: {}", e);
        }
    });

    log::info!("Connecting to VRChat OSC service");
    let vrchat_osc = VRChatOSC::new()
        .await
        .context("Failed to connect to VRChat OSC service")?;
    log::info!("Connected to VRChat OSC service");

    let mut bpm_rx = bpm_rx;
    tokio::spawn(async move {
        log::info!("OSC sender task started");
        loop {
            tokio::select! {
                _ = bpm_rx.changed() => {
                    let bpm = *bpm_rx.borrow();
                    if let Some(bpm) = bpm {
                        log::debug!("Sending heart rate to VRChat: {} BPM", bpm);
                        let normalized_bpm = ant::normalize_bpm(bpm);
                        if let Err(e) = osc::send_osc_heartbeat(&vrchat_osc, normalized_bpm).await {
                            log::error!("Failed to send OSC message: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    log::info!("OSC sender task shutting down");
                    break;
                }
            }
        }
        log::info!("OSC sender task finished");
    });

    log::info!("Application started. Press Ctrl+C to exit");
    signal::ctrl_c().await?;

    log::info!("Shutting down...");
    let _ = shutdown_tx.send(());

    Ok(())
}
