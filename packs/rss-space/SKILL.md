---
name: edits-rss-space
description: "Make short videos from Space RSS (6 feeds, /r/space: news, articles and discussion, Latest from Space.com, NASA and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Space videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Space

Space from 6 feeds, /r/space: news, articles and discussion, Latest from Space.com, NASA: videos from their newest posts.

```sh
edits rss space --list                    # the newest posts (--json for programs)
edits rss space -n 3                       # a video for each of the 3 newest
edits rss space --feed all -n 5 --preset sinema-bant
edits follow space --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- /r/space: news, articles and discussion https://www.reddit.com/r/space/.rss?format=xml
- Latest from Space.com https://www.space.com/feeds/all
- NASA https://www.nasa.gov/rss/dyn/breaking_news.rss
- New Scientist - Space https://www.newscientist.com/subject/space/feed/
- Space | The Guardian https://www.theguardian.com/science/space/rss
- SpaceX https://www.youtube.com/feeds/videos.xml?user=spacexchannel

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
