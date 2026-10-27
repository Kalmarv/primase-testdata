//! The self-check every consumer runs as one test ([`check_declared`]): every typed real-data query its sources write
//! names something its registry declares, by literals the check can see.
use std::path::{Path, PathBuf};

use crate::doc;
use crate::registry::Registry;

/// Fails, listing every problem, unless `registry` is whole ([`Registry::problems`]) and covers the real-data queries
/// of the crate whose folder is `crate_dir` ([`problems`]). A consumer runs it as one test:
///
/// ```text
/// mod inputs;
///
/// #[test]
/// fn real_data_inputs_are_declared() {
///     primase_testdata::check_declared(&inputs::REGISTRY, env!("CARGO_MANIFEST_DIR"));
/// }
/// ```
#[track_caller]
pub fn check_declared(registry: &Registry, crate_dir: impl AsRef<Path>) {
    let problems = problems(registry, crate_dir.as_ref());
    assert!(
        problems.is_empty(),
        "real-data inputs (declared in tests/inputs/mod.rs):\n{}",
        problems.join("\n")
    );
}

/// What [`check_declared`] fails on, in the crate whose folder is `crate_dir`:
/// - what the registry fails to say about itself ([`Registry::problems`]);
/// - in each source file that uses the inputs module (a path through `inputs::`), a query for a file of an install or
///   an oracle set (`oracle(..)`, `install_file(..)`, `Query::Oracle(..)`, `Query::Install(.., Some(..))`) whose
///   relative path is not a string literal or that the registry does not declare: the game may be a variable where the
///   set and path are literal (one file per game), and the run then refuses an undeclared game;
/// - an install or keys query naming a game literally (`install(Game::Sekiro)`) whose install or keys the registry does
///   not declare, and a debug key that is not a declared literal (`debug("shader.diff_limit")`);
/// - a source that re-exports the inputs module's query functions (`pub use` of them or of a glob), which the check
///   cannot follow;
/// - a source that names `GAME_DATA` or `GAME_DATA_DEBUG` itself: tests ask through their inputs module;
/// - a README (`README.md`, `tests/inputs/README.md`) whose block between [`doc::BEGIN`] and [`doc::END`] is not
///   [`doc::tables`].
pub fn problems(registry: &Registry, crate_dir: &Path) -> Vec<String> {
    let mut problems = registry.problems();
    for path in rust_files(crate_dir) {
        let shown = path
            .strip_prefix(crate_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // the module that declares the inputs writes their queries as declarations, not asks
        if shown.starts_with("tests/inputs/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            problems.push(format!("{shown}: unreadable"));
            continue;
        };
        problems.extend(
            variable_names(&text)
                .into_iter()
                .map(|n| format!("{shown}: names {n}: ask through the inputs module")),
        );
        problems.extend(reexports(&text).into_iter().map(|r| {
            format!(
                "{shown}: re-exports queries of the inputs module (`{r}`), which the check cannot follow: import them                  from the inputs module in each file that asks"
            )
        }));
        if !uses_inputs(&text) {
            continue;
        }
        problems.extend(
            file_problems(registry, &text)
                .into_iter()
                .map(|p| format!("{shown}: {p}")),
        );
    }
    let want = doc::tables(registry);
    for readme in ["README.md", "tests/inputs/README.md"] {
        let Ok(text) = std::fs::read_to_string(crate_dir.join(readme)) else {
            continue;
        };
        let Some((_, rest)) = text.split_once(doc::BEGIN) else {
            continue;
        };
        let block = rest
            .split_once(doc::END)
            .map(|(inside, _)| format!("{}{inside}{}\n", doc::BEGIN, doc::END));
        if block.as_deref() != Some(want.as_str()) {
            problems.push(format!(
                "{readme}: its real-data tables differ from the registry; between its markers they read:\n{want}"
            ));
        }
    }
    problems
}

/// The `.rs` files under `dir`, build output and hidden folders aside, sorted.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if p.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    pending.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Whether `text` uses an inputs module: a path through a segment `inputs`.
fn uses_inputs(text: &str) -> bool {
    text.match_indices("inputs::").any(|(at, _)| {
        !text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// The configuration's variables named in `text` as string literals, whole or begun by a format string.
pub(crate) fn variable_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(p) = rest.find("\"GAME_DATA") {
        let tail = &rest[p + 1..];
        let end = tail
            .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
            .unwrap_or(tail.len());
        if tail[end..].starts_with('"') || tail[end..].starts_with('{') {
            names.push(tail[..end].to_owned());
        }
        rest = &tail[end..];
    }
    names
}

/// The problems of one source file that uses the inputs module.
fn file_problems(registry: &Registry, text: &str) -> Vec<String> {
    let names = &registry.names;
    // `Game as Data`: what the file calls the inputs' enums
    let aliases = |name: &str| -> Vec<String> {
        let mut out = vec![name.to_owned()];
        out.extend(renames(&words(text), name).into_iter().map(str::to_owned));
        out
    };
    let (games, sets) = (aliases("Game"), aliases("OracleSet"));
    // the key `arg` names literally (`Game::Sekiro`, `inputs::Game::Sekiro`), if it is a literal of `enums`; `Err` for a
    // variant the vocabulary lacks
    let literal_of = |arg: &str,
                      enums: &[String],
                      vocabulary: &[(&str, &'static str)]|
     -> Option<Result<&'static str, String>> {
        let mut parts = arg.rsplit("::");
        let variant = parts.next()?;
        let ty = parts.next()?;
        if !enums.iter().any(|e| e == ty) || !variant.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        Some(
            vocabulary
                .iter()
                .find(|(v, _)| *v == variant)
                .map(|(_, key)| *key)
                .ok_or_else(|| format!("{ty}::{variant} is not declared")),
        )
    };
    let string = |a: &str| -> Option<String> {
        (a.len() >= 2 && a.starts_with('"') && a.ends_with('"') && !a[1..a.len() - 1].contains('"'))
            .then(|| a[1..a.len() - 1].to_owned())
    };
    let mut problems = Vec::new();
    for (kind, args) in query_calls(text) {
        let call = format!("{kind}({})", args.join(", "));
        // (set key or `install`, game argument, relative path argument) of a file query
        let (set, game, rel) = match (kind, args.as_slice()) {
            ("oracle" | "Query::Oracle", [set, game, rel]) => {
                match literal_of(set, &sets, names.oracle_sets) {
                    Some(Ok(set)) => (set, game, rel.clone()),
                    Some(Err(e)) => {
                        problems.push(format!("{call}: {e}"));
                        continue;
                    }
                    None => {
                        problems.push(format!("{call}: name the oracle set as OracleSet::<Set>"));
                        continue;
                    }
                }
            }
            ("install_file", [game, rel]) => ("install", game, rel.clone()),
            ("Query::Install", [game, some]) if some != "None" => {
                match some.strip_prefix("Some(").and_then(|r| r.strip_suffix(')')) {
                    Some(rel) => ("install", game, rel.trim().to_owned()),
                    None => {
                        problems.push(format!("{call}: write Some(\"path\") or None"));
                        continue;
                    }
                }
            }
            ("install" | "Query::Install" | "keys" | "Query::Keys", [game, ..]) => {
                let (section, entries) = if matches!(kind, "keys" | "Query::Keys") {
                    ("keys", registry.keys)
                } else {
                    ("installs", registry.installs)
                };
                match literal_of(game, &games, names.games) {
                    Some(Ok(key)) if !entries.iter().any(|e| e.name == key) => {
                        problems.push(format!("{call}: {section}.{key} is not declared"));
                    }
                    Some(Err(e)) => problems.push(format!("{call}: {e}")),
                    _ => {}
                }
                continue;
            }
            ("debug", [key]) => {
                match string(key) {
                    Some(key) if registry.debug.iter().any(|(k, _)| *k == key) => {}
                    Some(key) => problems.push(format!("{call}: debug.{key} is not declared")),
                    None => problems.push(format!("{call}: write the debug key as a string literal")),
                }
                continue;
            }
            _ => {
                problems.push(format!("{call} is not a query the check reads"));
                continue;
            }
        };
        let Some(rel) = string(&rel) else {
            problems.push(format!("{call}: write the relative path as a string literal"));
            continue;
        };
        let game = match literal_of(game, &games, names.games) {
            Some(Ok(key)) => Some(key),
            Some(Err(e)) => {
                problems.push(format!("{call}: {e}"));
                continue;
            }
            // a variable: any game's file of this set and path
            None => None,
        };
        let known = registry.declared.iter().any(|d| match d.query {
            crate::Query::Oracle(s, g, r) => s == set && r == rel && game.is_none_or(|game| game == g),
            crate::Query::Install(g, Some(r)) => {
                set == "install" && r == rel && game.is_none_or(|game| game == g)
            }
            _ => false,
        });
        if !known {
            problems.push(format!("{call} is not declared"));
        }
    }
    problems
}

/// The words of `text`: runs of letters, digits and `_`.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect()
}

/// What `words` rename `name` to (`name as alias`).
fn renames<'a>(words: &[&'a str], name: &str) -> Vec<&'a str> {
    words
        .windows(3)
        .filter_map(|w| match w {
            [n, "as", alias] if *n == name => Some(*alias),
            _ => None,
        })
        .collect()
}

