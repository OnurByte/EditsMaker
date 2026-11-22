---
name: edits-rss-cricket
description: "Make short videos from Cricket RSS (19 feeds, BBC Sport, Can't Bowl Can't Throw Cricket Show, Cricbuzz and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Cricket videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Cricket

Cricket from 19 feeds, BBC Sport, Can't Bowl Can't Throw Cricket Show, Cricbuzz: videos from their newest posts.

```sh
edits rss cricket --list                    # the newest posts (--json for programs)
edits rss cricket -n 3                       # a video for each of the 3 newest
edits rss cricket --feed all -n 5 --preset sinema-bant
edits follow cricket --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- BBC Sport http://feeds.bbci.co.uk/sport/cricket/rss.xml
- Can't Bowl Can't Throw Cricket Show http://feeds.feedburner.com/cantbowlcantthrow
- Cricbuzz https://www.youtube.com/feeds/videos.xml?channel_id=UCSRQXk5yErn4e14vN76upOw
- Cricket https://www.reddit.com/r/Cricket/.rss
- Cricket News Today, Latest Cricket Updates, Cricket Live Score, Upcoming Matches | Times of India https://timesofindia.indiatimes.com/rssfeeds/54829575.cms
- Cricket News: Stories, Features, Interviews, Opinion, Reports | Wisden https://www.wisden.com/feed
- Cricket news from ESPN Cricinfo.com http://www.espncricinfo.com/rss/content/story/feeds/0.xml
- Cricket | The Guardian https://www.theguardian.com/sport/cricket/rss
- Cricket | The Indian Express https://indianexpress.com/section/sports/cricket/feed/
- Cricket – The Roar https://www.theroar.com.au/cricket/feed/
- England & Wales Cricket Board https://www.youtube.com/feeds/videos.xml?user=ecbcricket
- NDTV News Search Records Found 1000 http://feeds.feedburner.com/ndtvsports-cricket
- Pakistan Cricket https://www.youtube.com/feeds/videos.xml?channel_id=UCiWrjBhlICf_L_RK5y6Vrxw
- Sri Lanka Cricket https://www.youtube.com/feeds/videos.xml?user=TheOfficialSLC
- Stumped https://podcasts.files.bbci.co.uk/p02gsrmh.rss
- Switch Hit Podcast https://feeds.megaphone.fm/ESP9247246951
- Tailenders https://podcasts.files.bbci.co.uk/p02pcb4w.rss
- Test Match Special https://podcasts.files.bbci.co.uk/p02nrsl2.rss
- cricket.com.au https://www.youtube.com/feeds/videos.xml?user=cricketaustraliatv

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
