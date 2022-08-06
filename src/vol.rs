use std::ops::{Range, RangeInclusive};

use windows::Win32::{
	Media::Audio::{
		eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
	},
	System::Com::{CoCreateInstance, CoInitialize, CLSCTX_INPROC_SERVER},
};

pub struct Volume {
	endpoint: IAudioEndpointVolume,
}

const VOL_RANGE: RangeInclusive<f32> = f32::MIN_POSITIVE..=1.0;

impl Volume {
	pub fn new() -> windows::core::Result<Self> {
		let endpoint = unsafe {
			CoInitialize(std::ptr::null())?;

			CoCreateInstance::<_, IMMDeviceEnumerator>(
				&MMDeviceEnumerator,
				None,
				CLSCTX_INPROC_SERVER,
			)?
			.GetDefaultAudioEndpoint(eRender, eConsole)?
			.Activate(CLSCTX_INPROC_SERVER, std::ptr::null())?
		};
		Ok(Self { endpoint })
	}

	pub fn get_intensity(&self, beat_volume: f32) -> u8 {
		match unsafe { self.endpoint.GetMasterVolumeLevelScalar() } {
			Ok(v) if VOL_RANGE.contains(&v) => (beat_volume * 20.0 / v) as u8,
			_ => 0,
		}
	}
}
