use crate::config::{BpmMode, HeartRateData};

pub struct BpmProcessor {
    prev_intra_beat_time: Option<u16>,
}

impl BpmProcessor {
    pub fn new() -> Self {
        Self {
            prev_intra_beat_time: None,
        }
    }

    pub fn process(&mut self, data: HeartRateData, mode: BpmMode) -> Option<u8> {
        fn bpm_from_intra_beat(intra_beat_time: u16) -> u8 {
            let bpm = 60.0 / (intra_beat_time as f32 / 1000.0);
            bpm as u8
        }

        match (mode, data.intra_beat_time) {
            (BpmMode::Computed, _) => Some(data.bpm),
            (BpmMode::IntraBeat, Some(intra_beat_time)) => {
                let should_skip = self.check_threshold(intra_beat_time);

                self.prev_intra_beat_time = Some(intra_beat_time);
                if should_skip {
                    None
                } else {
                    Some(bpm_from_intra_beat(intra_beat_time))
                }
            }
            (BpmMode::IntraBeatUnfiltered, Some(intra_beat_time)) => {
                self.prev_intra_beat_time = Some(intra_beat_time);
                Some(bpm_from_intra_beat(intra_beat_time))
            }
            _ => None,
        }
    }

    fn check_threshold(&self, intra_beat_time: u16) -> bool {
        self.prev_intra_beat_time
            .map(|prev_time| {
                let upper_threshold = (prev_time as f32 * 2.0) * 0.8;
                let lower_threshold = (prev_time as f32 / 2.0) * 1.2;
                let time_val = intra_beat_time as f32;
                if time_val > upper_threshold {
                    let unfiltered_bpm = 60.0 / (intra_beat_time as f32 / 1000.0);
                    log::warn!(
                        "Skipping {} BPM - potential missed beat ({} ms > {:.1} ms threshold)",
                        unfiltered_bpm,
                        intra_beat_time,
                        upper_threshold
                    );
                    true
                } else if time_val < lower_threshold {
                    let unfiltered_bpm = 60.0 / (intra_beat_time as f32 / 1000.0);
                    log::warn!(
                        "Skipping {} BPM - potential doubled beat ({} ms < {:.1} ms threshold)",
                        unfiltered_bpm,
                        intra_beat_time,
                        lower_threshold
                    );
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }
}

impl Default for BpmProcessor {
    fn default() -> Self {
        Self::new()
    }
}
