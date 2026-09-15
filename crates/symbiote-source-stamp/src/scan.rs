//! The inputs one source pulls in.
//!
//! A package's files are recorded by the directory walk in [`crate::walk`], but
//! a compile reads more than the files beside a crate: `include!`,
//! `include_bytes!` and `include_str!` name a file wherever it lives, and a
//! `#[path = "…"]` module attribute names a module source the compiler finds
//! outside the package. This module reads those out of the code — comments and
//! string literals are not code — and follows them to a fixed point, because an
//! included `.rs` file can include another.
//!
//! What it cannot follow it reports rather than skips: a record that cannot name
//! an input a binary compiled is the failure the crate exists to prevent.

use std::path::{Path, PathBuf};

/// The macros through which a source pulls in another file at compile time.
/// The scan compares identifiers, so the `!` that makes each one a macro is not
/// part of the name it looks for.
const INCLUDE_MACROS: [&str; 3] = ["include", "include_bytes", "include_str"];

/// The inputs one source pulls in — the files its include macros name and the
/// module sources its `#[path]` attributes name — or the reason this scan cannot
/// follow one of them. A literal path is resolved the way the compiler resolves
/// it — relative to the file that names it — and a `concat!` of literals and
/// `env!("CARGO_MANIFEST_DIR")` is resolved against `package`, the package
/// directory the caller found the file in. An input the compiler takes from
/// `OUT_DIR` is left to the build script that wrote it, which is itself
/// recorded.
pub(crate) fn followed_inputs(
    source: &Path,
    package: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let Ok(text) = std::fs::read_to_string(source) else {
        return Ok(Vec::new());
    };
    let Some(directory) = source.parent() else {
        return Ok(Vec::new());
    };
    let mut inputs = Vec::new();
    for include in include_arguments(&text) {
        match include {
            Include::Generated => {}
            Include::Literal(path) => inputs.push(directory.join(path)),
            Include::Parts(parts) => {
                let mut joined = String::new();
                for part in parts {
                    match part {
                        Part::Text(text) => joined.push_str(&text),
                        Part::ManifestDir => {
                            let package = package.ok_or_else(|| {
                                format!(
                                    "{} is not inside a workspace package, so its \
                                     env!(\"CARGO_MANIFEST_DIR\") cannot be resolved",
                                    source.display()
                                )
                            })?;
                            joined.push_str(&package.to_string_lossy());
                        }
                        Part::OutDir | Part::Variable => {
                            unreachable!(
                                "evaluate leaves OUT_DIR and unknown variables out of Parts"
                            )
                        }
                    }
                }
                let path = PathBuf::from(joined);
                inputs.push(if path.is_absolute() {
                    path
                } else {
                    directory.join(path)
                });
            }
            Include::Unknown(snippet) => {
                return Err(format!(
                    "{} names an input through a path it builds while compiling, which the source \
                     record cannot follow: {snippet}. Write the path as a literal, or as a \
                     concat! of literals and env!(\"CARGO_MANIFEST_DIR\"), so the record covers \
                     every input the binary compiles",
                    source.display()
                ));
            }
        }
    }
    for module in module_paths(&text) {
        match module {
            ModulePath::File { path, inline } => {
                inputs.extend(module_inputs(source, &path, &inline));
            }
            ModulePath::Inline { path } => {
                return Err(format!(
                    "{} attaches a `#[path = {path:?}]` to an inline module, which moves the \
                     directory that module's children are found in — including children this \
                     scan does not model — so the source record cannot follow it. Declare the \
                     module in its own file, where the record covers it",
                    source.display()
                ));
            }
            ModulePath::Unreadable(snippet) => {
                return Err(format!(
                    "{} names a module source through a `#[path]` this scan cannot read: \
                     {snippet}. Write it as a literal `#[path = \"…\"]` attribute directly on a \
                     `mod name;` declaration, so the record covers every input the binary \
                     compiles",
                    source.display()
                ));
            }
        }
    }
    Ok(inputs)
}

/// A module source a `#[path]` attribute names, as far as the scan can follow
/// it.
#[derive(Debug)]
pub(crate) enum ModulePath {
    /// `#[path = "…"] mod name;`, the file the module's own source lives in.
    File { path: String, inline: Vec<String> },
    /// `#[path = "…"] mod name { … }`, which moves the directory the module's
    /// children are found in.
    Inline { path: String },
    /// A `#[path]` this scan cannot read, spelled for the refusal.
    Unreadable(String),
}

