//! Source : docs/rgb-protocols/smbus-i2c-acces-et-securite.md §2 « Pilotes requis sous Windows » (PawnIO, mutex de coexistence).
//!
//! Accès SMBus réel via `PawnIOLib.dll`, chargée dynamiquement UNIQUEMENT depuis `%ProgramFiles%\PureRGB\openrgb\`
//! (dossier géré, ACL admin) avec les modules `SmbusI801.bin` (Intel) puis `SmbusPIIX4.bin` (AMD).
//! Le mutex `Global\Access_SMBUS.HTP.Method` est pris autour de CHAQUE transfert.
//!
//! INCONNU / hypothèses à valider (la note ne les détaille pas) :
//! - signatures C de la bibliothèque (HANDLE, octets du blob, noms d'ioctl, tailles EN MOTS de 64 bits) : lues d'après l'en-tête
//!   public de PawnIOLib, hors de la note, qui ne donne que les noms de fonctions ;
//! - répartition des données dans les 5 derniers mots d'entrée / 5 mots de sortie : buffer de 40 octets [longueur, données...]
//!   rangé en little-endian (convention [général] des unions SMBus de Linux : 34 octets utiles) ;
//! - codes de taille (2 = octet+commande, 5 = bloc) et sens (1 = lecture, 0 = écriture) : convention [général] Linux i2c-dev ;
//! - disposition des 3 mots de `ioctl_identity` (nom sur 8 octets ; IDs PCI) : seul le nom ASCII du mot 0 est affiché, la liste
//!   blanche PCI de la note n'est PAS appliquée (disposition INCONNUE) ; la signature Corsair reste le garde-fou.
//! Une erreur de ces hypothèses se traduit par un refus (échec ou signature fausse), la détection ne faisant que lire.

use super::bus::{ensure_writable, SmbusBus};
use anyhow::{bail, ensure, Context, Result};
use libloading::Library;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr::null;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

type OpenFn = unsafe extern "system" fn(*mut HANDLE) -> i32;
type LoadFn = unsafe extern "system" fn(HANDLE, *const u8, usize) -> i32;
type ExecFn = unsafe extern "system" fn(HANDLE, *const c_char, *const u64, usize, *mut u64, usize, *mut usize) -> i32;
type CloseFn = unsafe extern "system" fn(HANDLE) -> i32;

const MUTEX_NAME: &str = r"Global\Access_SMBUS.HTP.Method";
const MUTEX_WAIT_MS: u32 = 5000;
const MODULES: [&str; 2] = ["SmbusI801.bin", "SmbusPIIX4.bin"];
const BLOB_MAX: u64 = 1 << 20;
const ATTEMPTS: u64 = 3;
const SIZE_BYTE_DATA: u64 = 2;
const SIZE_BLOCK_DATA: u64 = 5;

pub struct PawnioBus {
    _lib: Library,
    exec: ExecFn,
    close: CloseFn,
    handle: HANDLE,
    mutex: HANDLE,
    /// Nom du contrôleur annoncé par `ioctl_identity` (diagnostic).
    pub controller: String,
}

// SAFETY: les HANDLE ne sont utilisés que depuis le thread qui détient le `&mut PawnioBus` (le backend est derrière un Mutex).
unsafe impl Send for PawnioBus {}

struct MutexGuard(HANDLE);
impl Drop for MutexGuard {
    fn drop(&mut self) {
        // SAFETY: le mutex a été acquis par ce thread dans `lock`.
        unsafe { ReleaseMutex(self.0) };
    }
}

fn hr(code: i32, what: &str) -> Result<()> {
    ensure!(code >= 0, "{what} a échoué (HRESULT 0x{:08X})", code as u32);
    Ok(())
}

impl PawnioBus {
    /// Ouvre PawnIO et charge le premier module qui s'accepte (Intel I801 puis AMD PIIX4).
    pub fn open() -> Result<Self> {
        let dir: PathBuf = crate::trust::managed_dir("openrgb").context("ProgramFiles introuvable")?;
        ensure!(crate::trust::is_trusted(&dir), "dossier {} hors de %ProgramFiles%", dir.display());
        let dll = dir.join("PawnIOLib.dll");
        ensure!(dll.is_file(), "PawnIOLib.dll introuvable dans {}", dir.display());
        // SAFETY: chargement d'une DLL signée depuis un dossier à ACL admin ; les symboles sont typés d'après PawnIOLib.h.
        let (lib, open, load, exec, close) = unsafe {
            let lib = Library::new(&dll).context("chargement de PawnIOLib.dll")?;
            let open = *lib.get::<OpenFn>(b"pawnio_open\0")?;
            let load = *lib.get::<LoadFn>(b"pawnio_load\0")?;
            let exec = *lib.get::<ExecFn>(b"pawnio_execute\0")?;
            let close = *lib.get::<CloseFn>(b"pawnio_close\0")?;
            (lib, open, load, exec, close)
        };
        let mutex = Self::create_mutex()?;
        let mut last = String::from("aucun module SMBus");
        for module in MODULES {
            match Self::try_module(&dir, module, open, load, close) {
                Ok(handle) => {
                    let mut bus = PawnioBus { _lib: lib, exec, close, handle, mutex, controller: String::new() };
                    bus.controller = bus.identity().context("ioctl_identity")?;
                    log::info!("smbus : module {module}, contrôleur « {} »", bus.controller);
                    return Ok(bus);
                }
                Err(e) => last = format!("{module} : {e:#}"),
            }
        }
        // SAFETY: handle créé plus haut et jamais partagé.
        unsafe { CloseHandle(mutex) };
        bail!("PawnIO : {last}")
    }

