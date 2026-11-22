---
name: edits-rss-business-and-economy
description: "Make short videos from Business & Economy RSS (17 feeds, All News, Bloomberg Originals, Breaking News on Seeking Alpha and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Business & Economy videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Business & Economy

Business & Economy from 17 feeds, All News, Bloomberg Originals, Breaking News on Seeking Alpha: videos from their newest posts.

```sh
edits rss business-and-economy --list                    # the newest posts (--json for programs)
edits rss business-and-economy -n 3                       # a video for each of the 3 newest
edits rss business-and-economy --feed all -n 5 --preset sinema-bant
edits follow business-and-economy --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- All News https://www.investing.com/rss/news.rss
- Bloomberg Originals https://www.youtube.com/feeds/videos.xml?user=Bloomberg
- Breaking News on Seeking Alpha https://seekingalpha.com/market_currents.xml
- Business Insider https://www.youtube.com/feeds/videos.xml?user=businessinsider
- EconTalk https://feeds.simplecast.com/wgl4xEgL
- Economic Times https://economictimes.indiatimes.com/rssfeedsdefault.cms
- Forbes - Business https://www.forbes.com/business/feed/
- Fortune | FORTUNE https://fortune.com/feed
- HBR IdeaCast http://feeds.harvardbusiness.org/harvardbusiness/ideacast
- How I Built This with Guy Raz https://feeds.npr.org/510313/podcast.xml
- Planet Money https://feeds.npr.org/510289/podcast.xml
- Startup Stories - Mixergy https://feeds.feedburner.com/Mixergy-main-podcast
- The Blog of Author Tim Ferriss https://tim.blog/feed/
- The Duct Tape Marketing Podcast https://ducttape.libsyn.com/rss
- The Growth Show http://thegrowthshow.hubspot.libsynpro.com/
- US Top News and Analysis https://www.cnbc.com/id/100003114/device/rss/rss.html
- Yahoo Finance https://finance.yahoo.com/news/rssindex

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