/// The module sources a source's `#[path]` attributes name, each with the
/// inline modules it sits inside. Read from the code, so a `#[path]` a comment
/// or a string literal names is not an input, and every enclosing inline module
/// is tracked because it moves the directory the attribute resolves against.
pub(crate) fn module_paths(source: &str) -> Vec<ModulePath> {
    let tokens = tokens(source);
    let mut paths = Vec::new();
    // One entry per open brace: the inline module it opens, when it opens one.
    let mut frames: Vec<Option<String>> = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index] {
            Token::Punct('{') => {
                frames.push(opened_module(&tokens, index));
                index += 1;
            }
            Token::Punct('}') => {
                frames.pop();
                index += 1;
            }
            Token::Punct('#') => match attribute(&tokens, index) {
                Some(Attribute::Path { value, next }) => {
                    paths.push(declaration(&tokens, next, value, &enclosing(&frames)));
                    index = next;
                }
                Some(Attribute::Other { next }) => index = next,
                None => index += 1,
            },
            _ => index += 1,
        }
    }
    paths
}

/// The inline modules a scan is inside, outermost first.
fn enclosing(frames: &[Option<String>]) -> Vec<String> {
    frames.iter().flatten().cloned().collect()
}

/// The inline module whose body the `{` at `index` opens, or `None` for a brace
/// that closes over a function body, a block or anything else.
fn opened_module(tokens: &[Token], index: usize) -> Option<String> {
    match tokens.get(..index) {
        Some([.., Token::Ident(keyword), Token::Ident(name)]) if keyword == "mod" => {
            Some(module_name(name).to_owned())
        }
        _ => None,
    }
}

/// The name a module declaration gives, without the `r#` of a raw identifier,
/// which is the name the compiler spells its directory with.
fn module_name(identifier: &str) -> &str {
    identifier.strip_prefix("r#").unwrap_or(identifier)
}

/// One `#[…]` attribute group.
enum Attribute {
    /// `#[path = …]`, with the value it names and the index after its `]`.
    Path {
        value: Result<String, String>,
        next: usize,
    },
    /// Any other attribute group, with the index after it.
    Other { next: usize },
}

/// The attribute group the `#` at `index` opens, or `None` when it opens none.
/// A `path` inside another attribute (`#[cfg_attr(unix, path = "…")]`) names an
/// input this scan cannot decide, so it is read as a value it cannot follow
/// rather than passed over.
fn attribute(tokens: &[Token], index: usize) -> Option<Attribute> {
    let mut cursor = index + 1;
    if matches!(tokens.get(cursor), Some(Token::Punct('!'))) {
        cursor += 1;
    }
    if !matches!(tokens.get(cursor), Some(Token::Punct('['))) {
        return None;
    }
    let end = matching_bracket(tokens, cursor)?;
    let content = &tokens[cursor + 1..end];
    let next = end + 1;
    Some(match content {
        [Token::Ident(name), Token::Punct('='), rest @ ..] if name == "path" => Attribute::Path {
            value: match rest {
                [Token::Literal(path)] => Ok(path.clone()),
                _ => Err(render(rest)),
            },
            next,
        },
        _ if names_a_path(content) => Attribute::Path {
            value: Err(render(content)),
            next,
        },
        _ => Attribute::Other { next },
    })
}

/// Whether an attribute's content assigns `path`, which only a conditional
/// attribute does without `path` being the attribute itself.
fn names_a_path(content: &[Token]) -> bool {
    content
        .windows(2)
        .any(|pair| matches!(pair, [Token::Ident(name), Token::Punct('=')] if name == "path"))
}

