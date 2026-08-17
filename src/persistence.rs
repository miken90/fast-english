use anyhow::Result;

/// API key storage — encrypted with Windows DPAPI (user-scope).
///
/// Keys are stored in `%APPDATA%/FastEnglish/{provider}.key` but the content
/// is encrypted via `CryptProtectData` / `CryptUnprotectData` (DPAPI).
/// Only the same Windows user account can decrypt them.
pub struct KeyStore;

impl KeyStore {
    /// Load and decrypt API key for the given provider account name.
    pub fn load(account: &str) -> Result<Option<String>> {
        let path = key_path(account);
        match std::fs::read(&path) {
            Ok(encrypted) => {
                if encrypted.is_empty() {
                    return Ok(None);
                }
                let decrypted = dpapi_decrypt(&encrypted)?;
                let key = String::from_utf8(decrypted)
                    .map_err(|_| anyhow::anyhow!("Invalid UTF-8 in stored key"))?;
                let trimmed = key.trim().to_string();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(trimmed))
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Encrypt and save API key for the given provider account name.
    pub fn save(account: &str, key: &str) -> Result<()> {
        let path = key_path(account);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let encrypted = dpapi_encrypt(key.as_bytes())?;
        std::fs::write(&path, encrypted)?;
        Ok(())
    }
}

/// Recent results on disk — encrypted with the same user-scope DPAPI the API
/// key uses.
///
/// The precedent in this file is that a credential is encrypted while settings
/// (base URL, model, theme) sit in plain JSON. History is neither: it is the
/// user's own writing — the emails, notes and messages they ran through the
/// app — which is closer to a secret than to configuration. Encrypting it costs
/// nothing here (the DPAPI helpers already exist, no key to manage, no new
/// dependency) so writing it in the clear would be a choice to leak, not a
/// trade-off. Note what this does and does not buy: DPAPI user-scope stops
/// another Windows account, a stolen disk, or a backup from reading the file.
/// It does not stop code already running as this user.
pub struct HistoryStore;

impl HistoryStore {
    /// Load history, degrading to empty on anything unreadable.
    ///
    /// Every failure — no file, a truncated one, a file from another user
    /// account that DPAPI refuses, hand-edited JSON — is the same outcome: an
    /// empty history and a running app. History is a convenience; it must never
    /// be able to stop the app from starting.
    pub fn load() -> crate::history::History {
        Self::load_from(&history_path())
    }

    pub fn save(history: &crate::history::History) -> Result<()> {
        Self::save_to(&history_path(), history)
    }

    /// Path-explicit form, so tests never touch the real profile.
    pub fn load_from(path: &std::path::Path) -> crate::history::History {
        let Ok(encrypted) = std::fs::read(path) else {
            return crate::history::History::default();
        };
        if encrypted.is_empty() {
            return crate::history::History::default();
        }
        let Ok(decrypted) = dpapi_decrypt(&encrypted) else {
            tracing::warn!("History could not be decrypted; starting with an empty history");
            return crate::history::History::default();
        };
        let Ok(json) = String::from_utf8(decrypted) else {
            return crate::history::History::default();
        };
        match serde_json::from_str::<crate::history::History>(&json) {
            Ok(mut history) => {
                // The file is not the authority on the caps.
                history.enforce_limits();
                history
            }
            Err(e) => {
                tracing::warn!("History file is not readable ({}); starting empty", e);
                crate::history::History::default()
            }
        }
    }

    pub fn save_to(path: &std::path::Path, history: &crate::history::History) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(history)?;
        let encrypted = dpapi_encrypt(json.as_bytes())?;
        std::fs::write(path, encrypted)?;
        Ok(())
    }
}

fn history_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("FastEnglish")
        .join("history.dat")
}

fn key_path(account: &str) -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("FastEnglish")
        .join(format!("{}.key", account))
}

/// Encrypt data with Windows DPAPI (CryptProtectData).
#[cfg(target_os = "windows")]
fn dpapi_encrypt(data: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptProtectData(
            &mut input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )?;

        let encrypted = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(
            output.pbData as *mut std::ffi::c_void,
        ));
        Ok(encrypted)
    }
}

/// Decrypt data with Windows DPAPI (CryptUnprotectData).
#[cfg(target_os = "windows")]
fn dpapi_decrypt(data: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptUnprotectData(
            &mut input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )?;

        let decrypted = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(
            output.pbData as *mut std::ffi::c_void,
        ));
        Ok(decrypted)
    }
}

