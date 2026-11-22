---
name: edits-rss-history
description: "Make short videos from History RSS (7 feeds, 30 for 30 Podcasts, Dan Carlin's Hardcore History, History in 28-minutes and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants History videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# History

History from 7 feeds, 30 for 30 Podcasts, Dan Carlin's Hardcore History, History in 28-minutes: videos from their newest posts.

```sh
edits rss history --list                    # the newest posts (--json for programs)
edits rss history -n 3                       # a video for each of the 3 newest
edits rss history --feed all -n 5 --preset sinema-bant
edits follow history --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- 30 for 30 Podcasts https://feeds.megaphone.fm/ESP5765452710
- Dan Carlin's Hardcore History https://feeds.feedburner.com/dancarlin/history?format=xml
- History in 28-minutes https://www.historyisnowmagazine.com/blog?format=RSS
- HistoryNet http://www.historynet.com/feed
- Throughline https://feeds.npr.org/510333/podcast.xml
- You Must Remember This https://feeds.megaphone.fm/YMRT7068253588
- the memory palace http://feeds.thememorypalace.us/thememorypalace

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
