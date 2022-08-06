use std::{
	ops::RangeInclusive,
	sync::mpsc::{channel, Receiver, Sender},
};

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

pub struct EndpointWrapper {
	volume: f32,
	rx: Receiver<f32>,
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

		let (tx, rx) = channel();
		unsafe {
			let cb: IAudioEndpointVolumeCallback = Callback(tx).try_into()?;
			endpoint.RegisterControlChangeNotify(&cb)
		}?;

		Ok(Self {
			volume: calc_factor(unsafe { endpoint.GetMasterVolumeLevelScalar() }?),
			rx,
			_ep: endpoint,
		})
	}

	pub fn get_intensity(&mut self, beat_volume: f32) -> u8 {
		match self.rx.try_recv() {
			Ok(new_volume) => self.volume = new_volume,
			Err(_) => {}
		}

		(self.volume * beat_volume) as u8
	}
}

const VOL_RANGE: RangeInclusive<f32> = 0.1..=1.0;

fn calc_factor(n: f32) -> f32 {
	if VOL_RANGE.contains(&n) {
		// Since the beat_volume will also be reduced by a lower volume, square it to counter that
		16.0 / n.powi(2)
	} else {
		0.0
	}
}

#[allow(non_snake_case)]
#[windows::core::implement(IAudioEndpointVolumeCallback)]
struct Callback(Sender<f32>);

#[allow(non_snake_case)]
impl IAudioEndpointVolumeCallback_Impl for Callback {
	fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
		self.0
			.send(calc_factor(unsafe { (*pnotify).fMasterVolume }))
			.ok();

		Ok(())
	}
}
