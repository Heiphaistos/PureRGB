//! Job Windows « tuer à la fermeture » : tout processus lancé par PureRGB (OpenRGB, sensord,
//! liquidctl, USBPcap, WebView2…) meurt avec lui, même en cas de plantage ou de fermeture
//! forcée depuis le gestionnaire de tâches. Aucun processus ne reste en arrière-plan.

#[cfg(windows)]
mod imp {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    /// Job configuré pour tuer ses processus à la fermeture de son dernier handle.
    pub struct KillJob(HANDLE);

    // Le handle n'est qu'un identifiant noyau, utilisable depuis n'importe quel thread.
    unsafe impl Send for KillJob {}
    unsafe impl Sync for KillJob {}

    impl KillJob {
        pub fn new() -> Result<KillJob, String> {
            // SAFETY: appels Win32 sans pointeur externe ; la structure est initialisée à zéro (valide).
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Err(format!("CreateJobObject: erreur {}", std::io::Error::last_os_error()));
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    let e = std::io::Error::last_os_error();
                    CloseHandle(job);
                    return Err(format!("SetInformationJobObject: {e}"));
                }
                Ok(KillJob(job))
            }
        }

        /// Ajoute un processus (handle Win32) au job.
        pub fn assign_raw(&self, process: HANDLE) -> Result<(), String> {
            // SAFETY: `self.0` est un job valide ; `process` est fourni par l'appelant.
            if unsafe { AssignProcessToJobObject(self.0, process) } == 0 {
                return Err(format!("AssignProcessToJobObject: {}", std::io::Error::last_os_error()));
            }
            Ok(())
        }

        pub fn assign_self(&self) -> Result<(), String> {
            // SAFETY: GetCurrentProcess renvoie un pseudo-handle toujours valide.
            self.assign_raw(unsafe { GetCurrentProcess() })
        }
    }

    impl Drop for KillJob {
        fn drop(&mut self) {
            // SAFETY: handle créé par `new`, fermé une seule fois ici.
            unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(windows)]
pub use imp::KillJob;

/// Place le processus courant dans un job « tuer à la fermeture » qu'il garde ouvert jusqu'à sa mort.
/// Les enfants lancés ensuite en héritent. Un échec n'est pas fatal (journalisé).
#[cfg(windows)]
pub fn kill_children_on_exit() {
    static JOB: std::sync::OnceLock<KillJob> = std::sync::OnceLock::new();
    match KillJob::new().and_then(|j| j.assign_self().map(|_| j)) {
        Ok(job) => {
            let _ = JOB.set(job);
        }
        Err(e) => log::warn!("job de fermeture indisponible : {e}"),
    }
}

#[cfg(not(windows))]
pub fn kill_children_on_exit() {}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    #[test]
    fn closing_the_job_kills_the_child_process() {
        let mut child = Command::new("ping")
            .args(["-n", "120", "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .expect("ping disponible");
        let job = KillJob::new().expect("job");
        job.assign_raw(child.as_raw_handle() as _).expect("assignation");
        assert!(child.try_wait().expect("état").is_none(), "le fils doit tourner avant la fermeture");
        drop(job);
        let t = Instant::now();
        while child.try_wait().expect("état").is_none() {
            assert!(t.elapsed() < Duration::from_secs(5), "le fils a survécu à la fermeture du job");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
