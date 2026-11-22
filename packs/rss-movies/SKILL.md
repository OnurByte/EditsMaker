---
name: edits-rss-movies
description: "Make short videos from Movies RSS (9 feeds, /Film, Ain't It Cool News Feed, ComingSoon.net – Movie Trailers, TV & Streaming News, and More and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Movies videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Movies

Movies from 9 feeds, /Film, Ain't It Cool News Feed, ComingSoon.net – Movie Trailers, TV & Streaming News, and More: videos from their newest posts.

```sh
edits rss movies --list                    # the newest posts (--json for programs)
edits rss movies -n 3                       # a video for each of the 3 newest
edits rss movies --feed all -n 5 --preset sinema-bant
edits follow movies --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- /Film https://feeds2.feedburner.com/slashfilm
- Ain't It Cool News Feed https://www.aintitcool.com/node/feed/
- ComingSoon.net – Movie Trailers, TV & Streaming News, and More https://www.comingsoon.net/feed
- Deadline https://deadline.com/feed/
- FirstShowing.net https://www.firstshowing.net/feed/
- IndieWire https://www.indiewire.com/feed
- Movie News and Discussion https://reddit.com/r/movies/.rss
- Movies https://www.bleedingcool.com/movies/feed/
- Variety https://variety.com/feed/

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
