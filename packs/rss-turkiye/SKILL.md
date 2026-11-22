---
name: edits-rss-turkiye
description: "Make short videos from Türkiye RSS (6 feeds, Anadolu Ajansı Güncel, Anadolu Ajansı Dünya, Anadolu Ajansı Ekonomi and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Türkiye videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Türkiye

Türkiye's news from Anadolu Ajansı, TRT Haber and BBC Türkçe: videos from their newest headlines.

```sh
edits rss turkiye --list                    # the newest posts (--json for programs)
edits rss turkiye -n 3                       # a video for each of the 3 newest
edits rss turkiye --feed all -n 5 --preset sinema-bant
edits follow turkiye --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Anadolu Ajansı Güncel https://www.aa.com.tr/tr/rss/default?cat=guncel
- Anadolu Ajansı Dünya https://www.aa.com.tr/tr/rss/default?cat=dunya
- Anadolu Ajansı Ekonomi https://www.aa.com.tr/tr/rss/default?cat=ekonomi
- Anadolu Ajansı Spor https://www.aa.com.tr/tr/rss/default?cat=spor
- TRT Haber Manşet https://www.trthaber.com/manset_articles.rss
- BBC Türkçe https://feeds.bbci.co.uk/turkce/rss.xml

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
