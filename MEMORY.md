
## wry WebView Research (2026-03-17)

**Focus:** Replace Fast English popup (iced 0.13) with wry for HTML/CSS/JS rendering.

**Key Findings:**
- **wry 0.54.3 + tao 0.34.6** fully supports transparent, frameless, always-on-top windows on Windows 8+
- **IPC pattern:** JS `window.ipc.postMessage()` → Rust `ipc_handler` closure. For reverse: `evaluate_script()` with custom events.
- **tokio integration:** Works via separate thread for tao EventLoop + Arc<Mutex> for shared state + EventLoopProxy for signaling
- **Window control:** Full via tao API (positioning, sizing, visibility, z-order)
- **No Tauri needed:** wry standalone is 15-20MB vs Tauri 40-60MB
- **Gotchas:** W7 transparent not supported, IPC handler not Send/Sync (use blocking_send), WebView lifecycle must be managed

**Architecture:** Separate UI thread with tao/wry, async event loop in tokio main thread, shared state via Arc<Mutex>.

**Unresolved:** Custom fonts bundling, SVG animation performance, DPI scaling, clipboard from JS, WebView2 min version.

**Report location:** `/mnt/d/WORKSPACES/PERSONAL/fast-eng/plans/reports/wry-research-findings.md`
