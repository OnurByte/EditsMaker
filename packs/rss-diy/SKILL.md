---
name: edits-rss-diy
description: "Make short videos from DIY RSS (8 feeds, A Beautiful Mess, Blog – Hackaday, Centsational Style and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants DIY videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# DIY

DIY from 8 feeds, A Beautiful Mess, Blog – Hackaday, Centsational Style: videos from their newest posts.

```sh
edits rss diy --list                    # the newest posts (--json for programs)
edits rss diy -n 3                       # a video for each of the 3 newest
edits rss diy --feed all -n 5 --preset sinema-bant
edits follow diy --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- A Beautiful Mess https://abeautifulmess.com/feed
- Blog – Hackaday https://hackaday.com/blog/feed/
- Centsational Style https://centsationalstyle.com/feed/
- How-To Geek https://www.howtogeek.com/feed/
- IKEA Hackers https://www.ikeahackers.net/feed
- MakeUseOf https://www.makeuseof.com/feed/
- Oh Happy Day! http://ohhappyday.com/feed/
- WonderHowTo https://www.wonderhowto.com/rss.xml

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
