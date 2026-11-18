# How Windows' own text fields group typing into steps of undo

Asked on 2026-11-18 on Windows 11 (build 26100), for `design/decisions.md`, "On Windows a run of typing is one
step of undo, as on a Mac". lurq had taken Windows' fields to make each word typed a step of its own, from what is
known of them; nobody had looked.

[`richedit_undo.py`](richedit_undo.py) makes hidden controls of its own, sends them characters and keys as
window messages (no input of the system's, nothing sent to another program), undoes until nothing is left, and
prints what each undo leaves. [`output.txt`](output.txt) is what it printed:

- **Windows' RichEdit** (`Msftedit.dll` 10.0.26100.8117, class `RICHEDIT50W`): WordPad's, and what XAML's
  `TextBox` and `RichEditBox` are built on.
- **Office's RichEdit** (`riched20.dll` 16.0.20330.42276, class `RICHEDIT60W`, in plain text with undo of many
  levels): the one Windows 11's Notepad (11.2609.11.0) ships in its own package and types into.
- **The classic edit control** (`EDIT`): Windows 10's Notepad and every Win32 field.

## What they do

| Typed | Windows' RichEdit | Notepad's RichEdit | lurq on a Mac (`Runs`) |
| --- | --- | --- | --- |
| `hello big world` | one step | one step | one step |
| `one,two.three` | one step | one step | one step |
| `abc`, 1.5 s, `def` | one step | one step | two (lurq's pause, `App::set_typing_undo_pause`) |
| `hello world`, Backspace ×3 | one step | two: the deleting, then the typing | two |
| `abc`, Left, `X` | two | two | two |

Neither RichEdit makes a word a step: a run of typing is one until the caret moves (and, in Notepad's, until
typing turns to deleting), as `NSTextView` does. The classic edit control has one level of undo, a toggle between
the text before and after the last edit; it is no model for undo of many levels.

What an app does on top of its engine is not seen here: a host may end a group by itself
(`EM_STOPGROUPTYPING`). Notepad's own window was not driven, for no input is sent to another program; what was
asked is the engine it types into. Word groups by word, but Word is not a field.

## So

`UndoGrouping::PLATFORM` is `Runs` on Windows too. The pause of a second that ends a step is lurq's own, on every
system, as before.
