---
name: edits-rss-gaming
description: "Make short videos from Gaming RSS (14 feeds, Eurogamer.net Latest Articles Feed, GameSpot - All Content, IGN Articles and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Gaming videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Gaming

Gaming from 14 feeds, Eurogamer.net Latest Articles Feed, GameSpot - All Content, IGN Articles: videos from their newest posts.

```sh
edits rss gaming --list                    # the newest posts (--json for programs)
edits rss gaming -n 3                       # a video for each of the 3 newest
edits rss gaming --feed all -n 5 --preset sinema-bant
edits follow gaming --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Eurogamer.net Latest Articles Feed https://www.eurogamer.net/?format=rss
- GameSpot - All Content https://www.gamespot.com/feeds/mashup/
- IGN Articles http://feeds.ign.com/ign/all
- Indie Games Plus https://indiegamesplus.com/feed
- Kotaku https://kotaku.com/rss
- Makeup and Beauty Blog | Makeup Reviews, Swatches and How-To Makeup https://www.makeupandbeautyblog.com/feed/
- PlayStation.Blog http://feeds.feedburner.com/psblog
- Polygon.com https://www.polygon.com/rss/index.xml
- Rock, Paper, Shotgun http://feeds.feedburner.com/RockPaperShotgun
- Steam RSS News Feed https://store.steampowered.com/feeds/news.xml
- The Ancient Gaming Noob http://feeds.feedburner.com/TheAncientGamingNoob
- The Escapist https://www.escapistmagazine.com/v2/feed/
- TouchArcade - iPhone, iPad, Android Games Forum https://toucharcade.com/community/forums/-/index.rss
- r/gaming https://www.reddit.com/r/gaming.rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
