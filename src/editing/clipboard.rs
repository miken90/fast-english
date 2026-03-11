use anyhow::Result;

/// Clipboard backup for save/restore operations.
pub struct ClipboardBackup {
    text: Option<String>,
}

/// Set text to clipboard.
pub fn set_text(text: &str) -> Result<()> {
    let mut clip = arboard::Clipboard::new()?;
    clip.set_text(text)?;
    Ok(())
}

/// Backup current clipboard content.
pub fn backup() -> ClipboardBackup {
    let text = arboard::Clipboard::new()
        .ok()
        .and_then(|mut c| c.get_text().ok());
    ClipboardBackup { text }
}

/// Restore clipboard from backup.
pub fn restore(backup: ClipboardBackup) -> Result<()> {
    if let Some(text) = backup.text {
        let mut clip = arboard::Clipboard::new()?;
        clip.set_text(&text)?;
    }
    Ok(())
}
