---
name: edits-rss-photography
description: "Make short videos from Photography RSS (8 feeds, 500px, Digital Photography School, Light Stalking and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Photography videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Photography

Photography from 8 feeds, 500px, Digital Photography School, Light Stalking: videos from their newest posts.

```sh
edits rss photography --list                    # the newest posts (--json for programs)
edits rss photography -n 3                       # a video for each of the 3 newest
edits rss photography --feed all -n 5 --preset sinema-bant
edits follow photography --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- 500px https://iso.500px.com/feed/
- Digital Photography School https://feeds.feedburner.com/DigitalPhotographySchool
- Light Stalking https://www.lightstalking.com/feed/
- One Big Photo http://feeds.feedburner.com/OneBigPhoto
- PetaPixel https://petapixel.com/feed/
- Strobist http://feeds.feedburner.com/blogspot/WOBq
- Stuck in Customs https://stuckincustoms.com/feed/
- The Boston Globe https://www.bostonglobe.com/rss/bigpicture

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
