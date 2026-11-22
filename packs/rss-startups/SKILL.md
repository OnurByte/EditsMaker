---
name: edits-rss-startups
description: "Make short videos from Startups RSS (18 feeds, AVC, Blog on Feld Thoughts, Entrepreneur – Latest and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Startups videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Startups

Startups from 18 feeds, AVC, Blog on Feld Thoughts, Entrepreneur – Latest: videos from their newest posts.

```sh
edits rss startups --list                    # the newest posts (--json for programs)
edits rss startups -n 3                       # a video for each of the 3 newest
edits rss startups --feed all -n 5 --preset sinema-bant
edits follow startups --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- AVC https://avc.com/feed/
- Blog on Feld Thoughts https://feld.com/archives/tag/blog/feed
- Entrepreneur – Latest http://feeds.feedburner.com/entrepreneur/latest
- Feld Thoughts https://feld.com/feed
- GaryVee https://www.youtube.com/feeds/videos.xml?user=GaryVaynerchuk
- Hacker News: Front Page https://hnrss.org/frontpage
- Inc.com https://www.inc.com/rss/
- Marie Forleo https://www.youtube.com/feeds/videos.xml?user=marieforleo
- Masters of Scale https://rss.art19.com/masters-of-scale
- Paul Graham: Essays http://www.aaronsw.com/2002/feeds/pgessays.rss
- Product Hunt — The best new products, every day https://www.producthunt.com/feed
- Small Business Trends https://feeds2.feedburner.com/SmallBusinessTrends
- Smart Passive Income http://feeds.feedburner.com/smartpassiveincome
- Steve Blank https://steveblank.com/feed/
- The Intercom Blog https://www.intercom.com/blog/feed
- The Startup Junkies Podcast https://startupjunkie.libsyn.com/rss
- The Tim Ferriss Show https://rss.art19.com/tim-ferriss-show
- VentureBeat https://feeds.feedburner.com/venturebeat/SZYF

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
