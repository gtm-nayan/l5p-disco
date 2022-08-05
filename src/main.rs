#![feature(duration_constants)]
#![feature(stmt_expr_attributes)]

use std::{
	ops::BitXor,
	sync::{atomic::AtomicBool, Arc},
	time::{Duration, Instant},
};

use lenovo_legion_hid::get_keyboard;
use vis_core::analyzer;

#[derive(Debug, Clone)]
pub struct VisInfo {
	beat: u64,
	beat_volume: f32,
}

fn main() {
	vis_core::default_config();
	vis_core::default_log();

	let mut keyboard = get_keyboard(Arc::new(AtomicBool::new(false))).unwrap();
	keyboard.set_brightness(2);

	let mut frames = {
		let mut beat = analyzer::BeatBuilder::new().build();
		let mut beat_num = 0;

		vis_core::Visualizer::new(
			VisInfo {
				beat: 0,
				beat_volume: 0.0,
			},
			move |info, samples| {
				if beat.detect(&samples) {
					beat_num += 1;
				}
				info.beat = beat_num;
				info.beat_volume = beat.last_volume();

				info
			},
		)
		.async_analyzer(300)
		.frames()
	};

	let frame_time = Duration::SECOND / 30;

	let mut beat_rolling = 0.0;
	let mut last_beat_num = 0;

	for frame in frames.iter() {
		let start = Instant::now();

		let base_volume = frame.info(|info| {
			if info.beat != last_beat_num {
				last_beat_num = info.beat;
			}

			info.beat_volume
		});

		beat_rolling = (beat_rolling * 0.95f32).max(base_volume);

		let primary = (beat_rolling * 32.0) as u8;
		let secondary = primary / 2;

		// Alternate zone 1 and 2 colors on beat
		let m = (last_beat_num & 1) as u8;
		let n = m.bitxor(1);

		keyboard.set_colors_to(
			#[rustfmt::skip]
            &[
                0          , secondary  , primary,
                m * primary, n * primary, 0      ,
                n * primary, m * primary, 0      ,
                0          , secondary  , primary,
            ],
		);

		frame_time
			.checked_sub(start.elapsed())
			.map(std::thread::sleep);
	}
}
