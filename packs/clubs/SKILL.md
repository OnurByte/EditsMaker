---
name: edits-clubs
description: "Make short videos from a football club's news with edits: 12 topics (galatasaray, fenerbahce, besiktas, trabzonspor, turkish-football, real-madrid, barcelona, arsenal, tottenham, bayern, juventus, fifa), each new story's headline cut from footage found for it and shown on screen. Use when the user wants videos from a football club's news, posts on autopilot, or an automation from RSS to video to social media."
---

# Football clubs

Every club is read from the paper or the fans' blog that follows it closest; each new story becomes an edit.

```sh
edits rss galatasaray --list                    # the newest posts (--json for programs)
edits rss galatasaray -n 3                       # a video for each of the 3 newest
edits rss real-madrid --feed all -n 5 --preset sinema-bant
edits follow galatasaray --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Topics:

- `galatasaray`: Galatasaray, 1 feed (Fotomaç Galatasaray)
- `fenerbahce`: Fenerbahçe, 1 feed (Fotomaç Fenerbahçe)
- `besiktas`: Beşiktaş, 1 feed (Fotomaç Beşiktaş)
- `trabzonspor`: Trabzonspor, 1 feed (Fotomaç Trabzonspor)
- `turkish-football`: Türk futbolu, 2 feeds (Fotomaç Futbol, NTV Spor Futbol)
- `real-madrid`: Real Madrid, 1 feed (Managing Madrid)
- `barcelona`: Barcelona, 1 feed (Barça Blaugranes)
- `arsenal`: Arsenal, 1 feed (football.london Arsenal)
- `tottenham`: Tottenham, 1 feed (Cartilage Free Captain)
- `bayern`: Bayern Munich, 1 feed (Bavarian Football Works)
- `juventus`: Juventus, 1 feed (Black & White & Read All Over)
- `fifa`: FIFA, 1 feed (FIFA)

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline
before posting it, and say whose post it was.
