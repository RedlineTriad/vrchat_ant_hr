use clap::ValueEnum;

#[derive(Debug, Clone)]
pub struct HeartRateData {
    pub bpm: u8,
    pub intra_beat_time: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BpmMode {
    Computed,
    IntraBeat,
    IntraBeatUnfiltered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputMode {
    Log,
    Vrchat,
}
