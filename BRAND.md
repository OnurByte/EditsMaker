# EditsMaker brand standards

<sub>Version 1, November 2026. Applies to the README, release pages, the app, the MCP server's text and anything else that carries the EditsMaker name.</sub>

## 0. Why these rules exist

A brand is a promise that keeps its shape. People judge a page in about 50 milliseconds, and that first judgement holds after they read further [9]. Anything that is quick to take in also feels better to look at [12]. So each rule below does one of two things: it makes EditsMaker recognisable at a glance, or it makes the page easier to take in.

The system descends from the 1976 NASA Graphics Standards Manual (NHB 1430.2) by Danne & Blackburn [5]: one logotype, one red, one typeface, a strict grid and nothing decorative. Since November 2026 one thing differs from the manual: sections sit on soft cards with round corners, because research and our readers both found that more comfortable (§ 6).

Every value here also lives as code: colours, type and sizes in [`src/app/theme.rs`](src/app/theme.rs), which the app draws with. The README pictures in [`assets/readme`](assets/readme) are SVG files, kept to these rules by hand. If this document and the code disagree, the code is the bug: fix it, then this page.

---

## 1. The mark

<img src="assets/mark.svg" width="96" alt="The EditsMaker mark">

A letter **E** drawn with one stroke width on a 96-unit grid, then cut on a slant. The cut half is red and drops half a stroke: a splice, a jump cut. It is the product in one shape.

