mod equicord;

use log::info;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| u.version))
}

#[tauri::command]
fn window_reveal(window: tauri::WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    let _ = window.request_user_attention(Some(tauri::UserAttentionType::Informational));
}

#[tauri::command]
fn notify_attention(window: tauri::WebviewWindow) {
    let _ = window.request_user_attention(Some(tauri::UserAttentionType::Informational));
}

#[tauri::command]
fn window_minimize(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::WebviewWindow) {
    if let Ok(is_maximized) = window.is_maximized() {
        if is_maximized {
            let _ = window.unmaximize();
        } else {
            let _ = window.maximize();
        }
    }
}

#[tauri::command]
fn window_close(window: tauri::WebviewWindow) {
    // Discord desktop behavior: closing window minimizes to system tray
    let _ = window.hide();
}

#[tauri::command]
fn window_start_dragging(window: tauri::WebviewWindow) {
    let _ = window.start_dragging();
}

#[tauri::command]
fn window_is_maximized(window: tauri::WebviewWindow) -> bool {
    window.is_maximized().unwrap_or(false)
}

#[cfg(target_os = "windows")]
extern "system" {
    fn EnumWindows(lp_enum_func: unsafe extern "system" fn(isize, isize) -> i32, l_param: isize) -> i32;
    fn GetWindowTextW(h_wnd: isize, lp_string: *mut u16, n_max_count: i32) -> i32;
    fn SetWindowPos(h_wnd: isize, h_wnd_insert_after: isize, x: i32, y: i32, cx: i32, cy: i32, u_flags: u32) -> i32;
    fn IsWindowVisible(h_wnd: isize) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: isize,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: i32,
    ) -> isize;
}

fn open_in_default_browser(url: &str) {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("mailto:")) {
        return;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        let op: Vec<u16> = std::ffi::OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let file: Vec<u16> = std::ffi::OsStr::new(trimmed)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            ShellExecuteW(0, op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(trimmed).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(trimmed).spawn();
    }
}

fn is_internal_discord_url(url: &tauri::Url) -> bool {
    let scheme = url.scheme();
    if scheme == "about" || scheme == "blob" || scheme == "data" {
        return true;
    }
    if scheme != "http" && scheme != "https" {
        return false;
    }

    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();

    // Media / attachment CDN links should open in the external browser for viewing or downloading
    if host == "cdn.discordapp.com" || host == "media.discordapp.net" || host == "dl.discordapp.net" {
        return false;
    }

    host == "discord.com"
        || host.ends_with(".discord.com")
        || host == "discordapp.com"
        || host.ends_with(".discordapp.com")
        || host == "accounts.google.com"
        || host == "appleid.apple.com"
        || host == "hcaptcha.com"
        || host.ends_with(".hcaptcha.com")
        || host == "cloudflare.com"
        || host.ends_with(".cloudflare.com")
        || host == "recaptcha.net"
        || host.ends_with(".recaptcha.net")
        || host == "gstatic.com"
        || host.ends_with(".gstatic.com")
        || host == "arkoselabs.com"
        || host.ends_with(".arkoselabs.com")
        || host == "funcaptcha.com"
        || host.ends_with(".funcaptcha.com")
        || host == "www.google.com"
}

#[tauri::command]
fn open_external(url: String) {
    open_in_default_browser(&url);
}

#[cfg(target_os = "windows")]
fn tile_devtools_side_by_side(main_window: &tauri::WebviewWindow) {
    let main_hwnd = match main_window.hwnd() {
        Ok(h) => h.0 as isize,
        Err(_) => return,
    };

    let (mon_x, mon_y, mon_w, mon_h) = if let Ok(Some(mon)) = main_window.current_monitor() {
        let pos = mon.position();
        let size = mon.size();
        (pos.x, pos.y, size.width as i32, size.height as i32)
    } else {
        (0, 0, 1920, 1080)
    };

    let discord_width = (mon_w as f64 * 0.65) as i32;
    let devtools_width = mon_w - discord_width;

    let _ = main_window.unmaximize();
    let _ = main_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: mon_x, y: mon_y }));
    let _ = main_window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
        width: discord_width as u32,
        height: mon_h as u32,
    }));

    std::thread::spawn(move || {
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(100));

            struct EnumData {
                main_hwnd: isize,
                found_hwnd: isize,
            }

            unsafe extern "system" fn enum_proc(hwnd: isize, lparam: isize) -> i32 {
                let data = &mut *(lparam as *mut EnumData);
                if hwnd == data.main_hwnd {
                    return 1;
                }
                let mut buf = [0u16; 512];
                let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
                if len > 0 {
                    let title = String::from_utf16_lossy(&buf[..len as usize]);
                    if title.starts_with("DevTools") && IsWindowVisible(hwnd) != 0 {
                        data.found_hwnd = hwnd;
                        return 0;
                    }
                }
                1
            }

            let mut data = EnumData {
                main_hwnd,
                found_hwnd: 0,
            };

            unsafe {
                EnumWindows(enum_proc, &mut data as *mut EnumData as isize);
            }

            if data.found_hwnd != 0 {
                unsafe {
                    SetWindowPos(
                        data.found_hwnd,
                        0,
                        mon_x + discord_width,
                        mon_y,
                        devtools_width,
                        mon_h,
                        0x0040 | 0x0004,
                    );
                }
                break;
            }
        }
    });
}

