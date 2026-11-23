---
name: edits-news
description: "Make short videos from the news with edits: 42 topics (android, android-development, animal-and-wildlife, apple, architecture, beauty, books, business-and-economy, cars, chess, cricket, cryptocurrency and more), each new story's headline cut from footage found for it and shown on screen. Use when the user wants videos from the news on a topic, posts on autopilot, or an automation from RSS to video to social media."
---

# News

Every topic reads the feeds that cover it best; each new story becomes an edit.

```sh
edits rss football --list                    # the newest posts (--json for programs)
edits rss football -n 3                       # a video for each of the 3 newest
edits rss space --feed all -n 5 --preset sinema-bant
edits follow football --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Topics:

- `android`: Android, 16 feeds (All About Android (Audio), Android, Android)
- `android-development`: Android Development, 23 feeds (Android Developers, Android Developers - Medium, Android Developers Backstage)
- `animal-and-wildlife`: Animal & Wildlife, 8 feeds (Active Wild, Focusing on Wildlife, Love Nature)
- `apple`: Apple, 16 feeds (9to5Mac, Apple, Apple Newsroom)
- `architecture`: Architecture, 11 feeds (A Weekly Dose of Architecture Books, ArchDaily Global, Architectural Digest)
- `beauty`: Beauty, 7 feeds (Beauty - ELLE, Fashionista Full, From Head To Toe)
- `books`: Books, 8 feeds (A year of reading the world, Aestas Book Blog, BOOK RIOT)
- `business-and-economy`: Business & Economy, 17 feeds (All News, Bloomberg Originals, Breaking News on Seeking Alpha)
- `cars`: Cars, 11 feeds (Autocar RSS Feed, BMWBLOG, BikeEXIF)
- `chess`: Chess, 16 feeds (Better Chess, Chess News, Chess Talk)
- `cricket`: Cricket, 19 feeds (BBC Sport, Can't Bowl Can't Throw Cricket Show, Cricbuzz)
- `cryptocurrency`: Cryptocurrency, 27 feeds (Bitcoin & Crypto Trading: Ledger Cast, Bitcoin - The Currency of the Internet, Bitcoin Audible)
- `cyber-security`: Cyber security, 21 feeds (Blackhat Library: Hacking techniques and research, CyberWire Daily, Darknet Diaries)
- `diy`: DIY, 8 feeds (A Beautiful Mess, Blog – Hackaday, Centsational Style)
- `environment`: Environment, 12 feeds ("Human Nature" Blog Feed, Bionic Planet: Reversing Climate Change by Restoring Nature, Conservation International)
- `fashion`: Fashion, 7 feeds (Fashion - ELLE, Fashion | The Guardian, Fashionista Full)
- `food`: Food, 19 feeds (Binging with Babish, Bon Appétit, Budget Bytes)
- `football`: Football, 8 feeds (BBC Sport, CNN.com - RSS Channel - Sport - Football, EFL Championship)
- `funny`: Funny, 13 feeds (Cracked: All Posts, Explosm.net, FAIL Blog)
- `gaming`: Gaming, 14 feeds (Eurogamer.net Latest Articles Feed, GameSpot - All Content, IGN Articles)
- `history`: History, 7 feeds (30 for 30 Podcasts, Dan Carlin's Hardcore History, History in 28-minutes)
- `interior-design`: Interior design, 19 feeds (BLOG - decor8, Core77, Design MilkInterior Design Ideas for Your Modern Home | Design Milk)
- `ios-development`: iOS Development, 15 feeds (Alberto De Bortoli, Augmented Code, Benoit Pasquier)
- `memes`: Memes, 16 feeds (/r/Memes the original since 2008, A sub for History Memes, Advice Animals)
- `movies`: Movies, 9 feeds (/Film, Ain't It Cool News Feed, ComingSoon.net – Movie Trailers, TV & Streaming News, and More)
- `music`: Music, 5 feeds (Consequence, Metal Injection, Music Business Worldwide)
- `nature`: Nature, 14 feeds (BBC Earth, BBC Earth Podcast, BLOG POSTS – Mark Avery)
- `news`: News, 10 feeds (BBC News, CNN.com - RSS Channel - World, International: Top News And Analysis)
- `personal-finance`: Personal finance, 30 feeds (Afford Anything®, Afford Anything®, Budgets Are Sexy)
- `photography`: Photography, 8 feeds (500px, Digital Photography School, Light Stalking)
- `programming`: Programming, 40 feeds (Better Programming - Medium, CodeNewbie, Coding Horror)
- `science`: Science, 23 feeds (60-Second Science, BBC News, Discovery)
- `space`: Space, 6 feeds (/r/space: news, articles and discussion, Latest from Space.com, NASA)
- `sports`: Sports, 10 feeds (BBC Sport, CNN.com - RSS Channel - Sport, NYT > Sports)
- `startups`: Startups, 18 feeds (AVC, Blog on Feld Thoughts, Entrepreneur – Latest)
- `tech`: Tech, 28 feeds (Accidental Tech Podcast, Analog(ue), Ars Technica - All content)
- `television`: Television, 5 feeds (TV, TV Fanatic, TVLine - All TV. No Interference. TV News & Spoilers by TVLine)
- `tennis`: Tennis, 10 feeds (BBC Sport, CNN.com - RSS Channel - Sport - Tennis, Essential Tennis Podcast - Instruction, Lessons, Tips)
- `travel`: Travel, 12 feeds (Adventure.com, Atlas Obscura - Latest Articles and Places, CNN.com - RSS Channel - App Travel Section)
- `turkiye`: Türkiye, 6 feeds (Anadolu Ajansı Güncel, Anadolu Ajansı Dünya, Anadolu Ajansı Ekonomi)
- `ui-ux`: UI / UX, 8 feeds (Articles on Smashing Magazine — For Web Designers And Developers, Boxes and Arrows, JUST™ Creative)
- `web-development`: Web Development, 5 feeds (A List Apart: The Full Feed, CSS-Tricks, Code Wall)

Check a headline before posting it, and say whose post it was. The feeds come from awesome-rss-feeds (CC0).
