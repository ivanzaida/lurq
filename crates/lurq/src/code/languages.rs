//! lurq's own languages, each a table the one scanner reads.

use super::{
  language::{CodeLanguage, CodeMode, KeyRule, NumberForms, StringForm},
  token,
};

/// Every language lurq registers a highlighter for at start.
pub(super) fn all() -> Vec<CodeLanguage> {
  vec![
    CodeLanguage::rust(),
    CodeLanguage::toml(),
    CodeLanguage::json(),
    CodeLanguage::javascript(),
    CodeLanguage::typescript(),
    CodeLanguage::shell(),
    CodeLanguage::python(),
    CodeLanguage::css(),
    CodeLanguage::html(),
    CodeLanguage::xml(),
    CodeLanguage::yaml(),
    CodeLanguage::sql(),
    CodeLanguage::markdown(),
  ]
}

/// The JavaScript of [`CodeLanguage::javascript`] and [`CodeLanguage::typescript`], under `name`.
fn script(name: &str) -> CodeLanguage {
  CodeLanguage::new(name)
    .words(
      token::KEYWORD,
      &[
        "var",
        "let",
        "const",
        "function",
        "class",
        "extends",
        "new",
        "delete",
        "typeof",
        "instanceof",
        "in",
        "of",
        "void",
        "async",
        "await",
        "import",
        "export",
        "from",
        "default",
        "as",
        "static",
        "get",
        "set",
        "debugger",
        "with",
      ],
    )
    .words(
      token::KEYWORD_CONTROL,
      &[
        "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "return", "throw", "try", "catch",
        "finally", "yield",
      ],
    )
    .words(
      token::CONSTANT_LANGUAGE,
      &["true", "false", "null", "undefined", "NaN", "Infinity"],
    )
    .words(token::VARIABLE_BUILTIN, &["this", "super", "arguments"])
    .declares(token::FUNCTION, &["function"])
    .declares(token::TYPE, &["class", "extends"])
    .line_comments(&["//"])
    .block_comment("/*", "*/", false)
    .doc_block_comment("/**", "*/")
    .strings(vec![
      StringForm::new("\"", "\""),
      StringForm::new("'", "'"),
      StringForm::new("`", "`").multiline(true),
    ])
    .numbers(NumberForms {
      prefixed: true,
      underscores: true,
      suffixes: true,
      dates: false,
    })
    .name_chars("_$", "_$")
    .capitals_are_types(true)
    .calls(true)
    .attributes(&["@"])
    .regexes(true)
}

impl CodeLanguage {
  /// Rust (`rust`, `rs`): its keywords, the words that change the flow, its own types, `self` and `Self`; line,
  /// documenting and nested block comments; strings, byte strings, raw strings with their `#`s, characters and
  /// lifetimes; numbers with `_` and suffixes; attributes (`#[derive`); macros (`println!`); a name before `::` a
  /// namespace, a name with a capital a type, one in capitals a constant; a name before `(` a function, and the
  /// names `fn`, `struct`, `enum`, `trait`, `type` and `mod` declare.
  pub fn rust() -> Self {
    Self::new("rust")
      .aliases(&["rs"])
      .words(
        token::KEYWORD,
        &[
          "as", "async", "await", "const", "crate", "dyn", "enum", "extern", "fn", "impl", "in", "let", "mod", "move",
          "mut", "pub", "ref", "static", "struct", "trait", "type", "union", "unsafe", "use", "where",
        ],
      )
      .words(
        token::KEYWORD_CONTROL,
        &[
          "break", "continue", "else", "for", "if", "loop", "match", "return", "while", "yield",
        ],
      )
      .words(token::CONSTANT_LANGUAGE, &["true", "false"])
      .words(token::VARIABLE_BUILTIN, &["self", "Self", "super"])
      .words(
        token::TYPE_BUILTIN,
        &[
          "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128",
          "isize", "f32", "f64",
        ],
      )
      .declares(token::FUNCTION, &["fn"])
      .declares(token::TYPE, &["struct", "enum", "trait", "type", "union"])
      .declares(token::NAMESPACE, &["mod"])
      .line_comments(&["//"])
      .doc_comments(&["///", "//!"])
      .block_comment("/*", "*/", true)
      .doc_block_comment("/**", "*/")
      .doc_block_comment("/*!", "*/")
      .strings(vec![
        StringForm::new("\"", "\"")
          .prefixes(&["r", "br", "cr"])
          .escapes(false)
          .hashes(true)
          .multiline(true),
        StringForm::new("\"", "\"").prefixes(&["", "b", "c"]).multiline(true),
      ])
      .characters(true)
      .numbers(NumberForms {
        prefixed: true,
        underscores: true,
        suffixes: true,
        dates: false,
      })
      .capitals_are_types(true)
      .path("::")
      .calls(true)
      .macros(true)
      .attributes(&["#![", "#["])
  }

