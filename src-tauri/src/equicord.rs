use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use log::{info, warn};

const BUNDLED_EQUICORD_JS: &str = include_str!("../resources/equicord.js");
const BUNDLED_EQUICORD_CSS: &str = include_str!("../resources/equicord.css");

const EQUICORD_JS_URL: &str = "https://github.com/Equicord/Equicord/releases/latest/download/browser.js";
const EQUICORD_CSS_URL: &str = "https://github.com/Equicord/Equicord/releases/latest/download/browser.css";

pub fn get_equicord_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("com.rustcord.app").join("equicord")
}

pub fn load_equicord_bundle() -> (String, String) {
    let dir = get_equicord_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        warn!("[Rustcord] Failed to create equicord dir: {}", e);
    }

    let js_path = dir.join("equicord.js");
    let css_path = dir.join("equicord.css");

    // Initialize with bundled fallback if not yet present
    if !js_path.exists() {
        let _ = fs::write(&js_path, BUNDLED_EQUICORD_JS);
    }
    if !css_path.exists() {
        let _ = fs::write(&css_path, BUNDLED_EQUICORD_CSS);
    }

    // Attempt online update check with timeout
    try_update_equicord_files(&js_path, &css_path);

    let js_content = fs::read_to_string(&js_path).unwrap_or_else(|_| BUNDLED_EQUICORD_JS.to_string());
    let css_content = fs::read_to_string(&css_path).unwrap_or_else(|_| BUNDLED_EQUICORD_CSS.to_string());

    info!(
        "[Rustcord] Equicord bundle loaded (JS: {} bytes, CSS: {} bytes)",
        js_content.len(),
        css_content.len()
    );

    (js_content, css_content)
}

fn try_update_equicord_files(js_path: &PathBuf, css_path: &PathBuf) {
    info!("[Rustcord] Checking for Equicord updates from GitHub releases...");
    let client = match reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Rustcord/1.0")
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("[Rustcord] Failed to create HTTP client: {}", e);
            return;
        }
    };

    // Download latest JS if available
    match client.get(EQUICORD_JS_URL).send() {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(text) = resp.text() {
                if text.len() > 100_000 {
                    let _ = fs::write(js_path, text);
                    info!("[Rustcord] Equicord JS updated successfully.");
                }
            }
        }
        Ok(resp) => {
            info!("[Rustcord] Equicord JS check returned status: {}", resp.status());
        }
        Err(err) => {
            info!("[Rustcord] Equicord update check skipped or timed out: {}", err);
        }
    }

    // Download latest CSS if available
    match client.get(EQUICORD_CSS_URL).send() {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(text) = resp.text() {
                if text.len() > 10_000 {
                    let _ = fs::write(css_path, text);
                    info!("[Rustcord] Equicord CSS updated successfully.");
                }
            }
        }
        _ => {}
    }
}

