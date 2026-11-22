---
name: edits-rss-science
description: "Make short videos from Science RSS (23 feeds, 60-Second Science, BBC News, Discovery and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Science videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Science

Science from 23 feeds, 60-Second Science, BBC News, Discovery: videos from their newest posts.

```sh
edits rss science --list                    # the newest posts (--json for programs)
edits rss science -n 3                       # a video for each of the 3 newest
edits rss science --feed all -n 5 --preset sinema-bant
edits follow science --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- 60-Second Science http://rss.sciam.com/sciam/60secsciencepodcast
- BBC News http://feeds.bbci.co.uk/news/science_and_environment/rss.xml
- Discovery https://podcasts.files.bbci.co.uk/p002w557.rss
- FlowingData https://flowingdata.com/feed
- Gastropod https://www.omnycontent.com/d/playlist/aaea4e69-af51-495e-afc9-a9760146922b/2a195077-f014-41d2-8313-ab190186b4c2/277bcd5c-0a05-4c14-8ba6-ab190186b4d5/podcast.rss
- Hidden Brain https://feeds.npr.org/510308/podcast.xml
- Invisibilia https://feeds.npr.org/510307/podcast.xml
- Latest Science News -- ScienceDaily https://www.sciencedaily.com/rss/all.xml
- NYT > Science https://rss.nytimes.com/services/xml/rss/nyt/Science.xml
- Nature https://www.nature.com/nature.rss
- Phys.org - latest science and technology news stories https://phys.org/rss-feed/
- Popular Science https://www.popsci.com/arcio/rss/
- Probably Science https://probablyscience.libsyn.com/rss
- Radiolab http://feeds.wnyc.org/radiolab
- Reddit Science https://reddit.com/r/science/.rss
- Sawbones: A Marital Tour of Misguided Medicine https://feeds.simplecast.com/y1LF_sn2
- Science https://gizmodo.com/tag/science/rss
- Science Latest https://www.wired.com/feed/category/science/latest/rss
- Scientific American Content: Global http://rss.sciam.com/ScientificAmerican-Global
- Shirtloads of Science https://shirtloadsofscience.libsyn.com/rss
- TED Talks Daily https://pa.tedcdn.com/feeds/talks.rss
- The Infinite Monkey Cage https://podcasts.files.bbci.co.uk/b00snr0w.rss
- This Week in Science – The Kickass Science Podcast http://www.twis.org/feed/

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
