---
name: edits-rss-animal-and-wildlife
description: "Make short videos from Animal & Wildlife RSS (8 feeds, Active Wild, Focusing on Wildlife, Love Nature and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Animal & Wildlife videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Animal & Wildlife

Animal & Wildlife from 8 feeds, Active Wild, Focusing on Wildlife, Love Nature: videos from their newest posts.

```sh
edits rss animal-and-wildlife --list                    # the newest posts (--json for programs)
edits rss animal-and-wildlife -n 3                       # a video for each of the 3 newest
edits rss animal-and-wildlife --feed all -n 5 --preset sinema-bant
edits follow animal-and-wildlife --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Active Wild https://www.activewild.com/feed/
- Focusing on Wildlife https://focusingonwildlife.com/news/feed/
- Love Nature https://www.youtube.com/feeds/videos.xml?channel_id=UCRZPkuHwaoKwTP3CYPdVldg
- Real Wild https://www.youtube.com/feeds/videos.xml?channel_id=UCbq-4OJxnziD3awH-aTezeA
- The Wildlife India https://www.thewildlifeindia.com/feeds/posts/default
- Wildlife Photography: Gear, Photos & Discussion https://www.reddit.com/r/wildlifephotography/.rss
- Wildlife | The Guardian https://www.theguardian.com/environment/wildlife/rss
- 🔥 Nature Is Fucking Lit https://www.reddit.com/r/NatureIsFuckingLit/.rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