  /// TOML (`toml`): sections (`[dependencies]`) and their names; keys, dotted and quoted, before `=`; the four
  /// forms of strings; numbers, dates and times; `true` and `false`; comments.
  pub fn toml() -> Self {
    Self::new("toml")
      .words(token::CONSTANT_LANGUAGE, &["true", "false", "inf", "nan"])
      .line_comments(&["#"])
      .strings(vec![
        StringForm::new("\"\"\"", "\"\"\"").multiline(true),
        StringForm::new("'''", "'''").escapes(false).multiline(true),
        StringForm::new("\"", "\""),
        StringForm::new("'", "'").escapes(false),
      ])
      .numbers(NumberForms {
        prefixed: true,
        underscores: true,
        suffixes: false,
        dates: true,
      })
      .name_chars("_-", "_")
      .keys(KeyRule::Equals)
      .sections(true)
  }

  /// JSON (`json`, and `jsonc` with its comments): keys, strings, numbers, `true`, `false` and `null`.
  pub fn json() -> Self {
    Self::new("json")
      .aliases(&["jsonc", "json5"])
      .words(token::CONSTANT_LANGUAGE, &["true", "false", "null"])
      .line_comments(&["//"])
      .block_comment("/*", "*/", false)
      .strings(vec![StringForm::new("\"", "\""), StringForm::new("'", "'")])
      .keys(KeyRule::Colon)
  }

  /// JavaScript (`javascript`, `js`, `jsx`, `mjs`): its keywords, constants and `this`; comments, documenting ones
  /// too; strings and template strings over lines; regular expressions where a value may start; numbers with `_`
  /// and `n`; decorators; a name with a capital a type, one before `(` a function, after `function` or `class`
  /// declared.
  pub fn javascript() -> Self {
    script("javascript").aliases(&["js", "jsx", "mjs", "cjs"])
  }

  /// TypeScript (`typescript`, `ts`, `tsx`): JavaScript's, with the words and the types of TypeScript.
  pub fn typescript() -> Self {
    script("typescript")
      .aliases(&["ts", "tsx", "mts", "cts"])
      .words(
        token::KEYWORD,
        &[
          "interface",
          "type",
          "enum",
          "implements",
          "declare",
          "namespace",
          "module",
          "abstract",
          "readonly",
          "private",
          "protected",
          "public",
          "keyof",
          "infer",
          "is",
          "satisfies",
          "asserts",
          "override",
          "unique",
        ],
      )
      .words(
        token::TYPE_BUILTIN,
        &[
          "string", "number", "boolean", "any", "unknown", "never", "object", "symbol", "bigint",
        ],
      )
      .declares(
        token::TYPE,
        &["class", "extends", "interface", "type", "enum", "implements"],
      )
      .declares(token::NAMESPACE, &["namespace", "module"])
  }

  /// A shell (`shell`, `sh`, `bash`, `zsh`, `console`): its keywords; the command each line or pipe starts with,
  /// a function; variables (`$HOME`, `${name}`, `$1`); strings, `'…'` with no escapes; comments from a `#` at a
  /// word's start.
  pub fn shell() -> Self {
    Self::new("shell")
      .aliases(&["sh", "bash", "zsh", "console", "shellscript", "ksh", "fish"])
      .words(
        token::KEYWORD_CONTROL,
        &[
          "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do", "done", "select", "time",
          "!",
        ],
      )
      .words(
        token::KEYWORD,
        &[
          "in", "function", "return", "local", "export", "readonly", "declare", "unset", "source", "alias", "exit",
        ],
      )
      .declares(token::FUNCTION, &["function"])
      .line_comments(&["#"])
      .comment_after_space(true)
      .strings(vec![
        StringForm::new("\"", "\"").multiline(true),
        StringForm::new("'", "'").escapes(false).multiline(true),
      ])
      .numbers(NumberForms {
        prefixed: false,
        underscores: false,
        suffixes: false,
        dates: false,
      })
      .name_chars("-./_:@%+,~", "-./_~")
      .variables(true)
      .commands(true)
      .commands_after(&[
        "if", "then", "else", "elif", "do", "while", "until", "time", "exec", "sudo", "!",
      ])
      .operators("=|&<>!")
  }

