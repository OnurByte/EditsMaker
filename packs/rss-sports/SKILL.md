---
name: edits-rss-sports
description: "Make short videos from Sports RSS (10 feeds, BBC Sport, CNN.com - RSS Channel - Sport, NYT > Sports and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Sports videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Sports

Sports from 10 feeds, BBC Sport, CNN.com - RSS Channel - Sport, NYT > Sports: videos from their newest posts.

```sh
edits rss sports --list                    # the newest posts (--json for programs)
edits rss sports -n 3                       # a video for each of the 3 newest
edits rss sports --feed all -n 5 --preset sinema-bant
edits follow sports --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- BBC Sport http://feeds.bbci.co.uk/sport/rss.xml
- CNN.com - RSS Channel - Sport http://rss.cnn.com/rss/edition_sport.rss
- NYT > Sports https://rss.nytimes.com/services/xml/rss/nyt/Sports.xml
- Reddit Sports https://www.reddit.com/r/sports.rss
- Sport | The Guardian https://www.theguardian.com/uk/sport/rss
- Sports News - Latest Sports and Football News | Sky News http://feeds.skynews.com/feeds/rss/sports.xml
- Sports News: Cricket Live Scorecard, Latest Cricket News, Football, NBA, NFL, WWE, NHL, MLB News & More https://timesofindia.indiatimes.com/rssfeeds/4719148.cms
- Sports | The Indian Express https://indianexpress.com/section/sports/feed/
- Sportskeeda https://www.sportskeeda.com/feed
- www.espn.com - TOP https://www.espn.com/espn/rss/news

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