#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) {
    if window.is_devtools_open() {
        window.close_devtools();
        let _ = window.maximize();
    } else {
        window.open_devtools();
        #[cfg(target_os = "windows")]
        tile_devtools_side_by_side(&window);
    }
}

pub struct AppState {
    default_icon: tauri::image::Image<'static>,
    speaking_icon: tauri::image::Image<'static>,
    silent_icon: tauri::image::Image<'static>,
}

fn generate_speaking_icon() -> tauri::image::Image<'static> {
    let size = 32usize;
    let c = 15.5f32;
    let r = 13.0f32;
    let mut pixels = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let d = (dx * dx + dy * dy).sqrt();
            let a = if d <= r - 0.5 {
                255u8
            } else if d >= r + 0.5 {
                0u8
            } else {
                ((r + 0.5 - d) * 255.0) as u8
            };
            pixels.extend_from_slice(&[35, 165, 90, a]);
        }
    }
    tauri::image::Image::new_owned(pixels, 32, 32)
}

fn generate_silent_icon() -> tauri::image::Image<'static> {
    let size = 32usize;
    let c = 15.5f32;
    let r_out = 13.0f32;
    let r_in = 9.5f32;
    let mut pixels = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let d = (dx * dx + dy * dy).sqrt();
            let a = if d > r_out + 0.5 {
                0u8
            } else if d >= r_out - 0.5 {
                ((r_out + 0.5 - d) * 255.0) as u8
            } else if d < r_in - 0.5 {
                25u8
            } else if d <= r_in + 0.5 {
                let factor = d - (r_in - 0.5);
                (25.0 + factor * (255.0 - 25.0)) as u8
            } else {
                255u8
            };
            pixels.extend_from_slice(&[35, 165, 90, a]);
        }
    }
    tauri::image::Image::new_owned(pixels, 32, 32)
}

fn build_default_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let open_item = MenuItem::with_id(app, "open", "Открыть Rustcord", true, None::<&str>)?;
    let mute_item = MenuItem::with_id(app, "toggle_mute", "Микрофон (Mute)", true, None::<&str>)?;
    let deaf_item = MenuItem::with_id(app, "toggle_deafen", "Звук (Deafen)", true, None::<&str>)?;
    let game_mode_item = MenuItem::with_id(app, "toggle_game_mode", "Игровой режим", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[&open_item, &mute_item, &deaf_item, &game_mode_item, &quit_item],
    )
}

fn build_call_tray_menu(app: &tauri::AppHandle, channel_name: &str) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let status_label = format!("В звонке: {}", channel_name);
    let status_item = MenuItem::with_id(app, "call_status", &status_label, false, None::<&str>)?;
    let leave_item = MenuItem::with_id(app, "leave_call", "Выйти из звонка", true, None::<&str>)?;
    let mute_item = MenuItem::with_id(app, "toggle_mute", "Микрофон (Mute)", true, None::<&str>)?;
    let deaf_item = MenuItem::with_id(app, "toggle_deafen", "Звук (Deafen)", true, None::<&str>)?;
    let open_item = MenuItem::with_id(app, "open", "Открыть Rustcord", true, None::<&str>)?;
    let game_mode_item = MenuItem::with_id(app, "toggle_game_mode", "Игровой режим", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[&status_item, &leave_item, &mute_item, &deaf_item, &open_item, &game_mode_item, &quit_item],
    )
}

