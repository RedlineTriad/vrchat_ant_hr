use crate::channels::{RxReceiver, TxSender};
use crate::config::HeartRateData;
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

static BPM_SENDER: OnceLock<watch::Sender<Option<HeartRateData>>> = OnceLock::new();

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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, AtomicU16, Ordering};

    static PREV_BEAT_COUNT: OnceLock<Arc<AtomicU8>> = OnceLock::new();
    static PREV_EVENT_TIME: OnceLock<Arc<AtomicU16>> = OnceLock::new();

    let _ = PREV_BEAT_COUNT.get_or_init(|| Arc::new(AtomicU8::new(0)));
    let _ = PREV_EVENT_TIME.get_or_init(|| Arc::new(AtomicU16::new(0)));

    match data {
        Ok(MonitorTxDataPage::PreviousHeartBeat(data)) => {
            let bpm = data.common.computed_heart_rate;
            if bpm > 0 {
                let beat_count = data.common.heart_beat_count;
                let event_time = data.common.heart_beat_event_time;

                let prev_beat_count = PREV_BEAT_COUNT.get().unwrap().load(Ordering::SeqCst);
                let prev_event_time = PREV_EVENT_TIME.get().unwrap().load(Ordering::SeqCst);

                let (intra_beat_time, skipped) = if prev_beat_count > 0 {
                    let count_diff = beat_count.wrapping_sub(prev_beat_count) as u16;
                    let time_diff = if event_time >= prev_event_time {
                        event_time - prev_event_time
                    } else {
                        event_time.wrapping_add(1024u16.wrapping_sub(prev_event_time))
                    };

                    let skipped = count_diff > 1;
                    let intra_beat_time = if count_diff > 0 {
                        Some(time_diff / count_diff)
                    } else {
                        None
                    };

                    (intra_beat_time, skipped)
                } else {
                    (None, false)
                };

                let new_beat = prev_beat_count == 0 || beat_count != prev_beat_count;

                PREV_BEAT_COUNT
                    .get()
                    .unwrap()
                    .store(beat_count, Ordering::SeqCst);
                PREV_EVENT_TIME
                    .get()
                    .unwrap()
                    .store(event_time, Ordering::SeqCst);

                log::debug!(
                    "Received heart rate: {} BPM, beat count: {}, event time: {}, intra-beat: {:?}, skipped: {}",
                    bpm,
                    beat_count,
                    event_time,
                    intra_beat_time,
                    skipped
                );

                if skipped {
                    log::warn!(
                        "Skipped heart beat(s) detected (beat count increased by more than 1)"
                    );
                }

                if new_beat {
                    if let Some(sender) = BPM_SENDER.get()
                        && let Err(e) = sender.send(Some(HeartRateData {
                            bpm,
                            intra_beat_time,
                        }))
                    {
                        log::warn!("Failed to send heart rate data to watch channel: {}", e);
                    }
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
    bpm_sender: watch::Sender<Option<HeartRateData>>,
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
