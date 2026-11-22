---
name: edits-rss-news
description: "Make short videos from News RSS (10 feeds, BBC News, CNN.com - RSS Channel - World, International: Top News And Analysis and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants News videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# News

News from 10 feeds, BBC News, CNN.com - RSS Channel - World, International: Top News And Analysis: videos from their newest posts.

```sh
edits rss news --list                    # the newest posts (--json for programs)
edits rss news -n 3                       # a video for each of the 3 newest
edits rss news --feed all -n 5 --preset sinema-bant
edits follow news --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- BBC News http://feeds.bbci.co.uk/news/world/rss.xml
- CNN.com - RSS Channel - World http://rss.cnn.com/rss/edition_world.rss
- International: Top News And Analysis https://www.cnbc.com/id/100727362/device/rss/rss.html
- NDTV News Search Records Found 1000 http://feeds.feedburner.com/ndtvnews-world-news
- NYT > World News https://rss.nytimes.com/services/xml/rss/nyt/World.xml
- Top stories - Google News https://news.google.com/rss
- World http://feeds.washingtonpost.com/rss/world
- World News https://www.reddit.com/r/worldnews/.rss
- World News, Today World News, Latest International News, World Breaking News, Trending News of World - Times of India https://timesofindia.indiatimes.com/rssfeeds/296589292.cms
- World news | The Guardian https://www.theguardian.com/world/rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
