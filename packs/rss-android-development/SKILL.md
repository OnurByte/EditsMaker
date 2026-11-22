---
name: edits-rss-android-development
description: "Make short videos from Android Development RSS (23 feeds, Android Developers, Android Developers - Medium, Android Developers Backstage and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants Android Development videos from the news, posts on autopilot, or an automation from RSS to video to social media."
---

# Android Development

Android Development from 23 feeds, Android Developers, Android Developers - Medium, Android Developers Backstage: videos from their newest posts.

```sh
edits rss android-development --list                    # the newest posts (--json for programs)
edits rss android-development -n 3                       # a video for each of the 3 newest
edits rss android-development --feed all -n 5 --preset sinema-bant
edits follow android-development --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Feeds:

- Android Developers https://www.youtube.com/feeds/videos.xml?user=androiddevelopers
- Android Developers - Medium https://medium.com/feed/androiddevelopers
- Android Developers Backstage http://feeds.feedburner.com/blogspot/androiddevelopersbackstage
- Android Developers Blog http://feeds.feedburner.com/blogspot/hsDu
- Android in MindOrks on Medium https://medium.com/feed/mindorks/tagged/android
- Android in The Airbnb Tech Blog on Medium https://medium.com/feed/airbnb-engineering/tagged/android
- Chris Keenan Codes https://chrynan.codes/rss/
- Company | The JetBrains Blog https://blog.jetbrains.com/blog/feed
- Dan Lew Codes https://blog.danlew.net/rss/
- Developing Android Apps https://reddit.com/r/androiddev.rss
- Fragmented - AI Developer Podcast https://feeds.simplecast.com/LpAGSLnY
- Handstand Sam https://handstandsam.com/feed/
- Jake Wharton https://jakewharton.com/atom.xml
- Kotlin https://www.youtube.com/feeds/videos.xml?playlist_id=PLQ176FUIyIUa6SChjajjVc-LMzxWiz6dy
- OkKotlin https://okkotlin.com/rss.xml
- ProAndroidDev - Medium https://proandroiddev.com/feed
- Public Object https://publicobject.com/rss/
- Saket Narayan https://saket.me/feed/
- The Android Arsenal https://feeds.feedburner.com/Android_Arsenal
- Zac Sweers https://www.zacsweers.dev/rss/
- Zarah Dominguez https://zarah.dev/feed.xml
- droidcon SF https://www.youtube.com/feeds/videos.xml?channel_id=UCKubKoe1CBw_-n_GXetEQbg
- zsmb.co https://zsmb.co/index.xml

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.
