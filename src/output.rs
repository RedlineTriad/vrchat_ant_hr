use crate::config::OutputMode;
use crate::osc;
use std::sync::Arc;

pub async fn send_output(
    mode: OutputMode,
    bpm: u8,
    bpm_mode: crate::config::BpmMode,
    vrchat_osc: Option<&Arc<vrchat_osc::VRChatOSC>>,
) -> anyhow::Result<()> {
    match mode {
        OutputMode::Log => {
            log::info!("Heart rate: {} BPM (mode: {:?})", bpm, bpm_mode);
        }
        OutputMode::Vrchat => {
            if let Some(vrchat_osc) = vrchat_osc {
                let normalized_bpm = normalize_bpm(bpm);
                log::info!("Sending to VRChat: {} BPM (mode: {:?})", bpm, bpm_mode);
                osc::send_osc_heartbeat(vrchat_osc, normalized_bpm).await?;
            } else {
                anyhow::bail!("VRChat OSC not connected but output mode is vrchat");
            }
        }
    }
    Ok(())
}

fn normalize_bpm(bpm: u8) -> f32 {
    (bpm as f32 / 255.0) * 2.0 - 1.0
}
