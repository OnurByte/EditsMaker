---
name: edits-world
description: "Make short videos from news from a country with edits: 25 topics (australia, bangladesh, brazil, canada, france, germany, hong-kong, india, indonesia, iran, ireland, italy and more), each new story's headline cut from footage found for it and shown on screen. Use when the user wants videos from news from a country, posts on autopilot, or an automation from RSS to video to social media."
---

# World

Every country reads its own newspapers; each new story becomes an edit.

```sh
edits rss japan --list                    # the newest posts (--json for programs)
edits rss japan -n 3                       # a video for each of the 3 newest
edits rss brazil --feed all -n 5 --preset sinema-bant
edits follow japan --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)
```

Topics:

- `australia`: Australia, 16 feeds (Latest News and News Headlines | Daily Telegraph, Sydney Morning Herald - Latest News, Latest News and News Headlines | Herald Sun)
- `bangladesh`: Bangladesh, 8 feeds (The Daily Star, BD24Live.com, bdnews24.com - Home)
- `brazil`: Brazil, 7 feeds (Folha de S.Paulo - Em cima da hora - Principal, Portal EBC - RSS, R7 - Notícias)
- `canada`: Canada, 9 feeds (CBC | Top Stories News, CTVNews.ca - Top Stories - Public RSS, Financial Post)
- `france`: France, 11 feeds (france24.com, Mediapart, Paris Star)
- `germany`: Germany, 5 feeds (ZEIT ONLINE | Nachrichten, Hintergründe und Debatten, FOCUS Online, Aktuell - FAZ.NET)
- `hong-kong`: Hong Kong, 5 feeds (Hong Kong Free Press HKFP, The Standard - Latest News, 頭條日報 Headline Daily - 頭條網)
- `india`: India, 36 feeds (BBC News - India, India | The Guardian, SEBI RSS Feed)
- `indonesia`: Indonesia, 4 feeds (Republika Online RSS Feed, Tribunnews.com, Merdeka.com)
- `iran`: Iran, 7 feeds (خبرگزاری باشگاه خبرنگاران | آخرین اخبار ایران و جهان | YJC, تابناک | TABNAK, خبرگزاری ایسنا | صفحه اصلی |  ISNA News Agency)
- `ireland`: Ireland, 6 feeds (TheJournal.ie, All: BreakingNews.ie, The42)
- `italy`: Italy, 13 feeds (RSS di   - ANSA.it, The Local, RSS DiariodelWeb.it)
- `japan`: Japan, 8 feeds (Japan Times latest articles, Japan Today, News On Japan)
- `mexico`: Mexico, 14 feeds (Mexico | The Guardian, Excélsior - RSS, Reforma)
- `myanmar`: Myanmar, 3 feeds (Myanmar Gazette News Media Forum Network, DVB Multimedia Group, THIT HTOO LWIN (Daily News))
- `nigeria`: Nigeria, 10 feeds (All Content, Nigeria News Links | Today's Updates - Nigerian Bulletin, Nigerian News. Latest Nigeria News. Your online Nigerian Newspaper.)
- `pakistan`: Pakistan, 7 feeds (The Express Tribune, The Nation - Top Stories, قومی خبریں)
- `philippines`: Philippines, 20 feeds (INQUIRER.net, Latest News | Philippine Information Agency, Interaksyon)
- `poland`: Poland, 8 feeds (Najnowsze, Newsweek Polska, Dziennik.pl Dziennik - dziennik.pl)
- `russia`: Russia, 13 feeds (Lenta.ru : Новости, Вести.Ru, Газета.Ru - Первая полоса)
- `south-africa`: South Africa, 11 feeds (SowetanLIVE, TechCentral, News24 Top Stories)
- `spain`: Spain, 9 feeds (The Local, EL PAÍS: el periódico global, España)
- `ukraine`: Ukraine, 12 feeds (News Agency UNIAN, ТЕЛЕГРАФ - последние новости Украины и мира, Последние новости на сайте korrespondent.net)
- `united-kingdom`: United Kingdom, 5 feeds (BBC News - Home, World news | The Guardian, Home | Mail Online)
- `united-states`: United States, 10 feeds (World News - Breaking News, Top Stories, NYT > Top Stories, FOX News)

Each video is footage found for the headline, with the headline on screen for its first seconds. Check a headline
before posting it, and say whose post it was. The feeds come from awesome-rss-feeds (CC0).
