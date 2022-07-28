use std::{
    cmp,
    collections::VecDeque,
    time::{Duration, Instant},
};

use settings_schema::Switch;

use crate::SERVER_DATA_MANAGER;

struct VideoBitrateManager {
    bitrate_mbs: u64,
    bit_count_history: VecDeque<u64>,
    network_latency_history: VecDeque<Duration>,
    frame_interval_history: VecDeque<Duration>,
    previous_frame_time: Instant,
}

impl VideoBitrateManager {
    pub fn new() -> Self {
        let data_manager = SERVER_DATA_MANAGER.read();
        Self {
            bitrate_mbs: data_manager.settings().video.encode_bitrate_mbs,
            bit_count_history: [0].into_iter().collect(),
            network_latency_history: [Duration::ZERO].into_iter().collect(),
            frame_interval_history: [Duration::ZERO].into_iter().collect(),
            previous_frame_time: Instant::now(),
        }
    }

    // Only for video packets
    pub fn report_packet_bytes(&mut self, count: u64) {
        self.bit_count_history.push_back(count * 8);

        if self.bit_count_history.len()
            > SERVER_DATA_MANAGER
                .read()
                .settings()
                .connection
                .statistics_history_size as usize
        {
            self.bit_count_history.pop_front();
        }
    }

    // this also counts frames. call once per frame
    pub fn report_network_latency(&mut self, latency: Duration) {
        let latency = {
            if latency > Duration::from_millis(500) || latency == Duration::ZERO {
                Duration::from_millis(500)
            } else {
                latency
            }
        };

        self.network_latency_history.push_back(latency);
        if self.network_latency_history.len()
            > SERVER_DATA_MANAGER
                .read()
                .settings()
                .connection
                .statistics_history_size as usize
        {
            self.network_latency_history.pop_front();
        }

        let now = Instant::now();
        self.frame_interval_history
            .push_back(Instant::now() - self.previous_frame_time);
        if self.frame_interval_history.len()
            > SERVER_DATA_MANAGER
                .read()
                .settings()
                .connection
                .statistics_history_size as usize
        {
            self.frame_interval_history.pop_front();
        }
        self.previous_frame_time = now;
    }

    pub fn bitrate_mbs(&mut self) -> u64 {
        let data_manager = SERVER_DATA_MANAGER.read();

        if let Switch::Enabled(config) = &data_manager.settings().video.adaptive_bitrate {
            let adaptive_bitrate_target_us = if let Switch::Enabled(config) =
                &config.latency_use_frametime
            {
                let average_frame_interval = self.frame_interval_history.iter().sum::<Duration>()
                    / self.frame_interval_history.len() as u32;

                cmp::min(
                    average_frame_interval.as_micros() as u64 + config.latency_target_offset as u64,
                    config.latency_target_maximum,
                )
            } else {
                config.latency_target
            };

            let average_latency = self.network_latency_history.iter().sum::<Duration>()
                / self.network_latency_history.len() as u32;
            let averate_past_bitrate_bits =
                self.bit_count_history.iter().sum::<u64>() / self.bit_count_history.len() as u64;

            if average_latency != Duration::ZERO {
                if average_latency.as_micros() as u64
                    <= adaptive_bitrate_target_us + config.latency_threshold
                {
                    if self.bitrate_mbs <= 5 + config.bitrate_down_rate {
                        self.bitrate_mbs += 5
                    } else {
                        self.bitrate_mbs -= config.bitrate_down_rate
                    }
                } else if (average_latency.as_micros() as u64)
                    < adaptive_bitrate_target_us - config.latency_threshold
                {
                    if self.bitrate_mbs >= config.bitrate_maximum - config.bitrate_up_rate {
                        self.bitrate_mbs = config.bitrate_maximum
                    } else if averate_past_bitrate_bits as f32 / 1e6
                        > self.bitrate_mbs as f32
                            * config.bitrate_light_load_threshold as f32
                            * data_manager.settings().video.preferred_fps
                            / average_latency.as_secs_f32()
                    {
                        self.bitrate_mbs += config.bitrate_up_rate
                    }
                }
            }

            self.bitrate_mbs
        } else {
            data_manager.settings().video.encode_bitrate_mbs
        }
    }
}
