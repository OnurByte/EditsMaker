//! The marketplace: packs (a preset, an agent skill or both, one thing to the person) on cards with their pictures,
//! and under it the automations, each from here and from the community's list (packs.json). Each pack lives in a
//! folder of its own; an added automation runs only after you read it and trust it (PACKS.md).
use crate::icons::{self, icon, Icon};
use crate::runs::{Listed, Pack};
use crate::theme::{brand, Brand, BOLD, HEADER, LIGHT, SMALL};
use crate::ui::{card, pill, primary, text};
use crate::workspace::Workspace;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

fn is_automation(kinds: &[String]) -> bool {
    kinds.iter().any(|k| k == "automation")
}

fn picture(p: &Pack) -> Option<Img> {
    icons::picture(&p.id, || {
        if let Some(svg) = &p.preview_svg {
            return Some((ImageFormat::Svg, svg.clone().into_bytes()));
        }
        let f = p.preview_file.as_ref()?;
        let format = match f.extension()?.to_string_lossy().to_lowercase().as_str() {
            "svg" => ImageFormat::Svg,
            "png" => ImageFormat::Png,
            "webp" => ImageFormat::Webp,
            _ => ImageFormat::Jpeg,
        };
        Some((format, std::fs::read(f).ok()?))
    })
}

/// Where an added pack's automation is, to read before trusting it.
fn script(id: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".config/edits/packs").join(id).join("automation.js")
}

impl Workspace {
    fn pack_card(&self, i: usize, p: &Pack, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.market.busy.as_deref() == Some(p.id.as_str());
        let (id, id2) = (p.id.clone(), p.id.clone());
        let action: AnyElement = if busy {
            text("Working...", 13., FontWeight::NORMAL, b.gray).into_any_element()
        } else if p.built_in {
            text("Built in", 13., FontWeight::NORMAL, b.gray).into_any_element()
        } else if !p.trusted {
            let read = script(&p.id);
            div().flex().gap(px(8.))
                .child(pill(("read", i), Some(Icon::Eye), "Read it", false, b).on_click(move |_, _, cx| cx.open_with_system(&read)))
                .child(primary(("trust", i), "Trust", b).on_click(cx.listener(move |this, _, window, cx| this.pack_action("trust", id.clone(), window, cx))))
                .into_any_element()
        } else {
            pill(("remove", i), Some(Icon::Trash), "Remove", false, b).on_click(cx.listener(move |this, _, window, cx| this.pack_action("remove", id2.clone(), window, cx))).into_any_element()
        };
        card(b).w(px(288.)).gap(px(8.))
            .child(div().w_full().h(px(144.)).rounded(SMALL).overflow_hidden().bg(b.hair)
                .children(picture(p).map(|img| img.size_full().object_fit(ObjectFit::Cover))))
            .child(text(p.name.clone(), 15., BOLD, b.ink).truncate())
            .child(text(format!("{} · {}", if is_automation(&p.kinds) { "Automation" } else { "Pack" }, p.author), 12., FontWeight::NORMAL, b.gray).truncate())
            .child(text(p.description.clone(), 13., LIGHT, b.gray).h(px(36.)).line_clamp(2))
            .child(div().pt(px(8.)).child(action))
    }

    fn listed_row(&self, i: usize, l: &Listed, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let id = l.id.clone();
        let busy = self.market.busy.as_deref() == Some(l.id.as_str());
        div().py(px(8.)).flex().items_center().gap(px(16.)).when(i > 0, |d| d.border_t_1().border_color(b.hair))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(px(4.))
                .child(text(l.name.clone(), 14., BOLD, b.ink).truncate())
                .child(text(l.description.clone(), 13., LIGHT, b.gray).truncate()))
            .child(if l.added || busy {
                text(if busy { "Working..." } else { "Added" }, 13., FontWeight::NORMAL, b.gray).into_any_element()
            } else {
                pill(("add", i), Some(Icon::Download), "Add", false, b).on_click(cx.listener(move |this, _, window, cx| this.pack_action("add", id.clone(), window, cx))).into_any_element()
            })
    }

    /// The packs, or with `automations` the automations under them.
    pub fn marketplace(&self, automations: bool, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let here: AnyElement = match &self.market.packs {
            None => text("Reading the packs...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Err(e)) => text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element(),
            Some(Ok(list)) => div().flex().flex_wrap().gap(px(16.))
                .children(list.iter().filter(|p| is_automation(&p.kinds) == automations).enumerate().map(|(i, p)| self.pack_card(i, p, &b, cx)))
                .into_any_element(),
        };
        let community: AnyElement = match &self.market.listed {
            None => text("Reading the list...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Err(e)) => text(format!("The list could not be read: {e}"), 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Ok(l)) => {
                let l: Vec<&Listed> = l.iter().filter(|x| is_automation(&x.kinds) == automations).collect();
                if l.is_empty() {
                    text("Nothing from the community yet. Yours could be the first: PACKS.md says how.", 14., LIGHT, b.gray).into_any_element()
                } else {
                    card(&b).children(l.into_iter().enumerate().map(|(i, x)| self.listed_row(i, x, &b, cx))).into_any_element()
                }
            }
        };
        let (i, title, about) = if automations {
            (Icon::Workflow, "Automations", "Jobs done on a website from your own signed-in account, like posting a clip. Each runs on its one site only; one you add runs after you read it and trust it.")
        } else {
            (Icon::Store, "Marketplace", "Packs: an edit's shape, the instructions an agent follows, or both. Each keeps to its own folder.")
        };
        div().size_full().flex().flex_col()
            .child(div().h(HEADER).flex_none().px(px(24.)).flex().items_center().gap(px(8.)).border_b_1().border_color(b.hair)
                .child(icon(i, b.gray).size(px(16.)))
                .when(automations, |d| d.child(text("Marketplace", 14., FontWeight::NORMAL, b.gray)).child(text("/", 14., LIGHT, b.gray)))
                .child(text(title, 14., BOLD, b.ink).flex_1()))
            .child(div().id("market").flex_1().min_h_0().overflow_y_scroll()
                .child(div().w_full().max_w(px(960.)).mx_auto().p(px(24.)).flex().flex_col().gap(px(24.))
                    .child(text(about, 15., LIGHT, b.gray))
                    .child(text("From the community", 19., BOLD, b.ink))
                    .child(community)
                    .child(text("Here", 19., BOLD, b.ink))
                    .child(here)))
    }
}
