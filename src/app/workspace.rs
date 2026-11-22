//! The window: t3code's AppSidebarLayout (sidebar on the left, the route on the right) holding the app's state. The
//! screens are in sidebar.rs, chat_view.rs, composer.rs, marketplace.rs, settings.rs and review.rs, each an
//! `impl Workspace`.
use crate::app::runs::{self, Flow, Job, Listed, Note, Pack, Run, Settings, Sheet};
use crate::app::theme::{self, brand};
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub enum Route {
    New,
    Job(usize),
    Run(usize),
    /// the marketplace's packs (presets and skills)
    Market,
    /// automations, sources and flows
    Automations,
    Review,
}

/// t3code's settings sections, as edits has them.
#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    General,
    Ai,
    Accounts,
    Agents,
    Appearance,
    About,
}

/// What the composer makes: edits from a link, a channel, a topic or a file; a pack from a video; or videos from an
/// RSS pack's newest posts.
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Edit,
    Pack,
    Rss,
}

/// The list open above the composer, or in a new flow.
#[derive(Clone, Copy, PartialEq)]
pub enum Picker {
    Preset,
    Source,
    FlowSource,
}

/// The `edits` commands the composer runs: one per attached file (or one for what is typed), with the chosen
/// preset, RSS source and count.
pub fn commands(mode: Mode, text: &str, files: &[PathBuf], preset: Option<&str>, source: Option<&str>, count: usize) -> Vec<Vec<String>> {
    let words = runs::split_args(text);
    let mut extra: Vec<String> = vec![];
    if let Some(p) = preset.filter(|_| mode != Mode::Pack && !words.iter().any(|w| w == "--preset")) {
        extra.extend(["--preset".to_string(), p.to_string()]);
    }
    if count > 1 && mode != Mode::Pack {
        extra.extend(["-n".to_string(), count.to_string()]);
    }
    match mode {
        Mode::Rss => source.map(|s| [vec!["rss".to_string(), s.to_string()], words, extra].concat()).into_iter().collect(),
        Mode::Pack => {
            let head = |input: String| [vec!["packs".into(), "create".into(), input], words.clone()].concat();
            if files.is_empty() { words.first().map(|_| [vec!["packs".to_string(), "create".to_string()], words.clone()].concat()).into_iter().collect() } else { files.iter().map(|f| head(f.display().to_string())).collect() }
        }
        Mode::Edit if files.is_empty() => if words.is_empty() { vec![] } else { vec![[words, extra].concat()] },
        Mode::Edit => files.iter().map(|f| [vec![f.display().to_string()], words.clone(), extra.clone()].concat()).collect(),
    }
}

#[derive(Default)]
pub struct Market {
    pub packs: Option<Result<Vec<Pack>, String>>,
    pub listed: Option<Result<Vec<Listed>, String>>,
    /// the pack an add, remove or trust is running on
    pub busy: Option<String>,
    /// built-in, community, local, or all
    pub origin: Option<&'static str>,
    pub tag: Option<String>,
    pub newest: bool,
}

pub struct Review {
    pub clip: PathBuf,
    pub sheet: Option<Result<Sheet, String>>,
    /// the frame shown
    pub at: usize,
    /// where in it the next note points (0..1 across, down)
    pub spot: Option<(f64, f64)>,
    pub notes: Vec<Note>,
    pub note: Entity<InputState>,
    pub general: Entity<InputState>,
}

