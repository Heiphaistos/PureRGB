//! Accès Windows aux écrans USB Mass Storage « USBLCD » : retrouve le bon lecteur physique par
//! VID/PID via SetupAPI + Configuration Manager (aucun PowerShell), puis envoie les CDB par
//! `IOCTL_SCSI_PASS_THROUGH_DIRECT` sur `\\.\PhysicalDriveN`.
//!
//! Seul module du pilote contenant du `unsafe` : appels FFI Win32, chacun isolé et commenté.
//! Le pass-through exige des droits d'administrateur (ouverture en lecture/écriture du disque).

use super::transport::ScsiTransport;
use anyhow::{anyhow, bail, Result};
use std::mem::{offset_of, size_of, zeroed};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::Storage::IscsiDisc::*;
use windows_sys::Win32::System::Ioctl::*;
use windows_sys::Win32::System::IO::DeviceIoControl;

/// Un lecteur physique correspondant à un VID/PID.
#[derive(Debug, Clone)]
pub struct DriveRef {
    pub number: u32,
    /// Fin de l'identifiant d'instance USB (numéro de série ou chemin de port), pour l'id d'écran.
    pub serial: String,
}

/// Poignée Win32 fermée à la destruction.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: la poignée a été obtenue de CreateFileW et n'est fermée qu'ici.
        unsafe { CloseHandle(self.0) };
    }
}

// SAFETY: une poignée de fichier Win32 peut être utilisée depuis n'importe quel thread.
unsafe impl Send for OwnedHandle {}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn open_path(path: &[u16], access: u32) -> Result<OwnedHandle> {
    // SAFETY: `path` est terminé par un NUL ; les pointeurs optionnels sont nuls.
    let h = unsafe {
        CreateFileW(path.as_ptr(), access, FILE_SHARE_READ | FILE_SHARE_WRITE, null(), OPEN_EXISTING, 0, null_mut())
    };
    if h == INVALID_HANDLE_VALUE {
        // SAFETY: lecture du dernier code d'erreur du thread courant.
        let code = unsafe { GetLastError() };
        if code == ERROR_ACCESS_DENIED {
            bail!("Accès refusé au lecteur de l'écran : lancez PureRGB en administrateur (requis pour le SCSI) et fermez TRCC.");
        }
        bail!("Ouverture du lecteur de l'écran impossible (erreur Windows {code}). Fermez TRCC et rebranchez l'écran.");
    }
    Ok(OwnedHandle(h))
}

