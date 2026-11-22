---
name: edits-rss-travel
description: "Make short videos from Travel RSS (12 feeds, Adventure.com, Atlas Obscura - Latest Articles and Places, CNN.com - RSS Channel - App Travel Section and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Travel videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Travel

Travel from 12 feeds, Adventure.com, Atlas Obscura - Latest Articles and Places, CNN.com - RSS Channel - App Travel Section: videos from their newest posts.

```sh
edits rss travel --list                    # the newest posts (--json for programs)
edits rss travel -n 3                       # a video for each of the 3 newest
edits rss travel --feed all -n 5 --preset sinema-bant
edits follow travel --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Adventure.com http://adventure.com/feed/
- Atlas Obscura - Latest Articles and Places https://www.atlasobscura.com/feeds/latest
- CNN.com - RSS Channel - App Travel Section http://rss.cnn.com/rss/cnn_travel.rss
- Everything Everywhere http://feeds2.feedburner.com/EverythingEverywhere/
- Live Life Travel https://www.livelifetravel.world/feed/
- Million Mile Secrets http://feeds.feedburner.com/MillionMileSecrets
- NYT > Travel https://rss.nytimes.com/services/xml/rss/nyt/Travel.xml
- TFIL (Traveling to EVERY COUNTRY in A-Z Order) https://www.youtube.com/feeds/videos.xml?user=ECastee
- The Points Guy Articles http://feeds.feedburner.com/thepointsguy
- Travel Blog - https://www.nomadicmatt.com/travel-blog/feed/
- Travel Dudes https://www.traveldudes.com/feed/
- Travel | The Guardian https://www.theguardian.com/uk/travel/rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