/// The index of the bracket closing the one at `index`.
fn matching_bracket(tokens: &[Token], index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, token) in tokens[index..].iter().enumerate() {
        match token {
            Token::Punct('[' | '(' | '{') => depth += 1,
            Token::Punct(']' | ')' | '}') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// The module a `#[path]` attribute declares: the file the module's source
/// lives in, an inline module, or the reason this scan cannot read it. The
/// attributes and the `pub` visibility between the attribute and the item are
/// skipped, and an item that is not a `mod` declaration is refused rather than
/// passed over, because a `#[path]` this scan walks past could name an input it
/// never records.
fn declaration(
    tokens: &[Token],
    mut index: usize,
    value: Result<String, String>,
    inline: &[String],
) -> ModulePath {
    while matches!(tokens.get(index), Some(Token::Punct('#'))) {
        match attribute(tokens, index) {
            Some(Attribute::Other { next }) => index = next,
            _ => break,
        }
    }
    if matches!(tokens.get(index..), Some([Token::Ident(word), ..]) if word == "pub") {
        index += 1;
        if matches!(tokens.get(index), Some(Token::Punct('('))) {
            index = matching_bracket(tokens, index).map_or(index, |end| end + 1);
        }
    }
    let path = match value {
        Ok(path) => path,
        Err(snippet) => return ModulePath::Unreadable(format!("#[path = {snippet}]")),
    };
    match module_declaration(tokens, index) {
        Some(false) => ModulePath::File {
            path,
            inline: inline.to_vec(),
        },
        Some(true) => ModulePath::Inline { path },
        None => ModulePath::Unreadable(format!(
            "#[path = {path:?}] on an item that is not a module declaration"
        )),
    }
}

/// Whether a module declaration starts at `index`, and whether it is an inline
/// module (`mod name {`) rather than one with a file of its own (`mod name;`).
fn module_declaration(tokens: &[Token], index: usize) -> Option<bool> {
    match tokens.get(index..) {
        Some(
            [
                Token::Ident(keyword),
                Token::Ident(_),
                Token::Punct(';'),
                ..,
            ],
        ) if keyword == "mod" => Some(false),
        Some(
            [
                Token::Ident(keyword),
                Token::Ident(_),
                Token::Punct('{'),
                ..,
            ],
        ) if keyword == "mod" => Some(true),
        _ => None,
    }
}

/// The files a `#[path]` module declaration compiles, from the directory the
/// compiler resolves it against. A declaration at the top level of a file
/// resolves against the file's own directory. One inside an inline module block
/// resolves against that directory with the inline modules appended, and with
/// the module's own name in front when the compiler found the file as `mod
/// name;` — which a file named by a `#[path]`, a crate root and a `mod.rs` do
/// not. A file alone does not say which it is, so both candidates are recorded:
/// one of them is the compiler's, and a record that is long is a rebuild.
pub(crate) fn module_inputs(source: &Path, path: &str, inline: &[String]) -> Vec<PathBuf> {
    let Some(directory) = source.parent() else {
        return Vec::new();
    };
    if inline.is_empty() {
        return vec![directory.join(path)];
    }
    let mut bases = vec![directory.to_path_buf()];
    if let Some(name) = added_module_directory(source) {
        bases.push(directory.join(name));
    }
    bases
        .into_iter()
        .map(|base| {
            inline
                .iter()
                .fold(base, |directory, name| directory.join(name))
                .join(path)
        })
        .collect()
}

/// The directory a file adds to the paths of its nested modules, or `None` when
/// it adds none: a crate root (`lib.rs`, `main.rs`) and a `mod.rs` are the
/// directory of their own module, while a file the compiler found as `mod
/// name;` keeps its modules under a directory of its own name.
fn added_module_directory(source: &Path) -> Option<&str> {
    let name = source.file_name()?.to_str()?;
    if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
        return None;
    }
    source.file_stem()?.to_str()
}

/// A path an include macro names, as far as the scan can follow it.
pub(crate) enum Include {
    /// A literal path, relative to the file that names it.
    Literal(String),
    /// A `concat!` of literal text and the package directory, in order.
    Parts(Vec<Part>),
    /// A path under the compiler's `OUT_DIR`, whose producer is recorded.
    Generated,
    /// A path built in a way this scan cannot follow, spelled for the refusal.
    Unknown(String),
}

/// One piece of a `concat!` an include macro names.
pub(crate) enum Part {
    Text(String),
    ManifestDir,
    OutDir,
    /// A path the scan cannot name, whose producer is unknown to it.
    Variable,
}

/// The include macros one source names, read from its code: comments are not
/// code and neither is a macro named inside a string literal, so neither can
/// add an input or stop a build.
pub(crate) fn include_arguments(source: &str) -> Vec<Include> {
    let tokens = tokens(source);
    let mut arguments = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let Token::Ident(name) = &tokens[index] else {
            index += 1;
            continue;
        };
        if !INCLUDE_MACROS.contains(&name.as_str())
            || !matches!(tokens.get(index + 1), Some(Token::Punct('!')))
            || !matches!(tokens.get(index + 2), Some(Token::Punct('(')))
        {
            index += 1;
            continue;
        }
        let mut depth = 1usize;
        let mut end = index + 3;
        while end < tokens.len() && depth > 0 {
            match tokens[end] {
                Token::Punct('(') => depth += 1,
                Token::Punct(')') => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                break;
            }
            end += 1;
        }
        let argument = tokens.get(index + 3..end).unwrap_or_default();
        arguments.push(match evaluate(argument) {
            Include::Unknown(snippet) => Include::Unknown(format!("{name}!({snippet})")),
            followed => followed,
        });
        index = end + 1;
    }
    arguments
}

