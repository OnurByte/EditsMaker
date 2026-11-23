//! t3code's Sidebar: the name on top, then the places (a new edit, the marketplace, automations, the library), the
//! search over every thread, then the threads, here runs of edits: this session's first, then the ones in the output
//! folder, newest first. A row is t3code's SidebarThreadRow: the project's badge and name with its age on the right,
//! then the thread's title. Collapsed (the panel button, Ctrl+B) it is a rail of the places' icons.
use crate::app::icons::{icon, Icon};
use crate::app::runs;
use crate::app::theme::{brand, Brand, BOLD, HEADER, SIDEBAR, SMALL};
use crate::app::ui::{badge, icon_button, text};
use crate::app::workspace::{Route, Section, Workspace};
use gpui_kit::component::input::Input;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// One row of the list: what it was made from, when, what it is, and how it went.
struct Row {
    to: Route,
    subject: String,
    title: String,
    age: String,
    state: Option<Icon>,
}

/// The places on top, in order.
const PLACES: &[(&str, Icon, &str, Route)] = &[
    ("new", Icon::Plus, "New edit", Route::New),
    ("market", Icon::Store, "Marketplace", Route::Market),
    ("automations", Icon::Workflow, "Automations", Route::Automations),
    ("library", Icon::Library, "Library", Route::Library),
];

/// Whether a thread matches what is typed in the search: every word in its project or title.
pub fn matches(q: &str, subject: &str, title: &str) -> bool {
    let hay = format!("{subject} {title}").to_lowercase();
    q.to_lowercase().split_whitespace().all(|w| hay.contains(w))
}

/// t3code's shortcut hint in the search: ⌘K on a Mac, Ctrl K elsewhere.
pub fn shortcut(b: &Brand) -> Div {
    div().px(px(6.)).h(px(18.)).flex().items_center().rounded(px(4.)).border_1().border_color(b.hair)
        .child(text(if cfg!(target_os = "macos") { "⌘K" } else { "Ctrl K" }, 11., FontWeight::NORMAL, b.gray))
}

