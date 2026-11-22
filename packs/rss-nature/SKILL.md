---
name: edits-rss-nature
description: "Make short videos from Nature RSS (14 feeds, BBC Earth, BBC Earth Podcast, BLOG POSTS – Mark Avery and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Nature videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Nature

Nature from 14 feeds, BBC Earth, BBC Earth Podcast, BLOG POSTS – Mark Avery: videos from their newest posts.

```sh
edits rss nature --list                    # the newest posts (--json for programs)
edits rss nature -n 3                       # a video for each of the 3 newest
edits rss nature --feed all -n 5 --preset sinema-bant
edits follow nature --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- BBC Earth https://www.youtube.com/feeds/videos.xml?channel_id=UCwmZiChSryoWQCZMIQezgTg
- BBC Earth Podcast https://rss.acast.com/bbcearthpodcast
- BLOG POSTS – Mark Avery https://markavery.info/blog/feed/
- Blog | Nature | PBS https://www.pbs.org/wnet/nature/blog/feed/
- EarthPorn: Amazing images of light and landscape https://www.reddit.com/r/EarthPorn/.rss
- Explore Oceans https://www.youtube.com/feeds/videos.xml?channel_id=UCSyg9cb3Iq-NtlbxqNB9wGw
- National Geographic https://www.youtube.com/feeds/videos.xml?channel_id=UCpVm7bg6pXKo1Pr6k5kxG9A
- Nature http://feeds.nature.com/nature/rss/current?x=1
- Nature https://www.reddit.com/r/nature/.rss
- Nature Gifs https://www.reddit.com/r/NatureGifs/.rss
- Nature on PBS https://www.youtube.com/feeds/videos.xml?channel_id=UCcBp_9YPyma4c3HTadmRJ3Q
- Reflections of the Natural World https://reflectionsofthenaturalworld.com/feed/
- Stories Archives - Nature Canada https://naturecanada.ca/category/news/blog/feed/
- nature video https://www.youtube.com/feeds/videos.xml?channel_id=UC7c8mE90qCtu11z47U0KErg

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
