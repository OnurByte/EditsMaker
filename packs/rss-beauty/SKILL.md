---
name: edits-rss-beauty
description: "Make short videos from Beauty RSS (7 feeds, Beauty - ELLE, Fashionista Full, From Head To Toe and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Beauty videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Beauty

Beauty from 7 feeds, Beauty - ELLE, Fashionista Full, From Head To Toe: videos from their newest posts.

```sh
edits rss beauty --list                    # the newest posts (--json for programs)
edits rss beauty -n 3                       # a video for each of the 3 newest
edits rss beauty --feed all -n 5 --preset sinema-bant
edits follow beauty --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Beauty - ELLE https://www.elle.com/rss/beauty.xml/
- Fashionista Full https://fashionista.com/.rss/excerpt/beauty
- From Head To Toe http://feeds.feedburner.com/frmheadtotoe
- Into The Gloss - Beauty Tips, Trends, And Product Reviews https://feeds.feedburner.com/intothegloss/oqoU
- POPSUGAR Beauty https://www.popsugar.com/beauty/feed
- Refinery29 https://www.refinery29.com/beauty/rss.xml
- The Beauty Look Book https://thebeautylookbook.com/feed

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
