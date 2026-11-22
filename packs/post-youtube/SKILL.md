---
name: edits-post-youtube
description: "Post finished clips to youtube from the person's own signed-in account with edits (Post to YouTube). Use when the user wants an edit posted on youtube."
---

# Post to YouTube

Signs in to YouTube once in a window of its own and posts your clips there from your own account: edits connect youtube, then edits post --to youtube.

```sh
edits connect youtube                     # once: sign in, in a window of its own
edits post clip.mp4 --to youtube --caption "..."
edits follow <channel|feed> --post youtube   # every new video, made and posted
```

It runs only on youtube, from the person's own account. Ask before posting anything they did not ask to post.
