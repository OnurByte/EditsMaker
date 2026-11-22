//! Flows, as in n8n but in one line each: a topic (the news on it, from a topic pack) or a channel, made into edits,
//! posted to accounts. Each is an `edits follow`; `edits watch` (or the background service) runs them.
use crate::app::icons::{icon, Icon};
use crate::app::runs::Flow;
use crate::app::theme::{Brand, BOLD, LIGHT};
use crate::app::ui::{card, pill, primary, text};
use crate::app::workspace::{Picker, Workspace};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// One step of a flow: an icon and what it does.
fn step(i: Icon, label: String, b: &Brand) -> Div {
    div().h(px(32.)).px(px(12.)).flex().items_center().gap(px(8.)).rounded(px(16.)).bg(b.hair).max_w(px(260.))
        .child(icon(i, b.ink).size(px(16.)))
        .child(text(label, 13., FontWeight::NORMAL, b.ink).truncate())
}

fn arrow(b: &Brand) -> Div {
    text("→", 14., LIGHT, b.gray)
}

impl Workspace {
    /// What a flow's source is called: its RSS pack's feed, else its address.
    fn source_name(&self, source: &str) -> String {
        let packs = self.market.packs.as_ref().and_then(|p| p.as_ref().ok());
        packs.into_iter().flatten().find_map(|p| p.feeds.iter().find(|f| f.url == source).map(|f| format!("{} · {}", p.name, f.name)))
            .unwrap_or_else(|| source.trim_start_matches("https://").trim_start_matches("www.").to_string())
    }

    fn flow_row(&self, i: usize, f: &Flow, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let preset = f.args.iter().position(|a| a == "--preset").and_then(|k| f.args.get(k + 1)).cloned();
        let made = match (f.titles, preset) {
            (true, Some(p)) => format!("Each story, {p}"),
            (true, None) => "Each new story".into(),
            (false, Some(p)) => p,
            (false, None) => "Edits".into(),
        };
        let source = f.source.clone();
        div().py(px(8.)).flex().items_center().gap(px(8.)).flex_wrap().when(i > 0, |d| d.border_t_1().border_color(b.hair))
            .child(step(Icon::Rss, self.source_name(&f.source), b))
            .child(arrow(b))
            .child(step(Icon::Film, made, b))
            .child(arrow(b))
            .child(step(Icon::Send, if f.post.is_empty() { "Kept, not posted".into() } else { f.post.clone() }, b))
            .child(text(format!("every {} min", f.every), 12., FontWeight::NORMAL, b.gray))
            .child(div().flex_1())
            .child(pill(("unfollow", i), Some(Icon::Trash), "Remove", false, b)
                .on_click(cx.listener(move |this, _, window, cx| this.change(vec!["unfollow".into(), source.clone()], window, cx))))
    }

    /// The flows there are, a new one to make, and the switch that keeps them running.
    pub fn flows_section(&self, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let list: AnyElement = match &self.flows {
            None => text("Reading the flows...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Err(e)) => text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element(),
            Some(Ok(l)) if l.is_empty() => text("No flows yet. Make one below: a topic, the edit, where it goes.", 14., LIGHT, b.gray).into_any_element(),
            Some(Ok(l)) => card(b).children(l.iter().enumerate().map(|(i, f)| self.flow_row(i, f, b, cx))).into_any_element(),
        };
        let accounts: Vec<String> = self.settings.as_ref().and_then(|s| s.as_ref().ok()).map(|s| s.accounts.iter().map(|a| a.name.clone()).collect()).unwrap_or_default();
        let post = if accounts.is_empty() {
            text("No account yet: Settings, Accounts", 13., FontWeight::NORMAL, b.gray).into_any_element()
        } else {
            div().flex().flex_wrap().gap(px(8.)).children(accounts.into_iter().enumerate().map(|(i, a)| {
                let on = self.flow_post.contains(&a);
                pill(("post-to", i), Some(Icon::Send), a.clone(), on, b).on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(k) = this.flow_post.iter().position(|x| *x == a) {
                        this.flow_post.remove(k);
                    } else {
                        this.flow_post.push(a.clone());
                    }
                    cx.notify();
                }))
            })).into_any_element()
        };
        let every = div().flex().gap(px(8.)).children([30u64, 60, 180].into_iter().map(|m| {
            pill(("every", m as usize), None, format!("{m} min"), self.flow_every == m, b).on_click(cx.listener(move |this, _, _, cx| {
                this.flow_every = m;
                cx.notify();
            }))
        }));
        let source = self.picker_pill("flow-source", Icon::Workflow, self.flow_source.clone().unwrap_or_else(|| "Pick a topic".into()), Picker::FlowSource, b, cx);
        let label = |s: &'static str| text(s, 13., BOLD, b.ink).w(px(96.)).flex_none();
        let new = card(b).gap(px(16.))
            .child(div().flex().items_center().gap(px(16.)).child(label("About")).child(source))
            .when(self.picker == Some(Picker::FlowSource), |d| d.child(self.picker_list(Picker::FlowSource, b, cx)))
            .child(div().flex().items_center().gap(px(16.)).child(label("Made with")).child(text(self.preset.clone().unwrap_or_else(|| "the composer's preset (none now)".into()), 13., FontWeight::NORMAL, b.gray)))
            .child(div().flex().items_center().gap(px(16.)).child(label("Posted to")).child(post))
            .child(div().flex().items_center().gap(px(16.)).child(label("Every")).child(every))
            .child(div().flex().gap(px(8.))
                .child(primary("make-flow", "Make the flow", b).on_click(cx.listener(|this, _, window, cx| {
                    let Some(src) = this.flow_source.clone() else { return };
                    let mut args = vec!["follow".to_string(), src, "--every".into(), this.flow_every.to_string()];
                    if !this.flow_post.is_empty() {
                        args.extend(["--post".to_string(), this.flow_post.join(",")]);
                    }
                    if let Some(p) = this.preset.clone() {
                        args.extend(["--preset".to_string(), p]);
                    }
                    this.change(args, window, cx);
                })))
                .child(pill("service", Some(Icon::Workflow), "Keep them running", false, b)
                    .on_click(cx.listener(|this, _, window, cx| this.change(vec!["service".into(), "install".into()], window, cx)))));
        div().flex().flex_col().gap(px(16.))
            .child(text("Flows", 19., BOLD, b.ink))
            .child(text("Pick a topic and where to post: each new story on it becomes an edit and goes to your accounts, by itself. They run while edits runs in the background (Keep them running).", 15., LIGHT, b.gray))
            .child(list)
            .child(new)
    }
}
