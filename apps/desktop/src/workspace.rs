//! The window: t3code's AppSidebarLayout (sidebar on the left, the route on the right) holding the app's state. The
//! screens are in sidebar.rs, chat_view.rs, composer.rs, marketplace.rs, settings.rs and review.rs, each an
//! `impl Workspace`.
use crate::runs::{self, Job, Listed, Note, Pack, Run, Settings, Sheet};
use crate::theme::{self, brand};
use gpui_kit::component::input::{InputEvent, InputState};
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
    /// under the marketplace: automations
    Automations,
    Settings(Section),
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

/// What the composer makes: edits from a link, a channel, a topic or a file, or a pack from a video.
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Edit,
    Pack,
}

#[derive(Default)]
pub struct Market {
    pub packs: Option<Result<Vec<Pack>, String>>,
    pub listed: Option<Result<Vec<Listed>, String>>,
    /// the pack an add, remove or trust is running on
    pub busy: Option<String>,
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
    pub input: Entity<InputState>,
    pub mode: Mode,
    pub jobs: Vec<Job>,
    pub past: Vec<Run>,
    pub route: Route,
    pub show_log: bool,
    pub market: Market,
    pub settings: Option<Result<Settings, String>>,
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
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("A link, a channel, a topic or a video. Options after it: --preset NAME"));
        let subscriptions = vec![
            cx.subscribe_in(&input, window, |this, _, e: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = e {
                    this.start(window, cx);
                }
            }),
            cx.observe_window_appearance(window, |_, window, cx| theme::apply(Some(window), cx)),
        ];
        let mut this = Self { input, mode: Mode::Edit, jobs: vec![], past: runs::past(&runs::out_dir()), route: Route::New, show_log: false,
            market: Market::default(), settings: None, doctor: None, changing: None, review: None, review_only: review.is_some(), _subscriptions: subscriptions };
        if let Some(clip) = review {
            this.open_review(clip, window, cx);
        }
        this
    }

    pub fn go(&mut self, to: Route, cx: &mut Context<Self>) {
        self.route = to;
        if matches!(to, Route::Market | Route::Automations) && self.market.packs.is_none() {
            self.load_market(cx);
        }
        if matches!(to, Route::Settings(_)) && self.settings.is_none() {
            self.load_settings(cx);
        }
        cx.notify();
    }

    /// Runs what the composer holds: edits, or a pack.
    pub fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut args = runs::split_args(&self.input.read(cx).value());
        if args.is_empty() {
            return;
        }
        if self.mode == Mode::Pack {
            args.splice(0..0, ["packs".to_string(), "create".to_string()]);
        }
        self.jobs.push(Job::start(args));
        let i = self.jobs.len() - 1;
        self.route = Route::Job(i);
        self.input.update(cx, |s, cx| s.set_value("", window, cx));
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
        cx.notify();
    }

    /// A file dropped or picked goes in the composer, quoted.
    pub fn put(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let text = format!("\"{}\"", path.display());
        self.input.update(cx, |s, cx| {
            s.set_value(text, window, cx);
            s.focus(window, cx);
        });
        self.route = Route::New;
        cx.notify();
    }

    pub fn pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: None });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                if let Some(p) = paths.first() {
                    let _ = this.update_in(cx, |this, window, cx| this.put(p, window, cx));
                }
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

    /// `edits packs add|remove|trust ID`, then the marketplace again, and a word on how it went.
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
                    Ok(_) => Notification::success(format!("{id}: {}", match verb { "add" => "added", "remove" => "removed", _ => "trusted" })),
                    Err(e) => Notification::error(e).title(format!("{verb} {id}")),
                };
                window.push_notification(note, cx);
                this.load_market(cx);
            });
        })
        .detach();
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
                if let Err(e) = r {
                    window.push_notification(Notification::error(e).title(format!("edits {what}")), cx);
                }
                this.load_settings(cx);
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
            Route::Settings(section) => self.settings_page(section, window, cx).into_any_element(),
            Route::Review => self.review_screen(window, cx).into_any_element(),
            _ => self.chat_view(window, cx).into_any_element(),
        };
        div().size_full().flex().bg(b.page).text_color(b.ink).font_family(theme::FONT).font_weight(theme::LIGHT)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                if let Some(p) = paths.paths().first() {
                    this.put(&PathBuf::from(p), window, cx);
                }
            }))
            .when(!self.review_only, |d| d.child(self.sidebar(cx)))
            .child(div().flex_1().min_w_0().h_full().flex().flex_col().child(main))
    }
}