| Rule | Value |
|---|---|
| Stroke | 14 of 96 units; bars and gaps never change width |
| Smallest size | 16 px on screen; below that use [`icon.svg`](assets/icon.svg) |
| Clear space | one stroke (14/96 of the mark's height) on every side |
| Colours | ink + red on paper, paper + red on ink; nothing else |

**Never**, following the manual's incorrect uses (NHB 1430.2 § 1.6) [5]: outline it, add a shadow or gradient, stretch, skew or rotate it, put a pattern over it, recolour the red half, or set it on a mid-tone background where neither half reads.

## 2. Name

- The product is **EditsMaker**: one word, capital E, capital M. The command is **`edits`**, always in code type.
- In sentences: "EditsMaker cuts…", never "the EditsMaker", never "Edits Maker" or "EDITSMAKER".
- The line is **The AI video harness.** with one full stop. It is never translated inside the logo lockup.

## 3. Colour

One red, one warm grey, ink and paper: the manual's economy [5]. Red is the only colour that ever means something.

| Token | Light | Dark | Use |
|---|---|---|---|
| `--ink` | `#0B0B0C` | `#F4F1EA` | text, the mark's first half |
| `--paper` | `#F4F1EA` | `#F4F1EA` | text on red |
| `--card` | `#F6F4EF` | `#161B22` | a section's card, one step off the page |
| `--hair` | `#E4E0D7` | `#30363D` | hairlines between rows |
| `--gray-text` | `#6F6A60` | `#9A958A` | labels, small notes |
| `--cut-red` | `#E5332A` | `#E5332A` | the accent: the mark, large type, bars, dots |
| `--red-text` | `#C8261E` | `#F25247` | red text smaller than 24 px (18.66 px bold) |
| `--red-deep` | `#C8261E` | `#C8261E` | red fills that carry small paper text |

### 3.1 Contrast, measured

All text meets WCAG 2.2 level AA [15]: 4.5 : 1 for body text, 3 : 1 for large text (24 px, or 18.66 px bold). Measured on the cards:

| Text | On light card | On dark card |
|---|---|---|
| Ink | 17.9 : 1 | 15.3 : 1 |
| Grey | 4.9 : 1 | 5.8 : 1 |
| Red text (`--red-text`) | 5.1 : 1 | 5.0 : 1 |
| Brand red (`--cut-red`), large only | 3.95 : 1 | 3.99 : 1 |
| Paper on `--red-deep` | 5.0 : 1 | 5.0 : 1 |

The brand red passes only at large sizes, which is why small red text switches to `--red-text`.

### 3.2 One red per view

Each picture, section or screen gets **one** red thing: the punchline, our row in a table, the step that is ours. An item that stands apart from its neighbours is noticed and remembered far more than the rest (the isolation effect) [14]. Two red things compete, and neither wins. Red is never put on a mid-tone background (NHB 1430.2 § 1.4) [5].

## 4. Type

**Helvetica** (Helvetica Neue, then Arial or Liberation Sans as stand-ins), in two weights only: **300 Light** for describing and **700 Bold** for naming. The manual sets the agency name in Light under a bold logotype, and so do we [5]. Code is in a monospace (`ui-monospace`, JetBrains Mono, SF Mono, Menlo).

| Role | Size / weight | Where |
|---|---|---|
| Display | 76 / 700, tracking −3.5 | the name in the title |
| Number | 64–104 / 700, tracking −3 to −6 | one big figure per view, e.g. 12.4 MB |
| Call to action | 32 / 700 | Download |
| Label | 19 / 700 | a section's name, left column |
| Body | 15 / 300 | what the section says |
| Item | 13–14 / 400–700 | table rows, lists, chips |
| Note | 11–12 / 400, grey | column headers, legends |

- **Line length**: at most 42 characters per column, 75 at the outside. Shorter lines are read with fewer regressions and are preferred by readers [6], [3].
- **Tracking**: tighten only at display sizes; body text keeps its default spacing.
- **Case**: sentence case everywhere. No all-caps headings; capitals are for caption punchlines inside videos only.

## 5. Layout

- **Grid**: three columns on a 960 px page at 0, 320 and 640. The **label sits on the left and the text on the right**, as in the manual (NHB 1430.2 § 5.14) [5]. Readers scan the left edge of a page first, then read across [11]: the left column is where the eye lands, so it carries the name of the thing.
- **Unit**: every space is a multiple of 8 px. A card has 40 px of padding; neighbouring items sit 12–40 px apart.
- **Grouping**: things that belong together sit close together and share a card; things that don't are separated by space before any line is drawn. Nearness is the strongest cue people use to group what they see [16].
- **Lines**: hairlines in `--hair` only, never black, never thicker than 1 px. A line only separates rows; it never frames anything.

## 6. Shape and comfort

- **Round corners**: 20 px on a card and the Download band, 6 px on anything inside a card (bars, boxes, chips get half their height). People prefer curved objects to sharp-angled ones, and sharp angles read as a mild threat [1]. The preference holds across studies, and it is driven as much by disliking angles as by liking curves [2]. Simple round shapes also carry warmer associations than angular ones [8].
- **Soft cards**: each section sits on `--card`, one step off GitHub's own page colour in both themes, so it reads as one calm unit and not as a box.
- **Room**: when in doubt, take an item away rather than shrink the space. What reads fluently feels more pleasant and more trustworthy [12].
- **No decoration**: no gradients, shadows, textures, emoji in pictures or icons for their own sake. Every mark on the page has to carry information.

## 7. Charts and comparisons

Our comparisons have to survive someone checking them.

- **Linear scale from zero** for every bar, so a bar twice as long means twice as big. A chart's visual effect must match the data's (Tufte's lie factor of 1) [13].
- **One measure per chart**. When a value is measured differently, it says so in the picture, next to the value ("to install", "npm + Chrome", "~ download"), never in a footnote.
- **Our row is the red one**, first, with its value in red.
- **Yes, partly, no**: a full, half and empty circle. Circles, not ticks: a half circle can say "partly", a tick cannot.
- Sources go into the README's folded **Sizes, sources** section with links, and are re-checked before every change to the numbers.

## 8. Other brands' logos

- Shown only to say what EditsMaker works with or is compared to (nominative use), **one colour**, at the same size as the text beside them.
- Taken from Simple Icons (CC0) or LobeHub icons (MIT).
- Where a brand has no free icon, such as Adobe, which asked Simple Icons to remove theirs, we set its **initials in our type** ("Pr", "Ae"). We never redraw or copy a logo.

## 9. Words

- **Plain words, short sentences, numbers before adjectives**: "12.4 MB", not "lightweight". Concrete claims are easier to process, and so feel truer [12].
- **One call to action**. Every extra choice adds decision time [7]: the page asks for one thing, Download.
- **Lists of at most six**. Working memory holds about four chunks at once [4]; six badges is the ceiling, and four is better.
- **The visible README is the sales page.** No licence, language, version, citations or caveats above the folded sections: those live below, in `<details>`. Honest labels inside a picture are fine; footnotes are not.
- Write "you" and "edits", not "the user" and "the software".

## 10. The README, in order

The first and the last things on a page are the ones people remember best (the serial position effect) [10]. So the page opens with what it is and how to get it, and closes with how to install it:

1. **Title**: the mark and the name, centred, "The AI video harness." Nothing above it.
2. **Download** band, red, linking to Install.
3. **Badges**: up to six chips, one red.
4. **Made with it**: the videos; proof before claims.
5. **How it works**: four steps, one command, the agent loop.
6. **Why it wins**: sounds and trends, size, sites.
7. **Everything it does**: features, effects, basics, presets.
8. **Install**: the two commands and the agent prompt.
9. Folded: command line, Premiere moves, presets, agents, install notes, sources, building, contributing.

## 11. Inside the videos

Edits carry the brand by their captions, not by a logo.

- Captions are 7 % of the video's width, with a 5 px outline, kept 28 % above the bottom of a vertical video, clear of TikTok's and Reels' buttons (`--caption-*` tokens).
- The punchline word is `--cut-red`; `src/render.rs` burns it in as BGR `2A33E5`, so keep the two in step.
- No watermark and no end card. The edit belongs to the person who made it.

## 12. Checklist before anything ships

- [ ] One red thing in view, and small red text uses `--red-text`
- [ ] Every text at 4.5 : 1, large text at 3 : 1, in both themes
- [ ] Helvetica in 300 and 700 only; columns at 42 characters or fewer
- [ ] Spaces on the 8 px unit; cards with 20 px corners and 40 px padding
- [ ] Bars from zero on a linear scale; any differently measured value labelled in the picture
- [ ] Other brands' logos in one colour, or their initials
- [ ] No licence, version or source text in the visible README
- [ ] Pictures and the app checked light and dark

---

## References

1. Bar, M., & Neta, M. (2006). Humans prefer curved visual objects. *Psychological Science, 17*(8), 645–648.
2. Bertamini, M., Palumbo, L., Gheorghes, T. N., & Galatsidas, M. (2016). Do observers like curvature or do they dislike angularity? *British Journal of Psychology, 107*(1), 154–178.
3. Bringhurst, R. (1992). *The elements of typographic style*. Hartley & Marks.
4. Cowan, N. (2001). The magical number 4 in short-term memory: A reconsideration of mental storage capacity. *Behavioral and Brain Sciences, 24*(1), 87–114.
5. Danne, R., & Blackburn, B. (1976). *NASA graphics standards manual* (NHB 1430.2). National Aeronautics and Space Administration.
6. Dyson, M. C. (2004). How physical text layout affects reading from screen. *Behaviour & Information Technology, 23*(6), 377–393.
7. Hick, W. E. (1952). On the rate of gain of information. *Quarterly Journal of Experimental Psychology, 4*(1), 11–26.
8. Larson, C. L., Aronoff, J., & Steuer, E. L. (2012). Simple geometric shapes are implicitly associated with affective value. *Motivation and Emotion, 36*(3), 404–413.
9. Lindgaard, G., Fernandes, G., Dudek, C., & Brown, J. (2006). Attention web designers: You have 50 milliseconds to make a good first impression! *Behaviour & Information Technology, 25*(2), 115–126.
10. Murdock, B. B. (1962). The serial position effect of free recall. *Journal of Experimental Psychology, 64*(5), 482–488.
11. Nielsen, J. (2006). *F-shaped pattern for reading web content*. Nielsen Norman Group.
12. Reber, R., Schwarz, N., & Winkielman, P. (2004). Processing fluency and aesthetic pleasure: Is beauty in the perceiver's processing experience? *Personality and Social Psychology Review, 8*(4), 364–382.
13. Tufte, E. R. (1983). *The visual display of quantitative information*. Graphics Press.
14. von Restorff, H. (1933). Über die Wirkung von Bereichsbildungen im Spurenfeld. *Psychologische Forschung, 18*(1), 299–342.
15. W3C. (2023). *Web Content Accessibility Guidelines (WCAG) 2.2*, success criterion 1.4.3, Contrast (Minimum).
16. Wertheimer, M. (1923). Untersuchungen zur Lehre von der Gestalt II. *Psychologische Forschung, 4*(1), 301–350.
