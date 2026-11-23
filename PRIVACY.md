# Privacy

EditsMaker runs on your computer. There is no account, no analytics and no tracking: nothing about you is collected.

## What stays on this computer

- Your videos, edits, presets, packs, settings and the usage numbers in Settings (`~/.config/edits/usage.jsonl`).
- Account sign-ins: kept in their own browser profiles and files only you can read. edits never sees a site's password.
- API keys: in `~/.config/edits/keys.json`, readable only by you.
- The plan and name shown beside an AI provider are read from that provider's own sign-in on this computer and never sent anywhere.

## What goes out, and only when it is needed

| When | Where | What |
| --- | --- | --- |
| You paste a link or name a topic | the video's site, YouTube search, RSS feeds | the request for that video or feed |
| The AI picks moments | the provider you chose (none, for a local model) | the transcript and the prompt |
| You post | the site you post to, from your account | the clip and its caption |
| Once a day, if updates are on | GitHub | a check for a newer release |
| You open the marketplace | GitHub | the community pack list |
| A 3D scene is drawn | jsDelivr | a download of three.js |
| Discord is running and the switch is on | Discord, on this computer | "Playing EditsMaker" and what you are making, shown on your profile |

Turn the Discord status off in Settings, General. Turn updates off there too.

Questions: [open an issue](https://github.com/OnurByte/EditsMaker/issues).
