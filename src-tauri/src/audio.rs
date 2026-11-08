//! Audio Windows des casques/micros : volume et sourdine de chaque point de terminaison (WASAPI, API publique Microsoft).
//! Rien n'est stocké dans l'appareil : on règle le volume Windows de la sortie/entrée, comme le mélangeur système.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AudioEndpoint {
    pub id: String,
    pub name: String,
    /// "sortie" (haut-parleurs/casque) ou "entree" (micro).
    pub flow: String,
    /// 0 à 100.
    pub volume: u8,
    pub muted: bool,
    pub is_default: bool,
    /// Nombre de mots distinctifs communs avec le nom de l'appareil demandé (0 si aucun appareil donné).
    pub score: u32,
}

const GENERIC: [&str; 8] = ["logitech", "gaming", "headset", "wireless", "casque", "microphone", "micro", "audio"];

fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty() && !GENERIC.contains(t))
        .map(str::to_owned)
        .collect()
}

/// Mots distinctifs partagés entre le nom de l'appareil PureRGB et celui du point de terminaison Windows.
pub fn match_score(device_name: &str, endpoint_name: &str) -> u32 {
    let e = tokens(endpoint_name);
    tokens(device_name).iter().filter(|t| e.contains(t)).count() as u32
}

#[cfg(windows)]
mod win {
    use super::{match_score, AudioEndpoint};
    use anyhow::{anyhow, bail, Result};
    use windows::core::{GUID, PCWSTR};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eCapture, eConsole, eRender, EDataFlow, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};

    /// COM initialisé pour la durée d'un appel (les commandes Tauri tournent sur un fil du pool).
    struct Com(bool);
    impl Com {
        fn new() -> Self {
            // Échec attendu (RPC_E_CHANGED_MODE) si le fil est déjà initialisé autrement : on n'appelle alors pas CoUninitialize.
            Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
        }
    }
    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    fn enumerator() -> Result<IMMDeviceEnumerator> {
        Ok(unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }?)
    }

    fn id_of(d: &IMMDevice) -> Result<String> {
        let p = unsafe { d.GetId() }?;
        let s = unsafe { p.to_string() }.map_err(|e| anyhow!("id audio illisible : {e}"));
        unsafe { CoTaskMemFree(Some(p.0 as *const _)) };
        s
    }

    fn name_of(d: &IMMDevice) -> String {
        let name = (|| -> Result<String> {
            let store = unsafe { d.OpenPropertyStore(STGM_READ) }?;
            let v = unsafe { store.GetValue(&PKEY_Device_FriendlyName) }?;
            Ok(v.to_string())
        })();
        name.unwrap_or_default()
    }

    fn volume_of(d: &IMMDevice) -> Result<IAudioEndpointVolume> {
        Ok(unsafe { d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) }?)
    }

    fn collect(e: &IMMDeviceEnumerator, flow: EDataFlow, label: &str, device_name: &str, out: &mut Vec<AudioEndpoint>) -> Result<()> {
        let default_id = unsafe { e.GetDefaultAudioEndpoint(flow, eConsole) }.ok().and_then(|d| id_of(&d).ok());
        let list = unsafe { e.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE) }?;
        for i in 0..unsafe { list.GetCount() }? {
            let d = unsafe { list.Item(i) }?;
            let id = id_of(&d)?;
            let Ok(vol) = volume_of(&d) else { continue };
            let (Ok(level), Ok(muted)) = (unsafe { vol.GetMasterVolumeLevelScalar() }, unsafe { vol.GetMute() }) else { continue };
            let name = name_of(&d);
            out.push(AudioEndpoint {
                score: match_score(device_name, &name),
                is_default: default_id.as_deref() == Some(id.as_str()),
                volume: (level * 100.0).round().clamp(0.0, 100.0) as u8,
                muted: muted.as_bool(),
                flow: label.to_owned(),
                name,
                id,
            });
        }
        Ok(())
    }

    pub fn list(device_name: &str) -> Result<Vec<AudioEndpoint>> {
        let _com = Com::new();
        let e = enumerator()?;
        let mut out = Vec::new();
        collect(&e, eRender, "sortie", device_name, &mut out)?;
        collect(&e, eCapture, "entree", device_name, &mut out)?;
        out.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(out)
    }

    pub fn set(id: &str, volume: Option<u8>, muted: Option<bool>) -> Result<()> {
        if volume.is_some_and(|v| v > 100) {
            bail!("volume hors de 0 à 100");
        }
        let _com = Com::new();
        let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        let d = unsafe { enumerator()?.GetDevice(PCWSTR(wide.as_ptr())) }.map_err(|_| anyhow!("point audio introuvable (débranché ?)"))?;
        let vol = volume_of(&d)?;
        if let Some(v) = volume {
            unsafe { vol.SetMasterVolumeLevelScalar(f32::from(v) / 100.0, &GUID::zeroed()) }?;
        }
        if let Some(m) = muted {
            unsafe { vol.SetMute(m, &GUID::zeroed()) }?;
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use win::{list, set};

#[cfg(not(windows))]
pub fn list(_device_name: &str) -> anyhow::Result<Vec<AudioEndpoint>> {
    Ok(Vec::new())
}
#[cfg(not(windows))]
pub fn set(_id: &str, _volume: Option<u8>, _muted: Option<bool>) -> anyhow::Result<()> {
    anyhow::bail!("audio : Windows uniquement")
}

#[tauri::command]
pub async fn audio_list(device_name: String) -> Result<Vec<AudioEndpoint>, String> {
    tauri::async_runtime::spawn_blocking(move || list(&device_name).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn audio_set(id: String, volume: Option<u8>, muted: Option<bool>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || set(&id, volume, muted).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headset_endpoints_are_matched_by_distinctive_words() {
        let dev = "Logitech G PRO X 2 LIGHTSPEED";
        assert!(match_score(dev, "Casque (PRO X 2 LIGHTSPEED Gaming Headset)") >= 4);
        assert_eq!(match_score(dev, "Haut-parleurs (Realtek Audio)"), 0);
    }

    #[cfg(windows)]
    #[test]
    fn real_endpoints_can_be_listed() {
        // Banc réel : la machine de test peut n'avoir aucune carte son, mais l'appel ne doit pas échouer.
        let l = list("x").expect("énumération WASAPI");
        assert!(l.iter().all(|e| e.volume <= 100));
    }

    #[test]
    fn out_of_range_volume_is_refused() {
        assert!(set("x", Some(101), None).is_err());
    }
}
