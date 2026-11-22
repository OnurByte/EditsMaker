---
name: edits-rss-cars
description: "Make short videos from Cars RSS (11 feeds, Autocar RSS Feed, BMWBLOG, BikeEXIF and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Cars videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Cars

Cars from 11 feeds, Autocar RSS Feed, BMWBLOG, BikeEXIF: videos from their newest posts.

```sh
edits rss cars --list                    # the newest posts (--json for programs)
edits rss cars -n 3                       # a video for each of the 3 newest
edits rss cars --feed all -n 5 --preset sinema-bant
edits follow cars --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Autocar RSS Feed https://www.autocar.co.uk/rss
- BMWBLOG https://feeds.feedburner.com/BmwBlog
- BikeEXIF https://www.bikeexif.com/feed
- Car Body Design https://www.carbodydesign.com/feed/
- Carscoops https://www.carscoops.com/feed/
- Formula 1 https://www.reddit.com/r/formula1/.rss
- Jalopnik - Obsessed with the culture of cars https://jalopnik.com/rss
- Latest Content - Car and Driver https://www.caranddriver.com/rss/all.xml/
- Speedhunters http://feeds.feedburner.com/speedhunters
- The best vintage and classic cars for sale online | Bring a Trailer https://bringatrailer.com/feed/
- TheTruthAboutCars https://www.thetruthaboutcars.com/feed/

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
