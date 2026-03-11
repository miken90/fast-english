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
