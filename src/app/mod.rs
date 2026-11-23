//! The EditsMaker app, inside edits: t3code's UI ported to GPUI (gpui-kit), in the brand's colours and type, Lucide
//! icons. `edits app` opens it; `--review CLIP` only the review screen (for `edits review`); `market`, `automations`
//! or `settings` opens there. Long jobs run as `edits` itself beside it, so their output streams and stays apart.
mod chat_view;
mod composer;
mod discord;
mod flows;
mod library;
mod icons;
mod marketplace;
mod player;
mod review;
mod runs;
pub mod settings;
mod sidebar;
mod theme;
mod ui;
mod usage;
mod welcome;
mod workspace;

use gpui_kit::*;
use std::path::PathBuf;

actions!(edits, [Quit, ToggleSidebar, Search]);


/// gpui-kit's own icons, and the Lucide ones the app adds.
struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        match icons::Extra.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v = gpui_kit::assets::Assets.list(path)?;
        v.extend(icons::Extra.list(path)?);
        v.sort();
        v.dedup();
        Ok(v)
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn FreeConsole() -> i32;
}

/// The app in the Linux launcher: its icon in the user's icon theme and a .desktop entry naming it, written again when
/// they differ (an old install pointed at a stock icon that shows blank). Done each start, so `edits update` mends it.
pub fn launcher() -> std::io::Result<()> {
    if !cfg!(target_os = "linux") {
        return Ok(());
    }
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| crate::util::home().join(".local/share"));
    let exe = std::env::current_exe()?;
    // a cargo build is someone working on edits: the launcher stays on the installed one
    if exe.components().any(|c| c.as_os_str() == "target") {
        return Ok(());
    }
    let files: [(PathBuf, Vec<u8>); 3] = [
        (data.join("icons/hicolor/scalable/apps/edits.svg"), include_bytes!("../../assets/icon.svg").to_vec()),
        (data.join("icons/hicolor/512x512/apps/edits.png"), include_bytes!("../../assets/icon.png").to_vec()),
        (data.join("applications/edits.desktop"), desktop_entry(&exe).into_bytes()),
    ];
    for (f, bytes) in files {
        if std::fs::read(&f).ok().as_deref() != Some(&bytes[..]) {
            std::fs::create_dir_all(f.parent().unwrap_or(std::path::Path::new(".")))?;
            std::fs::write(&f, bytes)?;
        }
    }
    Ok(())
}

pub fn desktop_entry(exe: &std::path::Path) -> String {
    format!("[Desktop Entry]\nType=Application\nName=EditsMaker\nComment=The AI video harness.\nExec={} app\nIcon=edits\nCategories=AudioVideo;Video;\nStartupWMClass=edits\n", exe.display())
}

pub fn run(args: &[String]) {
    let _ = launcher();
    // the installer's step: the launcher written, nothing opened
    if args.iter().any(|a| a == "--launcher") {
        return;
    }
    // started from the Start menu, edits.exe has a console it does not need
    #[cfg(windows)]
    unsafe {
        FreeConsole();
    }
    let review = args.iter().position(|a| a == "--review").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let start = [("market", workspace::Route::Market), ("automations", workspace::Route::Automations), ("library", workspace::Route::Library)]
        .into_iter().find(|(f, _)| args.iter().any(|a| a.trim_start_matches('-') == *f)).map(|x| x.1);
    // settings, or settings ai|accounts|agents|appearance|about: the settings window, there
    let settings = args.iter().position(|a| a.trim_start_matches('-') == "settings").map(|i| match args.get(i + 1).map(String::as_str) {
        Some("ai") => workspace::Section::Ai,
        Some("accounts") => workspace::Section::Accounts,
        Some("agents") => workspace::Section::Agents,
        Some("usage") => workspace::Section::Usage,
        Some("appearance") => workspace::Section::Appearance,
        Some("about") => workspace::Section::About,
        _ => workspace::Section::General,
    });
    let args_: Vec<String> = args.to_vec();
    gpui_kit::application().with_assets(AppAssets).run(move |cx| {
        gpui_kit::init(cx);
        // Windows posts system notifications under the app's identity
        cx.set_app_identity("io.github.onurbyte.edits", "EditsMaker");
        let (q, b, k) = if cfg!(target_os = "macos") { ("cmd-q", "cmd-b", "cmd-k") } else { ("ctrl-q", "ctrl-b", "ctrl-k") };
        cx.bind_keys([KeyBinding::new(q, Quit, None), KeyBinding::new(b, ToggleSidebar, None), KeyBinding::new(k, Search, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1200.), px(800.)), cx)),
            window_min_size: Some(size(px(800.), px(520.))),
            titlebar: Some(TitlebarOptions { title: Some("EditsMaker".into()), ..Default::default() }),
            app_id: Some("edits".into()),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| {
            let mut w = workspace::Workspace::new(review, window, cx);
            if let Some(to) = start {
                w.go(to, cx);
                // market PACK-ID: that pack's sheet, open
                w.market.open = args_.iter().find(|a| a.contains('.') && !a.starts_with('-')).cloned();
            }
            if let Some(at) = settings {
                w.open_settings(at, cx);
                // settings ai add: t3code's Add provider, open
                w.ai_adding = args_.iter().any(|a| a == "add");
            }
            if args_.iter().any(|a| a == "welcome") {
                w.welcome = true;
            }
            w
        })).expect("could not open the window");
        cx.activate(true);
    });
}

#[cfg(test)]
mod launcher_tests {
    #[test]
    fn the_launcher_names_its_own_icon() {
        let d = super::desktop_entry(std::path::Path::new("/home/a/.local/bin/edits"));
        assert!(d.contains("Icon=edits\n") && d.contains("Exec=/home/a/.local/bin/edits app\n") && !d.contains("video-x-generic"));
    }
}
