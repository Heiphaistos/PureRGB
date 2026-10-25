//! Diagnostic « Accès refusé » d'un écran USB : quel pilote/service Windows est lié au périphérique ?
//! SetupAPI + Configuration Manager (déjà utilisés par `scsi_win`), lecture seule, rien n'est ouvert ni modifié.
//! Le message est construit par une fonction pure ([`explain`]) testée sans matériel.

/// Un nœud PnP du périphérique : fin de l'identifiant d'instance, service lié (None = aucun).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub instance: String,
    pub service: Option<String>,
}

/// Services Microsoft courants d'un périphérique USB (tout autre nom = pilote tiers).
const MS_SERVICES: [&str; 12] = ["winusb", "usbccgp", "hidusb", "usbhub", "usbhub3", "usbstor", "usbscan", "hidclass", "usbprint", "usbvideo", "usbaudio", "usbaudio2"];

/// Fonction pure : message français décrivant les pilotes liés et ce qu'ils impliquent pour l'accès refusé.
pub fn explain(vid: u16, pid: u16, nodes: &[Node]) -> String {
    if nodes.is_empty() {
        return format!("Aucun nœud PnP {vid:04X}:{pid:04X} trouvé par Windows (périphérique débranché entre-temps ?).");
    }
    let list = nodes
        .iter()
        .map(|n| format!("{} → {}", n.instance, n.service.as_deref().unwrap_or("aucun service")))
        .collect::<Vec<_>>()
        .join(" ; ");
    let has = |s: &str| nodes.iter().any(|n| n.service.as_deref().is_some_and(|v| v.eq_ignore_ascii_case(s)));
    let third: Vec<&str> = nodes
        .iter()
        .filter_map(|n| n.service.as_deref())
        .filter(|s| !MS_SERVICES.iter().any(|m| m.eq_ignore_ascii_case(s)))
        .collect();
    let verdict = if !third.is_empty() {
        format!("un pilote tiers (« {} ») tient le périphérique : désinstallez le logiciel constructeur ou remplacez ce pilote par WinUSB (Zadig)", third.join("», « "))
    } else if has("winusb") {
        "WinUSB est bien lié : un autre processus (logiciel constructeur, 2e instance de PureRGB) a déjà réclamé l'interface".to_string()
    } else if nodes.iter().any(|n| n.service.is_some()) {
        "le périphérique est géré par un pilote Windows générique (usbccgp, HidUsb…), pas par WinUSB : installez WinUSB avec Zadig".to_string()
    } else {
        "aucun pilote lié : installez WinUSB avec Zadig".to_string()
    };
    format!("Pilotes Windows de {vid:04X}:{pid:04X} : {list}. Diagnostic : {verdict}.")
}

