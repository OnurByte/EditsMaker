---
name: edits-rss-tennis
description: "Make short videos from Tennis RSS (10 feeds, BBC Sport, CNN.com - RSS Channel - Sport - Tennis, Essential Tennis Podcast - Instruction, Lessons, Tips and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Tennis videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Tennis

Tennis from 10 feeds, BBC Sport, CNN.com - RSS Channel - Sport - Tennis, Essential Tennis Podcast - Instruction, Lessons, Tips: videos from their newest posts.

```sh
edits rss tennis --list                    # the newest posts (--json for programs)
edits rss tennis -n 3                       # a video for each of the 3 newest
edits rss tennis --feed all -n 5 --preset sinema-bant
edits follow tennis --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- BBC Sport http://feeds.bbci.co.uk/sport/tennis/rss.xml
- CNN.com - RSS Channel - Sport - Tennis http://rss.cnn.com/rss/edition_tennis.rss
- Essential Tennis Podcast - Instruction, Lessons, Tips https://feed.podbean.com/essentialtennis/feed.xml
- NYT > Sports > Tennis https://rss.nytimes.com/services/xml/rss/nyt/Tennis.xml
- Perfect Tennis https://www.perfect-tennis.com/feed/
- Tennis News & Discussion https://www.reddit.com/r/tennis/.rss
- Tennis News, Updates, Tennis Scores, ATP, WTA, Grand Slams | The Hindu https://www.thehindu.com/sport/tennis/feeder/default.rss
- Tennis | The Guardian https://www.theguardian.com/sport/tennis/rss
- Tennis | The Indian Express https://indianexpress.com/section/sports/tennis/feed/
- www.espn.com - TENNIS https://www.espn.com/espn/rss/tennis/news

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
