//! RSS made into videos. A feed pack (PACKS.md) carries RSS or Atom feeds; `edits rss PACK` reads the newest items
//! and makes each into an edit: footage found for its headline, the headline on screen. `edits follow PACK --post`
//! does it as items come, RSS in and posts out. OPML lists (any reader's export) become feed packs.
use crate::packs::{self, Feed};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub link: String,
    pub summary: String,
    pub image: Option<String>,
    /// a video the item carries (an enclosure, or a source's video): made from it, not from the headline
    pub video: Option<String>,
}

/// An RSS, Atom or RDF feed's items, in the feed's order (newest first).
pub fn items(xml: &str) -> Option<Vec<Item>> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    if !matches!(doc.root_element().tag_name().name(), "rss" | "feed" | "RDF") {
        return None;
    }
    // all of an element's text, tags inside it too (<title>Big <b>news</b></title>)
    let text = |n: roxmltree::Node, tag: &str| n.children().find(|c| c.tag_name().name() == tag)
        .map(|c| c.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect::<String>().trim().to_string());
    Some(doc.root_element().descendants().filter(|n| matches!(n.tag_name().name(), "item" | "entry")).filter_map(|n| {
        let link = n.children().filter(|c| c.tag_name().name() == "link")
            .find_map(|c| c.attribute("href").filter(|_| c.attribute("rel").is_none_or(|r| r == "alternate")).map(str::to_string).or_else(|| c.text().map(|t| t.trim().to_string())))
            .unwrap_or_default();
        let title = text(n, "title").filter(|t| !t.is_empty())?;
        // media:thumbnail, media:content or an image enclosure
        let image = n.descendants().find(|c| match c.tag_name().name() {
            "thumbnail" => true,
            "content" | "enclosure" => c.attribute("type").is_none_or(|t| t.starts_with("image/")) && c.attribute("medium").is_none_or(|m| m == "image"),
            _ => false,
        }).and_then(|c| c.attribute("url")).map(str::to_string);
        let video = n.descendants().find(|c| matches!(c.tag_name().name(), "enclosure" | "content") && c.attribute("type").is_some_and(|t| t.starts_with("video/")))
            .and_then(|c| c.attribute("url")).map(str::to_string);
        let summary = text(n, "description").or_else(|| text(n, "summary")).unwrap_or_default();
        let id = text(n, "guid").or_else(|| text(n, "id")).unwrap_or_else(|| link.clone());
        Some(Item { id, title: strip(&title), link, summary: strip(&summary), image, video })
    }).collect())
}

/// Text without its HTML tags and with its entities, on one line.
fn strip(s: &str) -> String {
    let no_tags = regex::Regex::new(r"<[^>]*>").map(|r| r.replace_all(s, " ").into_owned()).unwrap_or_else(|_| s.to_string());
    let t = no_tags.replace("&nbsp;", " ").replace("&quot;", "\"").replace("&#39;", "'").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The feeds of an OPML file, and the name of the first group of them.
pub fn opml(xml: &str) -> Result<(Option<String>, Vec<Feed>), String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("not an OPML file: {e}"))?;
    let outlines = || doc.descendants().filter(|n| n.tag_name().name() == "outline");
    let group = outlines().find(|n| n.attribute("xmlUrl").is_none() && n.children().any(|c| c.tag_name().name() == "outline"))
        .and_then(|n| n.attribute("title").or(n.attribute("text"))).map(str::to_string);
    let feeds: Vec<Feed> = outlines().filter_map(|n| Some(Feed {
        name: n.attribute("title").or(n.attribute("text")).unwrap_or("").trim().to_string(),
        url: n.attribute("xmlUrl")?.trim().to_string(),
    })).filter(|f| !f.name.is_empty() && (f.url.starts_with("https://") || f.url.starts_with("http://"))).collect();
    if feeds.is_empty() {
        return Err("the OPML file lists no feeds".into());
    }
    Ok((group, feeds))
}