#[tauri::command]
fn update_voice_state(
    app: tauri::AppHandle,
    #[allow(non_snake_case)] inCall: bool,
    #[allow(non_snake_case)] isSpeaking: bool,
    #[allow(non_snake_case)] channelName: Option<String>,
) {
    let state = app.state::<AppState>();
    if let Some(tray) = app.tray_by_id("main_tray") {
        if inCall {
            let icon = if isSpeaking {
                &state.speaking_icon
            } else {
                &state.silent_icon
            };
            let _ = tray.set_icon(Some(icon.clone()));

            let ch_title = channelName.unwrap_or_else(|| "Активен".into());
            let status_text = if isSpeaking {
                format!("Rustcord: В звонке ({}) [Говорит]", ch_title)
            } else {
                format!("Rustcord: В звонке ({})", ch_title)
            };
            let _ = tray.set_tooltip(Some(&status_text));

            if let Ok(menu) = build_call_tray_menu(&app, &ch_title) {
                let _ = tray.set_menu(Some(menu));
            }
        } else {
            let _ = tray.set_icon(Some(state.default_icon.clone()));
            let _ = tray.set_tooltip(Some("Rustcord"));

            if let Ok(menu) = build_default_tray_menu(&app) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
}

#[tauri::command]
fn app_ready(app: tauri::AppHandle) {
    info!("[Rustcord] App reported READY. Dismissing splashscreen and revealing main window...");
    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

#[derive(serde::Deserialize)]
pub struct NativeFetchRequest {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
}

#[derive(serde::Serialize)]
pub struct NativeFetchResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

#[tauri::command]
fn native_fetch(request: NativeFetchRequest) -> Result<NativeFetchResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let method = match request.method.as_deref().unwrap_or("GET").to_uppercase().as_str() {
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "HEAD" => reqwest::Method::HEAD,
        _ => reqwest::Method::GET,
    };

    let mut req_builder = client.request(method, &request.url);

    if let Some(headers) = request.headers {
        for (k, v) in headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(&v),
            ) {
                if name != reqwest::header::HOST && name != reqwest::header::CONTENT_LENGTH {
                    req_builder = req_builder.header(name, val);
                }
            }
        }
    }

    if let Some(body) = request.body {
        req_builder = req_builder.body(body);
    }

    let resp = req_builder.send().map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("OK").to_string();

    let mut resp_headers = std::collections::HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(val) = v.to_str() {
            resp_headers.insert(k.as_str().to_string(), val.to_string());
        }
    }

    let body = resp.text().unwrap_or_default();

    Ok(NativeFetchResponse {
        status,
        status_text,
        headers: resp_headers,
        body,
    })
}

