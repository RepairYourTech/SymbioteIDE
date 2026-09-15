//! What one source pulls in: the include macros that name a path, the `#[path]`
//! module attributes that move one, and the plain module declarations the
//! compiler resolves by the module's own name — read from code rather than from
//! text, and followed to the file the compiler would compile.

use super::*;

use crate::tests::{fixture, record_over};
use crate::walk::paths::canonical;
use crate::walk::recorded_sources;
use crate::{Input, changed_sources};

#[test]
fn the_scan_reads_code_and_not_comments_or_strings() {
    for text in [
        "/// include_str!(\"doc.txt\")",
        "// include_str!(\"doc.txt\")",
        "/* include_str!(\"doc.txt\") */",
        "/* /* nested */ include_str!(\"doc.txt\") */",
        "let example = \"include_str!(\\\"doc.txt\\\")\";",
        "let example = r#\"include_str!(\"doc.txt\")\"#;",
        "let example = '\\'';",
        "/// #[path = \"doc.rs\"]",
        "// #[path = \"doc.rs\"]",
        "/* #[path = \"doc.rs\"] */",
        "let example = \"#[path = \\\"doc.rs\\\"] mod m;\";",
        "let example = r#\"#[path = \"doc.rs\"] mod m;\"#;",
    ] {
        assert_eq!(
            (
                include_arguments(text).len(),
                module_paths(text).len(),
                declared_modules(text).len()
            ),
            (0, 0, 0),
            "{text} names no input: recording one would refuse a current binary for a change \
                 to a file the build never reads"
        );
    }
}

#[test]
fn an_include_after_a_string_is_still_code() {
    let text = "let quote = \"a \\\" b\"; include_str!(\"real.json\");";
    assert!(matches!(
        include_arguments(text).as_slice(),
        [Include::Literal(path)] if path == "real.json"
    ));
}

#[test]
fn a_literal_and_a_built_path_are_told_apart() {
    let literal = include_arguments("include_str!(\n  \"../fixtures/a.json\"\n)");
    assert!(matches!(
        literal.as_slice(),
        [Include::Literal(path)] if path == "../fixtures/a.json"
    ));
    assert!(matches!(
        include_arguments("include_bytes!(\"logo.svg\").as_slice()").as_slice(),
        [Include::Literal(path)] if path == "logo.svg"
    ));

    let manifest_dir = include_arguments(
        "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/fixtures/a.json\"))",
    );
    assert!(matches!(
        manifest_dir.as_slice(),
        [Include::Parts(parts)]
            if matches!(parts.as_slice(), [Part::ManifestDir, Part::Text(text)] if text == "/fixtures/a.json")
    ));

    assert!(matches!(
        include_arguments("include_bytes!(concat!(env!(\"OUT_DIR\"), \"/generated.bin\"))")
            .as_slice(),
        [Include::Generated]
    ));
}

#[test]
fn a_trailing_comma_in_a_built_path_is_not_a_refusal() {
    for text in [
        "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/fixtures/a.json\",))",
        "include_str!(concat!(\n    env!(\"CARGO_MANIFEST_DIR\"),\n    \"/fixtures/a.json\",\n))",
    ] {
        assert!(
            matches!(include_arguments(text).as_slice(), [Include::Parts(_)]),
            "{text} is ordinary Rust naming a path the scan can follow, so it must not stop \
                 the build"
        );
    }
}

#[test]
fn a_path_the_scan_cannot_follow_is_reported_rather_than_skipped() {
    for text in [
        "include_str!(PATH_CONST)",
        "include_str!(concat!(env!(\"SOMEWHERE_ELSE\"), \"/a.json\"))",
        "include_bytes!(path_from_the_build())",
    ] {
        assert!(
            matches!(include_arguments(text).as_slice(), [Include::Unknown(_)]),
            "{text} names an input the record cannot name, which must stop the build rather \
                 than leave the record quietly short"
        );
    }
}

#[test]
fn a_recorded_path_is_the_one_the_scan_resolves() {
    let root = fixture("resolved");
    let packages = [root.join("crates/app")];
    let source = root.join("crates/app/src/lib.rs");
    std::fs::create_dir_all(source.parent().expect("a parent")).expect("the package");
    std::fs::write(&source, "// no includes here\n").expect("the source");

    let literal = followed_inputs(&source, Some(&packages[0])).expect("a followable path");
    assert!(literal.is_empty());

    std::fs::write(
        &source,
        "const F: &str = include_str!(\"../../../fixtures/a.json\");\n",
    )
    .expect("the source");
    assert_eq!(
        followed_inputs(&source, Some(&packages[0])).expect("a followable path"),
        [root.join("crates/app/src/../../../fixtures/a.json")]
    );

    std::fs::write(
            &source,
            "const F: &str = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/fixtures/a.json\"));\n",
        )
        .expect("the source");
    assert_eq!(
        followed_inputs(&source, Some(&packages[0])).expect("a followable path"),
        [root.join("crates/app/fixtures/a.json")]
    );

    std::fs::write(&source, "const F: &str = include_str!(UNKNOWN_PATH);\n").expect("the source");
    let problem = followed_inputs(&source, Some(&packages[0])).expect_err("a refusal");
    assert!(problem.contains("include_str"), "{problem}");
    assert!(problem.contains("UNKNOWN_PATH"), "{problem}");
}