/// What one include macro's argument names: a literal, a `concat!` this scan
/// can resolve, a path under `OUT_DIR`, or something it cannot follow.
fn evaluate(argument: &[Token]) -> Include {
    if let [Token::Literal(path)] = argument {
        return Include::Literal(path.clone());
    }
    let Some(parts) = concat_parts(argument) else {
        return Include::Unknown(render(argument));
    };
    if parts.iter().any(|part| matches!(part, Part::OutDir)) {
        return Include::Generated;
    }
    if parts.iter().any(|part| matches!(part, Part::Variable)) {
        return Include::Unknown(render(argument));
    }
    Include::Parts(parts)
}

/// The pieces of a `concat!(...)` argument, or `None` when the argument is not
/// a `concat!` of string literals and `env!` values.
fn concat_parts(argument: &[Token]) -> Option<Vec<Part>> {
    let [
        Token::Ident(name),
        Token::Punct('!'),
        Token::Punct('('),
        inner @ ..,
        Token::Punct(')'),
    ] = argument
    else {
        return None;
    };
    if name != "concat" {
        return None;
    }
    split_commas(inner)
        .into_iter()
        // An empty element is the remainder after a trailing or repeated comma.
        // A trailing comma is ordinary Rust, so it must not read as a path
        // this scan cannot follow.
        .filter(|element| !element.is_empty())
        .map(part)
        .collect()
}

/// One `concat!` element: a string literal, or `env!("...")`.
fn part(element: &[Token]) -> Option<Part> {
    match element {
        [Token::Literal(text)] => Some(Part::Text(text.clone())),
        [
            Token::Ident(name),
            Token::Punct('!'),
            Token::Punct('('),
            Token::Literal(variable),
            Token::Punct(')'),
        ] if name == "env" => Some(match variable.as_str() {
            "CARGO_MANIFEST_DIR" => Part::ManifestDir,
            "OUT_DIR" => Part::OutDir,
            _ => Part::Variable,
        }),
        _ => None,
    }
}

