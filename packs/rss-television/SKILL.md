---
name: edits-rss-television
description: "Make short videos from Television RSS (5 feeds, TV, TV Fanatic, TVLine - All TV. No Interference. TV News & Spoilers by TVLine and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Television videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Television

Television from 5 feeds, TV, TV Fanatic, TVLine - All TV. No Interference. TV News & Spoilers by TVLine: videos from their newest posts.

```sh
edits rss television --list                    # the newest posts (--json for programs)
edits rss television -n 3                       # a video for each of the 3 newest
edits rss television --feed all -n 5 --preset sinema-bant
edits follow television --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- TV https://www.bleedingcool.com/tv/feed/
- TV Fanatic https://www.tvfanatic.com/rss.xml
- TVLine - All TV. No Interference. TV News & Spoilers by TVLine https://tvline.com/feed/
- the TV addict http://feeds.feedburner.com/thetvaddict/AXob
- 📺Television News and Discussion https://reddit.com/r/television/.rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
