---
name: edits-rss-environment
description: "Make short videos from Environment RSS (12 feeds, \"Human Nature\" Blog Feed, Bionic Planet: Reversing Climate Change by Restoring Nature, Conservation International and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Environment videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Environment

Environment from 12 feeds, "Human Nature" Blog Feed, Bionic Planet: Reversing Climate Change by Restoring Nature, Conservation International: videos from their newest posts.

```sh
edits rss environment --list                    # the newest posts (--json for programs)
edits rss environment -n 3                       # a video for each of the 3 newest
edits rss environment --feed all -n 5 --preset sinema-bant
edits follow environment --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- "Human Nature" Blog Feed http://feeds.feedburner.com/ConservationInternationalBlog
- Bionic Planet: Reversing Climate Change by Restoring Nature https://bionicplanet.libsyn.com/rss
- Conservation International https://www.youtube.com/feeds/videos.xml?channel_id=UCam5sCp6mzGBcn8ZBB2RBJg
- Environment https://www.reddit.com/r/environment/.rss
- Environment + Energy – The Conversation https://theconversation.com/au/environment/articles.atom
- Environment News, Earth News, Global Warming, Wild Life, Carbon Trading, Climate Business, Climate Change & Pollution News https://timesofindia.indiatimes.com/rssfeeds/2647163.cms
- Environment | The Guardian https://www.theguardian.com/us/environment/rss
- Good Good Good https://www.goodgoodgood.co/articles/rss.xml
- NPR Topics: Environment https://feeds.npr.org/1025/rss.xml
- NYT > Climate and Environment https://rss.nytimes.com/services/xml/rss/nyt/Climate.xml
- Prakati India https://www.prakati.in/feed/
- Vox https://www.vox.com/rss/energy-and-environment/index.xml

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