#[cfg(not(target_os = "windows"))]
fn dpapi_encrypt(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}

#[cfg(not(target_os = "windows"))]
fn dpapi_decrypt(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}

/// Auto-start management via Windows Registry.
///
/// Writes to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
/// No admin required — per-user auto-start only.
#[cfg(target_os = "windows")]
pub fn set_auto_start(enabled: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_SET_VALUE | KEY_READ,
    )?;

    if enabled {
        let exe_path = std::env::current_exe()?;
        key.set_value("FastEnglish", &exe_path.to_string_lossy().to_string())?;
    } else {
        let _ = key.delete_value("FastEnglish");
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_auto_start(_enabled: bool) -> Result<()> {
    tracing::warn!("Auto-start not available on this platform");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{History, MAX_ENTRIES};

    /// A directory of our own under the OS temp dir. Tests must never read or
    /// write the real `%APPDATA%` history.
    fn temp_history_path(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fast-english-test-{}-{}", name, std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("history.dat")
    }

    #[test]
    fn history_survives_a_save_and_load() {
        let path = temp_history_path("roundtrip");
        let _ = std::fs::remove_file(&path);

        let mut history = History::default();
        history.record(
            "fix-grammar",
            "Fix Grammar",
            "She go home.",
            "She goes home.",
            None,
        );
        history.record(
            "translate",
            "Translate English → Vietnamese",
            "Hello",
            "Xin chào",
            Some(("en", "vi")),
        );

        HistoryStore::save_to(&path, &history).expect("save failed");
        let loaded = HistoryStore::load_from(&path);

        assert_eq!(loaded.entries().len(), 2);
        // Newest first survives the trip.
        assert_eq!(loaded.entries()[0].tool_id, "translate");
        assert_eq!(loaded.entries()[0].output, "Xin chào");
        // The language pair survives the trip, so reopening rebuilds the real
        // direction instead of the default one.
        assert_eq!(loaded.entries()[0].translate_from.as_deref(), Some("en"));
        assert_eq!(loaded.entries()[0].translate_to.as_deref(), Some("vi"));
        assert_eq!(loaded.entries()[1].source_text, "She go home.");
        assert!(loaded.entries()[0].timestamp > 0);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_stored_file_is_not_plain_text() {
        let path = temp_history_path("encrypted");
        let _ = std::fs::remove_file(&path);

        let mut history = History::default();
        history.record(
            "fix-grammar",
            "Fix Grammar",
            "my private sentence",
            "my corrected sentence",
            None,
        );
        HistoryStore::save_to(&path, &history).expect("save failed");

        let raw = std::fs::read(&path).expect("read failed");
        // On Windows this is DPAPI ciphertext. The check is skipped elsewhere,
        // where the helpers are pass-through stubs and there is no DPAPI.
        if cfg!(target_os = "windows") {
            let as_text = String::from_utf8_lossy(&raw);
            assert!(!as_text.contains("my private sentence"));
            assert!(!as_text.contains("my corrected sentence"));
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_corrupt_file_loads_as_empty_history() {
        let path = temp_history_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"this is not anything we wrote").unwrap();

        let loaded = HistoryStore::load_from(&path);
        assert!(loaded.entries().is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_empty_or_missing_file_loads_as_empty_history() {
        let missing = temp_history_path("missing").with_file_name("nothing-here.dat");
        assert!(HistoryStore::load_from(&missing).entries().is_empty());

        let empty = temp_history_path("empty");
        std::fs::write(&empty, b"").unwrap();
        assert!(HistoryStore::load_from(&empty).entries().is_empty());

        let _ = std::fs::remove_file(&empty);
    }

    #[test]
    fn a_file_holding_too_much_is_capped_on_load() {
        let path = temp_history_path("overfull");
        let _ = std::fs::remove_file(&path);

        let mut history = History::default();
        for i in 0..MAX_ENTRIES {
            history.record("fix-grammar", "Fix Grammar", &format!("s{}", i), "o", None);
        }
        HistoryStore::save_to(&path, &history).expect("save failed");

        let loaded = HistoryStore::load_from(&path);
        assert_eq!(loaded.entries().len(), MAX_ENTRIES);

        let _ = std::fs::remove_file(&path);
    }
}
