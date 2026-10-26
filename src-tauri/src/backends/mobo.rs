//! Backend « mobo » : ventilateurs branchés sur les headers PWM de la carte
//! mère, pilotés via les canaux Control de LibreHardwareMonitor (sensord).
//! C'est ce qui manque à OpenRGB/liquidctl : les ventilos sans hub USB.

use crate::backends::Backend;
use crate::core::{Color, DeviceInfo, DeviceType, FanChannel};
use crate::sensors::{Sensor, SensorHub};
use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::sync::Arc;

pub struct MoboFanBackend {
    hub: Arc<SensorHub>,
    /// index canal → (id capteur Control, nom affiché).
    channels: Vec<(String, String)>,
    /// Canaux écrits par PureRGB depuis le dernier retour au BIOS : seuls ceux-là sont rendus (LHM mémorise l'état
    /// d'origine à la première écriture logicielle et le restaure sur `reset`).
    touched: HashSet<String>,
}

impl MoboFanBackend {
    pub fn new(hub: Arc<SensorHub>) -> Self {
        MoboFanBackend {
            hub,
            channels: Vec::new(),
            touched: HashSet::new(),
        }
    }

    /// Rend la main au BIOS sur les canaux que PureRGB a réglés (appel au quit / mode jeu) ; les autres ne sont pas touchés.
    pub fn release_all(&mut self) {
        for id in self.touched.drain() {
            let _ = self.hub.reset_control(&id);
        }
    }

    /// Apparie un Control ("Fan Control #2") au RPM ("Fan #2") du même matériel.
    fn rpm_for(controls_hw: &str, control_name: &str, sensors: &[Sensor]) -> Option<u32> {
        let suffix = control_name.rsplit('#').next()?.trim().to_string();
        sensors
            .iter()
            .find(|s| {
                s.kind == "Fan"
                    && s.hardware == controls_hw
                    && s.name.rsplit('#').next().map(|n| n.trim()) == Some(suffix.as_str())
            })
            .map(|s| s.value.round() as u32)
    }
}

