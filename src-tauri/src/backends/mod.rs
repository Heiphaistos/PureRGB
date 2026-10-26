pub mod hid;
pub mod liquidctl;
pub mod mobo;
pub mod openrgb;
pub mod smbus;

use crate::core::{Color, DeviceInfo};
use anyhow::Result;

/// Backend matériel : source d'appareils + application de couleurs / vitesses.
/// Chaque backend a un espace d'ids local; le registre préfixe avec son nom.
pub trait Backend: Send {
    fn name(&self) -> &'static str;

    /// Re-scanne le matériel. Retourne la liste courante des appareils.
    fn scan(&mut self) -> Result<Vec<DeviceInfo>>;

    /// Applique `colors` (une par LED) sur l'appareil `local_id`.
    fn set_colors(&mut self, local_id: &str, colors: &[Color]) -> Result<()>;

    /// Fixe la vitesse d'un canal ventilateur en % (0-100).
    fn set_fan_duty(&mut self, _local_id: &str, _channel: u8, _percent: u8) -> Result<()> {
        anyhow::bail!("contrôle ventilateur non supporté par ce backend")
    }

    /// Applique un réglage non lumineux d'appareil (`DeviceInfo.settings`), en RAM seulement.
    fn set_device_setting(&mut self, _local_id: &str, _setting: &str, _value: i32) -> Result<()> {
        anyhow::bail!("réglages non supportés par ce backend")
    }

    /// Rend un canal que PureRGB pilotait (courbe supprimée/désactivée). Par défaut : rien (le matériel garde sa consigne).
    fn release_fan(&mut self, _local_id: &str, _channel: u8) {}

    /// true si le backend est opérationnel (ex: OpenRGB connecté).
    fn is_available(&self) -> bool;

    /// Downcast vers le type concret (reconfiguration à chaud).
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
