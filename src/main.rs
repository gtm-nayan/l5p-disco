#![feature(duration_constants)]
#![feature(stmt_expr_attributes)]

use windows::Win32::{
	Media::Audio::{
		eConsole, eRender,
		Endpoints::{
			IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
		},
		IMMDeviceEnumerator, MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA,
	},
	System::Com::{CoCreateInstance, CoInitialize, CLSCTX_INPROC_SERVER},
};

use lenovo_legion_hid::get_keyboard;
use std::{
	cell::UnsafeCell,
	ops::{Mul, RangeInclusive},
	time::{Duration, Instant},
};
use vis_core::analyzer;

#[allow(non_snake_case)]
#[windows::core::implement(IAudioEndpointVolumeCallback)]
struct Callback(*mut f32);

#[allow(non_snake_case)]
impl IAudioEndpointVolumeCallback_Impl for Callback {
	fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
		unsafe { *self.0 = calc_factor((*pnotify).fMasterVolume) };
		Ok(())
	}
}

const VOL_RANGE: RangeInclusive<f32> = 0.1..=1.0;

fn calc_factor(n: f32) -> f32 {
	if VOL_RANGE.contains(&n) {
		// Since the beat_volume will also be reduced by a lower volume, square it to counter that
		dbg!(32.0 / n.powi(2))
	} else {
		0.0
	}
}

#[derive(Debug, Clone)]
struct VisInfo {
	beat: u64,
	beat_volume: f32,
}

fn main() -> windows::core::Result<()> {
	vis_core::default_config();
	vis_core::default_log();

	let mut volume = UnsafeCell::new(0.0);

	// Prevent dropping endpoint and callback handle otherwise it's not called
	let _callback_handle = unsafe {
		CoInitialize(std::ptr::null())?;

		let endpoint: IAudioEndpointVolume = CoCreateInstance::<_, IMMDeviceEnumerator>(
			&MMDeviceEnumerator,
			None,
			CLSCTX_INPROC_SERVER,
		)?
		.GetDefaultAudioEndpoint(eRender, eConsole)?
		.Activate(CLSCTX_INPROC_SERVER, std::ptr::null())?;

		let volume_ptr = volume.get();
		*volume_ptr = calc_factor(endpoint.GetMasterVolumeLevelScalar()?);
		let handle = Callback(volume_ptr).into();
		endpoint.RegisterControlChangeNotify(&handle)?;
		(endpoint, handle)
	};

	let mut keyboard = get_keyboard().unwrap();
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
				if beat.detect(samples) {
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

	for frame in frames.iter() {
		let start = Instant::now();

		let (base_volume, beat_num) = frame.info(|info| (info.beat_volume, info.beat));

		beat_rolling = (beat_rolling * 0.9f32).max(base_volume);

		let primary = volume.get_mut().mul(beat_rolling) as u8;
		let secondary = primary / 2;

		// Alternate zone 1 and 2 colors on beat
		let (m, n) = if beat_num % 2 == 0 {
			(primary, 0)
		} else {
			(0, primary)
		};

		keyboard.set_colors_to(
			#[rustfmt::skip]
	        &[
				0, secondary, primary,
				m, n        , 0      ,
				n, m        , 0      ,
				0, secondary, primary,
			],
		);

		if let Some(time) = frame_time.checked_sub(start.elapsed()) {
			std::thread::sleep(time);
		}
	}
	Ok(())
}
