---
name: edits-rss-architecture
description: "Make short videos from Architecture RSS (11 feeds, A Weekly Dose of Architecture Books, ArchDaily Global, Architectural Digest and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Architecture videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Architecture

Architecture from 11 feeds, A Weekly Dose of Architecture Books, ArchDaily Global, Architectural Digest: videos from their newest posts.

```sh
edits rss architecture --list                    # the newest posts (--json for programs)
edits rss architecture -n 3                       # a video for each of the 3 newest
edits rss architecture --feed all -n 5 --preset sinema-bant
edits follow architecture --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- A Weekly Dose of Architecture Books http://feeds.feedburner.com/archidose
- ArchDaily Global http://feeds.feedburner.com/Archdaily
- Architectural Digest https://www.architecturaldigest.com/feed/rss
- Architectural Digest https://www.youtube.com/feeds/videos.xml?user=ArchitecturalDigest
- Architecture https://www.reddit.com/r/architecture/.rss
- Architecture news and projects | Dezeen https://www.dezeen.com/architecture/feed/
- Design MilkArchitecture, Architectural Designs, and House Designs | Design Milk https://design-milk.com/category/architecture/feed/
- Journal https://architizer.wpengine.com/feed/
- Living Big In A Tiny House https://www.youtube.com/feeds/videos.xml?user=livingbigtinyhouse
- The Architect’s Newspaper https://archpaper.com/feed
- architecture archives | designboom | architecture & design magazine https://www.designboom.com/architecture/feed/

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
