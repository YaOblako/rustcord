// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebView2 browser-level flags:
    // 1. Chrome 131 User-Agent at process level (headers + navigator object)
    // 2. Disable QUIC/HTTP3 for reliable VPN/proxy routing to gateway.discord.gg
    // 3. Auto-accept media permissions for microphone/camera device enumeration
    // 4. Background stability for WebRTC voice during gaming
    // 5. Disable web-security so fetch() to cloud.equicord.org isn't blocked by CORS
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--user-agent=\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36\" \
         --disable-web-security \
         --disable-quic \
         --use-fake-ui-for-media-stream \
         --autoplay-policy=no-user-gesture-required \
         --disable-background-timer-throttling \
         --disable-backgrounding-occluded-windows \
         --disable-renderer-backgrounding \
         --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
    );

    rustcord_lib::run();
}
