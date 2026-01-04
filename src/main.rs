mod ant;
mod bpm;
mod channels;
mod config;
mod osc;
mod output;

use anyhow::{Context, Result};
use clap::Parser;
use std::thread;
use tokio::signal;
use tokio::sync::broadcast;
use vrchat_osc::VRChatOSC;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value = "intra-beat")]
    bpm: config::BpmMode,

    #[arg(long, default_value = "vrchat")]
    output: config::OutputMode,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::builder().init();

    log::info!("Starting vrchat_ant_hr");
    log::info!("BPM mode: {:?}", cli.bpm);
    log::info!("Output mode: {:?}", cli.output);

    let (bpm_tx, bpm_rx) = tokio::sync::watch::channel(None::<config::HeartRateData>);
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    let shutdown_rx_ant = shutdown_tx.subscribe();
    let bpm_tx_ant = bpm_tx.clone();

    thread::spawn(move || {
        if let Err(e) = ant::run_ant(bpm_tx_ant, shutdown_rx_ant) {
            log::error!("ANT+ thread failed: {}", e);
        }
    });

    let vrchat_osc = if matches!(cli.output, config::OutputMode::Vrchat) {
        log::info!("Connecting to VRChat OSC service");
        let osc = VRChatOSC::new()
            .await
            .context("Failed to connect to VRChat OSC service")?;
        log::info!("Connected to VRChat OSC service");
        Some(osc)
    } else {
        log::info!("Log-only mode enabled, skipping VRChat OSC connection");
        None
    };

    let bpm_mode = cli.bpm;
    let output_mode = cli.output;
    let mut bpm_rx = bpm_rx;
    let mut bpm_processor = bpm::BpmProcessor::new();
    tokio::spawn(async move {
        log::info!("Heart rate processing task started");
        loop {
            tokio::select! {
                _ = bpm_rx.changed() => {
                    let data = bpm_rx.borrow().clone();
                    if let Some(data) = data {
                        if let Some(selected_bpm) = bpm_processor.process(data, bpm_mode) {
                            if let Err(e) = output::send_output(output_mode, selected_bpm, bpm_mode, vrchat_osc.as_ref()).await {
                                log::error!("Failed to send output: {}", e);
                            }
                        } else {
                            log::debug!("Skipping heartbeat");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    log::info!("Heart rate processing task shutting down");
                    break;
                }
            }
        }
        log::info!("Heart rate processing task finished");
    });

    log::info!("Application started. Press Ctrl+C to exit");
    signal::ctrl_c().await?;

    log::info!("Shutting down...");
    let _ = shutdown_tx.send(());

    Ok(())
}
