---
name: edits-rss-football
description: "Make short videos from Football RSS (8 feeds, BBC Sport, CNN.com - RSS Channel - Sport - Football, EFL Championship and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Football videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Football

Football from 8 feeds, BBC Sport, CNN.com - RSS Channel - Sport - Football, EFL Championship: videos from their newest posts.

```sh
edits rss football --list                    # the newest posts (--json for programs)
edits rss football -n 3                       # a video for each of the 3 newest
edits rss football --feed all -n 5 --preset sinema-bant
edits follow football --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- BBC Sport https://feeds.bbci.co.uk/sport/football/rss.xml
- CNN.com - RSS Channel - Sport - Football http://rss.cnn.com/rss/edition_football.rss
- EFL Championship https://www.reddit.com/r/Championship/.rss?format=xml
- Football - The People's Sport https://www.reddit.com/r/football/.rss?format=xml
- Football News, Football Scores, Premier League, La Liga, ISL | The Hindu https://www.thehindu.com/sport/football/feeder/default.rss
- Football | The Guardian https://www.theguardian.com/football/rss
- NYT > Sports > Soccer https://rss.nytimes.com/services/xml/rss/nyt/Soccer.xml
- Soccer News https://www.soccernews.com/feed

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
