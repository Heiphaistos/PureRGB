//! Backend « mobo » : ventilateurs branchés sur les headers PWM de la carte
//! mère, pilotés via les canaux Control de LibreHardwareMonitor (sensord).
//! C'est ce qui manque à OpenRGB/liquidctl : les ventilos sans hub USB.

use crate::backends::Backend;
use crate::core::{Color, DeviceInfo, DeviceType, FanChannel};
use crate::sensors::{Sensor, SensorHub};
use anyhow::{bail, Context, Result};
use std::sync::Arc;

pub struct MoboFanBackend {
    hub: Arc<SensorHub>,
    /// index canal → (id capteur Control, nom affiché).
    channels: Vec<(String, String)>,
}

impl MoboFanBackend {
    pub fn new(hub: Arc<SensorHub>) -> Self {
        MoboFanBackend {
            hub,
            channels: Vec::new(),
        }
    }

    /// Rend la main au BIOS sur tous les canaux pilotés (appel au quit).
    pub fn release_all(&self) {
        for (id, _) in &self.channels {
            let _ = self.hub.reset_control(id);
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
        self.hub.set_control(id, percent)
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
        let sensors = vec![
            sensor("/lpc/nct/0/fan/1", "Nuvoton NCT6798D", "Fan #2", "Fan", 820.0, false),
            sensor("/lpc/nct/0/control/1", "Nuvoton NCT6798D", "Fan Control #2", "Control", 45.0, true),
        ];
        assert_eq!(
            MoboFanBackend::rpm_for("Nuvoton NCT6798D", "Fan Control #2", &sensors),
            Some(820)
        );
        assert_eq!(
            MoboFanBackend::rpm_for("Nuvoton NCT6798D", "Fan Control #9", &sensors),
            None
        );
    }
}