/// A topic's feeds (football), or a pack's by its id, and its preset; or a feed's own address.
pub fn resolve(what: &str) -> Result<(Vec<Feed>, Option<String>), String> {
    if what.starts_with("https://") || what.starts_with("http://") {
        return Ok((vec![Feed { name: what.into(), url: what.into() }], None));
    }
    let want = what.to_lowercase();
    // a topic of a many-topic pack, by its short name: football, or news/football
    let key = want.rsplit('/').next().unwrap_or(&want).trim_start_matches("edits.").trim_start_matches("rss-").to_string();
    for p in packs::all() {
        if let Some(t) = p.manifest.topics.iter().find(|t| t.key == key) {
            let preset = p.preset.as_deref().and_then(|t| serde_json::from_str::<crate::preset::Preset>(t).ok()).map(|p| p.name);
            return Ok((t.feeds.clone(), preset));
        }
    }
    let names = [want.clone(), format!("edits.rss-{want}"), format!("edits.{want}")];
    let p = packs::all().into_iter().find(|p| !p.manifest.feeds.is_empty() && names.contains(&p.manifest.id))
        .ok_or(format!("no feed pack or topic \"{what}\": edits packs lists them (the RSS ones say feed), or give a feed's address"))?;
    let preset = p.preset.as_deref().and_then(|t| serde_json::from_str::<crate::preset::Preset>(t).ok()).map(|p| p.name);
    Ok((p.manifest.feeds, preset))
}

pub fn fetch(url: &str) -> Result<Vec<Item>, String> {
    let (code, body) = crate::util::curl(&[("url", url), ("user-agent", "Mozilla/5.0"), ("max-time", "30")])?;
    if !(200..300).contains(&code) {
        return Err(format!("{url}: HTTP {code}"));
    }
    items(&body).ok_or(format!("{url} is not an RSS or Atom feed"))
}

/// The edits arguments that make an item an edit: its headline, searched for footage and put on screen first.
pub fn args_for(title: &str) -> Vec<String> {
    let line: String = title.replace('@', " ").chars().take(90).collect();
    vec![line.clone(), "--caption".into(), format!("{line}@0-4")]
}

/// What makes an item an edit: its video when it has one, else its headline.
pub fn args_of(item: &Item) -> Vec<String> {
    item.video.clone().map(|v| vec![v]).unwrap_or_else(|| args_for(&item.title))
}