pub struct Workspace {
    pub input: Entity<TextareaState>,
    pub mode: Mode,
    /// files dropped or picked, each made on its own
    pub files: Vec<PathBuf>,
    pub preset: Option<String>,
    /// the RSS pack, by its short name
    pub source: Option<String>,
    pub count: usize,
    pub picker: Option<Picker>,
    pub flows: Option<Result<Vec<Flow>, String>>,
    /// a new flow: its source, the accounts it posts to, minutes between checks
    pub flow_source: Option<String>,
    pub flow_post: Vec<String>,
    pub flow_every: u64,
    pub jobs: Vec<Job>,
    pub past: Vec<Run>,
    pub route: Route,
    pub show_log: bool,
    pub market: Market,
    pub settings: Option<Result<Settings, String>>,
    /// the settings window, over everything, as t3code opens it
    pub settings_open: Option<Section>,
    /// the AI provider being set up, its key and its server, and the models it offers
    pub ai_open: Option<String>,
    pub ai_key: Entity<InputState>,
    pub ai_endpoint: Entity<InputState>,
    pub ai_models: Option<Result<Vec<String>, String>>,
    /// t3code's Add provider window, and whether it shows every provider
    pub ai_adding: bool,
    pub ai_more: bool,
    /// what `edits doctor` said, in About
    pub doctor: Option<String>,
    /// the setting being changed
    pub changing: Option<String>,
    pub review: Option<Review>,
    /// started by `edits review CLIP`: saving the notes closes the app
    pub review_only: bool,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(review: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        theme::apply(Some(window), cx);
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 6).submit_on_enter(true)
            .placeholder("A link, a channel or a topic, or drop a video. Enter sends, Shift+Enter is a new line"));
        let subscriptions = vec![
            cx.subscribe_in(&input, window, |this, _, e: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = e {
                    this.start(window, cx);
                }
            }),
            cx.observe_window_appearance(window, |_, window, cx| theme::apply(Some(window), cx)),
        ];
        let ai_key = cx.new(|cx| InputState::new(window, cx).masked(true).placeholder("API key"));
        let ai_endpoint = cx.new(|cx| InputState::new(window, cx).placeholder("http://127.0.0.1:11434/v1"));
        // following Omarchy: when its theme changes, so does the app's
        cx.spawn_in(window, async move |this, cx| {
            let mut seen = theme::omarchy_generation();
            loop {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                let now = theme::omarchy_generation();
                if now != seen {
                    seen = now;
                    if this.update_in(cx, |_, window, cx| {
                        theme::apply(Some(window), cx);
                        window.refresh();
                    }).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();
        let mut this = Self { input, mode: Mode::Edit, settings_open: None, ai_open: None, ai_key, ai_endpoint, ai_models: None, ai_adding: false, ai_more: false, files: vec![], preset: None, source: None, count: 1, picker: None, flows: None,
            flow_source: None, flow_post: vec![], flow_every: 60, jobs: vec![], past: runs::past(&runs::out_dir()), route: Route::New, show_log: false,
            market: Market::default(), settings: None, doctor: None, changing: None, review: None, review_only: review.is_some(), _subscriptions: subscriptions };
        if let Some(clip) = review {
            this.open_review(clip, window, cx);
        } else {
            // the composer's preset and source lists
            this.load_market(cx);
        }
        this
    }

    pub fn go(&mut self, to: Route, cx: &mut Context<Self>) {
        self.route = to;
        if matches!(to, Route::Market | Route::Automations) && self.market.packs.is_none() {
            self.load_market(cx);
        }
        if to == Route::Automations && self.settings.is_none() {
            self.load_settings(cx);
        }
        if to == Route::Automations {
            self.load_flows(cx);
        }
        self.picker = None;
        cx.notify();
    }

    /// Runs what the composer holds: edits, a pack, or videos from RSS.
    pub fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let list = commands(self.mode, &text, &self.files, self.preset.as_deref(), self.source.as_deref(), self.count);
        if list.is_empty() {
            let what = if self.mode == Mode::Rss { "Pick an RSS source first" } else { "Type a link or a topic, or drop a video" };
            window.push_notification(Notification::info(what), cx);
            return;
        }
        for args in list {
            self.launch(args, window, cx);
        }
        self.files.clear();
        self.picker = None;
        self.input.update(cx, |s, cx| s.set_value("", window, cx));
        cx.notify();
    }

    fn launch(&mut self, args: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.jobs.push(Job::start(args));
        let i = self.jobs.len() - 1;
        self.route = Route::Job(i);
        // the job's lines arrive on threads: look a few times a second until it ends, then say so once
        cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(250)).await;
            let ended = this.update_in(cx, |this, window, cx| {
                cx.notify();
                let ok = this.jobs[i].finished()?;
                this.jobs[i].took = Some(this.jobs[i].started.elapsed());
                this.past = runs::past(&runs::out_dir());
                let what = this.jobs[i].title();
                let note = if ok { Notification::success(what).title("Ready") } else { Notification::error(what).title("Did not work") };
                window.push_notification(note.in_app_and_system(), cx);
                Some(())
            });
            if !matches!(ended, Ok(None)) {
                break;
            }
        })
        .detach();
    }

    pub fn load_flows(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let f = cx.background_executor().spawn(async { runs::flows() }).await;
            let _ = this.update(cx, |this, cx| {
                this.flows = Some(f);
                cx.notify();
            });
        })
        .detach();
    }

    /// A file dropped or picked is attached to the composer.
    pub fn put(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        if !self.files.iter().any(|f| f == path) {
            self.files.push(path.to_path_buf());
        }
        if self.mode == Mode::Rss {
            self.mode = Mode::Edit;
        }
        self.input.update(cx, |s, cx| s.focus(window, cx));
        if !matches!(self.route, Route::New | Route::Job(_) | Route::Run(_)) {
            self.route = Route::New;
        }
        cx.notify();
    }

    pub fn pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: true, prompt: None });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                let _ = this.update_in(cx, |this, window, cx| paths.iter().for_each(|p| this.put(p, window, cx)));
            }
        })
        .detach();
    }

    pub fn load_market(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let (packs, listed) = cx.background_executor().spawn(async { (runs::packs(), runs::listed()) }).await;
            let _ = this.update(cx, |this, cx| {
                (this.market.packs, this.market.listed) = (Some(packs), Some(listed));
                cx.notify();
            });
        })
        .detach();
    }

    /// `edits packs add|remove|trust|on|off|share ID`, then the marketplace again, and a word on how it went.
    pub fn pack_action(&mut self, verb: &'static str, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.market.busy = Some(id.clone());
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let args = ["packs", verb, id.as_str()];
            let r = cx.background_executor().spawn({
                let args = args.map(String::from);
                async move { runs::output(&args.iter().map(String::as_str).collect::<Vec<_>>()) }
            }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.market.busy = None;
                let note = match r {
                    Ok(_) => Notification::success(format!("{id}: {}", match verb { "add" => "added", "remove" => "deleted", "on" => "on", "off" => "off", "share" => "packed up; attach it to the GitHub issue that opened", _ => "trusted" })),
                    Err(e) => Notification::error(e).title(format!("{verb} {id}")),
                };
                window.push_notification(note, cx);
                this.load_market(cx);
            });
        })
        .detach();
    }

    /// The settings window, at a section.
    pub fn open_settings(&mut self, at: Section, cx: &mut Context<Self>) {
        self.settings_open = Some(at);
        if self.settings.is_none() {
            self.load_settings(cx);
        }
        cx.notify();
    }

    pub fn load_settings(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let s = cx.background_executor().spawn(async { runs::settings() }).await;
            let _ = this.update(cx, |this, cx| {
                this.settings = Some(s);
                cx.notify();
            });
        })
        .detach();
    }

    /// Changes a setting with its own edits command (`edits ai codex`, `edits update off`), then reads them all again.
    pub fn change(&mut self, args: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.changing = Some(args.join(" "));
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let what = args.join(" ");
            let r = cx.background_executor().spawn(async move { runs::output(&args.iter().map(String::as_str).collect::<Vec<_>>()) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.changing = None;
                match r {
                    Err(e) => window.push_notification(Notification::error(e).title(format!("edits {what}")), cx),
                    Ok(out) if !out.trim().is_empty() => window.push_notification(Notification::success(out.lines().next().unwrap_or("").to_string()), cx),
                    _ => {}
                }
                this.load_settings(cx);
                if this.route == Route::Automations {
                    this.load_flows(cx);
                }
            });
        })
        .detach();
    }

    /// `edits doctor`, for About.
    pub fn check(&mut self, cx: &mut Context<Self>) {
        self.doctor = Some("Checking...".into());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let out = cx.background_executor().spawn(async { runs::output(&["doctor"]).unwrap_or_else(|e| e) }).await;
            let _ = this.update(cx, |this, cx| {
                this.doctor = Some(out);
                cx.notify();
            });
        })
        .detach();
    }

    pub fn open_review(&mut self, clip: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let note = cx.new(|cx| InputState::new(window, cx).placeholder("What to change here"));
        let general = cx.new(|cx| InputState::new(window, cx).placeholder("About the whole edit (the song, the length)"));
        self._subscriptions.push(cx.subscribe_in(&note, window, |this, _, e: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = e {
                this.add_note(window, cx);
            }
        }));
        self.review = Some(Review { clip: clip.clone(), sheet: None, at: 0, spot: None, notes: vec![], note, general });
        self.route = Route::Review;
        cx.spawn(async move |this, cx| {
            let sheet = cx.background_executor().spawn(async move { runs::sheet(&clip) }).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(r) = this.review.as_mut() {
                    r.sheet = Some(sheet);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let main = match self.route {
            Route::Market => self.marketplace(false, window, cx).into_any_element(),
            Route::Automations => self.marketplace(true, window, cx).into_any_element(),
            Route::Review => self.review_screen(window, cx).into_any_element(),
            _ => self.chat_view(window, cx).into_any_element(),
        };
        let settings = self.settings_open.map(|at| self.settings_window(at, window, cx));
        div().size_full().relative().flex().bg(b.page).text_color(b.ink).font_family(theme::FONT).font_weight(theme::LIGHT)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                if let Some(p) = paths.paths().first() {
                    this.put(&PathBuf::from(p), window, cx);
                }
            }))
            .when(!self.review_only, |d| d.child(self.sidebar(cx)))
            .child(div().flex_1().min_w_0().h_full().flex().flex_col().child(main))
            .children(settings)
    }
}

