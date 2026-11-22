---
name: edits-rss-tech
description: "Make short videos from Tech RSS (28 feeds, Accidental Tech Podcast, Analog(ue), Ars Technica - All content and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Tech videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Tech

Tech from 28 feeds, Accidental Tech Podcast, Analog(ue), Ars Technica - All content: videos from their newest posts.

```sh
edits rss tech --list                    # the newest posts (--json for programs)
edits rss tech -n 3                       # a video for each of the 3 newest
edits rss tech --feed all -n 5 --preset sinema-bant
edits follow tech --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Accidental Tech Podcast https://atp.fm/rss
- Analog(ue) https://www.relay.fm/analogue/feed
- Ars Technica - All content http://feeds.arstechnica.com/arstechnica/index
- CNET https://www.cnet.com/rss/news/
- CNET https://www.youtube.com/feeds/videos.xml?user=CNETTV
- Clockwise https://www.relay.fm/clockwise/feed
- Crunch Hype http://feeds.feedburner.com/TechCrunch
- Engadget - Technology News & Expert Reviews https://www.engadget.com/rss.xml
- Gizmodo https://gizmodo.com/rss
- Hacker News https://news.ycombinator.com/rss
- Lifehacker https://lifehacker.com/rss
- Linus Tech Tips https://www.youtube.com/feeds/videos.xml?user=LinusTechTips
- Marques Brownlee https://www.youtube.com/feeds/videos.xml?user=marquesbrownlee
- Mashable http://feeds.mashable.com/Mashable
- NYT > Technology https://rss.nytimes.com/services/xml/rss/nyt/Technology.xml
- ReadWrite https://readwrite.com/feed/
- Reply All https://feeds.megaphone.fm/replyall
- Rocket https://www.relay.fm/rocket/feed
- Slashdot http://rss.slashdot.org/Slashdot/slashdotMain
- Stratechery by Ben Thompson http://stratechery.com/feed/
- The Keyword https://www.blog.google/rss/
- The Next Web https://thenextweb.com/feed/
- The Verge https://www.theverge.com/rss/index.xml
- The Verge https://www.youtube.com/feeds/videos.xml?user=TheVerge
- The Vergecast https://feeds.megaphone.fm/vergecast
- This Week in Tech (Audio) https://feeds.twit.tv/twit.xml
- Unbox Therapy https://www.youtube.com/feeds/videos.xml?user=unboxtherapy
- www.theregister.com - Articles https://www.theregister.com/headlines.rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
