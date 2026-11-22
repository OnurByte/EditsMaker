---
name: edits-rss-funny
description: "Make short videos from Funny RSS (13 feeds, Cracked: All Posts, Explosm.net, FAIL Blog and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Funny videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Funny

Funny from 13 feeds, Cracked: All Posts, Explosm.net, FAIL Blog: videos from their newest posts.

```sh
edits rss funny --list                    # the newest posts (--json for programs)
edits rss funny -n 3                       # a video for each of the 3 newest
edits rss funny --feed all -n 5 --preset sinema-bant
edits follow funny --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Cracked: All Posts http://feeds.feedburner.com/CrackedRSS
- Explosm.net http://feeds.feedburner.com/Explosm
- FAIL Blog http://feeds.feedburner.com/failblog
- I Can Has Cheezburger? http://feeds.feedburner.com/icanhascheezburger
- PHD Comics http://phdcomics.com/gradfeed.php
- Penny Arcade https://www.penny-arcade.com/feed
- PostSecret https://postsecret.com/feed/?alt=rss
- Saturday Morning Breakfast Cereal https://www.smbc-comics.com/comic/rss
- The Bloggess https://thebloggess.com/feed/
- The Daily WTF http://syndication.thedailywtf.com/TheDailyWtf
- The Oatmeal - Comics by Matthew Inman http://feeds.feedburner.com/oatmealfeed
- The Onion https://www.theonion.com/rss
- xkcd.com https://xkcd.com/rss.xml

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
