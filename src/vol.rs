use std::{
	cell::UnsafeCell,
	ops::{Mul, RangeInclusive},
};

use windows::Win32::{
	Media::Audio::{
		eConsole, eRender,
		Endpoints::{
			IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
		},
		IMMDeviceEnumerator, MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA,
	},
	System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_INPROC_SERVER},
};

pub struct EndpointWrapper {
	// Data races don't matter for a single frame, so get away with blatant mutation
	volume: UnsafeCell<f32>,
	handle: IAudioEndpointVolumeCallback,
	_ep: IAudioEndpointVolume,
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

		let volume = UnsafeCell::new(calc_factor(unsafe {
			endpoint.GetMasterVolumeLevelScalar()?
		}));

		Ok(Self {
			handle: unsafe {
				let handle = Callback(volume.get()).try_into()?;
				endpoint.RegisterControlChangeNotify(&handle)?;
				handle
			},
			volume,
			_ep: endpoint,
		})
	}

	pub fn get_intensity(&self, beat_volume: f32) -> u8 {
		(unsafe { *self.volume.get() } * beat_volume) as u8
	}
}

impl Drop for EndpointWrapper {
	fn drop(&mut self) {
		unsafe {
			self._ep.UnregisterControlChangeNotify(&self.handle).ok();
			CoUninitialize();
		};
	}
}

const VOL_RANGE: RangeInclusive<f32> = 0.1..=1.0;

fn calc_factor(n: f32) -> f32 {
	if VOL_RANGE.contains(&n) {
		// Since the beat_volume will also be reduced by a lower volume, square it to counter that
		dbg!(16.0 / n.powi(2))
	} else {
		0.0
	}
}

#[allow(non_snake_case)]
#[windows::core::implement(IAudioEndpointVolumeCallback)]
struct Callback(*mut f32);

#[allow(non_snake_case)]
impl IAudioEndpointVolumeCallback_Impl for Callback {
	fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
		// SAFETY: EndpointWrapper's UnsafeCell is dropped after Callback
		unsafe { *self.0 = calc_factor((*pnotify).fMasterVolume) };
		Ok(())
	}
}