/// `edits rss <pack|feed> [--list] [--json] [--feed NAME|all] [-n N] [edit options...]`
pub fn cmd(args: &[String]) -> Result<(), String> {
    let usage = "usage: edits rss <pack|feed address> [--list] [--json] [--feed NAME|all] [-n N] [edit options: --preset, --music ...]";
    let what = args.first().filter(|a| !a.starts_with('-')).ok_or(usage)?;
    let (feeds, preset) = resolve(what)?;
    let (mut list, mut json, mut n, mut pick, mut rest) = (false, false, 1usize, None::<String>, vec![]);
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--list" => list = true,
            "--json" => (list, json) = (true, true),
            "-n" => {
                n = args.get(i + 1).and_then(|v| v.parse().ok()).filter(|n| (1..=20).contains(n)).ok_or("-n takes 1-20")?;
                i += 1;
            }
            "--feed" => {
                pick = Some(args.get(i + 1).ok_or("--feed takes a feed's name, or all")?.to_lowercase());
                i += 1;
            }
            a => rest.push(a.to_string()),
        }
        i += 1;
    }
    let chosen: Vec<&Feed> = match pick.as_deref() {
        Some("all") => feeds.iter().collect(),
        Some(p) => feeds.iter().filter(|f| f.name.to_lowercase().contains(p)).take(1).collect(),
        // the first feed that answers
        None => feeds.iter().collect(),
    };
    if chosen.is_empty() {
        return Err(format!("no feed like that; its feeds: {}", feeds.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ")));
    }
    let mut got: Vec<(String, Item)> = vec![];
    for f in chosen {
        match fetch(&f.url) {
            Ok(items) => got.extend(items.into_iter().map(|i| (f.name.clone(), i))),
            Err(e) => eprintln!("edits: {}: {e}", f.name),
        }
        if pick.is_none() && !got.is_empty() {
            break;
        }
    }
    if got.is_empty() {
        return Err("no feed answered".into());
    }
    if list {
        if json {
            let v: Vec<serde_json::Value> = got.iter().map(|(f, i)| serde_json::json!({"feed": f, "title": i.title, "link": i.link, "summary": i.summary, "image": i.image})).collect();
            println!("{}", serde_json::Value::Array(v));
        } else {
            for (f, i) in &got {
                println!("{}\n  {f} · {}{}", i.title, i.link, i.image.as_ref().map(|m| format!("\n  image {m}")).unwrap_or_default());
            }
        }
        return Ok(());
    }
    if let Some(p) = preset.filter(|_| !rest.iter().any(|a| a == "--preset")) {
        rest.extend(["--preset".to_string(), p]);
    }
    let mut made: Vec<PathBuf> = vec![];
    for (_, item) in got.iter().take(n) {
        eprintln!("edits: {}", item.title);
        made.extend(crate::follow::make_from(&[args_of(item), rest.clone()].concat())?);
    }
    made.iter().for_each(|c| println!("{}", c.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_with_their_pictures() {
        let rss = r#"<rss xmlns:media="http://search.yahoo.com/mrss/"><channel>
            <item><title>Big &amp; <b>news</b></title><link>https://bbc.co.uk/1</link><guid>g1</guid><description>&lt;p&gt;What happened&lt;/p&gt;</description>
              <media:thumbnail url="https://i.bbc/1.jpg"/></item>
            <item><title>Two</title><link>https://x/2</link><enclosure url="https://x/2.jpg" type="image/jpeg"/></item>
            <item><title> </title><link>https://x/3</link></item></channel></rss>"#;
        let v = items(rss).unwrap();
        assert_eq!(v.len(), 2, "an item without a title is skipped");
        assert_eq!((v[0].title.as_str(), v[0].summary.as_str(), v[0].image.as_deref(), v[0].id.as_str()), ("Big & news", "What happened", Some("https://i.bbc/1.jpg"), "g1"));
        assert_eq!(v[1].image.as_deref(), Some("https://x/2.jpg"));
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><id>a</id><title>Hi</title><link rel="alternate" href="https://x/a"/><summary>s</summary></entry></feed>"#;
        assert_eq!(items(atom).unwrap()[0].link, "https://x/a");
        assert!(items("<html/>").is_none());
    }

    #[test]
    fn opml_lists_become_feeds() {
        let x = r#"<opml version="1.0"><head><title>Export</title></head><body><outline text="Football" title="Football">
            <outline text="BBC Sport" title="BBC Sport" xmlUrl="https://feeds.bbci.co.uk/sport/football/rss.xml" type="rss"/>
            <outline text="CNN" xmlUrl="http://rss.cnn.com/rss/edition_football.rss"/>
            <outline text="bad" xmlUrl="ftp://x"/></outline></body></opml>"#;
        let (name, feeds) = opml(x).unwrap();
        assert_eq!(name.as_deref(), Some("Football"));
        assert_eq!(feeds.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["BBC Sport", "CNN"]);
        assert!(opml("<opml><body/></opml>").is_err());
    }

    #[test]
    fn packs_are_found_by_their_short_name() {
        let (feeds, _) = resolve("turkiye").unwrap();
        assert!(feeds.iter().any(|f| f.name == "BBC Türkçe"));
        assert_eq!(resolve("edits.rss-football").unwrap().0, resolve("football").unwrap().0, "an old pack id still finds its topic");
        assert_eq!(resolve("news/space").unwrap().0, resolve("space").unwrap().0);
        assert!(resolve("akis-whip").is_err(), "a preset pack has no feeds");
        assert_eq!(resolve("https://x/rss").unwrap().0[0].url, "https://x/rss");
    }

    #[test]
    fn a_headline_is_searched_and_shown() {
        assert_eq!(args_for("Rates @ 5%"), ["Rates   5%", "--caption", "Rates   5%@0-4"]);
        assert!(crate::wrap::parse_caption(&args_for("x@y")[2]).is_ok());
    }
}