/// `tokens` split on the commas that are not inside brackets or parentheses.
fn split_commas(tokens: &[Token]) -> Vec<&[Token]> {
    let mut elements = Vec::new();
    let (mut start, mut depth) = (0, 0usize);
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Punct('(' | '[' | '{') => depth += 1,
            Token::Punct(')' | ']' | '}') => depth = depth.saturating_sub(1),
            Token::Punct(',') if depth == 0 => {
                elements.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    elements.push(&tokens[start..]);
    elements
}

/// A source spelled back as text, for a refusal a reader can act on.
fn render(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| match token {
            Token::Ident(name) => name.clone(),
            Token::Literal(value) => format!("{value:?}"),
            Token::Punct(character) => character.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the scan needs to tell code from text: an identifier, a string
/// literal's value, or one punctuation character. Whitespace is dropped, so an
/// argument written across lines reads the same as one written on a line;
/// comments are dropped, so a path a comment names is not an input; a literal is
/// kept as a value, so a macro named inside a string is not code; and a raw
/// identifier is one identifier, so `mod r#type` names the module `type`.
enum Token {
    Ident(String),
    Literal(String),
    Punct(char),
}

fn tokens(source: &str) -> Vec<Token> {
    let characters: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '/' && characters.get(index + 1) == Some(&'/') {
            while index < characters.len() && characters[index] != '\n' {
                index += 1;
            }
        } else if character == '/' && characters.get(index + 1) == Some(&'*') {
            index = after_block_comment(&characters, index);
        } else if let Some((literal, next)) = string_literal(&characters, index) {
            tokens.push(Token::Literal(literal));
            index = next;
        } else if let Some(next) = char_literal(&characters, index) {
            index = next;
        } else if character == 'r'
            && characters.get(index + 1) == Some(&'#')
            && characters
                .get(index + 2)
                .is_some_and(|next| next.is_alphanumeric() || *next == '_')
        {
            let start = index;
            index += 2;
            while index < characters.len()
                && (characters[index].is_alphanumeric() || characters[index] == '_')
            {
                index += 1;
            }
            tokens.push(Token::Ident(characters[start..index].iter().collect()));
        } else if character.is_alphanumeric() || character == '_' {
            let start = index;
            while index < characters.len()
                && (characters[index].is_alphanumeric() || characters[index] == '_')
            {
                index += 1;
            }
            tokens.push(Token::Ident(characters[start..index].iter().collect()));
        } else if character.is_whitespace() {
            index += 1;
        } else {
            tokens.push(Token::Punct(character));
            index += 1;
        }
    }
    tokens
}

/// The index after the block comment opening at `index`. Rust's block comments
/// nest, so the depth is counted rather than the first `*/` taken.
fn after_block_comment(characters: &[char], index: usize) -> usize {
    let mut index = index + 2;
    let mut depth = 1usize;
    while index < characters.len() && depth > 0 {
        if characters[index] == '/' && characters.get(index + 1) == Some(&'*') {
            depth += 1;
            index += 2;
        } else if characters[index] == '*' && characters.get(index + 1) == Some(&'/') {
            depth -= 1;
            index += 2;
        } else {
            index += 1;
        }
    }
    index
}

/// The string literal starting at `index` — plain, byte or raw — with the
/// index after it. Its escapes are resolved for the bytes a path can hold, so
/// the value the compiler would see is what the walk is given.
fn string_literal(characters: &[char], index: usize) -> Option<(String, usize)> {
    let mut cursor = index;
    if characters.get(cursor) == Some(&'b') {
        cursor += 1;
    }
    if characters.get(cursor) == Some(&'r') {
        let mut hashes = 0usize;
        cursor += 1;
        while characters.get(cursor) == Some(&'#') {
            hashes += 1;
            cursor += 1;
        }
        if characters.get(cursor) != Some(&'"') {
            return None;
        }
        cursor += 1;
        let start = cursor;
        while cursor < characters.len() {
            if characters[cursor] == '"'
                && characters
                    .get(cursor + 1..cursor + 1 + hashes)
                    .is_some_and(|closing| closing.iter().all(|character| *character == '#'))
            {
                return Some((
                    characters[start..cursor].iter().collect(),
                    cursor + 1 + hashes,
                ));
            }
            cursor += 1;
        }
        return None;
    }
    if characters.get(cursor) != Some(&'"') {
        return None;
    }
    cursor += 1;
    let mut value = String::new();
    while cursor < characters.len() {
        match characters[cursor] {
            '"' => return Some((value, cursor + 1)),
            '\\' => {
                let escaped = *characters.get(cursor + 1)?;
                cursor += 2;
                match escaped {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    '0' => value.push('\0'),
                    'u' => {
                        if characters.get(cursor) == Some(&'{') {
                            cursor += 1;
                            while cursor < characters.len() && characters[cursor] != '}' {
                                cursor += 1;
                            }
                            cursor += 1;
                        }
                    }
                    other => value.push(other),
                }
            }
            character => {
                value.push(character);
                cursor += 1;
            }
        }
    }
    None
}

/// The index after the character literal starting at `index`, or `None` for a
/// lifetime (which is an identifier's apostrophe, not a literal).
fn char_literal(characters: &[char], index: usize) -> Option<usize> {
    let mut cursor = index;
    if characters.get(cursor) == Some(&'b') {
        cursor += 1;
    }
    if characters.get(cursor) != Some(&'\'') {
        return None;
    }
    cursor += 1;
    if characters.get(cursor) == Some(&'\\') {
        cursor += 1;
        match characters.get(cursor)? {
            'x' => cursor += 3,
            'u' => {
                cursor += 1;
                if characters.get(cursor) == Some(&'{') {
                    while cursor < characters.len() && characters[cursor] != '}' {
                        cursor += 1;
                    }
                    cursor += 1;
                }
            }
            _ => cursor += 1,
        }
    } else {
        cursor += 1;
    }
    (characters.get(cursor) == Some(&'\'')).then_some(cursor + 1)
}

#[cfg(test)]
mod tests;
