---
name: edits-rss-books
description: "Make short videos from Books RSS (8 feeds, A year of reading the world, Aestas Book Blog, BOOK RIOT and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Books videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Books

Books from 8 feeds, A year of reading the world, Aestas Book Blog, BOOK RIOT: videos from their newest posts.

```sh
edits rss books --list                    # the newest posts (--json for programs)
edits rss books -n 3                       # a video for each of the 3 newest
edits rss books --feed all -n 5 --preset sinema-bant
edits follow books --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- A year of reading the world https://ayearofreadingtheworld.com/feed/
- Aestas Book Blog https://aestasbookblog.com/feed/
- BOOK RIOT https://bookriot.com/feed/
- Harry Potter and the Sacred Text https://rss.acast.com/harrypottersacredtext
- Kirkus Reviews https://www.kirkusreviews.com/feeds/rss/
- NewInBooks https://www.newinbooks.com/feed/
- So many books, so little time https://reddit.com/r/books/.rss
- Wokeread https://wokeread.home.blog/feed/

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