    fn create_mutex() -> Result<HANDLE> {
        let name: Vec<u16> = MUTEX_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: nom UTF-16 terminé par 0, attributs par défaut.
        let h = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
        ensure!(!h.is_null(), "création du mutex {MUTEX_NAME} impossible");
        Ok(h)
    }

    fn try_module(dir: &std::path::Path, module: &str, open: OpenFn, load: LoadFn, close: CloseFn) -> Result<HANDLE> {
        let path = dir.join(module);
        let len = std::fs::metadata(&path).with_context(|| format!("{} introuvable", path.display()))?.len();
        ensure!(len > 0 && len <= BLOB_MAX, "taille de module invalide ({len} octets)");
        let blob = std::fs::read(&path)?;
        let mut handle: HANDLE = std::ptr::null_mut::<c_void>();
        // SAFETY: pointeurs valides pour la durée des appels.
        unsafe {
            hr(open(&mut handle), "pawnio_open")?;
            if let Err(e) = hr(load(handle, blob.as_ptr(), blob.len()), "pawnio_load") {
                close(handle);
                return Err(e);
            }
        }
        Ok(handle)
    }

    fn call(&self, name: &str, input: &[u64], out: &mut [u64]) -> Result<()> {
        let cname = CString::new(name)?;
        let mut returned = 0usize;
        // SAFETY: tailles passées en nombre de mots de 64 bits, tampons valides pendant l'appel.
        let code = unsafe { (self.exec)(self.handle, cname.as_ptr(), input.as_ptr(), input.len(), out.as_mut_ptr(), out.len(), &mut returned) };
        hr(code, name)
    }

    fn identity(&self) -> Result<String> {
        let mut out = [0u64; 3];
        self.call("ioctl_identity", &[], &mut out)?;
        let bytes = out[0].to_le_bytes();
        let name: String = bytes.iter().take_while(|b| **b != 0).filter(|b| b.is_ascii_graphic()).map(|b| *b as char).collect();
        Ok(name)
    }

    fn lock(&self) -> Result<MutexGuard> {
        // SAFETY: handle de mutex valide tant que `self` vit.
        match unsafe { WaitForSingleObject(self.mutex, MUTEX_WAIT_MS) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(MutexGuard(self.mutex)),
            _ => bail!("mutex SMBus global occupé (autre logiciel sur le bus)"),
        }
    }

    /// Un transfert complet sous mutex, avec réessais bornés (3 x tentative x 10 ms de recul).
    fn xfer(&mut self, addr: u8, read: bool, cmd: u8, size: u64, buf: &[u8]) -> Result<[u64; 5]> {
        ensure!(buf.len() <= 34, "données SMBus trop longues");
        let mut data = [0u8; 40];
        data[..buf.len()].copy_from_slice(buf);
        let mut input = [0u64; 9];
        input[..4].copy_from_slice(&[addr as u64, read as u64, cmd as u64, size]);
        for (w, chunk) in input[4..].iter_mut().zip(data.chunks_exact(8)) {
            *w = u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8]));
        }
        let mut last = None;
        for attempt in 1..=ATTEMPTS {
            let guard = self.lock()?;
            let mut out = [0u64; 5];
            let res = self.call("ioctl_smbus_xfer", &input, &mut out);
            drop(guard);
            match res {
                Ok(()) => return Ok(out),
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(3 * attempt * 10));
                }
            }
        }
        Err(last.unwrap_or_else(|| anyhow::anyhow!("transfert SMBus échoué")))
    }
}

impl SmbusBus for PawnioBus {
    fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8> {
        Ok((self.xfer(addr, true, reg, SIZE_BYTE_DATA, &[])?[0] & 0xFF) as u8)
    }

    fn write_byte_data(&mut self, addr: u8, reg: u8, value: u8) -> Result<()> {
        ensure_writable(addr)?;
        log::debug!("smbus écriture 0x{addr:02X} reg 0x{reg:02X} = 0x{value:02X}");
        self.xfer(addr, false, reg, SIZE_BYTE_DATA, &[value]).map(drop)
    }

    fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
        ensure_writable(addr)?;
        ensure!(!data.is_empty() && data.len() <= 32, "bloc SMBus de {} octets invalide", data.len());
        log::debug!("smbus bloc 0x{addr:02X} reg 0x{reg:02X} : {} octets", data.len());
        let mut buf = Vec::with_capacity(data.len() + 1);
        buf.push(data.len() as u8);
        buf.extend_from_slice(data);
        self.xfer(addr, false, reg, SIZE_BLOCK_DATA, &buf).map(drop)
    }
}

impl Drop for PawnioBus {
    fn drop(&mut self) {
        // SAFETY: handles créés dans `open`, libérés une seule fois ; la DLL (`_lib`) est encore chargée.
        unsafe {
            (self.close)(self.handle);
            CloseHandle(self.mutex);
        }
    }
}
