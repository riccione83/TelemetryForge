use windows::Win32::{
    Media::Audio::{
        eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
    },
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
};

pub struct Reader {
    endpoint: Option<IAudioEndpointVolume>,
}

impl Reader {
    pub fn new() -> Self {
        Self { endpoint: None }
    }

    pub fn read(&mut self) -> Option<f32> {
        if self.endpoint.is_none() {
            self.endpoint = create_endpoint().ok();
        }
        let result = unsafe {
            self.endpoint
                .as_ref()?
                .GetMasterVolumeLevelScalar()
                .ok()
                .map(|value| value.clamp(0.0, 1.0) * 100.0)
        };
        if result.is_none() {
            self.endpoint = None;
        }
        result
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

fn create_endpoint() -> windows::core::Result<IAudioEndpointVolume> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
    }
}
