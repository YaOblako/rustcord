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
    background-color: var(--background-tertiary, var(--bg-base-tertiary, #1e1f22)) !important;
    z-index: 2147483647 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: space-between !important;
    padding: 0 0 0 10px !important;
    margin: 0 !important;
    box-sizing: border-box !important;
    -webkit-app-region: drag !important;
    user-select: none !important;
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.06)) !important;
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
                    <button class="rc-tb-btn rc-min" id="rc-tb-btn-min" title="Свернуть">${SVG_MIN}</button>
                    <button class="rc-tb-btn rc-max" id="rc-tb-btn-max" title="Развернуть">${SVG_MAX}</button>
                    <button class="rc-tb-btn rc-close" id="rc-tb-btn-close" title="Закрыть в трей">${SVG_CLOSE}</button>
                </div>
            `;
            body.appendChild(tb);

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

    // 11. Complete Theme Management Engine with Recursive @import Inliner (Apple Emojis & Online Themes)
    const cssCache = new Map();

    const resolveAndInlineCss = async (css, depth = 0, seen = new Set()) => {
        if (!css || depth > 4) return css;
        const importRegex = /@import\s+(?:url\s*\(\s*['"]?([^'")]+)['"]?\s*\)|['"]([^'"]+)['"])\s*;/g;
        let match;
        const matches = [];
        while ((match = importRegex.exec(css)) !== null) {
            matches.push({ full: match[0], url: (match[1] || match[2] || '').trim() });
        }
        if (matches.length === 0) return css;

        let result = css;
        for (const item of matches) {
            const rawUrl = item.url;
            if (!rawUrl || seen.has(rawUrl)) {
                result = result.replace(item.full, '');
                continue;
            }
            seen.add(rawUrl);

            try {
                let fetchedText = cssCache.get(rawUrl) || '';
                if (!fetchedText) {
                    if (rawUrl.startsWith('blob:')) {
                        const resp = await fetch(rawUrl);
                        if (resp.ok) fetchedText = await resp.text();
                    } else if (rawUrl.startsWith('http://') || rawUrl.startsWith('https://')) {
                        try {
                            const res = await invoke('native_fetch', {
                                request: { url: rawUrl, method: 'GET', headers: {}, body: null }
                            });
                            if (res && res.status >= 200 && res.status < 300 && res.body) {
                                fetchedText = res.body;
                            }
                        } catch(e) {
                            const resp = await fetch(rawUrl);
                            if (resp.ok) fetchedText = await resp.text();
                        }
                    }
                    if (fetchedText) cssCache.set(rawUrl, fetchedText);
                }

                if (fetchedText) {
                    const nested = await resolveAndInlineCss(fetchedText, depth + 1, seen);
                    result = result.replace(item.full, '/* Inlined: ' + rawUrl + ' */\n' + nested);
                } else {
                    result = result.replace(item.full, '/* Failed: ' + rawUrl + ' */');
                }
            } catch(e) {
                result = result.replace(item.full, '/* Error: ' + rawUrl + ' */');
            }
        }
        return result;
    };

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

    const activeThemeStyles = new Map();

    const applyThemeStyle = (id, css) => {
        let el = document.getElementById(id);
        if (!el) {
            el = document.createElement('style');
            el.id = id;
            (document.head || document.documentElement).appendChild(el);
        }
        if (el.textContent !== css) {
            el.textContent = css;
        }
    };

    const removeThemeStyle = (id) => {
        const el = document.getElementById(id);
        if (el) el.remove();
    };

    const syncThemes = async () => {
        try {
            const settings = window.Vencord?.Settings;
            const enabledThemes = settings?.enabledThemes || [];
            const enabledLinks = settings?.enabledThemeLinks || [];
            const hasThemes = (Array.isArray(enabledThemes) && enabledThemes.length > 0) ||
                              (Array.isArray(enabledLinks) && enabledLinks.length > 0);

            if (document.documentElement) {
                document.documentElement.classList.toggle('custom-theme-active', hasThemes);
            }

            const currentActiveIds = new Set();

            if (!hasThemes) {
                for (const [id] of activeThemeStyles) {
                    removeThemeStyle(id);
                }
                activeThemeStyles.clear();
                return;
            }

            // 1. Process local themes
            if (Array.isArray(enabledThemes)) {
                for (const tName of enabledThemes) {
                    const styleId = 'rustcord-theme-' + tName.replace(/[^a-zA-Z0-9_-]/g, '_');
                    currentActiveIds.add(styleId);

                    let rawCss = '';
                    try {
                        if (window.VencordNative?.themes?.getThemeData) {
                            rawCss = await window.VencordNative.themes.getThemeData(tName);
                        }
                    } catch(e) {}

                    if (!rawCss) {
                        try {
                            const localThemes = await invoke('get_local_themes');
                            const match = localThemes.find(t => t.filename === tName || t.name === tName);
                            if (match && match.css) rawCss = match.css;
                        } catch(e) {}
                    }

                    if (rawCss) {
                        // Check local cache first
                        const cacheKey = 'rc_theme_v2_' + tName;
                        let inlined = localStorage.getItem(cacheKey);
                        if (!inlined || !inlined.includes('/* Inlined:')) {
                            inlined = await resolveAndInlineCss(rawCss);
                            try { localStorage.setItem(cacheKey, inlined); } catch(e) {}
                        }

                        // Extra enhancement for Apple Emojis theme
                        if (tName.toLowerCase().includes('emoji') || inlined.includes('mwittrien.github.io')) {
                            inlined += '\n/* Apple Emoji Universal Fix */\n.emoji[src*="/assets/"], img.emoji[src*="/assets/"], [class*="emoji_"][src*="/assets/"] { background-color: transparent !important; }\n';
                        }

                        applyThemeStyle(styleId, inlined);
                        activeThemeStyles.set(styleId, inlined);
                    }
                }
            }

            // 2. Process online theme links
            if (Array.isArray(enabledLinks)) {
                for (const link of enabledLinks) {
                    const cleanLink = link.replace(/^@(light|dark)\s+/, '').trim();
                    const styleId = 'rustcord-theme-link-' + cleanLink.replace(/[^a-zA-Z0-9_-]/g, '_');
                    currentActiveIds.add(styleId);

                    const cacheKey = 'rc_link_v2_' + cleanLink;
                    let inlined = localStorage.getItem(cacheKey);
                    if (!inlined) {
                        try {
                            let text = '';
                            try {
                                const res = await invoke('native_fetch', {
                                    request: { url: cleanLink, method: 'GET', headers: {}, body: null }
                                });
                                if (res && res.status >= 200 && res.status < 300) text = res.body;
                            } catch(e) {
                                const resp = await fetch(cleanLink);
                                if (resp.ok) text = await resp.text();
                            }
                            if (text) {
                                inlined = await resolveAndInlineCss(text);
                                try { localStorage.setItem(cacheKey, inlined); } catch(e) {}
                            }
                        } catch(e) {}
                    }

                    if (inlined) {
                        applyThemeStyle(styleId, inlined);
                        activeThemeStyles.set(styleId, inlined);
                    }
                }
            }

            // 3. Clean up unmounted themes
            for (const [id] of activeThemeStyles) {
                if (!currentActiveIds.has(id)) {
                    removeThemeStyle(id);
                    activeThemeStyles.delete(id);
                }
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
        showToast(text) {
            let t = document.getElementById('rustcord-toast');
            if (!t) {
                t = document.createElement('div');
                t.id = 'rustcord-toast';
                t.style.cssText = 'position:fixed;bottom:24px;right:24px;z-index:2147483647;padding:10px 18px;background:#000;border:1px solid #F84800;border-radius:8px;color:#fff;font:600 13px/-apple-system,BlinkMacSystemFont,sans-serif;box-shadow:0 8px 24px rgba(0,0,0,.95),0 0 12px rgba(248,72,0,.25);transition:opacity .22s;pointer-events:none;display:flex;align-items:center;gap:10px;opacity:0';
                (document.body||document.documentElement).appendChild(t);
            }
            t.innerHTML = '<svg width="15" height="15" viewBox="0 0 127.14 96.36" fill="#F84800"><path d="M107.7,8.07A105.15,105.15,0,0,0,81.47,0a72.06,72.06,0,0,0-3.36,6.83A97.68,97.68,0,0,0,49,6.83,72.37,72.37,0,0,0,45.64,0,105.89,105.89,0,0,0,19.39,8.09C2.79,32.65-1.71,56.6.54,80.21h0A105.73,105.73,0,0,0,32.71,96.36,77.7,77.7,0,0,0,39.6,85.25a68.42,68.42,0,0,1-10.85-5.18c.91-.66,1.8-1.34,2.66-2a75.57,75.57,0,0,0,64.32,0c.87.71,1.76,1.39,2.66,2a68.68,68.68,0,0,1-10.87,5.19,77,77,0,0,0,6.89,11.1A105.25,105.25,0,0,0,126.6,80.21c2.64-27.38-4.51-51.11-18.9-72.15ZM42.45,65.69C36.18,65.69,31,60,31,53s5-12.74,11.43-12.74S54,45.91,53.89,53,48.84,65.69,42.45,65.69Zm42.24,0C78.41,65.69,73.25,60,73.25,53s5-12.74,11.44-12.74S96.23,45.91,96.12,53,91.08,65.69,84.69,65.69Z"/></svg><span>' + text + '</span>';
            t.style.opacity = '1';
            clearTimeout(t._tmr);
            t._tmr = setTimeout(() => { t.style.opacity = '0'; }, 2500);
        }
    };

    console.log('[Rustcord] Pre-load hooks v4 installed.');
})();
"###;

    template
        .replace("%EQUICORD_CSS_JSON%", &css_json)
        .replace("%EQUICORD_JS%", equicord_js)
}