/// The query functions the inputs module gives a test, by the word each is called with in a call.
const QUERY_FUNCTIONS: [&str; 5] = ["oracle", "install_file", "install", "keys", "debug"];

/// One `use` statement that names the inputs module.
struct Import<'a> {
    words: Vec<&'a str>,
    glob: bool,
    /// `pub use`, `pub(crate) use`: it hands what it imports on.
    public: bool,
}

/// Whether `before`, the text ahead of a `use`, ends in `pub` or `pub(..)`.
fn is_pub(before: &str) -> bool {
    let before = before.trim_end();
    before.ends_with("pub")
        || before.strip_suffix(')').is_some_and(|rest| {
            rest.rsplit_once("pub(").is_some_and(|(_, scope)| {
                scope
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == ' ')
            })
        })
}

/// The `use` statements of `code` that name the inputs module.
fn inputs_imports(code: &str) -> Vec<Import<'_>> {
    code.match_indices("use ")
        .filter(|&(at, _)| {
            !code
                .split_at(at)
                .0
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
        .filter_map(|(at, _)| {
            let (before, rest) = code.split_at(at);
            let statement = rest.split(';').next().unwrap_or_default();
            let words = words(statement);
            let public = is_pub(before);
            words.contains(&"inputs").then(|| Import {
                words,
                glob: statement.contains('*'),
                public,
            })
        })
        .collect()
}

/// The `pub use` statements of `text` that hand on the inputs module's query functions (by name or by a glob): a
/// test that calls them through the re-export is out of the check's sight.
pub(crate) fn reexports(text: &str) -> Vec<String> {
    inputs_imports(text)
        .into_iter()
        .filter(|i| i.public && (i.glob || QUERY_FUNCTIONS.iter().any(|q| i.words.contains(q))))
        .map(|i| i.words.join(" "))
        .collect()
}

/// The calls of a typed real-data query in a source text, each with its arguments as written (top-level commas,
/// comments left out, any path before the last `inputs::` dropped): `oracle(..)`, `install_file(..)`, `install(..)`,
/// `keys(..)` and `debug(..)` called through `inputs::` (or the name a `use` renames the module to), imported from it
/// by a `use` through `inputs::` (under the name the import gives, or by a glob), and `Query::Oracle(..)`,
/// `Query::Install(..)` and `Query::Keys(..)`. A function re-exported from the inputs module is not followed:
/// [`reexports`] names those.
pub(crate) fn query_calls(text: &str) -> Vec<(&'static str, Vec<String>)> {
    let code: String = text
        .lines()
        .map(|l| if l.trim_start().starts_with("//") { "" } else { l })
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    let imports = inputs_imports(&code);
    // what the file calls the module: `inputs`, or a name a `use` gives it
    let modules: Vec<String> = std::iter::once("inputs")
        .chain(imports.iter().flat_map(|i| renames(&i.words, "inputs")))
        .map(|m| format!("{m}::"))
        .collect();
    // the names a `use ...inputs::...;` brings in, however many lines its braces take
    let imported_as = |name: &str| -> Vec<String> {
        let mut names = Vec::new();
        for import in &imports {
            let renamed = renames(&import.words, name);
            if import.glob || import.words.contains(&name) {
                names.push(name.to_owned());
            }
            names.extend(renamed.into_iter().map(str::to_owned));
        }
        names
    };
    let strip = |a: &str| -> String {
        let a = a.trim();
        modules
            .iter()
            .filter_map(|m| a.rsplit_once(m.as_str()))
            .map(|(_, rest)| rest)
            .min_by_key(|rest| rest.len())
            .unwrap_or(a)
            .to_owned()
    };
    let mut calls = Vec::new();
    let typed = ["Query::Oracle", "Query::Install", "Query::Keys"];
    let searched = QUERY_FUNCTIONS
        .iter()
        .flat_map(|&kind| {
            let mut names = imported_as(kind);
            names.push(kind.to_owned());
            names.sort_unstable();
            names.dedup();
            names.into_iter().map(move |name| (kind, name, false))
        })
        .chain(typed.iter().map(|&kind| (kind, kind.to_owned(), true)));
    for (kind, name, is_typed) in searched {
        let word = format!("{name}(");
        let imported = is_typed || imported_as(kind).contains(&name);
        let mut from = 0;
        while let Some(i) = code
            .get(from..)
            .and_then(|rest| rest.find(&word))
            .map(|i| i + from)
        {
            from = i + word.len();
            let before = code.get(..i).unwrap_or_default();
            let prev = before.chars().next_back();
            if prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                continue;
            }
            let qualified = modules.iter().any(|m| before.ends_with(m.as_str()));
            if !is_typed && !qualified && !(prev != Some(':') && imported) {
                continue;
            }
            if before.trim_end().ends_with("fn") {
                continue;
            }
            // the arguments up to the matching parenthesis
            let (mut depth, mut args, mut arg, mut quoted) = (1, Vec::new(), String::new(), false);
            let mut escaped = false;
            for c in code.get(from..).unwrap_or_default().chars() {
                match c {
                    _ if escaped => escaped = false,
                    '\\' if quoted => escaped = true,
                    '"' => quoted = !quoted,
                    '(' if !quoted => depth += 1,
                    ')' if !quoted => depth -= 1,
                    ',' if !quoted && depth == 1 => {
                        args.push(std::mem::take(&mut arg));
                        continue;
                    }
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
                arg.push(c);
            }
            args.push(arg);
            let args = args.iter().map(|a| strip(a)).filter(|a| !a.is_empty()).collect();
            calls.push((kind, args));
        }
    }
    calls
}
