//! The EditsMaker app, inside edits: t3code's UI ported to GPUI (gpui-kit), in the brand's colours and type, Lucide
//! icons. `edits app` opens it; `--review CLIP` only the review screen (for `edits review`); `market`, `automations`
//! or `settings` opens there. Long jobs run as `edits` itself beside it, so their output streams and stays apart.
mod chat_view;
mod composer;
mod flows;
mod icons;
mod marketplace;
mod review;
mod runs;
mod settings;
mod sidebar;
mod theme;
mod ui;
mod workspace;

use gpui_kit::*;
use std::path::PathBuf;

actions!(edits, [Quit]);

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

pub fn run(args: &[String]) {
    // started from the Start menu, edits.exe has a console it does not need
    #[cfg(windows)]
    unsafe {
        FreeConsole();
    }
    let review = args.iter().position(|a| a == "--review").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let start = [("market", workspace::Route::Market), ("automations", workspace::Route::Automations)]
        .into_iter().find(|(f, _)| args.iter().any(|a| a.trim_start_matches('-') == *f)).map(|x| x.1);
    // settings, or settings ai|accounts|agents|appearance|about: the settings window, there
    let settings = args.iter().position(|a| a.trim_start_matches('-') == "settings").map(|i| match args.get(i + 1).map(String::as_str) {
        Some("ai") => workspace::Section::Ai,
        Some("accounts") => workspace::Section::Accounts,
        Some("agents") => workspace::Section::Agents,
        Some("appearance") => workspace::Section::Appearance,
        Some("about") => workspace::Section::About,
        _ => workspace::Section::General,
    });
    let args_: Vec<String> = args.to_vec();
    gpui_kit::application().with_assets(AppAssets).run(move |cx| {
        gpui_kit::init(cx);
        // Windows posts system notifications under the app's identity
        cx.set_app_identity("io.github.onurbyte.edits", "EditsMaker");
        cx.bind_keys([KeyBinding::new(if cfg!(target_os = "macos") { "cmd-q" } else { "ctrl-q" }, Quit, None)]);
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
            }
            if let Some(at) = settings {
                w.open_settings(at, cx);
                // settings ai add: t3code's Add provider, open
                w.ai_adding = args_.iter().any(|a| a == "add");
            }
            w
        })).expect("could not open the window");
        cx.activate(true);
    });
}
