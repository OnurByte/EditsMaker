---
name: edits-rss-food
description: "Make short videos from Food RSS (19 feeds, Binging with Babish, Bon Appétit, Budget Bytes and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Food videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Food

Food from 19 feeds, Binging with Babish, Bon Appétit, Budget Bytes: videos from their newest posts.

```sh
edits rss food --list                    # the newest posts (--json for programs)
edits rss food -n 3                       # a video for each of the 3 newest
edits rss food --feed all -n 5 --preset sinema-bant
edits follow food --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Binging with Babish https://www.youtube.com/feeds/videos.xml?user=bgfilms
- Bon Appétit https://www.youtube.com/feeds/videos.xml?user=BonAppetitDotCom
- Budget Bytes http://budgetbytes.blogspot.com/feeds/posts/default
- Chocolate & Zucchini https://cnz.to/feed/
- David Lebovitz https://www.davidlebovitz.com/feed/
- Dinner: A Love Story http://www.dinneralovestory.com/feed/
- Food52 http://feeds.feedburner.com/food52-TheAandMBlog
- Green Kitchen Stories https://greenkitchenstories.com/feed/
- How Sweet Eats https://www.howsweeteats.com/feed/
- Kitchn | Inspiring cooks, nourishing homes https://www.thekitchn.com/main.rss
- Laura in the Kitchen https://www.youtube.com/feeds/videos.xml?user=LauraVitalesKitchen
- Love and Lemons http://www.loveandlemons.com/feed/
- Love and Olive Oil https://www.loveandoliveoil.com/feed
- NYT > Food https://rss.nytimes.com/services/xml/rss/nyt/DiningandWine.xml
- Serious Eats https://www.youtube.com/feeds/videos.xml?user=SeriousEats
- Serious Eats: Recipes http://feeds.feedburner.com/seriouseats/recipes
- Shutterbean http://www.shutterbean.com/feed/
- Sprouted Kitchen https://www.sproutedkitchen.com/home?format=rss
- smitten kitchen http://feeds.feedburner.com/smittenkitchen

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