/// Nœuds PnP USB portant ce VID/PID (périphérique et interfaces `MI_xx`) avec leur service.
#[cfg(windows)]
pub fn nodes(vid: u16, pid: u16) -> Vec<Node> {
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::*;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

    let needle = format!("VID_{vid:04X}&PID_{pid:04X}");
    let usb: Vec<u16> = "USB\0".encode_utf16().collect();
    // SAFETY: énumérateur « USB » terminé par NUL ; autres arguments nuls ; liste détruite plus bas.
    let set = unsafe { SetupDiGetClassDevsW(null(), usb.as_ptr(), null_mut(), DIGCF_ALLCLASSES | DIGCF_PRESENT) };
    if set == INVALID_HANDLE_VALUE as isize {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut index = 0u32;
    loop {
        let mut dev: SP_DEVINFO_DATA = unsafe { zeroed() }; // SAFETY: structure C de nombres, zéro valide.
        dev.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
        // SAFETY: `set` valide, `dev.cbSize` renseigné.
        if unsafe { SetupDiEnumDeviceInfo(set, index, &mut dev) } == 0 {
            break;
        }
        index += 1;
        let mut id = [0u16; 512];
        // SAFETY: `id` fait 512 u16, longueur exacte passée.
        if unsafe { CM_Get_Device_IDW(dev.DevInst, id.as_mut_ptr(), id.len() as u32, 0) } != CR_SUCCESS {
            continue;
        }
        let n = id.iter().position(|&c| c == 0).unwrap_or(id.len());
        let full = String::from_utf16_lossy(&id[..n]);
        if !full.to_ascii_uppercase().contains(&needle) {
            continue;
        }
        let mut buf = [0u8; 256];
        let mut kind = 0u32;
        // SAFETY: `buf` de 256 octets, taille exacte passée ; `kind` valide en écriture.
        let ok = unsafe { SetupDiGetDeviceRegistryPropertyW(set, &dev, SPDRP_SERVICE, &mut kind, buf.as_mut_ptr(), buf.len() as u32, null_mut()) };
        let service = (ok != 0).then(|| {
            let w: Vec<u16> = buf.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
            String::from_utf16_lossy(&w)
        });
        // Fin de l'identifiant (sans le numéro de série d'instance) : « VID_..&PID_..&MI_00 » ou « VID_..&PID_.. ».
        let instance = full.split('\\').nth(1).unwrap_or(&full).to_string();
        out.push(Node { instance, service: service.filter(|s| !s.is_empty()) });
    }
    // SAFETY: `set` obtenu de SetupDiGetClassDevsW, plus utilisé ensuite.
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}

#[cfg(not(windows))]
pub fn nodes(_vid: u16, _pid: u16) -> Vec<Node> {
    Vec::new()
}

/// Message complet pour ce VID/PID (interroge Windows puis [`explain`]).
pub fn describe(vid: u16, pid: u16) -> String {
    explain(vid, pid, &nodes(vid, pid))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(i: &str, s: Option<&str>) -> Node {
        Node { instance: i.into(), service: s.map(String::from) }
    }

    #[test]
    fn winusb_means_someone_else_holds_it() {
        let m = explain(0x0416, 0x5408, &[n("VID_0416&PID_5408", Some("WinUSB"))]);
        assert!(m.contains("VID_0416&PID_5408 → WinUSB"), "{m}");
        assert!(m.contains("déjà réclamé l'interface"), "{m}");
    }

    #[test]
    fn generic_windows_driver_asks_for_winusb() {
        let m = explain(0x0416, 0x5408, &[n("VID_0416&PID_5408", Some("usbccgp")), n("VID_0416&PID_5408&MI_00", Some("HidUsb"))]);
        assert!(m.contains("usbccgp") && m.contains("MI_00 → HidUsb") && m.contains("Zadig"), "{m}");
    }

    #[test]
    fn third_party_driver_is_named() {
        let m = explain(0x0416, 0x5408, &[n("VID_0416&PID_5408", Some("TRCCDrv"))]);
        assert!(m.contains("pilote tiers (« TRCCDrv »"), "{m}");
    }

    #[test]
    fn no_service_and_no_node() {
        assert!(explain(0x0416, 0x5408, &[n("VID_0416&PID_5408", None)]).contains("aucun service"));
        assert!(explain(0x0416, 0x5408, &[]).contains("Aucun nœud PnP 0416:5408"));
    }

    /// Appel réel à Windows : `PURERGB_TEST_VIDPID=046D:C52B cargo test -- --ignored real_windows`.
    #[test]
    #[ignore = "nécessite un périphérique USB réel (variable PURERGB_TEST_VIDPID)"]
    fn real_windows_lookup() {
        let v = std::env::var("PURERGB_TEST_VIDPID").expect("PURERGB_TEST_VIDPID");
        let (a, b) = v.split_once(':').expect("VID:PID");
        let (vid, pid) = (u16::from_str_radix(a, 16).expect("vid"), u16::from_str_radix(b, 16).expect("pid"));
        let found = nodes(vid, pid);
        println!("{}", explain(vid, pid, &found));
        assert!(!found.is_empty());
    }
}