/// Identifiants d'instance du périphérique et de ses parents (jusqu'à 5 niveaux).
fn instance_chain(devinst: u32) -> Vec<String> {
    let (mut chain, mut cur) = (Vec::new(), devinst);
    for _ in 0..5 {
        let mut buf = [0u16; 512];
        // SAFETY: `buf` fait 512 u16 et on passe sa longueur exacte.
        if unsafe { CM_Get_Device_IDW(cur, buf.as_mut_ptr(), buf.len() as u32, 0) } != CR_SUCCESS {
            break;
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        chain.push(String::from_utf16_lossy(&buf[..n]));
        let mut parent = 0u32;
        // SAFETY: `parent` est un u32 valide en écriture.
        if unsafe { CM_Get_Parent(&mut parent, cur, 0) } != CR_SUCCESS {
            break;
        }
        cur = parent;
    }
    chain
}

fn device_number(path: &[u16]) -> Result<u32> {
    let h = open_path(path, 0)?; // aucun droit demandé : simple interrogation
    let mut num: STORAGE_DEVICE_NUMBER = unsafe { zeroed() }; // SAFETY: structure de nombres entiers, zéro valide.
    let mut ret = 0u32;
    // SAFETY: sortie = `num` (taille exacte), pas d'entrée, pas d'OVERLAPPED.
    let ok = unsafe {
        DeviceIoControl(h.0, IOCTL_STORAGE_GET_DEVICE_NUMBER, null(), 0, (&mut num as *mut STORAGE_DEVICE_NUMBER).cast(), size_of::<STORAGE_DEVICE_NUMBER>() as u32, &mut ret, null_mut())
    };
    if ok == 0 {
        bail!("Numéro de lecteur illisible.");
    }
    Ok(num.DeviceNumber)
}

/// Lecteurs physiques dont un ancêtre USB porte `VID_xxxx&PID_xxxx`.
pub fn list_drives(vid: u16, pid: u16) -> Result<Vec<DriveRef>> {
    let needle = format!("VID_{vid:04X}&PID_{pid:04X}");
    // SAFETY: GUID statique valide ; les autres arguments sont nuls.
    let set = unsafe { SetupDiGetClassDevsW(&GUID_DEVINTERFACE_DISK, null(), null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE) };
    if set == INVALID_HANDLE_VALUE as isize {
        bail!("Énumération des disques Windows impossible.");
    }
    let mut out = Vec::new();
    let mut index = 0u32;
    loop {
        let mut ifd: SP_DEVICE_INTERFACE_DATA = unsafe { zeroed() }; // SAFETY: structure C de nombres, zéro valide.
        ifd.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
        // SAFETY: `set` valide, `ifd.cbSize` renseigné.
        if unsafe { SetupDiEnumDeviceInterfaces(set, null(), &GUID_DEVINTERFACE_DISK, index, &mut ifd) } == 0 {
            break; // plus d'interface (ERROR_NO_MORE_ITEMS)
        }
        index += 1;
        if let Some(d) = inspect(set, &ifd, &needle) {
            out.push(d);
        }
    }
    // SAFETY: `set` a été obtenu de SetupDiGetClassDevsW et n'est plus utilisé.
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    Ok(out)
}

/// Un disque de l'énumération : correspond-il au VID/PID ? Si oui, son numéro de lecteur.
fn inspect(set: HDEVINFO, ifd: &SP_DEVICE_INTERFACE_DATA, needle: &str) -> Option<DriveRef> {
    let mut needed = 0u32;
    // SAFETY: appel de dimensionnement (tampon nul, taille 0) ; l'échec attendu renseigne `needed`.
    unsafe { SetupDiGetDeviceInterfaceDetailW(set, ifd, null_mut(), 0, &mut needed, null_mut()) };
    if (needed as usize) < size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() || needed > 4096 {
        return None;
    }
    // Tampon aligné sur 4 octets, taille arrondie.
    let mut buf = vec![0u32; (needed as usize).div_ceil(4) + 1];
    let detail = buf.as_mut_ptr().cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
    let mut devinfo: SP_DEVINFO_DATA = unsafe { zeroed() }; // SAFETY: structure C de nombres, zéro valide.
    devinfo.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
    // SAFETY: `detail` pointe un tampon d'au moins `needed` octets, aligné ; cbSize = taille de la structure fixe.
    let ok = unsafe {
        (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        SetupDiGetDeviceInterfaceDetailW(set, ifd, detail, needed, &mut needed, &mut devinfo)
    };
    if ok == 0 {
        return None;
    }
    let path_off = offset_of!(SP_DEVICE_INTERFACE_DETAIL_DATA_W, DevicePath);
    let max_u16 = (buf.len() * 4 - path_off) / 2;
    // SAFETY: DevicePath démarre à `path_off` dans le tampon et s'étend jusqu'à son NUL (borné par `max_u16`).
    let path: Vec<u16> = unsafe {
        let p = detail.cast::<u8>().add(path_off).cast::<u16>();
        let mut v = Vec::new();
        for i in 0..max_u16 {
            let c = *p.add(i);
            v.push(c);
            if c == 0 {
                break;
            }
        }
        v
    };
    if path.last() != Some(&0) {
        return None;
    }
    let chain = instance_chain(devinfo.DevInst);
    let usb = chain.iter().find(|id| id.to_ascii_uppercase().contains(needle))?;
    let number = device_number(&path).ok()?;
    let serial = usb.rsplit('\\').next().unwrap_or("").to_ascii_lowercase();
    Some(DriveRef { number, serial })
}

#[repr(C)]
struct SptdWithSense {
    sptd: SCSI_PASS_THROUGH_DIRECT,
    sense: [u8; 32],
}

/// Bloc de 512 octets aligné, pour les tampons de données (contrainte d'alignement des pilotes de stockage).
#[repr(C, align(512))]
#[derive(Clone, Copy)]
struct Block([u8; 512]);

/// Lecteur physique ouvert pour le pass-through.
pub struct WinScsi {
    handle: OwnedHandle,
    timeout_s: u32,
}

impl WinScsi {
    pub fn open(number: u32) -> Result<Self> {
        let path = wide(&format!(r"\\.\PhysicalDrive{number}"));
        Ok(WinScsi { handle: open_path(&path, GENERIC_READ | GENERIC_WRITE)?, timeout_s: 10 })
    }

    fn io(&mut self, cdb: &[u8; 16], data_in: bool, buf: &mut [Block], len: usize) -> Result<()> {
        // SAFETY: structure C de nombres/pointeurs, zéro valide ; remplie ci-dessous.
        let mut s: SptdWithSense = unsafe { zeroed() };
        s.sptd.Length = size_of::<SCSI_PASS_THROUGH_DIRECT>() as u16;
        s.sptd.CdbLength = 16;
        s.sptd.SenseInfoLength = s.sense.len() as u8;
        s.sptd.DataIn = if data_in { SCSI_IOCTL_DATA_IN as u8 } else { SCSI_IOCTL_DATA_OUT as u8 };
        s.sptd.DataTransferLength = len as u32;
        s.sptd.TimeOutValue = self.timeout_s;
        s.sptd.DataBuffer = buf.as_mut_ptr().cast();
        s.sptd.SenseInfoOffset = offset_of!(SptdWithSense, sense) as u32;
        s.sptd.Cdb = *cdb;
        let size = size_of::<SptdWithSense>() as u32;
        let mut ret = 0u32;
        // SAFETY: entrée/sortie = `s` (taille exacte) ; `DataBuffer` pointe `buf`, qui couvre `len` octets
        // et reste vivant pendant l'appel synchrone (pas d'OVERLAPPED).
        let ok = unsafe {
            DeviceIoControl(self.handle.0, IOCTL_SCSI_PASS_THROUGH_DIRECT, (&s as *const SptdWithSense).cast(), size, (&mut s as *mut SptdWithSense).cast(), size, &mut ret, null_mut())
        };
        if ok == 0 {
            // SAFETY: lecture du dernier code d'erreur du thread courant.
            let code = unsafe { GetLastError() };
            return Err(anyhow!("Commande SCSI échouée (erreur Windows {code})."));
        }
        if s.sptd.ScsiStatus != 0 {
            bail!("L'écran a refusé la commande SCSI (statut {:#04X}).", s.sptd.ScsiStatus);
        }
        Ok(())
    }
}

fn blocks_for(len: usize) -> Result<Vec<Block>> {
    if len > 1 << 20 {
        bail!("Transfert SCSI trop gros : {len} octets.");
    }
    Ok(vec![Block([0; 512]); len.div_ceil(512).max(1)])
}

impl ScsiTransport for WinScsi {
    fn read_cdb(&mut self, cdb: &[u8; 16], len: usize) -> Result<Vec<u8>> {
        let mut blocks = blocks_for(len)?;
        self.io(cdb, true, &mut blocks, len)?;
        // SAFETY: `Block` est repr(C) sans remplissage ; la tranche couvre exactement les blocs alloués.
        let bytes = unsafe { std::slice::from_raw_parts(blocks.as_ptr().cast::<u8>(), blocks.len() * 512) };
        Ok(bytes[..len].to_vec())
    }

    fn write_cdb(&mut self, cdb: &[u8; 16], data: &[u8]) -> Result<()> {
        let mut blocks = blocks_for(data.len())?;
        // SAFETY: même tranche que ci-dessus, en écriture ; `data.len() <= blocks.len() * 512`.
        let bytes = unsafe { std::slice::from_raw_parts_mut(blocks.as_mut_ptr().cast::<u8>(), blocks.len() * 512) };
        bytes[..data.len()].copy_from_slice(data);
        self.io(cdb, false, &mut blocks, data.len())
    }
}