pub fn build_initialization_script(equicord_js: &str, equicord_css: &str) -> String {
    let css_json = serde_json::to_string(equicord_css).unwrap_or_else(|_| "\"\"".to_string());

    let template = r###"
// === [Rustcord Core Injection Layer v4] ===
(function() {
    'use strict';
    console.log('[Rustcord] Initializing pre-load hooks v4...');

    // 1. Safe invoke helper
    const invoke = (cmd, args) => {
        try {
            const t = window.__TAURI_INTERNALS__;
            if (t && t.invoke) return t.invoke(cmd, args || {});
        } catch(e) {}
        return Promise.reject(new Error('Tauri IPC unavailable: ' + cmd));
    };

    // 2. Window control actions
    const windowMinimize       = () => invoke('plugin:window|minimize').catch(() => invoke('window_minimize'));
    const windowToggleMaximize = () => invoke('plugin:window|internal_toggle_maximize').catch(() => invoke('window_toggle_maximize'));
    const windowClose          = () => invoke('plugin:window|hide').catch(() => invoke('window_close'));
    const windowStartDragging  = () => invoke('plugin:window|start_dragging').catch(() => invoke('window_start_dragging'));
    const windowIsMaximized    = () => invoke('plugin:window|is_maximized').catch(() => invoke('window_is_maximized')).catch(() => false);
    const appReady             = () => invoke('app_ready');

    // F12 / Ctrl+Shift+I to open DevTools
    window.addEventListener('keydown', (e) => {
        if (e.key === 'F12' || (e.ctrlKey && e.shiftKey && (e.key === 'I' || e.key === 'i'))) {
            invoke('open_devtools');
        }
    });

    // === [Rustcord Telegram-Style Notification Engine & Polyfill] ===
    const recentlySeenMsgIds = new Set();

    function escapeHtml(str) {
        if (!str) return '';
        return String(str)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#39;');
    }

    function playTelegramChime() {
        try {
            const AudioCtx = window.AudioContext || window.webkitAudioContext;
            if (!AudioCtx) return;
            const ctx = new AudioCtx();
            if (ctx.state === 'suspended') {
                ctx.resume().catch(() => {});
            }
            const now = ctx.currentTime;

            // Soft 2-tone melodic chime (D5 -> A5)
            const osc1 = ctx.createOscillator();
            const gain1 = ctx.createGain();
            osc1.type = 'sine';
            osc1.frequency.setValueAtTime(587.33, now);
            gain1.gain.setValueAtTime(0.18, now);
            gain1.gain.exponentialRampToValueAtTime(0.001, now + 0.12);
            osc1.connect(gain1);
            gain1.connect(ctx.destination);
            osc1.start(now);
            osc1.stop(now + 0.12);

            const osc2 = ctx.createOscillator();
            const gain2 = ctx.createGain();
            osc2.type = 'sine';
            osc2.frequency.setValueAtTime(880.0, now + 0.08);
            gain2.gain.setValueAtTime(0.22, now + 0.08);
            gain2.gain.exponentialRampToValueAtTime(0.0001, now + 0.28);
            osc2.connect(gain2);
            gain2.connect(ctx.destination);
            osc2.start(now + 0.08);
            osc2.stop(now + 0.28);
        } catch(e) {}
    }

    function showTelegramToast(opts) {
        try {
            // 1. Trigger Desktop Floating Notification Window (visible outside app over games/desktop/browser)
            invoke('show_desktop_notification', {
                payload: {
                    title: opts.author || 'Discord',
                    title_html: opts.titleHtml || null,
                    body: opts.text || '',
                    avatar: opts.avatar || 'https://cdn.discordapp.com/embed/avatars/0.png',
                    guild_id: opts.guildId || null,
                    channel_id: opts.channelId || ''
                }
            }).catch(() => {});

            // 2. In-App Notification Toast inside Discord
            let container = document.getElementById('rustcord-notifications-container');
            if (!container) {
                container = document.createElement('div');
                container.id = 'rustcord-notifications-container';
                (document.body || document.documentElement).appendChild(container);
            }

            // Keep max 4 visible notifications
            while (container.children.length >= 4) {
                const oldest = container.lastElementChild;
                if (oldest) oldest.remove();
            }

            playTelegramChime();

            const toast = document.createElement('div');
            toast.className = 'rc-tg-toast';

            const authorText = opts.author || 'Discord';
            const titleHtml = opts.titleHtml || escapeHtml(authorText);
            const msgText = opts.text || '';
            const avatarUrl = opts.avatar || 'https://cdn.discordapp.com/embed/avatars/0.png';

            toast.innerHTML = `
                <img class="rc-tg-avatar" src="${escapeHtml(avatarUrl)}" onerror="this.src='https://cdn.discordapp.com/embed/avatars/0.png'" alt="" />
                <div class="rc-tg-body-col">
                    <div class="rc-tg-top-row">
                        <span class="rc-tg-title">${titleHtml}</span>
                        <button class="rc-tg-close-btn" title="Закрыть">
                            <svg width="10" height="10" viewBox="0 0 10 10">
                                <polygon fill="currentColor" points="10 0.7 9.3 0 5 4.3 0.7 0 0 0.7 4.3 5 0 9.3 0.7 10 5 5.7 9.3 10 10 9.3 5.7 5"/>
                            </svg>
                        </button>
                    </div>
                    <div class="rc-tg-text">${escapeHtml(msgText)}</div>
                </div>
            `;

            let isDismissed = false;
            let autoDismissTimer = null;

            const dismiss = () => {
                if (isDismissed) return;
                isDismissed = true;
                clearTimeout(autoDismissTimer);
                toast.classList.remove('rc-tg-visible');
                setTimeout(() => {
                    if (toast.parentNode) toast.parentNode.removeChild(toast);
                }, 250);
            };

            const startDismissTimer = (ms = 6500) => {
                clearTimeout(autoDismissTimer);
                autoDismissTimer = setTimeout(dismiss, ms);
            };

            const closeBtn = toast.querySelector('.rc-tg-close-btn');
            if (closeBtn) {
                closeBtn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    e.preventDefault();
                    dismiss();
                    if (typeof opts.onClose === 'function') opts.onClose();
                });
            }

            toast.addEventListener('click', (e) => {
                dismiss();
                if (typeof opts.onClick === 'function') {
                    opts.onClick();
                } else {
                    invoke('window_reveal').catch(() => {});
                }
            });

            toast.addEventListener('mouseenter', () => clearTimeout(autoDismissTimer));
            toast.addEventListener('mouseleave', () => startDismissTimer(3500));

            container.insertBefore(toast, container.firstChild);

            requestAnimationFrame(() => {
                toast.classList.add('rc-tg-visible');
            });

            startDismissTimer(6500);
        } catch(err) {
            console.warn('[Rustcord] showTelegramToast error:', err);
        }
    }

    // Polyfill window.Notification with permission = 'granted'
    class RustcordNotification extends EventTarget {
        static get permission() {
            return 'granted';
        }
        static requestPermission(cb) {
            if (typeof cb === 'function') cb('granted');
            return Promise.resolve('granted');
        }
        constructor(title, options = {}) {
            super();
            this.title = title || 'Discord';
            this.body = options.body || '';
            this.icon = options.icon || '';
            this.tag = options.tag || '';
            this.data = options.data || null;

            const dedupeKey = 'notif_' + this.title + '_' + this.body;
            if (!recentlySeenMsgIds.has(dedupeKey)) {
                recentlySeenMsgIds.add(dedupeKey);
                setTimeout(() => recentlySeenMsgIds.delete(dedupeKey), 4000);

                showTelegramToast({
                    context: 'Discord',
                    author: this.title,
                    text: this.body || 'Новое уведомление',
                    avatar: this.icon || 'https://cdn.discordapp.com/embed/avatars/0.png',
                    onClick: () => {
                        invoke('window_reveal').catch(() => {});
                        if (typeof this.onclick === 'function') this.onclick(new Event('click'));
                        this.dispatchEvent(new Event('click'));
                    },
                    onClose: () => {
                        if (typeof this.onclose === 'function') this.onclose(new Event('close'));
                        this.dispatchEvent(new Event('close'));
                    }
                });

                if (typeof document.hasFocus === 'function' && !document.hasFocus()) {
                    invoke('notify_attention').catch(() => {});
                }
            }

            setTimeout(() => {
                if (typeof this.onshow === 'function') this.onshow(new Event('show'));
                this.dispatchEvent(new Event('show'));
            }, 30);
        }
        close() {
            if (typeof this.onclose === 'function') this.onclose(new Event('close'));
            this.dispatchEvent(new Event('close'));
        }
    }

    try {
        Object.defineProperty(window, 'Notification', {
            value: RustcordNotification,
            writable: false,
            configurable: true
        });
    } catch(e) {
        window.Notification = RustcordNotification;
    }

    // 3. Selective Native Fetch Proxy (ONLY for Cloud OAuth & External Themes)
    // NEVER touches relative paths, Discord APIs, Discord CDNs, or WebSockets!
    const _origFetch = window.fetch;
    window.fetch = function(input, init) {
        const origArgs = arguments;
        let url = '';
        try {
            if (typeof input === 'string') {
                url = input;
            } else if (input instanceof URL) {
                url = input.href;
            } else if (input && typeof input === 'object' && input.url) {
                url = input.url;
            }
        } catch(e) {}

        const isDiscordHost = (h) => {
            if (!h) return true;
            h = h.toLowerCase();
            return h === 'discord.com' || h.endsWith('.discord.com') ||
                   h === 'discordapp.com' || h.endsWith('.discordapp.com') ||
                   h === 'discord.gg' || h.endsWith('.discord.gg') ||
                   h === 'discordapp.net' || h.endsWith('.discordapp.net');
        };

        let isProxyTarget = false;
        if (url && (url.startsWith('http://') || url.startsWith('https://'))) {
            try {
                const u = new URL(url);
                const h = u.hostname.toLowerCase();
                if (!isDiscordHost(h)) {
                    if (
                        h === 'api.vencord.dev' || h.endsWith('.vencord.dev') ||
                        h === 'cloud.equicord.org' || h.endsWith('.equicord.org') ||
                        h === 'themes.equicord.org' ||
                        h === 'raw.githubusercontent.com' || h === 'gist.githubusercontent.com' ||
                        h === 'github.com' || h.endsWith('.github.com') ||
                        u.pathname.toLowerCase().endsWith('.css') ||
                        url.includes('/oauth/') ||
                        url.includes('/themes')
                    ) {
                        isProxyTarget = true;
                    }
                }
            } catch(e) {}
        }

        if (isProxyTarget) {
            return (async () => {
                try {
                    const method = (init && init.method) || (input && typeof input === 'object' && input.method) || 'GET';
                    let headers = {};
                    const srcHeaders = (init && init.headers) || (input && typeof input === 'object' && input.headers);
                    if (srcHeaders) {
                        if (srcHeaders instanceof Headers) {
                            srcHeaders.forEach((v, k) => { headers[k] = v; });
                        } else if (Array.isArray(srcHeaders)) {
                            srcHeaders.forEach(([k, v]) => { headers[k] = v; });
                        } else if (typeof srcHeaders === 'object') {
                            headers = { ...srcHeaders };
                        }
                    }
                    let body = (init && init.body) || (input && typeof input === 'object' && input.body);
                    if (body && typeof body !== 'string') {
                        try { body = await new Response(body).text(); } catch(e) { body = String(body); }
                    }

                    const res = await invoke('native_fetch', {
                        request: { url, method, headers, body: body ? String(body) : null }
                    });

                    return new Response(res.body, {
                        status: res.status,
                        statusText: res.status_text || 'OK',
                        headers: new Headers(res.headers)
                    });
                } catch(err) {
                    console.warn('[Rustcord] native_fetch fallback:', err);
                    return _origFetch.apply(window, origArgs);
                }
            })();
        }

        // Pass everything else to original fetch completely untouched
        return _origFetch.apply(this, origArgs);
    };

    // 4. Styles: Discord Native Layout + Dedicated Top Titlebar (26px)
    const TITLEBAR_CSS = `
/* ===== Base Window Layout ===== */
html {
    box-sizing: border-box !important;
    height: 100% !important;
    overflow: hidden !important;
}
body {
    margin: 0 !important;
    padding: 0 !important;
    height: 100% !important;
    overflow: hidden !important;
}

/* ===== Discord App Container below 26px Top Titlebar ===== */
#app-mount {
    position: fixed !important;
    top: 26px !important;
    left: 0 !important;
    right: 0 !important;
    bottom: 0 !important;
    width: 100% !important;
    height: calc(100% - 26px) !important;
    box-sizing: border-box !important;
    overflow: hidden !important;
}

/* ===== Dedicated Discord Native Top Titlebar (26px) ===== */
#rustcord-titlebar {
    position: fixed !important;
    top: 0 !important;
    left: 0 !important;
    right: 0 !important;
    height: 26px !important;
    background-color: var(--background-tertiary, var(--bg-base-tertiary, #111214)) !important;
    z-index: 2147483647 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: space-between !important;
    padding: 0 0 0 10px !important;
    margin: 0 !important;
    box-sizing: border-box !important;
    -webkit-app-region: drag !important;
    user-select: none !important;
    border: none !important;
    border-bottom: none !important;
    box-shadow: none !important;
}
.rc-tb-left {
    display: flex !important;
    align-items: center !important;
    gap: 7px !important;
    color: var(--header-secondary, #949ba4) !important;
    font-size: 12px !important;
    font-weight: 600 !important;
    letter-spacing: 0.2px !important;
    pointer-events: none !important;
    font-family: 'gg sans', 'Noto Sans', sans-serif !important;
    -webkit-app-region: drag !important;
}
.rc-tb-icon {
    width: 14px !important;
    height: 14px !important;
    fill: currentColor !important;
}
.rc-tb-title {
    color: var(--header-secondary, #949ba4) !important;
}
.rc-tb-drag-area {
    flex: 1 !important;
    height: 26px !important;
    -webkit-app-region: drag !important;
}
.rc-tb-controls {
    display: flex !important;
    align-items: stretch !important;
    height: 26px !important;
    -webkit-app-region: no-drag !important;
}
.rc-tb-btn {
    background: transparent !important;
    border: none !important;
    outline: none !important;
    color: var(--interactive-normal, #949ba4) !important;
    width: 38px !important;
    height: 26px !important;
    display: inline-flex !important;
    align-items: center !important;
    justify-content: center !important;
    cursor: pointer !important;
    padding: 0 !important;
    margin: 0 !important;
    transition: background-color 0.12s ease, color 0.12s ease !important;
    -webkit-app-region: no-drag !important;
}
.rc-tb-btn:hover {
    color: var(--interactive-hover, #ffffff) !important;
    background-color: var(--background-modifier-hover, rgba(255, 255, 255, 0.08)) !important;
}
.rc-tb-btn.rc-close {
    width: 44px !important;
}
.rc-tb-btn.rc-close:hover {
    color: #ffffff !important;
    background-color: #da373c !important;
}
.rc-tb-btn.rc-close:active {
    background-color: #b82e33 !important;
}

/* Telegram-style borderless rounded notifications, toasts, notices, and popouts */
[class*="toast_"],
[class*="notice_"],
[class*="notification_"],
[class*="popout_"],
[class*="tooltip_"],
#rustcord-toast {
    border-radius: 12px !important;
    border: none !important;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3) !important;
}

/* ===== Telegram-Style Borderless Dark Notifications Container & Cards ===== */
#rustcord-notifications-container {
    position: fixed !important;
    bottom: 24px !important;
    right: 24px !important;
    z-index: 2147483647 !important;
    display: flex !important;
    flex-direction: column-reverse !important;
    gap: 8px !important;
    pointer-events: none !important;
    width: 340px !important;
    max-width: calc(100vw - 48px) !important;
}

.rc-tg-toast {
    position: relative !important;
    pointer-events: auto !important;
    display: flex !important;
    align-items: center !important;
    background-color: #111214 !important;
    border-radius: 12px !important;
    border: none !important;
    outline: none !important;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.75), 0 2px 8px rgba(0, 0, 0, 0.5) !important;
    padding: 10px 14px !important;
    cursor: pointer !important;
    user-select: none !important;
    box-sizing: border-box !important;
    transition: opacity 0.22s cubic-bezier(0.16, 1, 0.3, 1), transform 0.22s cubic-bezier(0.16, 1, 0.3, 1) !important;
    opacity: 0;
    transform: translateY(16px) scale(0.96);
    backdrop-filter: blur(20px) !important;
    -webkit-backdrop-filter: blur(20px) !important;
    font-family: 'gg sans', 'Segoe UI', -apple-system, Roboto, sans-serif !important;
    gap: 12px !important;
}

.rc-tg-toast.rc-tg-visible {
    opacity: 1 !important;
    transform: translateY(0) scale(1) !important;
}

.rc-tg-toast:hover {
    background-color: #18191c !important;
}

.rc-tg-avatar {
    width: 40px !important;
    height: 40px !important;
    border-radius: 50% !important;
    object-fit: cover !important;
    flex-shrink: 0 !important;
    background-color: #2b2d31 !important;
}

.rc-tg-body-col {
    display: flex !important;
    flex-direction: column !important;
    flex: 1 !important;
    min-width: 0 !important;
}

.rc-tg-top-row {
    display: flex !important;
    align-items: center !important;
    justify-content: space-between !important;
    margin-bottom: 2px !important;
}

.rc-tg-title {
    font-size: 13.5px !important;
    font-weight: 600 !important;
    color: #f2f3f5 !important;
    white-space: nowrap !important;
    overflow: hidden !important;
    text-overflow: ellipsis !important;
    display: flex !important;
    align-items: center !important;
    gap: 4px !important;
    line-height: 1.25 !important;
}

.rc-tg-title .you {
    color: #949ba4 !important;
    font-weight: 500 !important;
    font-size: 12.5px !important;
    margin-left: 2px !important;
}

.rc-tg-close-btn {
    background: transparent !important;
    border: none !important;
    outline: none !important;
    color: #949ba4 !important;
    cursor: pointer !important;
    padding: 2px 4px !important;
    border-radius: 4px !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    line-height: 1 !important;
    transition: color 0.12s ease, background-color 0.12s ease !important;
}

.rc-tg-close-btn:hover {
    color: #ffffff !important;
    background-color: rgba(255, 255, 255, 0.12) !important;
}

.rc-tg-text {
    font-size: 12.5px !important;
    color: #949ba4 !important;
    line-height: 1.35 !important;
    white-space: nowrap !important;
    overflow: hidden !important;
    text-overflow: ellipsis !important;
}

/* Hide web-generated or duplicate winButtons */
[class*="winButtons_"], [class*="winButton_"] {
    display: none !important;
}

/* Ensure Discord HTML5 draggable elements, server icons, folders, and sidebar resize handles work without window dragging interference */
[class*="resizeHandle_"],
[class*="resizeHandle"],
[draggable="true"],
[class*="guilds_"],
[class*="listItem_"],
[class*="sidebar_"],
[class*="chat_"],
[class*="panels_"] {
    -webkit-app-region: no-drag !important;
}
    `;

    // 5. Mount Native Top Titlebar
    const SVG_DISCORD = '<svg class="rc-tb-icon" viewBox="0 0 127.14 96.36"><path d="M107.7,8.07A105.15,105.15,0,0,0,81.47,0a72.06,72.06,0,0,0-3.36,6.83A97.68,97.68,0,0,0,49,6.83,72.37,72.37,0,0,0,45.64,0,105.89,105.89,0,0,0,19.39,8.09C2.79,32.65-1.71,56.6.54,80.21h0A105.73,105.73,0,0,0,32.71,96.36,77.7,77.7,0,0,0,39.6,85.25a68.42,68.42,0,0,1-10.85-5.18c.91-.66,1.8-1.34,2.66-2a75.57,75.57,0,0,0,64.32,0c.87.71,1.76,1.39,2.66,2a68.68,68.68,0,0,1-10.87,5.19,77,77,0,0,0,6.89,11.1A105.25,105.25,0,0,0,126.6,80.21c2.64-27.38-4.51-51.11-18.9-72.15ZM42.45,65.69C36.18,65.69,31,60,31,53s5-12.74,11.43-12.74S54,45.91,53.89,53,48.84,65.69,42.45,65.69Zm42.24,0C78.41,65.69,73.25,60,73.25,53s5-12.74,11.44-12.74S96.23,45.91,96.12,53,91.08,65.69,84.69,65.69Z"/></svg>';
    const SVG_MIN = '<svg width="10" height="1" viewBox="0 0 10 1"><rect fill="currentColor" width="10" height="1"/></svg>';
    const SVG_MAX = '<svg width="10" height="10" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/></svg>';
    const SVG_RESTORE = '<svg width="10" height="10" viewBox="0 0 10 10"><path d="M2.5,0.5 H9.5 V7.5 H8.5 V1.5 H2.5 Z M0.5,2.5 H7.5 V9.5 H0.5 Z" fill="none" stroke="currentColor" stroke-width="1"/></svg>';
    const SVG_CLOSE = '<svg width="10" height="10" viewBox="0 0 10 10"><polygon fill="currentColor" points="10 0.7 9.3 0 5 4.3 0.7 0 0 0.7 4.3 5 0 9.3 0.7 10 5 5.7 9.3 10 10 9.3 5.7 5"/></svg>';

    const SVG_DEVTOOLS = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline></svg>';

    let _isMax = false;
    const updateMaxUi = (isMax) => {
        _isMax = isMax;
        const btn = document.getElementById('rc-tb-btn-max');
        if (btn) {
            btn.innerHTML = isMax ? SVG_RESTORE : SVG_MAX;
            btn.title = isMax ? 'Восстановить' : 'Развернуть';
        }
        if (document.documentElement) {
            document.documentElement.classList.toggle('is-maximized', isMax);
        }
    };

    const checkMaxUi = () => {
        windowIsMaximized().then(v => updateMaxUi(!!v)).catch(() => {});
    };

    const injectTopTitlebar = () => {
        const body = document.body;
        if (!body) return;

        // Ensure CSS style tag is in head or documentElement
        if (!document.getElementById('rustcord-native-titlebar-css')) {
            const s = document.createElement('style');
            s.id = 'rustcord-native-titlebar-css';
            s.textContent = TITLEBAR_CSS;
            (document.head || document.documentElement).appendChild(s);
        }

        // Ensure Titlebar element is in body
        let tb = document.getElementById('rustcord-titlebar');
        if (!tb) {
            tb = document.createElement('div');
            tb.id = 'rustcord-titlebar';
            tb.innerHTML = `
                <div class="rc-tb-left">
                    ${SVG_DISCORD}
                    <span class="rc-tb-title">Discord</span>
                </div>
                <div class="rc-tb-drag-area"></div>
                <div class="rc-tb-controls">
                    <button class="rc-tb-btn rc-devtools" id="rc-tb-btn-devtools" title="DevTools (F12)">${SVG_DEVTOOLS}</button>
                    <button class="rc-tb-btn rc-min" id="rc-tb-btn-min" title="Свернуть">${SVG_MIN}</button>
                    <button class="rc-tb-btn rc-max" id="rc-tb-btn-max" title="Развернуть">${SVG_MAX}</button>
                    <button class="rc-tb-btn rc-close" id="rc-tb-btn-close" title="Закрыть в трей">${SVG_CLOSE}</button>
                </div>
            `;
            body.appendChild(tb);

            tb.querySelector('#rc-tb-btn-devtools')?.addEventListener('click', (e) => {
                e.stopPropagation();
                e.preventDefault();
                invoke('open_devtools');
            });

            tb.querySelector('#rc-tb-btn-min')?.addEventListener('click', (e) => {
                e.stopPropagation();
                e.preventDefault();
                windowMinimize();
            });
            tb.querySelector('#rc-tb-btn-max')?.addEventListener('click', (e) => {
                e.stopPropagation();
                e.preventDefault();
                windowToggleMaximize();
                setTimeout(checkMaxUi, 120);
            });
            tb.querySelector('#rc-tb-btn-close')?.addEventListener('click', (e) => {
                e.stopPropagation();
                e.preventDefault();
                windowClose();
            });

            checkMaxUi();
        } else if (tb.parentNode !== body) {
            body.appendChild(tb);
        }
    };

    if (document.body) injectTopTitlebar();
    document.addEventListener('DOMContentLoaded', injectTopTitlebar);
    window.addEventListener('load', injectTopTitlebar);
    setInterval(injectTopTitlebar, 400);

    // Observe body mutations to immediately re-insert if removed
    const bodyObs = new MutationObserver(() => {
        if (!document.getElementById('rustcord-titlebar') && document.body) {
            injectTopTitlebar();
        }
    });
    if (document.documentElement) {
        bodyObs.observe(document.documentElement, { childList: true, subtree: true });
    }

    window.addEventListener('resize', () => setTimeout(checkMaxUi, 80));

    // Top Titlebar Dragging - strictly scoped to #rustcord-titlebar only
    document.addEventListener('mousedown', (e) => {
        if (e.button !== 0) return;
        const t = e.target;
        if (!t) return;
        if (!t.closest('#rustcord-titlebar')) return;
        if (t.closest('.rc-tb-controls, button, [role="button"], input, a, select')) return;
        if (e.detail === 2) {
            windowToggleMaximize();
            setTimeout(checkMaxUi, 120);
        } else {
            windowStartDragging();
        }
    });

    // 6. User-Agent spoof
    try {
        const ua = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36';
        Object.defineProperty(navigator, 'userAgent',  { get: () => ua, configurable: true });
        Object.defineProperty(navigator, 'appVersion', { get: () => ua.replace('Mozilla/',''), configurable: true });
    } catch(e) {}

    // 7. Auto-request media
    try {
        if (navigator.mediaDevices?.getUserMedia) {
            navigator.mediaDevices.getUserMedia({ audio: true, video: true })
                .then(s => { s.getTracks().forEach(t => t.stop()); })
                .catch(() => {});
        }
    } catch(e) {}

    // 8. Equicord CSS injection
    try {
        const equicordCss = %EQUICORD_CSS_JSON%;
        const injectCss = () => {
            if (document.getElementById('rustcord-equicord-css')) return;
            const st = document.createElement('style');
            st.id = 'rustcord-equicord-css';
            st.textContent = equicordCss;
            (document.head || document.documentElement).appendChild(st);
        };
        if (document.head) injectCss();
        else document.addEventListener('DOMContentLoaded', injectCss);
    } catch(e) {}

    // 9. Discord UI ready detection -> close splash
    let _ready = false;
    const notifyReady = () => {
        if (_ready) return;
        _ready = true;
        console.log('[Rustcord] Discord UI ready — revealing main window.');
        appReady();
    };
    const checkReady = () => {
        if (_ready) return;
        if (document.querySelector('#app-mount [class*="appAsidePanelWrapper"], #app-mount [class*="channels_"], #app-mount nav, #app-mount [class*="loginForm_"], #app-mount [class*="authBox_"]')) {
            notifyReady();
        }
    };
    const rObs = new MutationObserver(checkReady);
    if (document.documentElement) rObs.observe(document.documentElement, { childList: true, subtree: true });
    document.addEventListener('DOMContentLoaded', checkReady);
    setTimeout(notifyReady, 5000);

    // 10. Equicord Bundle Execution
    window.Vencord = window.Vencord || {};
    window.Equicord = window.Vencord;
    try {
        console.log('[Rustcord] Executing Equicord bundle...');
        %EQUICORD_JS%
        if (typeof Vencord !== 'undefined') {
            Object.assign(window.Vencord, Vencord);
            window.Vencord = Vencord;
            window.Equicord = Vencord;
            console.log('[Rustcord] Equicord registered globally.');
        }
    } catch(err) {
        console.error('[Rustcord] Equicord execution error:', err);
    }

    // 11. Complete Theme Management Engine (Local files, Blob URLs, Hoisted @imports & Apple Emojis)
    const patchThemes = () => {
        if (window.VencordNative?.themes) {
            window.VencordNative.themes.openFolder = () => invoke('open_themes_folder');
            invoke('get_local_themes').then(themes => {
                if (Array.isArray(themes) && themes.length > 0 && window.VencordNative?.themes?.uploadTheme) {
                    themes.forEach(t => {
                        try {
                            window.VencordNative.themes.uploadTheme(t.filename, t.css);
                        } catch(e) {}
                    });
                }
            }).catch(() => {});
        }
    };
    patchThemes();
    setInterval(patchThemes, 3000);

    const onlineCssCache = new Map();
    const blobCssCache = new Map();

    const APPLE_EMOJI_IMPORTS = [
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/base/Apple.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/spritesheets.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/people.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/nature.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/food.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/activity.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/travel.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/objects.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/symbols.css");',
        '@import url("https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/src/Apple/flags.css");'
    ];

    const syncThemes = async () => {
        try {
            // Clean up any legacy split style elements from previous versions
            document.querySelectorAll('[id^="rustcord-theme-"]').forEach(el => el.remove());

            const settings = window.Vencord?.Settings;
            const enabledThemes = settings?.enabledThemes || [];
            const enabledLinks = settings?.enabledThemeLinks || [];
            const hasThemes = (Array.isArray(enabledThemes) && enabledThemes.length > 0) ||
                              (Array.isArray(enabledLinks) && enabledLinks.length > 0);

            if (document.documentElement) {
                document.documentElement.classList.toggle('custom-theme-active', hasThemes);
            }

            let st = document.getElementById('rustcord-active-theme-css');
            if (!st) {
                st = document.createElement('style');
                st.id = 'rustcord-active-theme-css';
                (document.head || document.documentElement).appendChild(st);
            }

            if (!hasThemes) {
                if (st.textContent !== '') st.textContent = '';
                return;
            }

            const rawSnippets = [];
            let needsAppleEmojis = false;

            // 1. Fetch CSS for local themes
            if (Array.isArray(enabledThemes)) {
                for (const tName of enabledThemes) {
                    if (tName.toLowerCase().includes('emoji')) {
                        needsAppleEmojis = true;
                    }
                    try {
                        if (window.VencordNative?.themes?.getThemeData) {
                            const data = await window.VencordNative.themes.getThemeData(tName);
                            if (data && typeof data === 'string' && data.length > 0) {
                                rawSnippets.push(data);
                            }
                        }
                    } catch(e) {}
                }
            }

            // Fallback / sync from local files in themes folder
            try {
                const localThemes = await invoke('get_local_themes');
                if (Array.isArray(localThemes)) {
                    for (const t of localThemes) {
                        const isEnabled = enabledThemes.includes(t.filename) || enabledThemes.includes(t.name);
                        if (isEnabled) {
                            if (t.name.toLowerCase().includes('emoji') || t.filename.toLowerCase().includes('emoji')) {
                                needsAppleEmojis = true;
                            }
                            if (t.css && !rawSnippets.some(s => s.includes(t.css.slice(0, 60)))) {
                                rawSnippets.push(t.css);
                            }
                        }
                    }
                }
            } catch(e) {}

            // 2. Online themes
            if (Array.isArray(enabledLinks)) {
                for (const link of enabledLinks) {
                    const cleanLink = link.replace(/^@(light|dark)\s+/, '').trim();
                    if (cleanLink.toLowerCase().includes('emoji') || cleanLink.includes('mwittrien.github.io')) {
                        needsAppleEmojis = true;
                    }
                    let onlineCss = onlineCssCache.get(cleanLink);
                    if (!onlineCss) {
                        try {
                            const res = await invoke('native_fetch', {
                                request: { url: cleanLink, method: 'GET', headers: {}, body: null }
                            });
                            if (res && res.status >= 200 && res.status < 300 && res.body) {
                                onlineCss = res.body;
                            }
                        } catch(e) {
                            try {
                                const resp = await fetch(cleanLink);
                                if (resp.ok) onlineCss = await resp.text();
                            } catch(e2) {}
                        }
                        if (onlineCss) onlineCssCache.set(cleanLink, onlineCss);
                    }
                    if (onlineCss) rawSnippets.push(onlineCss);
                }
            }

            // 3. Inspect <style id="vencord-themes"> for any blob URLs
            const vencordThemesTag = document.getElementById('vencord-themes');
            if (vencordThemesTag && vencordThemesTag.textContent) {
                const blobUrls = vencordThemesTag.textContent.match(/blob:https?:\/\/[^\s'")]+/g) || [];
                for (const bUrl of blobUrls) {
                    let bText = blobCssCache.get(bUrl);
                    if (!bText) {
                        try {
                            const bResp = await fetch(bUrl);
                            if (bResp.ok) {
                                bText = await bResp.text();
                                blobCssCache.set(bUrl, bText);
                            }
                        } catch(e) {}
                    }
                    if (bText && !rawSnippets.some(s => s.includes(bText.slice(0, 60)))) {
                        rawSnippets.push(bText);
                    }
                }
            }

            // Check if any snippet references Apple emojis
            for (const s of rawSnippets) {
                if (s.includes('EmojiReplace') || s.includes('Apple.css') || s.includes('mwittrien.github.io')) {
                    needsAppleEmojis = true;
                    break;
                }
            }

            // 4. Extract and HOIST all @import statements to the top
            const importStatements = new Set();
            const bodyStatements = [];
            const importRegex = /@import\s+(?:url\s*\(\s*['"]?([^'")]+)['"]?\s*\)|['"]([^'"]+)['"])\s*;/g;

            if (needsAppleEmojis) {
                APPLE_EMOJI_IMPORTS.forEach(imp => importStatements.add(imp));
            }

            for (const snippet of rawSnippets) {
                let cleaned = snippet;
                let match;
                while ((match = importRegex.exec(snippet)) !== null) {
                    const fullImport = match[0].trim();
                    const url = (match[1] || match[2] || '').trim();
                    if (url.includes('Apple.css')) {
                        needsAppleEmojis = true;
                        APPLE_EMOJI_IMPORTS.forEach(imp => importStatements.add(imp));
                    } else if (fullImport) {
                        importStatements.add(fullImport);
                    }
                }
                cleaned = cleaned.replace(importRegex, '');
                if (cleaned.trim()) {
                    bodyStatements.push(cleaned.trim());
                }
            }

            if (needsAppleEmojis) {
                bodyStatements.push(`
/* Apple Emoji Compatibility & Transparency Overrides */
.emoji[src*="/assets/"], img.emoji[src*="/assets/"], [class*="emoji_"][src*="/assets/"] {
    background-color: transparent !important;
}
`);
            }

            const finalCombined = Array.from(importStatements).join('\n') + '\n\n' + bodyStatements.join('\n\n');

            if (st.textContent !== finalCombined) {
                st.textContent = finalCombined;
                console.log('[Rustcord] Theme engine active with ' + rawSnippets.length + ' theme snippet(s), ' + importStatements.size + ' hoisted @import(s).');
            }
        } catch(err) {
            console.warn('[Rustcord] Theme sync error:', err);
        }
    };

    setTimeout(syncThemes, 1000);
    setTimeout(syncThemes, 2500);
    setTimeout(syncThemes, 5000);
    setInterval(syncThemes, 1500);

    const hookThemeListeners = () => {
        if (window.Vencord?.SettingsStore?.addChangeListener) {
            window.Vencord.SettingsStore.addChangeListener('enabledThemes', syncThemes);
            window.Vencord.SettingsStore.addChangeListener('enabledThemeLinks', syncThemes);
            window.Vencord.SettingsStore.addChangeListener('useQuickCss', syncThemes);
        }
    };
    hookThemeListeners();
    setInterval(hookThemeListeners, 3000);

    // 12. Gaming helpers, Mic/Audio control & OLED toasts
    const clickPanelButton = (type) => {
        // Search buttons in user panel
        const panelButtons = Array.from(document.querySelectorAll('section[class*="panels_"] button, [class*="panels_"] button, [class*="accountProfile_"] button'));
        for (const btn of panelButtons) {
            const label = (btn.getAttribute('aria-label') || '').toLowerCase();
            if (type === 'mute' && (label.includes('микрофон') || label.includes('mute') || label.includes('mic'))) {
                btn.click();
                const nowMuted = label.includes('отключить') || label.includes('mute');
                return nowMuted ? 'Микрофон выключен' : 'Микрофон включен';
            }
            if (type === 'deafen' && (label.includes('звук') || label.includes('deaf') || label.includes('заглуш'))) {
                btn.click();
                const nowDeaf = label.includes('отключить') || label.includes('заглуш') || label.includes('deaf');
                return nowDeaf ? 'Звук выключен' : 'Звук включен';
            }
        }

        // Search all buttons
        const allButtons = Array.from(document.querySelectorAll('button[aria-label]'));
        for (const btn of allButtons) {
            const label = (btn.getAttribute('aria-label') || '').toLowerCase();
            if (type === 'mute' && (label.includes('микрофон') || label.includes('mute') || label.includes('mic'))) {
                btn.click();
                const nowMuted = label.includes('отключить') || label.includes('mute');
                return nowMuted ? 'Микрофон выключен' : 'Микрофон включен';
            }
            if (type === 'deafen' && (label.includes('звук') || label.includes('deaf') || label.includes('заглуш'))) {
                btn.click();
                const nowDeaf = label.includes('отключить') || label.includes('заглуш') || label.includes('deaf');
                return nowDeaf ? 'Звук выключен' : 'Звук включен';
            }
        }

        // Positional fallback in account panel (0 = Mic, 1 = Deafen, 2 = Settings)
        if (panelButtons.length >= 3) {
            if (type === 'mute') {
                panelButtons[0].click();
                return 'Микрофон переключен';
            }
            if (type === 'deafen') {
                panelButtons[1].click();
                return 'Звук переключен';
            }
        }

        return null;
    };

    window.__RUSTCORD__ = {
        gameMode: false,
        toggleGameMode(enable) {
            if (enable === undefined) enable = !this.gameMode;
            this.gameMode = enable;
            let st = document.getElementById('rustcord-gamemode-css');
            if (this.gameMode) {
                if (!st) {
                    st = document.createElement('style');
                    st.id = 'rustcord-gamemode-css';
                    st.textContent = '*, *::before, *::after { animation-duration:0.001ms!important; animation-iteration-count:1!important; transition-duration:0.001ms!important; } [class*="avatarDecoration"],[class*="profileEffect"],[class*="banner"] video,[class*="banner"] canvas{display:none!important} [class*="backdrop"],[class*="blur"],[class*="modal_"]{backdrop-filter:none!important;-webkit-backdrop-filter:none!important} img[src*=".gif"],[class*="sticker"] img{image-rendering:pixelated!important}';
                    (document.head || document.documentElement).appendChild(st);
                }
                this.showToast('Игровой режим включен');
            } else {
                if (st) st.remove();
                this.showToast('Игровой режим выключен');
            }
            return this.gameMode;
        },
        toggleMute() {
            try {
                const act = window.Vencord?.Webpack?.findByProps?.("toggleSelfMute", "toggleSelfDeaf")
                    || window.Vencord?.Webpack?.findByProps?.("toggleSelfMute");
                if (act && typeof act.toggleSelfMute === 'function') {
                    act.toggleSelfMute();
                    const store = window.Vencord?.Webpack?.findByProps?.("isSelfMute");
                    const isMuted = store?.isSelfMute ? store.isSelfMute() : null;
                    const msg = isMuted === true ? 'Микрофон выключен' : (isMuted === false ? 'Микрофон включен' : 'Микрофон переключен');
                    this.showToast(msg);
                    return;
                }
            } catch(e) {}

            try {
                const wp = window.webpackChunkdiscord_app;
                if (wp) {
                    let act = null;
                    wp.push([[Symbol()], {}, (r) => {
                        for (const m of Object.values(r.c || {})) {
                            if (m.exports?.toggleSelfMute) { act = m.exports; break; }
                            if (m.exports?.default?.toggleSelfMute) { act = m.exports.default; break; }
                            if (m.exports?.Z?.toggleSelfMute) { act = m.exports.Z; break; }
                        }
                    }]);
                    if (act && typeof act.toggleSelfMute === 'function') {
                        act.toggleSelfMute();
                        this.showToast('Микрофон переключен');
                        return;
                    }
                }
            } catch(e) {}

            const res = clickPanelButton('mute');
            this.showToast(res || 'Микрофон переключен');
        },
        toggleDeafen() {
            try {
                const act = window.Vencord?.Webpack?.findByProps?.("toggleSelfDeaf", "toggleSelfMute")
                    || window.Vencord?.Webpack?.findByProps?.("toggleSelfDeaf");
                if (act && typeof act.toggleSelfDeaf === 'function') {
                    act.toggleSelfDeaf();
                    const store = window.Vencord?.Webpack?.findByProps?.("isSelfDeaf");
                    const isDeaf = store?.isSelfDeaf ? store.isSelfDeaf() : null;
                    const msg = isDeaf === true ? 'Звук выключен' : (isDeaf === false ? 'Звук включен' : 'Звук переключен');
                    this.showToast(msg);
                    return;
                }
            } catch(e) {}

            try {
                const wp = window.webpackChunkdiscord_app;
                if (wp) {
                    let act = null;
                    wp.push([[Symbol()], {}, (r) => {
                        for (const m of Object.values(r.c || {})) {
                            if (m.exports?.toggleSelfDeaf) { act = m.exports; break; }
                            if (m.exports?.default?.toggleSelfDeaf) { act = m.exports.default; break; }
                            if (m.exports?.Z?.toggleSelfDeaf) { act = m.exports.Z; break; }
                        }
                    }]);
                    if (act && typeof act.toggleSelfDeaf === 'function') {
                        act.toggleSelfDeaf();
                        this.showToast('Звук переключен');
                        return;
                    }
                }
            } catch(e) {}

            const res = clickPanelButton('deafen');
            this.showToast(res || 'Звук переключен');
        },
        disconnectVoice() {
            try {
                const act = window.Vencord?.Webpack?.findByProps?.("selectVoiceChannel");
                if (act && typeof act.selectVoiceChannel === 'function') {
                    act.selectVoiceChannel(null);
                    this.showToast('Вы вышли из звонка');
                    return;
                }
            } catch(e) {}

            try {
                const act = window.Vencord?.Webpack?.findByProps?.("disconnect");
                if (act && typeof act.disconnect === 'function') {
                    act.disconnect();
                    this.showToast('Вы вышли из звонка');
                    return;
                }
            } catch(e) {}

            const btn = document.querySelector('button[aria-label="Отключиться"], button[aria-label="Disconnect"], button[aria-label*="Отключ"], section[class*="panels_"] button[aria-label*="Отключ"]');
            if (btn) {
                btn.click();
                this.showToast('Вы вышли из звонка');
            }
        },
        showToast(text) {
            let t = document.getElementById('rustcord-toast');
            if (!t) {
                t = document.createElement('div');
                t.id = 'rustcord-toast';
                t.style.cssText = 'position:fixed;bottom:24px;right:24px;z-index:2147483647;padding:10px 18px;background:var(--background-floating, var(--bg-surface-overlay, #111214));border:none;border-radius:12px;color:var(--text-normal, #dbdee1);font:500 13px/-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;box-shadow:0 8px 28px rgba(0,0,0,0.5),0 2px 8px rgba(0,0,0,0.3);transition:opacity 0.2s ease, transform 0.2s ease;pointer-events:none;display:flex;align-items:center;gap:10px;opacity:0;transform:translateY(8px);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);';
                (document.body||document.documentElement).appendChild(t);
            }
            t.innerHTML = '<svg width="15" height="15" viewBox="0 0 127.14 96.36" fill="currentColor"><path d="M107.7,8.07A105.15,105.15,0,0,0,81.47,0a72.06,72.06,0,0,0-3.36,6.83A97.68,97.68,0,0,0,49,6.83,72.37,72.37,0,0,0,45.64,0,105.89,105.89,0,0,0,19.39,8.09C2.79,32.65-1.71,56.6.54,80.21h0A105.73,105.73,0,0,0,32.71,96.36,77.7,77.7,0,0,0,39.6,85.25a68.42,68.42,0,0,1-10.85-5.18c.91-.66,1.8-1.34,2.66-2a75.57,75.57,0,0,0,64.32,0c.87.71,1.76,1.39,2.66,2a68.68,68.68,0,0,1-10.87,5.19,77,77,0,0,0,6.89,11.1A105.25,105.25,0,0,0,126.6,80.21c2.64-27.38-4.51-51.11-18.9-72.15ZM42.45,65.69C36.18,65.69,31,60,31,53s5-12.74,11.43-12.74S54,45.91,53.89,53,48.84,65.69,42.45,65.69Zm42.24,0C78.41,65.69,73.25,60,73.25,53s5-12.74,11.44-12.74S96.23,45.91,96.12,53,91.08,65.69,84.69,65.69Z"/></svg><span>' + text + '</span>';
            t.style.opacity = '1';
            t.style.transform = 'translateY(0)';
            clearTimeout(t._tmr);
            t._tmr = setTimeout(() => {
                t.style.opacity = '0';
                t.style.transform = 'translateY(8px)';
            }, 2500);
        },
        showNotification: showTelegramToast,
        testNotification() {
            showTelegramToast({
                author: 'Slim md3 (you)',
                titleHtml: '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" style="vertical-align:middle;margin-right:4px;"><path d="m3 11 18-5v12L3 14v-3z"></path><path d="M11.6 16.8a3 3 0 1 1-5.8-1.6"></path></svg>Slim md3 <span class="you">(you)</span>',
                text: 'На какой бете вы сидите?',
                avatar: 'https://cdn.discordapp.com/embed/avatars/0.png'
            });
        }
    };

    // 13. Voice State Monitoring & Tray Synchronization (Call presence & speaking state)
    let _lastInCall = null;
    let _lastSpeaking = null;
    let _lastChName = null;

    const checkVoiceState = () => {
        try {
            let inCall = false;
            let isSpeaking = false;
            let channelName = '';

            try {
                const chStore = window.Vencord?.Webpack?.findByProps?.("getVoiceChannelId", "getChannel");
                const vChId = chStore?.getVoiceChannelId ? chStore.getVoiceChannelId() : null;
                if (vChId) {
                    inCall = true;
                    const ch = chStore?.getChannel ? chStore.getChannel(vChId) : null;
                    if (ch && ch.name) channelName = ch.name;
                }
            } catch(e) {}

            if (!inCall) {
                const callPanel = document.querySelector('section[class*="panels_"] [class*="rtcConnectionStatus_"], section[class*="panels_"] [class*="connection_"], button[aria-label="Отключиться"], button[aria-label="Disconnect"], button[aria-label*="Отключ"]');
                if (callPanel) {
                    inCall = true;
                    const chEl = document.querySelector('section[class*="panels_"] [class*="channel_"], section[class*="panels_"] [class*="title_"]');
                    if (chEl && chEl.textContent) channelName = chEl.textContent.trim();
                }
            }

            if (inCall) {
                try {
                    const userStore = window.Vencord?.Webpack?.findByProps?.("getCurrentUser");
                    const currentUserId = userStore?.getCurrentUser ? userStore.getCurrentUser()?.id : null;
                    const speakingStore = window.Vencord?.Webpack?.findStore?.("SpeakingStore") 
                        || window.Vencord?.Webpack?.findByProps?.("isSpeaking");
                    if (currentUserId && speakingStore?.isSpeaking) {
                        isSpeaking = !!speakingStore.isSpeaking(currentUserId);
                    }
                } catch(e) {}

                if (!isSpeaking) {
                    const speakingEl = document.querySelector('section[class*="panels_"] [class*="avatarSpeaking_"], [class*="speaking_"][class*="avatar_"], [class*="border_"][style*="rgb(35, 165, 90)"]');
                    if (speakingEl) isSpeaking = true;
                }
            }

            if (inCall !== _lastInCall || isSpeaking !== _lastSpeaking || channelName !== _lastChName) {
                _lastInCall = inCall;
                _lastSpeaking = isSpeaking;
                _lastChName = channelName;
                invoke('update_voice_state', {
                    inCall,
                    isSpeaking,
                    channelName: channelName || null
                }).catch(() => {});
            }
        } catch(err) {}
    };

    setInterval(checkVoiceState, 150);

    // 14. Telegram Notifications Dispatcher Hook
    let _dispatcherHooked = false;
    const hookFluxDispatcher = () => {
        if (_dispatcherHooked) return;
        try {
            let dispatcher = window.Vencord?.Webpack?.Common?.FluxDispatcher
                || window.Vencord?.Webpack?.findByProps?.("dispatch", "subscribe");

            if (!dispatcher && window.webpackChunkdiscord_app) {
                window.webpackChunkdiscord_app.push([[Symbol()], {}, (r) => {
                    for (const m of Object.values(r.c || {})) {
                        if (m.exports?.dispatch && m.exports?.subscribe) {
                            dispatcher = m.exports;
                            break;
                        }
                        if (m.exports?.default?.dispatch && m.exports?.default?.subscribe) {
                            dispatcher = m.exports.default;
                            break;
                        }
                    }
                }]);
            }

            if (dispatcher && typeof dispatcher.subscribe === 'function') {
                dispatcher.subscribe('MESSAGE_CREATE', (e) => {
                    try {
                        if (!e || !e.message) return;
                        const msg = e.message;
                        const author = msg.author;
                        if (!author) return;

                        // 1. Current user check (skip own messages)
                        let currentUserId = null;
                        let currentUserUsername = null;
                        try {
                            const userStore = window.Vencord?.Webpack?.Common?.UserStore
                                || window.Vencord?.Webpack?.findByProps?.("getCurrentUser")
                                || window.Vencord?.Webpack?.findStore?.("UserStore");
                            const u = userStore?.getCurrentUser?.();
                            currentUserId = u?.id;
                            currentUserUsername = u?.username;
                        } catch(err) {}

                        if (currentUserId && author.id === currentUserId) return;

                        // 2. STRICT FILTER: ONLY PINGS (MENTIONS), REPLIES TO ME, @EVERYONE, AND DMS ALLOWED!
                        const isDM = !e.guildId;
                        let isMentioned = false;
                        let isReplyToMe = false;

                        if (currentUserId && Array.isArray(msg.mentions)) {
                            isMentioned = msg.mentions.some(m => m && m.id === currentUserId);
                        }

                        if (!isMentioned && currentUserId && typeof msg.content === 'string') {
                            isMentioned = msg.content.includes('<@' + currentUserId + '>') || msg.content.includes('<@!' + currentUserId + '>');
                        }

                        if (currentUserId && msg.referenced_message?.author?.id === currentUserId) {
                            isReplyToMe = true;
                        }

                        const isEveryone = msg.mention_everyone === true;

                        // Check role mentions
                        if (!isMentioned && currentUserId && e.guildId && Array.isArray(msg.mention_roles) && msg.mention_roles.length > 0) {
                            try {
                                const guildMemberStore = window.Vencord?.Webpack?.Common?.GuildMemberStore
                                    || window.Vencord?.Webpack?.findByProps?.("getMember")
                                    || window.Vencord?.Webpack?.findStore?.("GuildMemberStore");
                                const member = guildMemberStore?.getMember?.(e.guildId, currentUserId);
                                if (member && Array.isArray(member.roles)) {
                                    isMentioned = msg.mention_roles.some(r => member.roles.includes(r));
                                }
                            } catch(err) {}
                        }

                        // STRICT FILTER: If NOT a DM and NOT mentioned and NOT a reply to me and NOT @everyone -> SKIP!
                        if (!isDM && !isMentioned && !isReplyToMe && !isEveryone) {
                            return;
                        }

                        // 3. Active channel check (if user is currently viewing this exact chat and window has focus, skip)
                        let currentChannelId = null;
                        try {
                            const chStore = window.Vencord?.Webpack?.Common?.SelectedChannelStore
                                || window.Vencord?.Webpack?.findByProps?.("getChannelId", "getVoiceChannelId")
                                || window.Vencord?.Webpack?.findStore?.("SelectedChannelStore");
                            currentChannelId = chStore?.getChannelId?.();
                        } catch(err) {}

                        const isViewingThisChannel = (e.channelId && currentChannelId && e.channelId === currentChannelId);
                        const isDocFocused = typeof document.hasFocus === 'function' ? document.hasFocus() : true;

                        if (isViewingThisChannel && isDocFocused) return;

                        // 4. Deduplication (skip duplicate events)
                        const msgId = msg.id || (author.id + '_' + (msg.content || '').slice(0, 30));
                        if (recentlySeenMsgIds.has(msgId)) return;
                        recentlySeenMsgIds.add(msgId);
                        setTimeout(() => recentlySeenMsgIds.delete(msgId), 5000);

                        // 5. Author name & Avatar
                        const authorName = author.global_name || author.username || 'Пользователь';
                        let avatarUrl = 'https://cdn.discordapp.com/embed/avatars/0.png';
                        if (author.id && author.avatar) {
                            avatarUrl = `https://cdn.discordapp.com/avatars/${author.id}/${author.avatar}.png?size=128`;
                        }

                        // 6. Title and formatting (matches photo 2)
                        let titleHtml = escapeHtml(authorName);
                        let titleText = authorName;

                        const SVG_PING_ICON = '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" style="vertical-align:middle;margin-right:4px;"><path d="m3 11 18-5v12L3 14v-3z"></path><path d="M11.6 16.8a3 3 0 1 1-5.8-1.6"></path></svg>';

                        if (isMentioned || isReplyToMe) {
                            titleHtml = SVG_PING_ICON + escapeHtml(authorName) + ' <span class="you">(you)</span>';
                            titleText = authorName + ' (you)';
                        } else if (isEveryone) {
                            titleHtml = SVG_PING_ICON + escapeHtml(authorName) + ' <span class="you">(@everyone)</span>';
                            titleText = authorName + ' (@everyone)';
                        }

                        // 7. Message preview
                        let bodyText = (msg.content || '').trim();
                        if (!bodyText && msg.attachments && msg.attachments.length > 0) {
                            bodyText = '[Вложение]';
                        } else if (!bodyText && msg.embeds && msg.embeds.length > 0) {
                            bodyText = '[Встроенное сообщение]';
                        } else if (!bodyText && msg.sticker_items && msg.sticker_items.length > 0) {
                            bodyText = '[Стикер]';
                        }
                        if (!bodyText) bodyText = 'Новое сообщение';

                        // 8. Show Desktop Floating Notification & In-App Toast!
                        showTelegramToast({
                            author: titleText,
                            titleHtml: titleHtml,
                            text: bodyText,
                            avatar: avatarUrl,
                            guildId: e.guildId || null,
                            channelId: e.channelId,
                            onClick: () => {
                                invoke('window_reveal').catch(() => {});
                                try {
                                    const nav = window.Vencord?.Webpack?.findByProps?.("selectChannel")
                                        || window.Vencord?.Webpack?.Common?.NavigationUtils;
                                    if (nav && typeof nav.selectChannel === 'function') {
                                        nav.selectChannel(e.guildId || null, e.channelId);
                                    } else if (window.Vencord?.Webpack?.findByProps?.("transitionTo")) {
                                        const path = e.guildId ? `/channels/${e.guildId}/${e.channelId}` : `/channels/@me/${e.channelId}`;
                                        window.Vencord.Webpack.findByProps("transitionTo").transitionTo(path);
                                    }
                                } catch(err) {}
                            }
                        });

                        // 9. Flash taskbar if window is not focused
                        if (!isDocFocused) {
                            invoke('notify_attention').catch(() => {});
                        }
                    } catch(err) {
                        console.warn('[Rustcord] handleMessageCreate error:', err);
                    }
                });
                _dispatcherHooked = true;
                console.log('[Rustcord] Telegram notifications hooked into FluxDispatcher.');
            }
        } catch(e) {}
    };

    setInterval(hookFluxDispatcher, 1000);

    console.log('[Rustcord] Pre-load hooks v4 installed.');
})();
"###;

    template
        .replace("%EQUICORD_CSS_JSON%", &css_json)
        .replace("%EQUICORD_JS%", equicord_js)
}
