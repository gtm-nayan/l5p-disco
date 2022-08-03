use std::sync::{atomic::AtomicBool, Arc};

use lenovo_legion_hid::get_keyboard;
use vis_core::analyzer;

#[derive(Debug, Clone)]
pub struct VisInfo {
    beat: u64,
    beat_volume: f32,
    volume: f32,
    analyzer: analyzer::FourierAnalyzer,
    spectrum: analyzer::Spectrum<Vec<f32>>,
}

fn main() {
    vis_core::default_config();
    vis_core::default_log();

    let mut keyboard = get_keyboard(Arc::new(AtomicBool::new(false))).unwrap();

    let mut frames = {
        // Analyzer {{{
        let mut beat = analyzer::BeatBuilder::new().build();
        let mut beat_num = 0;

        let analyzer = analyzer::FourierBuilder::new().plan();

        vis_core::Visualizer::new(
            VisInfo {
                beat: 0,
                beat_volume: 0.0,
                volume: 0.0,
                spectrum: analyzer::Spectrum::new(vec![0.0; analyzer.buckets()], 0.0, 1.0),
                analyzer,
            },
            move |info, samples| {
                if beat.detect(&samples) {
                    beat_num += 1;
                }
                info.beat = beat_num;
                info.beat_volume = beat.last_volume();
                info.volume = samples.volume(0.3);

                info.analyzer.analyze(&samples);
                info.spectrum.fill_from(&info.analyzer.average());

                info
            },
        )
        .async_analyzer(300)
        .frames()
        // }}}
    };

    let frame_time =
        std::time::Duration::from_micros(1000000 / vis_core::CONFIG.get_or("rgb.fps", 30));

    let mut previous_time = 0.0;

    let mut last_beat = -100.0;

    let mut beat_rolling = 0.0;
    let mut last_beat_num = 0;

    for frame in frames.iter() {
        let start = std::time::Instant::now();
        let delta = frame.time - previous_time;

        let (volume, /* maxima, */ /* notes_rolling_spectrum, */ base_volume) =
            frame.info(|info| {
                if info.beat != last_beat_num {
                    last_beat = frame.time;
                    last_beat_num = info.beat;
                }
                let blue = (beat_rolling * 25.0) as u8;
                let green = blue / 2;

                let (a, b) = if last_beat_num.rem_euclid(2) == 0 {
                    (1, 2)
                } else {
                    (2, 1)
                };
                keyboard.set_zone_by_index(a, [blue, 0, 0]);
                keyboard.set_zone_by_index(b, [0, blue, 0]);
                keyboard.set_zone_by_index(0, [0, green, blue]);
                keyboard.set_zone_by_index(3, [0, green, blue]);
                keyboard.refresh();
                (info.volume, info.beat_volume)
            });

        beat_rolling = (beat_rolling * 0.95f32).max(base_volume);

        previous_time = frame.time;

        let end = std::time::Instant::now();
        let dur = end - start;
        if dur < frame_time {
            let sleep = frame_time - dur;
            std::thread::sleep(sleep);
        }
    }
}
