use std::ops::RangeInclusive;

use windows::Win32::{
	Media::Audio::{
		eConsole, eRender,
		Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback},
		IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA,
	},
	System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_INPROC_SERVER},
};

pub struct EndpointWrapper {
	volume: Volume,
	endpoint: IAudioEndpointVolume,
}

impl EndpointWrapper {
	pub fn new() -> windows::core::Result<Self> {
		let endpoint: IAudioEndpointVolume = unsafe {
			CoInitialize(std::ptr::null())?;

			CoCreateInstance::<_, IMMDeviceEnumerator>(
				&MMDeviceEnumerator,
				None,
				CLSCTX_INPROC_SERVER,
			)?
			.GetDefaultAudioEndpoint(eRender, eConsole)?
			.Activate(CLSCTX_INPROC_SERVER, std::ptr::null())?
		};

		// let cb: IAudioEndpointVolumeCallback =
		// 	unsafe { device.Activate(CLSCTX_INPROC_SERVER, std::ptr::null())? };

		Ok(Self {
			volume: Volume::new(&endpoint),
			endpoint,
		})
	}

	pub fn get_intensity(&self, beat_volume: f32) -> u8 {
		(self.volume.0 * beat_volume) as u8
	}
}

const VOL_RANGE: RangeInclusive<f32> = f32::MIN_POSITIVE..=1.0;

struct Volume(f32);

impl Volume {
	pub fn new(endpoint: &IAudioEndpointVolume) -> Self {
		let new = Volume(match unsafe { endpoint.GetMasterVolumeLevelScalar() } {
			Ok(v) if VOL_RANGE.contains(&v) => 20.0 / v,
			_ => 0.0,
		});

		new
	}

	
}
