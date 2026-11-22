---
name: edits-rss-web-development
description: "Make short videos from Web Development RSS (5 feeds, A List Apart: The Full Feed, CSS-Tricks, Code Wall and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Web Development videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Web Development

Web Development from 5 feeds, A List Apart: The Full Feed, CSS-Tricks, Code Wall: videos from their newest posts.

```sh
edits rss web-development --list                    # the newest posts (--json for programs)
edits rss web-development -n 3                       # a video for each of the 3 newest
edits rss web-development --feed all -n 5 --preset sinema-bant
edits follow web-development --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- A List Apart: The Full Feed https://alistapart.com/main/feed/
- CSS-Tricks https://css-tricks.com/feed/
- Code Wall https://www.codewall.co.uk/feed/
- David Walsh Blog https://davidwalsh.name/feed
- Sink In - Tech Learnings https://gosink.in/rss/

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
