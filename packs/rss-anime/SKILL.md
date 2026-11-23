---
name: edits-rss-anime
description: "Make short videos from Anime RSS (6 feeds, Anime News Network, Anime News Network (all), MyAnimeList News and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Anime videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Anime

Anime news as it breaks: Anime News Network, MyAnimeList, Anime Corner, Otaku USA and Crunchyroll's trailers. Each new story becomes an edit; a flow can post it for you.

```sh
edits rss anime --list                    # the newest posts (--json for programs)
edits rss anime -n 3                       # a video for each of the 3 newest
edits rss anime --feed all -n 5 --preset sinema-bant
edits follow anime --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Anime News Network https://www.animenewsnetwork.com/news/rss.xml
- Anime News Network (all) https://www.animenewsnetwork.com/all/rss.xml
- MyAnimeList News https://myanimelist.net/rss/news.xml
- Anime Corner https://animecorner.me/feed/
- Otaku USA https://otakuusamagazine.com/feed/
- Crunchyroll https://www.youtube.com/feeds/videos.xml?user=crunchyroll

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