impl Workspace {
    fn place(&self, id: &'static str, i: Icon, label: &'static str, to: Route, b: &Brand, cx: &mut Context<Self>) -> Stateful<Div> {
        let on = self.route == to;
        let (bg, hover) = (if on { b.accent.opacity(0.14) } else { transparent_black() }, b.card);
        div().id(id).h(px(32.)).px(px(8.)).flex().items_center().gap(px(8.)).rounded(SMALL).cursor_pointer().bg(bg).hover(move |s| s.bg(hover))
            .child(icon(i, if on { b.accent } else { b.gray }).size(px(16.)))
            .when(self.sidebar_open, |d| d.child(text(label, 14., FontWeight::NORMAL, b.ink)))
            .when(!self.sidebar_open, |d| d.justify_center().tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(label).build(window, cx)))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.go(to, cx);
                if to == Route::New {
                    this.input.update(cx, |s, cx| s.focus(window, cx));
                }
            }))
    }

    fn row(&self, i: usize, r: Row, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let on = self.route == r.to;
        let (bg, hover, to) = (if on { b.accent.opacity(0.14) } else { transparent_black() }, b.card, r.to);
        div().id(("thread", i)).px(px(8.)).py(px(8.)).flex().flex_col().gap(px(4.)).rounded(SMALL).cursor_pointer().bg(bg).hover(move |s| s.bg(hover))
            .child(div().flex().items_center().gap(px(8.)).min_w_0()
                .child(badge(&r.subject, b))
                .child(text(r.subject, 12., FontWeight::NORMAL, b.gray).flex_1().min_w_0().truncate())
                .children(r.state.map(|s| icon(s, b.gray).size(px(12.))))
                .child(text(r.age, 12., FontWeight::NORMAL, b.gray)))
            .child(text(r.title, 14., FontWeight::NORMAL, b.ink).truncate())
            .on_click(cx.listener(move |this, _, _, cx| this.go(to, cx)))
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    pub fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let open = self.sidebar_open;
        let places = PLACES.iter().map(|&(id, i, label, to)| self.place(id, i, label, to, &b, cx)).collect::<Vec<_>>();
        let toggle = icon_button("sidebar-toggle", Icon::PanelLeft, &b)
            .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("Toggle sidebar (Ctrl+B)").build(window, cx))
            .on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx)));
        let hover = b.card;
        let settings = div().id("settings").h(px(32.)).px(px(8.)).flex().items_center().gap(px(8.)).rounded(SMALL).cursor_pointer().hover(move |s| s.bg(hover))
            .child(icon(Icon::Settings, b.gray).size(px(16.)))
            .when(open, |d| d.child(text("Settings", 14., FontWeight::NORMAL, b.ink)))
            .when(!open, |d| d.justify_center())
            .on_click(cx.listener(|this, _, _, cx| this.open_settings(Section::General, cx)));
        if !open {
            return div().w(px(56.)).h_full().flex_none().flex().flex_col().items_center().border_r_1().border_color(b.hair)
                .child(div().h(HEADER).flex_none().flex().items_center().when(cfg!(target_os = "macos"), |d| d.pt(px(24.))).child(toggle))
                .child(div().w_full().px(px(8.)).flex().flex_col().gap(px(2.)).children(places))
                .child(div().flex_1())
                .child(div().w_full().flex_none().p(px(8.)).border_t_1().border_color(b.hair).child(settings))
                .into_any_element();
        }
        let q = self.thread_search.read(cx).value().to_string();
        let mut rows: Vec<Row> = self.jobs.iter().enumerate().rev().map(|(i, j)| Row {
            to: Route::Job(i),
            subject: "This session".into(),
            title: j.title(),
            age: runs::ago(std::time::SystemTime::now() - j.started.elapsed()),
            state: Some(match j.finished() {
                None => Icon::Running,
                Some(true) => Icon::Done,
                Some(false) => Icon::Failed,
            }),
        }).collect();
        rows.extend(self.past.iter().enumerate().map(|(i, r)| {
            let n = runs::clips(&r.dir).len();
            Row { to: Route::Run(i), subject: r.subject.clone(), title: if n == 1 { "1 clip".into() } else { format!("{n} clips") }, age: runs::ago(r.made), state: None }
        }));
        rows.retain(|r| matches(&q, &r.subject, &r.title));
        let empty = rows.is_empty();
        let list: Vec<AnyElement> = rows.into_iter().enumerate().map(|(i, r)| self.row(i, r, &b, cx).into_any_element()).collect();
        div().w(SIDEBAR).h_full().flex_none().flex().flex_col().border_r_1().border_color(b.hair)
            .child(div().h(HEADER).flex_none().pl(px(16.)).pr(px(8.)).flex().items_center().gap(px(8.))
                // macOS draws its window buttons here
                .when(cfg!(target_os = "macos"), |d| d.pl(px(80.)))
                .child(crate::app::icons::mark(b.ink).size(px(20.)))
                .child(text("EditsMaker", 15., BOLD, b.ink).flex_1())
                .child(toggle))
            .child(div().px(px(8.)).flex().flex_col().gap(px(2.)).children(places))
            .child(div().px(px(8.)).pt(px(16.)).child(Input::new(&self.thread_search).prefix(icon(Icon::Search, b.gray).size(px(14.))).suffix(shortcut(&b)).cleanable(true)))
            .child(text("Edits", 12., FontWeight::NORMAL, b.gray).px(px(16.)).pt(px(16.)).pb(px(8.)))
            .child(div().id("threads").flex_1().min_h_0().overflow_y_scroll().px(px(8.)).pb(px(8.)).flex().flex_col().gap(px(2.)).children(list)
                .when(empty, |d| d.child(text(if q.trim().is_empty() { "Nothing made yet." } else { "No edit like that." }, 13., crate::app::theme::LIGHT, b.gray).px(px(8.)))))
            // t3code keeps its settings at the foot of the sidebar, and opens them over everything
            .child(div().flex_none().p(px(8.)).border_t_1().border_color(b.hair).child(settings))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn search_finds_a_thread_by_any_word() {
        use super::matches;
        assert!(matches("madara 3", "Madara", "3 clips"));
        assert!(matches("", "x", "y"));
        assert!(!matches("gojo", "Madara", "3 clips"));
    }
}
