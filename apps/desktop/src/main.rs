//! The EditsMaker app: t3code's UI ported to GPUI (gpui-kit) in the brand's colours and type, over the `edits` beside
//! it. `edits-app --review CLIP` opens only the review screen, for `edits review`; `--market`, `--automations` or
//! `--settings` opens there.
#![cfg_attr(windows, windows_subsystem = "windows")]
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let review = args.iter().position(|a| a == "--review").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx| {
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
        // --market, --automations, --settings: open there
        let start = [("--market", workspace::Route::Market), ("--automations", workspace::Route::Automations),
            ("--settings", workspace::Route::Settings(workspace::Section::General))].into_iter().find(|(f, _)| args.iter().any(|a| a == f)).map(|x| x.1);
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| {
            let mut w = workspace::Workspace::new(review, window, cx);
            if let Some(to) = start {
                w.go(to, cx);
            }
            w
        })).expect("could not open the window");
        cx.activate(true);
    });
}
