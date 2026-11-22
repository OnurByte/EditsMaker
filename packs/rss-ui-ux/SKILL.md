---
name: edits-rss-ui-ux
description: "Make short videos from UI / UX RSS (8 feeds, Articles on Smashing Magazine — For Web Designers And Developers, Boxes and Arrows, JUST™ Creative and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants UI / UX videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# UI / UX

UI / UX from 8 feeds, Articles on Smashing Magazine — For Web Designers And Developers, Boxes and Arrows, JUST™ Creative: videos from their newest posts.

```sh
edits rss ui-ux --list                    # the newest posts (--json for programs)
edits rss ui-ux -n 3                       # a video for each of the 3 newest
edits rss ui-ux --feed all -n 5 --preset sinema-bant
edits follow ui-ux --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Articles on Smashing Magazine — For Web Designers And Developers https://www.smashingmagazine.com/feed
- Boxes and Arrows http://boxesandarrows.com/rss/
- JUST™ Creative https://feeds.feedburner.com/JustCreativeDesignBlog
- NN/g latest articles and announcements https://www.nngroup.com/feed/rss/
- UX Collective - Medium https://uxdesign.cc/feed
- UX Movement https://uxmovement.com/feed/
- Usability Geek https://usabilitygeek.com/feed/
- User Experience https://www.reddit.com/r/userexperience/.rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
