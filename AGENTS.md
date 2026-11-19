# For agents working in this repository

How components are named and shaped is in `design/component-playbook.md`. What is here is what holds across the
whole repository, whoever writes it.

## Every word lurq writes by itself is in the app's one locale

lurq draws and reads out words of its own: the name of a button ("Play"), a placeholder, a line that says what
went wrong ("Unsupported format"), "Try again", a month. An app in another language, or with another voice, must
be able to replace every one of them in one place. That place is `lurq::core::Locale`, as `DateLocale` was for
dates before it, and now is a part of it (`Locale::dates`).

- **No literal in a component or in the engine for a word a person sees or a tool reads.** The word is a field of
  `Locale`, in the part for its area: `dates`, `media`, `input` and `display` (the signature pad, the angle
  slider, the QR code, the transfer and the cascader), `common` (what many components say alike: "Close", "More",
  "No results", a count chosen), `fields`, `navigation` (windows, panels, tabs, pages, steps, notifications, the
  Mac's menu of the app), `data` (tables, filters, sums, charts), `editing` (find, rich text, code, the command
  palette, Markdown's notes) and `keys`. A word of a new area adds a part, as `core/media_words.rs` does; a word
  of an area that has one goes into it. English is the default, and the part has the languages lurq knows: `en`,
  `uk`, `de`, `fr`, `es`, `pl`.
- **A component takes its words from `ui::current_locale()`** where it is built. The app sets its locale once
  (`App::set_locale(Locale::for_tag("uk"))`), and what read it is rendered again when it changes. Outside a render
  it is English.
- **A component may be given a locale of its own** (`.locale(&Locale)`), and the playbook's single words
  (`play_label`, `retry_text`) stay over both: one word told by itself is that word.
- **What went wrong is a kind, not a sentence.** A failure is an enum (`core::MediaError`); the locale puts it
  into words (`MediaWords::error`), short enough for the line that shows it. What the system said in its own
  words is kept beside the kind for a log (`MediaError::detail`), and is never shown to a person.
- **A sentence with a number in it** has a place for the number in the locale's word (`{status}`, `{at}` of
  `{total}`, filled with `core::fill`), or is a function of the locale, as a date's format is. Where the word after
  the number changes with it, the word is a `Plural` (one, few, many, other) and the locale says it
  (`locale.count(&locale.data.rows, n)`): Ukrainian and Polish have three forms, French says 0 as it says 1. Never
  an English-shaped "{n} items" or a `if n == 1`.

What is left as a literal on purpose: the names of keys a handler matches (`"Enter"`, `"ArrowDown"`), the names of
parts, states, roles and ids, the text forms of a state (a dock's, a table's), what a panic, a log, DevTools or a
`DevtoolsInspectable` says (they are for the developer, and in English), the abbreviations of a color's formats
(HEX, RGB, HSL, and the letters of its channels), `SummaryKind::label` (English, as its docs say), and the title of
the eyedropper's overlay window on Windows, which nothing shows. Numbers are written with a point in every
language: a number's format is not in the locale yet.

## Decisions are written down

What Ivan decides about lurq is written in `design/decisions.md` (and what is about phones and tablets in `design/mobile-decisions.md`) the day it is decided, before the work on it
starts: the question, the decision, why, and its state. Read it before working on something it names, and add to it
when a decision is taken in a session.

## Find your way with the code graph

The repository has a knowledge graph of its code, built by [Graphify](https://github.com/Graphify-Labs/graphify)
(installed with `uv tool install graphifyy`; the `/graphify` skill is installed for Claude Code). It is built
locally from the code's syntax, with no model and no key, into `graphify-out/` (ignored by git).

- **Build it in your checkout before you explore**, and again after you merge or pull a lot: `graphify update .`
  from the root of your worktree (about two minutes; it reuses what has not changed). A worktree has no graph of
  its own until you build it there.
- **Ask it before you search by hand**: `graphify query "how does LinkPreview fetch a page"`, `graphify explain
  "TextInput"` (a symbol and what it touches), `graphify path "A" "B"` (how two things connect), `graphify affected
  "X"` (what a change to X reaches), `graphify god-nodes` (the hubs). Read the files it points to; it is a map, not
  the source of truth.
- Docs, papers and pictures are not in the graph: that pass needs a model, and is not run.
