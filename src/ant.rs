use crate::channels::{RxReceiver, TxSender};
use ant::drivers::{UsbDriver, is_ant_usb_device_from_device};
use ant::messages::config::SetNetworkKey;
use ant::plus::profiles::heart_rate::{
    Display, DisplayConfig, Error as HrError, MonitorTxDataPage, Period,
};
use ant::router::Router;
use anyhow::{Context, Result};
use dialoguer::Select;
use rusb::{Device, DeviceList};
use std::sync::OnceLock;
use std::thread;
use thingbuf::mpsc::channel;
use tokio::sync::broadcast;
use tokio::sync::watch;

static BPM_SENDER: OnceLock<watch::Sender<Option<u8>>> = OnceLock::new();

/// Normalizes BPM value to OSC-compatible range (-1.0 to 1.0)
///
/// Maps 0-255 BPM to -1.0 to 1.0 using linear interpolation.
pub fn normalize_bpm(bpm: u8) -> f32 {
    (bpm as f32 / 255.0) * 2.0 - 1.0
}

/// Selects an ANT+ USB device from available devices
fn select_ant_device() -> Result<Device<rusb::GlobalContext>> {
    let mut devices: Vec<Device<_>> = DeviceList::new()
        .context("Failed to enumerate USB devices")?
        .iter()
        .filter(is_ant_usb_device_from_device)
        .collect();

    if devices.is_empty() {
        anyhow::bail!("No ANT+ USB devices found");
    }

    log::info!("Found {} ANT+ device(s)", devices.len());

    let device = if devices.len() == 1 {
        devices.remove(0)
    } else {
        let items: Vec<String> = devices
            .iter()
            .map(|x| x.device_descriptor().unwrap())
            .map(|x| format!("USB {:04x}:{:04x}", x.vendor_id(), x.product_id()))
            .collect();

        let selection = Select::new()
            .with_prompt("Multiple devices found, please select a radio to use")
            .items(&items)
            .interact()
            .context("Failed to get device selection")?;

        devices.remove(selection)
    };

    Ok(device)
}

fn handle_rx(data: Result<MonitorTxDataPage, HrError>) {
    match data {
        Ok(MonitorTxDataPage::PreviousHeartBeat(data)) => {
            let bpm = data.common.computed_heart_rate;
            if bpm > 0 {
                log::debug!("Received heart rate: {} BPM", bpm);
                if let Some(sender) = BPM_SENDER.get()
                    && let Err(e) = sender.send(Some(bpm))
                {
                    log::warn!("Failed to send BPM to watch channel: {}", e);
                }
            }
        }
        Ok(page) => {
            log::trace!("Received other data page: {:?}", page);
        }
        Err(e) => {
            log::error!("ANT+ Rx error: {:?}", e);
        }
    }
}

pub fn run_ant(
    bpm_sender: watch::Sender<Option<u8>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("ANT+ thread started");

    BPM_SENDER
        .set(bpm_sender)
        .expect("BPM_SENDER already initialized");

    let device = select_ant_device()?;

    let driver = UsbDriver::new(device)
        .map_err(|e| anyhow::anyhow!("Failed to create USB driver: {:?}", e))?;

    let (channel_tx, router_rx) = channel(8);
    let (router_tx, channel_rx) = channel(8);

    let mut router = Router::new(
        driver,
        RxReceiver {
            receiver: router_rx,
        },
    )
    .map_err(|e| anyhow::anyhow!("Failed to create ANT+ router: {:?}", e))?;

    let snk = SetNetworkKey::new(0, [0xB9, 0xA5, 0x21, 0xFB, 0xBD, 0x72, 0xC3, 0x45]);
    router
        .send(&snk)
        .map_err(|e| anyhow::anyhow!("Failed to set network key: {:?}", e))?;

    let chan = router
        .add_channel(TxSender { sender: router_tx })
        .map_err(|e| anyhow::anyhow!("Failed to add channel: {:?}", e))?;

    let config = DisplayConfig {
        device_number: 0,
        device_number_extension: 0.into(),
        channel: chan,
        period: Period::FourHz,
        ant_plus_key_index: 0,
    };

    let mut hr = Display::new(
        config,
        TxSender { sender: channel_tx },
        RxReceiver {
            receiver: channel_rx,
        },
    );

    hr.set_rx_datapage_callback(Some(handle_rx));

    log::info!("Opening heart rate monitor channel");
    hr.open();
    log::info!("ANT+ setup complete, listening for heart rate data");

    loop {
        if shutdown_rx.try_recv().is_ok() {
            log::info!("Shutting down ANT+ session");
            break;
        }

        router
            .process()
            .map_err(|e| anyhow::anyhow!("ANT+ router process error: {:?}", e))?;

        hr.process()
            .map_err(|e| anyhow::anyhow!("ANT+ heart rate process error: {:?}", e))?;

        thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
}
