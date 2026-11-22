---
name: edits-rss-fashion
description: "Make short videos from Fashion RSS (7 feeds, Fashion - ELLE, Fashion | The Guardian, Fashionista Full and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Fashion videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Fashion

Fashion from 7 feeds, Fashion - ELLE, Fashion | The Guardian, Fashionista Full: videos from their newest posts.

```sh
edits rss fashion --list                    # the newest posts (--json for programs)
edits rss fashion -n 3                       # a video for each of the 3 newest
edits rss fashion --feed all -n 5 --preset sinema-bant
edits follow fashion --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Fashion - ELLE https://www.elle.com/rss/fashion.xml/
- Fashion | The Guardian https://www.theguardian.com/fashion/rss
- Fashionista Full https://fashionista.com/.rss/excerpt/
- Latest from Who What Wear https://www.whowhatwear.com/rss
- NYT > Style https://rss.nytimes.com/services/xml/rss/nyt/FashionandStyle.xml
- POPSUGAR Fashion https://www.popsugar.com/fashion/feed
- Refinery29 https://www.refinery29.com/fashion/rss.xml

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