#[cfg(test)]
mod tests {
    // not super::*: gpui_kit's own `test` attribute would come with it
    use super::{commands, Mode};
    use std::path::PathBuf;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn the_composer_runs_one_edit_per_file_with_its_choices() {
        let files = [PathBuf::from("/v/a b.mp4"), PathBuf::from("/v/c.mp4")];
        assert_eq!(commands(Mode::Edit, "--music trend:1", &files, Some("akis-whip"), None, 3),
            [s(&["/v/a b.mp4", "--music", "trend:1", "--preset", "akis-whip", "-n", "3"]), s(&["/v/c.mp4", "--music", "trend:1", "--preset", "akis-whip", "-n", "3"])]);
        assert_eq!(commands(Mode::Edit, "https://youtu.be/x --preset mine", &[], Some("akis-whip"), None, 1), [s(&["https://youtu.be/x", "--preset", "mine"])], "a typed preset wins");
        assert!(commands(Mode::Edit, "  ", &[], None, None, 1).is_empty());
        assert_eq!(commands(Mode::Rss, "", &[], None, Some("football"), 3), [s(&["rss", "football", "-n", "3"])]);
        assert!(commands(Mode::Rss, "x", &[], None, None, 1).is_empty(), "no source picked");
        assert_eq!(commands(Mode::Pack, "--name mine", &files[..1], Some("x"), None, 3), [s(&["packs", "create", "/v/a b.mp4", "--name", "mine"])]);
    }
}
