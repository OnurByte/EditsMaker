---
name: edits-rss-apple
description: "Make short videos from Apple RSS (16 feeds, 9to5Mac, Apple, Apple Newsroom and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Apple videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Apple

Apple from 16 feeds, 9to5Mac, Apple, Apple Newsroom: videos from their newest posts.

```sh
edits rss apple --list                    # the newest posts (--json for programs)
edits rss apple -n 3                       # a video for each of the 3 newest
edits rss apple --feed all -n 5 --preset sinema-bant
edits follow apple --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- 9to5Mac https://9to5mac.com/feed
- Apple https://www.youtube.com/feeds/videos.xml?user=Apple
- Apple Newsroom https://www.apple.com/newsroom/rss-feed.rss
- AppleInsider News https://appleinsider.com/rss/news/
- Cult of Mac https://www.cultofmac.com/feed
- Daring Fireball https://daringfireball.net/feeds/main
- MacRumors https://www.youtube.com/feeds/videos.xml?user=macrumors
- MacRumors: Mac News and Rumors - Mac Blog http://feeds.macrumors.com/MacRumors-Mac
- MacStories https://www.macstories.net/feed
- Macworld https://www.macworld.com/index.rss
- Marco.org https://marco.org/rss
- OS X Daily http://feeds.feedburner.com/osxdaily
- The Loop https://www.loopinsight.com/feed
- iMore - The #1 iPhone, iPad, and iPod touch blog http://feeds.feedburner.com/TheiPhoneBlog
- r/Apple: Unofficial Apple Community https://www.reddit.com/r/apple/.rss
- r/iPhone https://www.reddit.com/r/iphone/.rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
