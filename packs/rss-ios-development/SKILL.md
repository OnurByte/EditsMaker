---
name: edits-rss-ios-development
description: "Make short videos from iOS Development RSS (15 feeds, Alberto De Bortoli, Augmented Code, Benoit Pasquier and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants iOS Development videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# iOS Development

iOS Development from 15 feeds, Alberto De Bortoli, Augmented Code, Benoit Pasquier: videos from their newest posts.

```sh
edits rss ios-development --list                    # the newest posts (--json for programs)
edits rss ios-development -n 3                       # a video for each of the 3 newest
edits rss ios-development --feed all -n 5 --preset sinema-bant
edits follow ios-development --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Alberto De Bortoli https://albertodebortoli.com/rss/
- Augmented Code https://augmentedcode.io/feed/
- Benoit Pasquier https://benoitpasquier.com/index.xml
- Bruno Rocha https://swiftrocks.com/rss.xml
- Latest News - Apple Developer https://developer.apple.com/news/rss/news.rss
- Mobile A11y https://mobilea11y.com/index.xml
- Ole Begemann https://oleb.net/blog/atom.xml
- Swift by Sundell https://www.swiftbysundell.com/feed.rss
- Swift by Sundell https://swiftbysundell.com/feed.rss
- The Atomic Birdhouse https://atomicbird.com/index.xml
- The One And Only Blog Of Joe Fabisevich https://www.fabisevi.ch/feed.xml
- Under the Radar https://www.relay.fm/radar/feed
- Use Your Loaf - iOS Development News & Tips https://useyourloaf.com/blog/rss.xml
- inessential.com https://inessential.com/xml/rss.xml
- tyler.io https://tyler.io/feed/

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