  /// Python (`python`, `py`): its keywords, the words that change the flow, `True`, `False`, `None`, `self`;
  /// strings with their prefixes (`r`, `b`, `f`), in three quotes over lines; decorators; numbers; comments; a
  /// name after `def` a function, after `class` a type, before `(` a function, with a capital a type.
  pub fn python() -> Self {
    let plain = ["", "b", "f", "u", "fb", "bf"];
    let raw = ["r", "br", "rb", "fr", "rf"];
    Self::new("python")
      .aliases(&["py", "python3", "pyi"])
      .words(
        token::KEYWORD,
        &[
          "and", "as", "assert", "async", "await", "class", "def", "del", "from", "global", "import", "in", "is",
          "lambda", "nonlocal", "not", "or", "pass",
        ],
      )
      .words(
        token::KEYWORD_CONTROL,
        &[
          "break", "case", "continue", "elif", "else", "except", "finally", "for", "if", "match", "raise", "return",
          "try", "while", "with", "yield",
        ],
      )
      .words(token::CONSTANT_LANGUAGE, &["True", "False", "None"])
      .words(token::VARIABLE_BUILTIN, &["self", "cls"])
      .declares(token::FUNCTION, &["def"])
      .declares(token::TYPE, &["class"])
      .line_comments(&["#"])
      .prefixes_any_case(true)
      .strings(vec![
        StringForm::new("\"\"\"", "\"\"\"")
          .prefixes(&raw)
          .escapes(false)
          .multiline(true),
        StringForm::new("'''", "'''")
          .prefixes(&raw)
          .escapes(false)
          .multiline(true),
        StringForm::new("\"\"\"", "\"\"\"").prefixes(&plain).multiline(true),
        StringForm::new("'''", "'''").prefixes(&plain).multiline(true),
        StringForm::new("\"", "\"").prefixes(&raw).escapes(false),
        StringForm::new("'", "'").prefixes(&raw).escapes(false),
        StringForm::new("\"", "\"").prefixes(&plain),
        StringForm::new("'", "'").prefixes(&plain),
      ])
      .numbers(NumberForms {
        prefixed: true,
        underscores: true,
        suffixes: true,
        dates: false,
      })
      .capitals_are_types(true)
      .calls(true)
      .attributes(&["@"])
  }

  /// CSS (`css`, and `scss` and `less` as far as they are CSS): selectors (an element a tag, `.class` and `#id`
  /// attributes, `:hover` a keyword); properties and their values, numbers with units, colors (`#fff`),
  /// functions, `!important`; `@media` and the other at-rules; custom properties (`--gap`); comments.
  pub fn css() -> Self {
    Self::new("css")
      .aliases(&["scss", "less"])
      .block_comment("/*", "*/", false)
      .strings(vec![StringForm::new("\"", "\""), StringForm::new("'", "'")])
      .numbers(NumberForms {
        prefixed: false,
        underscores: false,
        suffixes: true,
        dates: false,
      })
      .name_chars("-_", "-_")
      .keys(KeyRule::Declaration)
      .selectors(true)
      .at_words(true)
      .calls(true)
      .operators("=+*/>~:,")
  }

  /// HTML (`html`, `htm`, `vue`, `svelte`): tags and their names, attributes and their values, entities
  /// (`&amp;`), comments, `<!DOCTYPE`. The text between tags is plain, and so is a script's or a style's.
  pub fn html() -> Self {
    Self::new("html")
      .aliases(&["htm", "xhtml", "vue", "svelte"])
      .mode(CodeMode::Markup)
      .block_comment("<!--", "-->", false)
      .name_chars("-:._", "_:")
  }

  /// XML (`xml`, `svg`, `plist`): HTML's, with `<?xml … ?>` and CDATA, a string.
  pub fn xml() -> Self {
    Self::new("xml")
      .aliases(&["svg", "plist", "xsd", "xsl", "rss", "atom"])
      .mode(CodeMode::Markup)
      .block_comment("<!--", "-->", false)
      .strings(vec![StringForm::new("<![CDATA[", "]]>").escapes(false).multiline(true)])
      .name_chars("-:._", "_:")
  }