#[tauri::command]
fn open_themes_folder() -> Result<(), String> {
    let themes_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.rustcord.app")
        .join("themes");
    let _ = std::fs::create_dir_all(&themes_dir);

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg(themes_dir.to_string_lossy().as_ref())
            .spawn();
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct LocalThemeInfo {
    pub name: String,
    pub filename: String,
    pub css: String,
}

#[tauri::command]
fn get_local_themes() -> Result<Vec<LocalThemeInfo>, String> {
    let mut dirs_to_scan = Vec::new();

    // 1. App local themes folder
    if let Some(local) = dirs::data_local_dir() {
        let app_themes = local.join("com.rustcord.app").join("themes");
        let _ = std::fs::create_dir_all(&app_themes);

        // Pre-seed default EmojiReplace theme (Apple Emojis by DevilBro) if not already created
        let emoji_theme = app_themes.join("EmojiReplace.theme.css");
        if !emoji_theme.exists() {
            let default_emoji_css = r#"/**
 * @name EmojiReplace
 * @description Replaces Discord's Emojis with Emojis of a different Provider (Apple, Facebook...)
 * @author DevilBro
 * @version 1.0.0
 * @authorId 278543574059057154
 * @website https://mwittrien.github.io/
 * @source https://github.com/mwittrien/BetterDiscordAddons/tree/master/Themes/EmojiReplace/
 */

@import url(https://mwittrien.github.io/BetterDiscordAddons/Themes/EmojiReplace/base/Apple.css);
"#;
            let _ = std::fs::write(&emoji_theme, default_emoji_css);
        }
        dirs_to_scan.push(app_themes);
    }

    // 2. Scan Equicord, BetterDiscord, and Vencord themes folders in Roaming AppData
    if let Some(config) = dirs::config_dir() {
        dirs_to_scan.push(config.join("Equicord").join("themes"));
        dirs_to_scan.push(config.join("BetterDiscord").join("themes"));
        dirs_to_scan.push(config.join("Vencord").join("themes"));
    }

    let mut list = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for themes_dir in dirs_to_scan {
        if let Ok(entries) = std::fs::read_dir(&themes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if filename.ends_with(".css") {
                        let name = filename.replace(".theme.css", "").replace(".css", "");
                        if seen_names.insert(name.clone()) {
                            if let Ok(css) = std::fs::read_to_string(&path) {
                                list.push(LocalThemeInfo {
                                    name,
                                    filename,
                                    css,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(list)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Disable web security for WebView2 to allow theme loading, external fonts, and OAuth callbacks
    #[cfg(target_os = "windows")]
    {
        let current_args = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
        let new_args = format!(
            "{} --disable-web-security --allow-running-insecure-content --autoplay-policy=no-user-gesture-required --disable-background-timer-throttling --disable-renderer-backgrounding",
            current_args
        );
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", new_args.trim());
    }

    tauri::Builder::default()
        // 1. Single Instance Protection (Prevents multiple instances/windows from Windows Search/Start Menu)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            info!("[Rustcord] Secondary process launch intercepted. Focusing primary instance.");
            if let Some(splash) = app.get_webview_window("splashscreen") {
                let _ = splash.close();
            }
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.unminimize();
                let _ = main.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            check_for_updates,
            window_reveal,
            notify_attention,
            window_minimize,
            window_toggle_maximize,
            window_close,
            window_start_dragging,
            window_is_maximized,
            open_devtools,
            update_voice_state,
            app_ready,
            native_fetch,
            open_themes_folder,
            get_local_themes,
            open_external
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            info!("[Rustcord] Initializing client backend...");

            let window_icon = app.default_window_icon().cloned().expect("window icon required");

            // 2. Prepare Equicord bundle & script injection
            let (equicord_js, equicord_css) = equicord::load_equicord_bundle();
            let init_script = equicord::build_initialization_script(&equicord_js, &equicord_css);

            // 3. Create Vertical Splashscreen window (300x350, frameless, dark background)
            let _splash = tauri::WebviewWindowBuilder::new(
                app,
                "splashscreen",
                tauri::WebviewUrl::App("splash.html".into()),
            )
            .title("Rustcord")
            .icon(window_icon.clone())?
            .inner_size(300.0, 350.0)
            .resizable(false)
            .decorations(false)
            .center()
            .shadow(true)
            .build()?;

            info!("[Rustcord] Splashscreen window displayed (300x350).");

            // 4. Create Main Discord Webview window (1280x800, min 940x600, frameless, initially hidden)
            let _main_window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External("https://discord.com/app".parse().expect("valid Discord URL")),
            )
            .title("Rustcord")
            .icon(window_icon.clone())?
            .inner_size(1280.0, 800.0)
            .min_inner_size(940.0, 600.0)
            .resizable(true)
            .decorations(false)
            .center()
            .shadow(true)
            .visible(false)
            .disable_drag_drop_handler()
            .on_new_window(|url, _features| {
                let raw = url.as_str();
                if raw == "about:blank" {
                    return tauri::webview::NewWindowResponse::Allow;
                }
                if let Some(host) = url.host_str() {
                    if host.ends_with("discord.com") && url.path().starts_with("/popout") {
                        return tauri::webview::NewWindowResponse::Allow;
                    }
                }
                open_in_default_browser(raw);
                tauri::webview::NewWindowResponse::Deny
            })
            .on_navigation(|url| {
                if is_internal_discord_url(&url) {
                    true
                } else {
                    open_in_default_browser(url.as_str());
                    false
                }
            })
            .on_download(|_webview, event| {
                if let tauri::webview::DownloadEvent::Requested { url, .. } = event {
                    open_in_default_browser(url.as_str());
                }
                false
            })
            .initialization_script(&init_script)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()?;

            info!("[Rustcord] Main webview window prepared for https://discord.com/app.");

            // 5. Safety Transition Timer (Ensures main window opens even on slow network connections)
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(5000));
                if let Some(splash) = app_handle.get_webview_window("splashscreen") {
                    let _ = splash.close();
                }
                if let Some(main) = app_handle.get_webview_window("main") {
                    if let Ok(is_visible) = main.is_visible() {
                        if !is_visible {
                            info!("[Rustcord] Transition timer elapsed. Revealing main window.");
                            let _ = main.show();
                            let _ = main.unminimize();
                            let _ = main.set_focus();
                        }
                    }
                }
            });

            // 6. System Tray Setup (Single persistent tray with dynamic call controls & speaking indicator)
            let default_icon = tauri::image::Image::new_owned(
                window_icon.rgba().to_vec(),
                window_icon.width(),
                window_icon.height(),
            );
            let speaking_icon = generate_speaking_icon();
            let silent_icon = generate_silent_icon();

            app.manage(AppState {
                default_icon: default_icon.clone(),
                speaking_icon,
                silent_icon,
            });

            let tray_menu = build_default_tray_menu(app.handle())?;

            let _tray = TrayIconBuilder::with_id("main_tray")
                .icon(window_icon)
                .tooltip("Rustcord")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if let Some(window) = app.get_webview_window("main") {
                        match event.id.as_ref() {
                            "open" => {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                            "leave_call" => {
                                let _ = window.eval("window.__RUSTCORD__ && window.__RUSTCORD__.disconnectVoice();");
                            }
                            "toggle_mute" => {
                                let _ = window.eval("window.__RUSTCORD__ && window.__RUSTCORD__.toggleMute();");
                            }
                            "toggle_deafen" => {
                                let _ = window.eval("window.__RUSTCORD__ && window.__RUSTCORD__.toggleDeafen();");
                            }
                            "toggle_game_mode" => {
                                let _ = window.eval("window.__RUSTCORD__ && window.__RUSTCORD__.toggleGameMode();");
                            }
                            "quit" => {
                                app.exit(0);
                            }
                            _ => {}
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // Only react on MouseButtonState::Up to avoid double-triggering
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(is_visible) = window.is_visible() {
                                if is_visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            info!("[Rustcord] System tray registered.");

            // 7. Global Hotkeys for In-Game Overlay Actions (PTT/Mute/Deafen/Game Mode)
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(|app, shortcut, event| {
                        if event.state() == ShortcutState::Pressed {
                            if let Some(win) = app.get_webview_window("main") {
                                if shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyM) {
                                    let _ = win.eval("window.__RUSTCORD__ && window.__RUSTCORD__.toggleMute();");
                                } else if shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyD) {
                                    let _ = win.eval("window.__RUSTCORD__ && window.__RUSTCORD__.toggleDeafen();");
                                } else if shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyG) {
                                    let _ = win.eval("window.__RUSTCORD__ && window.__RUSTCORD__.toggleGameMode();");
                                }
                            }
                        }
                    })
                    .build(),
            )?;

            let mute_sc = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyM);
            let deaf_sc = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyD);
            let game_sc = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyG);

            let _ = app.global_shortcut().register(mute_sc);
            let _ = app.global_shortcut().register(deaf_sc);
            let _ = app.global_shortcut().register(game_sc);

            info!("[Rustcord] Global shortcuts registered (Ctrl+Shift+M, Ctrl+Shift+D, Ctrl+Shift+G).");

            // 7. Auto-updater background check on launch (Discord-style silent file update)
            let updater_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Clean up any old executable leftover from previous in-place file replacement
                if let Ok(exe_path) = std::env::current_exe() {
                    let old_path = exe_path.with_extension("exe.old");
                    let _ = std::fs::remove_file(old_path);
                }

                std::thread::sleep(std::time::Duration::from_secs(8));
                tauri::async_runtime::block_on(async move {
                    if let Ok(updater) = updater_handle.updater() {
                        match updater.check().await {
                            Ok(Some(update)) => {
                                info!("[Rustcord Updater] New update available: v{}. Downloading file update...", update.version);
                                let _ = update.download_and_install(|_, _| {}, || {
                                    info!("[Rustcord Updater] File update downloaded and applied in-place successfully.");
                                }).await;
                            }
                            Ok(None) => {
                                info!("[Rustcord Updater] Application is up to date.");
                            }
                            Err(e) => {
                                info!("[Rustcord Updater] Update check: {}", e);
                            }
                        }
                    }
                });
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Minimize to system tray on window close button (X)
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
