use ant::channel::{RxError, TxError, TxHandler, RxHandler};
use ant::drivers::{is_ant_usb_device_from_device, UsbDriver};
use ant::messages::config::SetNetworkKey;
use ant::plus::profiles::heart_rate::{Display, DisplayConfig, MonitorTxDataPage, Period, Error as HrError};
use ant::router::Router;
use dialoguer::Select;
use once_cell::sync::OnceCell;
use rusb::{Device, DeviceList};
use std::thread;
use thingbuf::mpsc::errors::{TryRecvError, TrySendError};
use thingbuf::mpsc::{channel, Receiver, Sender};
use tokio::sync::mpsc as tokio_mpsc;
use vrchat_osc::{
    rosc::{OscMessage, OscPacket, OscType},
    VRChatOSC,
};

static BPM_SENDER: OnceCell<tokio_mpsc::Sender<u8>> = OnceCell::new();

// Boilerplate for ant-rs channels
struct TxSender<T> {
    sender: Sender<T>,
}

struct RxReceiver<T> {
    receiver: Receiver<T>,
}

impl<T: Default + Clone> TxHandler<T> for TxSender<T> {
    fn try_send(&self, msg: T) -> Result<(), TxError> {
        match self.sender.try_send(msg) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full(_)) => Err(TxError::Full),
            Err(TrySendError::Closed(_)) => Err(TxError::Closed),
            Err(_) => Err(TxError::UnknownError),
        }
    }
}

impl<T: Default + Clone> RxHandler<T> for RxReceiver<T> {
    fn try_recv(&self) -> Result<T, RxError> {
        match self.receiver.try_recv() {
            Ok(e) => Ok(e),
            Err(TryRecvError::Empty) => Err(RxError::Empty),
            Err(TryRecvError::Closed) => Err(RxError::Closed),
            Err(_) => Err(RxError::UnknownError),
        }
    }
}

fn handle_rx(data: Result<MonitorTxDataPage, HrError>) {
    match data {
        Ok(MonitorTxDataPage::PreviousHeartBeat(data)) => {
            let bpm = data.common.computed_heart_rate;
            if bpm > 0 {
                log::info!("Received Heart Rate: {} BPM", bpm);
                if let Some(tx) = BPM_SENDER.get() {
                    if let Err(e) = tx.try_send(bpm) {
                        log::warn!("Failed to send BPM to async task: {}", e);
                    }
                }
            }
        },
        Ok(page) => {
            log::debug!("Received other data page: {:?}", page);
        }
        Err(e) => {
            log::error!("ANT+ Rx Error: {:?}", e);
        }
    }
}

/// Synchronous function to handle ANT+ device communication.
/// This will run in a separate thread.
fn ant_main() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("ANT+ thread started. Searching for ANT+ USB devices...");
    let mut devices: Vec<Device<_>> = DeviceList::new().map_err(|e| format!("{:?}", e))?
        .iter()
        .filter(|x| is_ant_usb_device_from_device(x))
        .collect();

    if devices.is_empty() {
        log::error!("No ANT+ USB devices found.");
        return Err("No devices found".into());
    }
    log::info!("Found {} devices.", devices.len());


    let device = if devices.len() == 1 {
        devices.remove(0)
    } else {
        let items: Vec<String> = devices
            .iter()
            .map(|x| x.device_descriptor().unwrap())
            .map(|x| format!("USB {:04x}:{:04x}", x.vendor_id(), x.product_id()))
            .collect();
        let selection = Select::new()
            .with_prompt("Multiple devices found, please select a radio to use.")
            .items(&items)
            .interact()?;
        devices.remove(selection)
    };

    let driver = UsbDriver::new(device).map_err(|e| format!("{:?}", e))?;

    let (channel_tx, router_rx) = channel(8);
    let (router_tx, channel_rx) = channel(8);

    let mut router = Router::new(driver, RxReceiver { receiver: router_rx }).map_err(|e| format!("{:?}", e))?;
    
    // ANT+ network key. Get your own from thisisant.com if you need it.
    let snk = SetNetworkKey::new(0, [0xB9, 0xA5, 0x21, 0xFB, 0xBD, 0x72, 0xC3, 0x45]);
    router.send(&snk).map_err(|e| format!("{:?}", e))?;
    
    let chan = router.add_channel(TxSender { sender: router_tx }).map_err(|e| format!("{:?}", e))?;

    let config = DisplayConfig {
        device_number: 0, // Wildcard search
        device_number_extension: 0.into(),
        channel: chan,
        period: Period::FourHz, // Matched to default HR sensor output
        ant_plus_key_index: 0,
    };
    
    let mut hr = Display::new(
        config,
        TxSender { sender: channel_tx },
        RxReceiver { receiver: channel_rx },
    );

    hr.set_rx_datapage_callback(Some(handle_rx));

    log::info!("Opening heart rate monitor channel...");
    hr.open();
    log::info!("ANT+ setup complete, listening for heart rate data...");

    loop {
        router.process().map_err(|e| format!("{:?}", e))?;
        hr.process().map_err(|e| format!("{:?}", e))?;
        // Small sleep to avoid pegging CPU
        thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Channel to send BPM from sync ANT+ thread to async tokio task
    let (bpm_tx, mut bpm_rx) = tokio_mpsc::channel(32);
    BPM_SENDER.set(bpm_tx).unwrap();

    // Spawn a thread for the blocking ANT+ work
    thread::spawn(move || {
        if let Err(e) = ant_main() {
            log::error!("ANT+ thread failed: {}", e);
        }
    });

    log::info!("Connecting to VRChat OSC...");
    let vrchat_osc = VRChatOSC::new().await?;
    log::info!("Connected to VRChat OSC service.");

    // This task will receive BPMs and send them over OSC
    tokio::spawn(async move {
        log::info!("OSC sender task started.");
        while let Some(bpm) = bpm_rx.recv().await {
            log::info!("Sending heart rate to VRChat: {} BPM", bpm);
            let normalized_bpm = (bpm as f32 / 255.0) * 2.0 - 1.0;
            let packet = OscPacket::Message(OscMessage {
                addr: "/avatar/parameters/Heartrate".to_string(),
                args: vec![OscType::Float(normalized_bpm)],
            });
            // Send to all VRChat clients
            if let Err(e) = vrchat_osc.send(packet, "VRChat-Client-*").await {
                log::error!("Failed to send OSC message: {}", e);
            }
        }
        log::info!("OSC sender task finished.");
    });

    log::info!("Application started. Press Ctrl+C to exit.");
    tokio::signal::ctrl_c().await?;
    log::info!("Shutting down...");

    Ok(())
}