  /// YAML (`yaml`, `yml`): keys before `: `, quoted ones too; strings, `''` in one a quote; blocks of text under
  /// `|` and `>`; `true`, `false`, `null`, `~`; numbers; the marks of a document; comments.
  pub fn yaml() -> Self {
    Self::new("yaml")
      .aliases(&["yml"])
      .words(
        token::CONSTANT_LANGUAGE,
        &[
          "true", "false", "True", "False", "TRUE", "FALSE", "null", "Null", "NULL", "yes", "no", "on", "off",
        ],
      )
      .line_comments(&["#"])
      .comment_after_space(true)
      .strings(vec![
        StringForm::new("\"", "\"").multiline(true),
        StringForm::new("'", "'").escapes(false).doubled(true).multiline(true),
      ])
      .numbers(NumberForms {
        prefixed: true,
        underscores: true,
        suffixes: false,
        dates: true,
      })
      .name_chars("-_./", "_/")
      .keys(KeyRule::Colon)
      .block_scalars(true)
      .operators("-:?|>&*!~,")
  }

  /// SQL (`sql`, and its dialects by name): its keywords in any case, the words that change the flow, its types;
  /// strings, `''` in one a quote; quoted names; numbers; `--` and block comments; a name before `(` a function;
  /// `NULL`, `TRUE` and `FALSE`.
  pub fn sql() -> Self {
    Self::new("sql")
      .aliases(&[
        "postgres",
        "postgresql",
        "psql",
        "mysql",
        "sqlite",
        "plsql",
        "tsql",
        "mssql",
      ])
      .words_any_case(true)
      .words(
        token::KEYWORD,
        &[
          "select",
          "from",
          "where",
          "and",
          "or",
          "not",
          "insert",
          "into",
          "values",
          "update",
          "set",
          "delete",
          "create",
          "table",
          "drop",
          "alter",
          "add",
          "column",
          "index",
          "view",
          "primary",
          "key",
          "foreign",
          "references",
          "unique",
          "default",
          "constraint",
          "join",
          "inner",
          "left",
          "right",
          "full",
          "outer",
          "cross",
          "on",
          "as",
          "group",
          "by",
          "order",
          "having",
          "limit",
          "offset",
          "distinct",
          "union",
          "all",
          "exists",
          "in",
          "between",
          "like",
          "ilike",
          "is",
          "asc",
          "desc",
          "with",
          "recursive",
          "returning",
          "conflict",
          "do",
          "nothing",
          "cascade",
          "database",
          "schema",
          "if",
          "begin",
          "commit",
          "rollback",
          "transaction",
          "grant",
          "revoke",
          "trigger",
          "function",
          "procedure",
          "declare",
          "replace",
          "temporary",
          "temp",
          "using",
          "natural",
          "over",
          "partition",
          "window",
          "lateral",
          "fetch",
          "first",
          "next",
          "rows",
          "only",
          "check",
          "collate",
          "explain",
          "analyze",
          "vacuum",
          "pragma",
          "autoincrement",
          "serial",
        ],
      )
      .words(
        token::KEYWORD_CONTROL,
        &[
          "case", "when", "then", "else", "end", "return", "returns", "loop", "while", "for",
        ],
      )
      .words(
        token::TYPE_BUILTIN,
        &[
          "int",
          "integer",
          "bigint",
          "smallint",
          "tinyint",
          "decimal",
          "numeric",
          "real",
          "float",
          "double",
          "precision",
          "char",
          "varchar",
          "text",
          "boolean",
          "bool",
          "date",
          "time",
          "timestamp",
          "timestamptz",
          "interval",
          "uuid",
          "json",
          "jsonb",
          "blob",
          "bytea",
          "money",
        ],
      )
      .words(token::CONSTANT_LANGUAGE, &["null", "true", "false"])
      .line_comments(&["--"])
      .block_comment("/*", "*/", false)
      .strings(vec![
        StringForm::new("'", "'").escapes(false).doubled(true).multiline(true),
        StringForm::new("\"", "\"")
          .escapes(false)
          .doubled(true)
          .name(token::VARIABLE),
        StringForm::new("`", "`").escapes(false).name(token::VARIABLE),
      ])
      .numbers(NumberForms {
        prefixed: true,
        underscores: false,
        suffixes: false,
        dates: false,
      })
      .calls(true)
      .variables(true)
  }

  /// Markdown (`markdown`, `md`): headings, quotes, the markers of lists and thematic breaks by the start of their
  /// lines; fenced code over lines; in a line code spans, strong text, emphasis, links and their targets; comments.
  pub fn markdown() -> Self {
    Self::new("markdown")
      .aliases(&["md", "mdx", "gfm"])
      .mode(CodeMode::Markdown)
      .block_comment("<!--", "-->", false)
  }
}
