---
name: edits-post
description: "Post finished clips to TikTok, Instagram or YouTube from the person's own signed-in account with edits. Use when the user wants an edit posted on TikTok, Instagram or YouTube, or every new edit posted by itself."
---

# Post for you

Signs in to each site once, in a window of its own, and posts from the person's own account. edits never sees the password.

```sh
edits connect tiktok                         # once each: tiktok, instagram, youtube
edits post clip.mp4 --to tiktok --caption "..."
edits post clip.mp4 --to all                 # every connected account
edits follow <channel|topic> --post tiktok   # every new video, made and posted
```

Each site's script runs only on that site's own pages. Ask before posting anything the person did not ask to post.