#[test]
fn a_module_attribute_at_the_top_of_a_file_is_recorded_and_named() {
    let root = fixture("module");
    std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\n",
    )
    .expect("the root manifest");
    std::fs::write(
        root.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .expect("the package manifest");
    std::fs::write(
        root.join("crates/app/src/lib.rs"),
        "#[path = \"../../../fixtures/module.rs\"]\nmod module;\n",
    )
    .expect("the source that declares a module outside its package");
    let compiled = root.join("fixtures/module.rs");
    std::fs::create_dir_all(compiled.parent().expect("a parent")).expect("the fixtures");
    std::fs::write(&compiled, "pub const ONE: u8 = 1;\n").expect("the module");

    let (sources, unfollowed, _) = recorded_sources(&root.join("crates/app"), &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&canonical(&compiled)),
        "a module declared outside the package must be recorded; the walk found {sources:?}"
    );

    let binary = record_over(&root, &sources);
    std::fs::write(&compiled, "pub const ONE: u8 = 2;\n").expect("the changed module");
    assert_eq!(
        changed_sources(&binary, &root).expect("a readable record"),
        [Input::File(compiled)]
    );
}

#[test]
fn a_module_attribute_inside_an_inline_module_records_both_directories() {
    let root = fixture("nested-module");
    let package = root.join("crates/app");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::write(package.join("Cargo.toml"), "[package]\nname = \"app\"\n")
        .expect("the package manifest");
    std::fs::write(
        package.join("src/leaf.rs"),
        "mod inline {\n    #[path = \"deep.rs\"]\n    mod deep;\n}\n",
    )
    .expect("the source that nests a module attribute");
    // The compiler resolves this against `src/leaf/` when it found the file
    // as `mod leaf;`, and against `src/` when a `#[path]` named it. The scan
    // cannot tell the two apart from the file alone, so a record that is
    // never short names both.
    let candidates = ["src/leaf/inline/deep.rs", "src/inline/deep.rs"];
    for path in candidates {
        let file = package.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("the directories");
        std::fs::write(&file, "pub const DEEP: u8 = 1;\n").expect("the module");
    }

    let (sources, unfollowed, _) = recorded_sources(&package, &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for path in candidates {
        assert!(
            sources.contains(&package.join(path)),
            "the record must not be short of {path}; the walk found {sources:?}"
        );
    }
}

#[test]
fn a_module_declaration_names_a_file_the_walk_skips() {
    let root = fixture("declared-module");
    let package = root.join("crates/app");
    std::fs::create_dir_all(package.join("src/target")).expect("the package");
    std::fs::create_dir_all(package.join("tests")).expect("the tests directory");
    std::fs::write(package.join("Cargo.toml"), "[package]\nname = \"app\"\n")
        .expect("the package manifest");
    // The compiler finds `src/target/mod.rs` by `mod target;` and a package-root
    // `tests/mod.rs` by `mod tests;`, so neither file is outside the build
    // however the wire file names the directory it sits in — measured, the first
    // of them was left out of a record whose walk skipped `src/target`, and both
    // readers then reported the binary current after that module changed.
    std::fs::write(package.join("src/main.rs"), "mod target;\n\nfn main() {}\n")
        .expect("the crate root");
    std::fs::write(
        package.join("src/target/mod.rs"),
        "pub const TARGET: u8 = 1;\n",
    )
    .expect("the compiled module");
    std::fs::write(package.join("build.rs"), "mod tests;\n\nfn main() {}\n")
        .expect("the build script");
    std::fs::write(package.join("tests/mod.rs"), "pub const TESTS: u8 = 1;\n")
        .expect("the module the build script declares");

    let (sources, unfollowed, _) = recorded_sources(&package, &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for path in ["src/target/mod.rs", "tests/mod.rs"] {
        assert!(
            sources.contains(&package.join(path)),
            "a file the compiler finds by a module's own name must be recorded; the walk found \
             {sources:?}"
        );
    }
}

#[test]
fn a_plain_module_resolves_where_the_compiler_looks_for_it() {
    let root = PathBuf::from("example");
    let declared = |name: &str, inline: &[&str]| Declared {
        name: name.to_owned(),
        inline: inline.iter().map(|name| (*name).to_owned()).collect(),
    };
    assert_eq!(
        declared_inputs(&root.join("src/lib.rs"), &declared("target", &[])),
        [root.join("src/target.rs"), root.join("src/target/mod.rs")]
    );
    // The file itself does not say whether the compiler found it as `mod leaf;`,
    // which puts its modules under `src/leaf/`, or as a crate root or through a
    // `#[path]`, which put them under `src/` — so a record that is never short
    // names both.
    assert_eq!(
        declared_inputs(&root.join("src/leaf.rs"), &declared("target", &[])),
        [
            root.join("src/target.rs"),
            root.join("src/target/mod.rs"),
            root.join("src/leaf/target.rs"),
            root.join("src/leaf/target/mod.rs"),
        ]
    );
    assert_eq!(
        declared_inputs(&root.join("src/leaf/mod.rs"), &declared("deep", &["inner"])),
        [
            root.join("src/leaf/inner/deep.rs"),
            root.join("src/leaf/inner/deep/mod.rs"),
        ]
    );
}

#[test]
fn a_declaration_names_a_module_unless_an_attribute_names_its_file() {
    let names = |text: &str| {
        declared_modules(text)
            .into_iter()
            .map(|declared| (declared.name, declared.inline))
            .collect::<Vec<_>>()
    };
    assert_eq!(names("mod plain;\n"), [("plain".to_owned(), Vec::new())]);
    assert_eq!(
        names("pub mod r#type;\n"),
        [("type".to_owned(), Vec::new())]
    );
    assert_eq!(
        names("mod inline {\n    mod deep;\n}\n"),
        [("deep".to_owned(), ["inline".to_owned()].to_vec())]
    );
    // An inline module declares its body where it stands and compiles no file
    // the compiler finds by its name.
    assert_eq!(names("mod inline {\n}\n"), []);
    // A `#[path]` names the file, which `module_paths` carries instead.
    assert_eq!(names("#[path = \"../shared.rs\"]\nmod shared;\n"), []);
    assert_eq!(
        names("#[cfg(test)]\n#[path = \"../shared.rs\"]\nmod shared;\n"),
        []
    );
    // A condition on a declaration is not a path: the module still compiles from
    // a file of its own name wherever the condition holds.
    assert_eq!(
        names("#[cfg(test)]\nmod cfg_only;\n"),
        [("cfg_only".to_owned(), Vec::new())]
    );
}

#[test]
fn a_module_path_resolves_where_the_compiler_looks_for_it() {
    let root = PathBuf::from("example");
    let nested = ["inner".to_owned()];
    assert_eq!(
        module_inputs(&root.join("src/lib.rs"), "a.rs", &[]),
        [root.join("src/a.rs")]
    );
    assert_eq!(
        module_inputs(&root.join("src/lib.rs"), "a.rs", &nested),
        [root.join("src/inner/a.rs")]
    );
    assert_eq!(
        module_inputs(&root.join("src/leaf/mod.rs"), "a.rs", &nested),
        [root.join("src/leaf/inner/a.rs")]
    );
    assert_eq!(
        module_inputs(&root.join("src/leaf.rs"), "a.rs", &nested),
        [
            root.join("src/inner/a.rs"),
            root.join("src/leaf/inner/a.rs")
        ]
    );
}

#[test]
fn a_module_attribute_is_read_from_the_item_it_declares() {
    assert!(matches!(
        module_paths("#[path = \"../shared.rs\"]\npub mod shared;\n").as_slice(),
        [ModulePath::File { path, inline }] if path == "../shared.rs" && inline.is_empty()
    ));
    assert!(matches!(
        module_paths("#[cfg(test)]\n#[path = \"../shared.rs\"]\nmod shared;\n").as_slice(),
        [ModulePath::File { path, .. }] if path == "../shared.rs"
    ));
    assert!(matches!(
        module_paths("mod inline {\n    #[path = \"../shared.rs\"]\n    mod shared;\n}\n").as_slice(),
        [ModulePath::File { inline, .. }] if inline.as_slice() == ["inline"]
    ));
    assert!(matches!(
        module_paths("pub mod r#type {\n    #[path = \"deep.rs\"]\n    mod deep;\n}\n")
            .as_slice(),
        [ModulePath::File { inline, .. }] if inline.as_slice() == ["type"]
    ));
}

#[test]
fn a_module_attribute_the_scan_cannot_read_is_reported_rather_than_skipped() {
    for text in [
        "#[path = concat!(\"src\", \"/a.rs\")]\nmod a;\n",
        "#[cfg_attr(unix, path = \"../a.rs\")]\nmod a;\n",
        "#[path = \"sub\"]\nmod inline {\n}\n",
        "mod outer {\n    #[path = \"sub\"]\n    mod inner {\n    }\n}\n",
        "#[path = \"../a.rs\"]\nfn a() {}\n",
    ] {
        let paths = module_paths(text);
        assert!(
            matches!(
                paths.as_slice(),
                [ModulePath::Unreadable(_)] | [ModulePath::Inline { .. }]
            ),
            "{text} names a module source the record cannot follow, which must stop the build \
                 rather than leave the record quietly short; the scan read {paths:?}"
        );
    }
}
