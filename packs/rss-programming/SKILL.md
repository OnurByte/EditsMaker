---
name: edits-rss-programming
description: "Make short videos from Programming RSS (40 feeds, Better Programming - Medium, CodeNewbie, Coding Horror and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Programming videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Programming

Programming from 40 feeds, Better Programming - Medium, CodeNewbie, Coding Horror: videos from their newest posts.

```sh
edits rss programming --list                    # the newest posts (--json for programs)
edits rss programming -n 3                       # a video for each of the 3 newest
edits rss programming --feed all -n 5 --preset sinema-bant
edits follow programming --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Better Programming - Medium https://medium.com/feed/better-programming
- CodeNewbie http://feeds.codenewbie.org/cnpodcast.xml
- Coding Horror https://feeds.feedburner.com/codinghorror
- Developer Tea https://feeds.simplecast.com/dLRotFGk
- Engineering at Meta https://engineering.fb.com/feed/
- Engineering at Slack https://slack.engineering/feed
- Etsy Engineering | Code as Craft https://codeascraft.com/feed/atom/
- FLOSS Weekly (Audio) https://feeds.twit.tv/floss.xml
- GitLab https://about.gitlab.com/atom.xml
- Google Developers Blog http://feeds.feedburner.com/GDBcode
- Google TechTalks https://www.youtube.com/feeds/videos.xml?user=GoogleTechTalks
- HackerNoon.com - Medium https://medium.com/feed/hackernoon
- Hanselminutes with Scott Hanselman https://feeds.simplecast.com/gvtxUiIf
- InfoQ https://feed.infoq.com
- Java, SQL and jOOQ. https://blog.jooq.org/feed
- Joel on Software https://www.joelonsoftware.com/feed/
- Martin Fowler https://martinfowler.com/feed.atom
- Overflow - Buffer Resources https://buffer.com/resources/overflow/rss/
- Podcast Archives - Software Engineering Daily https://softwareengineeringdaily.com/category/podcast/feed
- Programming Archives - The Crazy Programmer https://www.thecrazyprogrammer.com/category/programming/feed
- Programming Throwdown http://feeds.feedburner.com/ProgrammingThrowdown
- Robert Heaton | Blog https://robertheaton.com/feed.xml
- Scott Hanselman's Blog http://feeds.hanselman.com/ScottHanselman
- Scripting News http://scripting.com/rss.xml
- Software Defined Talk https://feeds.fireside.fm/sdt/rss
- Software Engineering Radio - The Podcast for Professional Software Developers http://feeds.feedburner.com/se-radio
- SoundCloud Backstage Blog https://developers.soundcloud.com/blog/blog.rss
- Spotify Engineering https://labs.spotify.com/feed/
- Stack Abuse https://stackabuse.com/rss/
- Stack Overflow Blog https://stackoverflow.blog/feed/
- The 6 Figure Developer http://6figuredev.com/feed/rss/
- The Airbnb Tech Blog - Medium https://medium.com/feed/airbnb-engineering
- The Cynical Developer https://cynicaldeveloper.com/feed/podcast
- The GitHub Blog https://github.blog/feed/
- The JetBrains Blog https://blog.jetbrains.com/feed
- The PIT Show: Reflections and Interviews in the Tech World https://feeds.transistor.fm/productivity-in-tech-podcast
- The Stack Overflow Podcast https://feeds.simplecast.com/XA_851k3
- The Women in Tech Show: A Technical Podcast https://thewomenintechshow.com/category/podcast/feed/
- overreacted — A blog by Dan Abramov https://overreacted.io/rss.xml
- programming https://www.reddit.com/r/programming/.rss

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