impl Backend for MoboFanBackend {
    fn name(&self) -> &'static str {
        "mobo"
    }

    fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
        let sensors = self.hub.snapshot();
        self.channels.clear();
        // Canaux Control pilotables du Super I/O carte mère (id /lpc/...).
        // Les Control GPU/AIO sont exclus : gérés par leurs propres outils.
        let controls: Vec<&Sensor> = sensors
            .iter()
            .filter(|s| s.kind == "Control" && s.controllable && s.id.starts_with("/lpc/"))
            .collect();
        if controls.is_empty() {
            return Ok(Vec::new());
        }
        let hw_name = controls[0].hardware.clone();
        let mut fan_channels = Vec::new();
        for (i, c) in controls.iter().enumerate() {
            self.channels.push((c.id.clone(), c.name.clone()));
            fan_channels.push(FanChannel {
                index: i as u8,
                name: c.name.clone(),
                duty_percent: Some(c.value.round().clamp(0.0, 100.0) as u8),
                rpm: Self::rpm_for(&c.hardware, &c.name, &sensors),
            });
        }
        Ok(vec![DeviceInfo {
            id: "0".into(),
            name: format!("Ventilateurs carte mère ({hw_name})"),
            vendor: String::new(),
            backend: String::new(),
            device_type: DeviceType::Fan,
            zones: Vec::new(),
            led_count: 0,
            fan_channels,
            settings: Vec::new(),
            controllable: true,
            has_lcd: false,
            modes: Vec::new(),
            active_mode: -1,
            note: "headers PWM pilotés via LibreHardwareMonitor — RGB via OpenRGB".into(),
        }])
    }

    fn set_colors(&mut self, _local_id: &str, _colors: &[Color]) -> Result<()> {
        bail!("backend ventilation uniquement — RGB via OpenRGB")
    }

    fn set_fan_duty(&mut self, _local_id: &str, channel: u8, percent: u8) -> Result<()> {
        let (id, _) = self
            .channels
            .get(channel as usize)
            .with_context(|| format!("canal carte mère {channel} inconnu"))?;
        let id = id.clone();
        self.hub.set_control(&id, percent)?;
        self.touched.insert(id);
        Ok(())
    }

    fn release_fan(&mut self, _local_id: &str, channel: u8) {
        if let Some((id, _)) = self.channels.get(channel as usize) {
            if self.touched.remove(id) {
                let _ = self.hub.reset_control(id);
            }
        }
    }

    fn is_available(&self) -> bool {
        self.hub.running()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor(id: &str, hw: &str, name: &str, kind: &str, value: f64, ctl: bool) -> Sensor {
        Sensor {
            id: id.into(),
            hardware: hw.into(),
            name: name.into(),
            kind: kind.into(),
            value,
            controllable: ctl,
        }
    }

    #[test]
    fn rpm_pairing_by_number() {
        // Vérifié contre le source LHM (SuperIOHardware.cs / Nct677X.cs) :
        // le capteur RPM et le canal Control d'un même ventilateur portent
        // le MÊME nom "Fan #N" (seul le SensorType diffère) — la mise en
        // correspondance par suffixe fonctionne donc trivialement.
        let sensors = vec![
            sensor("/lpc/nct6798d/0/fan/1", "Nuvoton NCT6798D", "Fan #2", "Fan", 820.0, false),
            sensor("/lpc/nct6798d/0/control/1", "Nuvoton NCT6798D", "Fan #2", "Control", 45.0, true),
        ];
        assert_eq!(
            MoboFanBackend::rpm_for("Nuvoton NCT6798D", "Fan #2", &sensors),
            Some(820)
        );
        assert_eq!(
            MoboFanBackend::rpm_for("Nuvoton NCT6798D", "Fan #9", &sensors),
            None
        );
    }

    #[test]
    fn control_id_prefix_matches_lhm_identifier_scheme() {
        // SuperIOHardware.cs: base(..., new Identifier("lpc", chip, index), ...)
        // => tout capteur d'un Super I/O carte mère est sous "/lpc/...".
        // Les GPU/AIO utilisent d'autres préfixes (/nvidiagpu/, etc.) et ne
        // doivent jamais apparaître dans ce backend.
        let mobo = sensor("/lpc/nct6798d/0/control/1", "Nuvoton NCT6798D", "Fan #2", "Control", 45.0, true);
        let gpu = sensor("/nvidiagpu/0/control/0", "NVIDIA GeForce RTX", "GPU Fan", "Control", 60.0, true);
        assert!(mobo.id.starts_with("/lpc/"));
        assert!(!gpu.id.starts_with("/lpc/"));
    }

    fn backend() -> MoboFanBackend {
        let mut b = MoboFanBackend::new(SensorHub::new());
        b.channels = vec![("/lpc/a/control/0".into(), "Fan #1".into()), ("/lpc/a/control/1".into(), "Fan #2".into())];
        b
    }

    fn sent(b: &MoboFanBackend) -> Vec<(String, String)> {
        b.hub.sent.lock().iter().map(|v| (v["cmd"].as_str().unwrap_or("").to_string(), v["id"].as_str().unwrap_or("").to_string())).collect()
    }

    #[test]
    fn untouched_channels_are_never_written_nor_reset() {
        let mut b = backend();
        b.release_all();
        b.release_fan("0", 0);
        assert!(sent(&b).is_empty(), "aucune commande vers sensord sans réglage de l'utilisateur");
    }

    #[test]
    fn only_the_channel_set_by_the_user_is_handed_back_to_the_bios_once() {
        let mut b = backend();
        b.set_fan_duty("0", 1, 60).unwrap();
        b.release_all();
        b.release_all();
        let s = sent(&b);
        assert_eq!(s, vec![("set".to_string(), "/lpc/a/control/1".to_string()), ("reset".to_string(), "/lpc/a/control/1".to_string())]);
        assert_eq!(b.hub.sent.lock()[0]["value"], 60);
    }

    #[test]
    fn release_fan_hands_back_one_channel_only() {
        let mut b = backend();
        b.set_fan_duty("0", 0, 40).unwrap();
        b.set_fan_duty("0", 1, 50).unwrap();
        b.release_fan("0", 0);
        b.release_all();
        let resets: Vec<String> = sent(&b).into_iter().filter(|(c, _)| c == "reset").map(|(_, id)| id).collect();
        assert_eq!(resets, vec!["/lpc/a/control/0".to_string(), "/lpc/a/control/1".to_string()]);
    }
}
