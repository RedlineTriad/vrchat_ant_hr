use anyhow::Context;
use vrchat_osc::{
    VRChatOSC,
    rosc::{OscMessage, OscPacket, OscType},
};

/// Sends a normalized heart rate value to VRChat via OSC
pub async fn send_osc_heartbeat(vrchat_osc: &VRChatOSC, normalized_bpm: f32) -> anyhow::Result<()> {
    let packet = OscPacket::Message(OscMessage {
        addr: "/avatar/parameters/Heartrate".to_string(),
        args: vec![OscType::Float(normalized_bpm)],
    });

    vrchat_osc
        .send(packet, "VRChat-Client-*")
        .await
        .context("Failed to send OSC message to VRChat")
}
