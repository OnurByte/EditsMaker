---
name: edits-rss-chess
description: "Make short videos from Chess RSS (16 feeds, Better Chess, Chess News, Chess Talk and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Chess videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Chess

Chess from 16 feeds, Better Chess, Chess News, Chess Talk: videos from their newest posts.

```sh
edits rss chess --list                    # the newest posts (--json for programs)
edits rss chess -n 3                       # a video for each of the 3 newest
edits rss chess --feed all -n 5 --preset sinema-bant
edits follow chess --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Better Chess https://betterchess.net/feed/
- Chess News https://en.chessbase.com/feed
- Chess Talk https://www.youtube.com/feeds/videos.xml?channel_id=UC6HDXr-sNPnWLF_Q-y3KduA
- Chess | The Guardian https://www.theguardian.com/sport/chess/rss
- Chess.com https://www.youtube.com/feeds/videos.xml?user=wwwChesscom
- Chess.com News https://www.chess.com/rss/news
- ChessBase India https://www.youtube.com/feeds/videos.xml?channel_id=UCIsEhwBMPkRHsEgqYAPQHsA
- Chessable Blog https://www.chessable.com/blog/feed/
- For the budding patzer https://www.reddit.com/r/chessbeginners/.rss
- Lichess's Blog https://lichess.org/blog.atom
- Perpetual Chess Podcast https://feeds.megaphone.fm/BLU4811105299
- The Week in Chess https://theweekinchess.com/twic-rss-feed
- agadmator's Chess Channel https://www.youtube.com/feeds/videos.xml?channel_id=UCL5YbN5WLFD8dLIegT5QAbA
- chess24 https://www.youtube.com/feeds/videos.xml?user=chess24media
- r/Chess https://www.reddit.com/r/chess/.rss
- thechesswebsite https://www.youtube.com/feeds/videos.xml?user=thechesswebsite

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
