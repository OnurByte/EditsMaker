---
name: edits-rss-music
description: "Make short videos from Music RSS (5 feeds, Consequence, Metal Injection, Music Business Worldwide and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Music videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Music

Music from 5 feeds, Consequence, Metal Injection, Music Business Worldwide: videos from their newest posts.

```sh
edits rss music --list                    # the newest posts (--json for programs)
edits rss music -n 3                       # a video for each of the 3 newest
edits rss music --feed all -n 5 --preset sinema-bant
edits follow music --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Consequence http://consequenceofsound.net/feed
- Metal Injection http://feeds.feedburner.com/metalinjection
- Music Business Worldwide https://www.musicbusinessworldwide.com/feed/
- RSS: News http://pitchfork.com/rss/news
- Song Exploder http://feed.songexploder.net/songexploder

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
