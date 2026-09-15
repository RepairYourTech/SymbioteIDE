//! The wire a record is spelled on, and the marks a build writes where it could
//! not stand behind one.
//!
//! The wire is the bytes a record is framed, decoded, divided and spelled with —
//! how it begins and ends inside a binary and what those bytes are encoded as,
//! which locators a record can hold at all, where its lines end, the one
//! environment variable it pins, how an environment locator is spelled, the
//! content an input that is not there is given, what a record line is, and every
//! mark a build writes where it could not establish a fact the record rests on.
//! This crate
//! writes a record and `planning/integrity/source_record.py` refuses one, so the
//! two must agree about all of it to the byte; rather than each keeping a copy,
//! they parse one file ([`wire.txt`][WIRE_FILE] beside this module), and neither
//! can drift from the other without editing the file the other reads. What each
//! value is, and the rule it makes, is stated there and nowhere twice.
//!
//! The file is embedded with `include_str!`, so a build reads the wire compiled
//! into the crate rather than whatever is on disk when it runs, and a malformed
//! file stops the build it is compiled into rather than a proof later.
//!
//! A mark is written as a line of its own — [`unset`](Wire::unset) as its
//! content, the locator it names as its locator — and a record may carry more
//! than one, so [`Wire::marks`] is ordered by what a rebuild has to clear first.
//! [`mark`](Wire::mark) is how this crate's own code asks for one by role, and
//! [`mark_line`](Wire::mark_line) writes one.

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// `wire.txt`: the one copy of the wire, embedded in the crate at compile time.
const WIRE_FILE: &str = include_str!("wire.txt");

/// One mark: what the build could not establish (its `role`, which is how this
/// crate's own code asks for it), the locator a record spells it with, and the
/// remedy a refusal prints.
#[derive(Debug)]
pub(crate) struct Mark {
    pub(crate) role: &'static str,
    pub(crate) locator: &'static str,
    pub(crate) remedy: &'static str,
}

/// The wire, parsed once.
pub(crate) struct Wire {
    /// How a record begins inside a binary.
    pub(crate) start: &'static str,
    /// How it ends.
    pub(crate) end: &'static str,
    /// The one environment variable a record pins: the cargo that ran the build,
    /// which names the toolchain a `rust-toolchain.toml` selects. It is set for a
    /// build script and for a test process alike — and to the same value whether
    /// a build is started through the rustup shim or through that toolchain's own
    /// binary — so the record can be checked against the run rather than only
    /// described.
    ///
    /// Two neighbours are deliberately not it:
    ///
    /// * `RUSTFLAGS`, because the two sides never see the same value. Measured,
    ///   cargo passes it to a test process untouched (`TEST RUSTFLAGS=Some("--cfg
    ///   from_shell")`) while a build script gets it stripped (`BUILD
    ///   RUSTFLAGS=None`) and sees only the effective flags in
    ///   `CARGO_ENCODED_RUSTFLAGS` (`Some("--cfg\u{1f}from_shell")`). Recording
    ///   what the build saw would refuse a current binary whenever a flag is set;
    ///   recording what the run sees would refuse one built from a
    ///   `.cargo/config`. The flags a build is given are covered through the
    ///   configuration files that carry them.
    /// * `RUSTUP_TOOLCHAIN`, because it reports how cargo was invoked rather than
    ///   which compiler ran: a build through the rustup shim sets it and an
    ///   invocation of that toolchain's own cargo does not, which would make the
    ///   two refuse each other alternately. The toolchain is named by this
    ///   variable's own path, which does not move.
    pub(crate) environment: &'static str,
    /// How that variable's locator is spelled: `env:NAME`.
    pub(crate) prefix: &'static str,
    /// The encoding the bytes between the markers are spelled in, which the file
    /// states and both readers decode them by. This crate decodes with
    /// [`str::from_utf8`], so a file naming another is one this crate cannot
    /// implement, and it is refused where it is read rather than silently decoded
    /// as UTF-8 anyway.
    pub(crate) encoding: &'static str,
    /// The content a record gives an input that is not there: a variable that is
    /// not set, or a recorded file that has gone.
    pub(crate) unset: &'static str,
    /// The length and the alphabet of a hash — the content a record gives an
    /// input that is there — which is what tells a line that holds one from a
    /// line that says the record cannot be stood behind.
    pub(crate) hash_length: usize,
    pub(crate) hash_alphabet: &'static str,
    /// The remedy a refusal prints for a mark the file does not name, since
    /// neither reader keeps its own.
    pub(crate) unknown_remedy: &'static str,
    /// The remedy a refusal prints for a line the file cannot read.
    pub(crate) malformed_remedy: &'static str,
    /// The whole text a refusal prints for a record whose bytes are not
    /// [`encoding`](Wire::encoding) — a fact about the record rather than a binary
    /// that carries none — with the encoding and the byte the bytes stop being it
    /// at spelled `{encoding}` and `{byte}` for [`undecodable`](Wire::undecodable)
    /// to fill in.
    pub(crate) undecodable_remedy: &'static str,
    /// The remedy a refusal prints for a record whose own bytes spell one of the
    /// markers it is framed with, so where it ends cannot be read from its bytes.
    pub(crate) unframed_remedy: &'static str,
    /// Every mark, in the order a rebuild has to clear them.
    pub(crate) marks: Vec<Mark>,
}

/// What one line of a record is, by the rule [`wire.txt`][WIRE_FILE] states —
/// the one rule both readers decide a line by.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Line<'a> {
    /// A mark the file names: the locator a `mark` line spells, whatever its
    /// content, so the fact the build could not establish is known. The locator
    /// is the file's own, which lives as long as the program.
    Named(&'static str),
    /// A mark the file does not name: an `unset` content under a locator that is
    /// neither one the file names nor an environment locator. What the build
    /// could not establish is unknown, so the file's remedy for one is printed.
    Unknown(&'a str),
    /// An input the record holds: the locator it is spelled with, and the content
    /// the record gives it.
    Input(&'a str, &'a str),
    /// No line the file reads, so neither reader reads the record at all.
    Malformed,
}

/// The lines a record is spelled with, by the division [`wire.txt`][WIRE_FILE]
/// states: at each LF, the record's last line ending with one, so that final LF
/// is where the record stops rather than a line of its own. A blank line
/// anywhere else is a line like any other, which [`Wire::line`] reads as
/// malformed rather than passing over: a record that lost a line is refused, not
/// read short.
pub(crate) fn record_lines(record: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = record.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    lines
}

/// The record the wire frames inside a binary, decoded by the encoding the file
/// states — `None` for a binary carrying no record, or an error whose text is the
/// file's remedy for bytes it cannot read.
///
/// Both readers take a record out of a binary here and nowhere else: the framing
/// is the file's ([`start`](Wire::start) and [`end`](Wire::end)) and so is the
/// encoding, so neither decides for itself where a record begins, where it ends,
/// or what its bytes are. A record whose bytes are not that encoding is refused
/// here rather than raised over — measured, one reader raised on such bytes while
/// the other reported the binary as carrying no record, so the same artifact was
/// described two ways. And the `end` that ends a record is the one that follows
/// the LF its last line ends with, so a marker the record's own bytes spell does
/// not end it: such a record is refused in the file's remedy for one rather than
/// read up to that marker, which would silently drop every line after it.
pub(crate) fn embedded_record(binary: &[u8]) -> Result<Option<&str>, String> {
    let wire = wire();
    let Some(start) = find(binary, wire.start.as_bytes()) else {
        return Ok(None);
    };
    let start = start + wire.start.len();
    let mut ends_a_line = vec![b'\n'];
    ends_a_line.extend_from_slice(wire.end.as_bytes());
    let Some(end) = find(&binary[start..], &ends_a_line) else {
        return Ok(None);
    };
    let record = &binary[start..start + end + 1];
    for marker in [wire.start, wire.end] {
        if find(record, marker.as_bytes()).is_some() {
            return Err(wire.unframed_remedy.to_string());
        }
    }
    std::str::from_utf8(record)
        .map(Some)
        .map_err(|error| wire.undecodable(error.valid_up_to()))
}

/// The first place `needle` occurs in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

impl Wire {
    /// The mark with that role — this crate's own name for one. Panics naming the
    /// role where the file holds none, which is the crate asking for a mark it no
    /// longer writes rather than a record a build got wrong.
    pub(crate) fn mark(&self, role: &str) -> &Mark {
        self.marks
            .iter()
            .find(|mark| mark.role == role)
            .unwrap_or_else(|| panic!("the wire file names no mark with the role {role:?}"))
    }

    /// The line a record writes for that mark: [`unset`](Wire::unset) as the
    /// content and the mark's locator as the locator. `build_stamp` writes one
    /// for each fact its readings could not establish, and the tests build the
    /// records a marked build produces with them.
    pub(crate) fn mark_line(&self, role: &str) -> String {
        let mark = self.mark(role);
        format!("{}\t{}\n", self.unset, mark.locator)
    }

    /// What one record line is, by the rule the file states; its own text carries
    /// the order, which is part of the rule. This is the whole of what a record
    /// line can be, so a shape neither reader can place is read by neither of
    /// them rather than guessed at by one.
    pub(crate) fn line<'a>(&self, line: &'a str) -> Line<'a> {
        if line.contains('\r') {
            return Line::Malformed;
        }
        let Some((content, locator)) = line.split_once('\t') else {
            return Line::Malformed;
        };
        if locator.is_empty() {
            return Line::Malformed;
        }
        if let Some(mark) = self.marks.iter().find(|mark| mark.locator == locator) {
            return Line::Named(mark.locator);
        }
        if locator.starts_with(self.prefix) {
            return Line::Input(locator, content);
        }
        if content == self.unset {
            return Line::Unknown(locator);
        }
        if self.is_hash(content) {
            return Line::Input(locator, content);
        }
        Line::Malformed
    }

    /// What a locator holds that no record can, where it holds one — `None` for a
    /// locator a record reads back as it was written, and this crate's own words
    /// for the first thing in one it cannot.
    ///
    /// A record line is a content, a TAB and a locator, divided at each LF and its
    /// last line ending with one: so a locator holding an LF would end the line it
    /// is spelled in, and one holding a CR is malformed to both readers. And the
    /// framed bytes hold neither marker, so a locator spelling one would leave the
    /// record it is written into unframed. A build refuses to stamp all three
    /// rather than write a record neither reader can read back — measured, a
    /// path holding a line feed and one legally named after the end marker each
    /// built successfully and left an artifact both readers refused, in a remedy
    /// asking for a rebuild that reproduces the same bytes.
    ///
    /// The separator itself is not one of them: the locator a record spells is
    /// what follows the first TAB, so a TAB inside a locator is read back by both
    /// readers as it was written. Every one of these is read from the file's own
    /// rule rather than from a second list kept here.
    pub(crate) fn unholdable(&self, locator: &str) -> Option<String> {
        if locator.contains('\n') {
            return Some("a line feed, which ends the line it would be spelled in".to_owned());
        }
        if locator.contains('\r') {
            return Some("a carriage return, which makes that line unreadable".to_owned());
        }
        [self.start, self.end]
            .into_iter()
            .find(|marker| locator.contains(marker))
            .map(|marker| format!("the marker {marker:?}, which frames every record"))
    }

    /// The file's own text for a record whose bytes are not
    /// [`encoding`](Wire::encoding), with the encoding and the byte the bytes stop
    /// being it at filled in. This is the one place either reader spells that
    /// text, so rewording the refusal — including where its offset stands — is an
    /// edit to the file rather than one in each of two readers.
    pub(crate) fn undecodable(&self, offset: usize) -> String {
        self.undecodable_remedy
            .replace("{encoding}", self.encoding)
            .replace("{byte}", &offset.to_string())
    }

    /// Whether a content is the hash a record gives an input: the length and the
    /// alphabet the file spells, so neither reader decides that for itself.
    fn is_hash(&self, content: &str) -> bool {
        content.len() == self.hash_length
            && content
                .bytes()
                .all(|byte| self.hash_alphabet.as_bytes().contains(&byte))
    }
}

/// The wire, read once.
pub(crate) fn wire() -> &'static Wire {
    static WIRE: OnceLock<Wire> = OnceLock::new();
    WIRE.get_or_init(read)
}

/// The wire the embedded file spells. Every failure here is a malformed file in
/// the crate rather than anything a build did, so each names what is wrong with
/// it and stops the build.
fn read() -> Wire {
    const SCALARS: [&str; 10] = [
        "start",
        "end",
        "encoding",
        "environment",
        "prefix",
        "unset",
        "unknown",
        "malformed",
        "undecodable",
        "unframed",
    ];
    let mut scalars = BTreeMap::new();
    let mut marks: Vec<Mark> = Vec::new();
    let mut hash: Option<(usize, &'static str)> = None;
    for line in WIRE_FILE.lines().map(str::trim_end) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (keyword, fields) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("the wire file line {line:?} is not a keyword and fields"));
        match keyword {
            "mark" => {
                let (role, rest) = fields.split_once('\t').unwrap_or_else(|| {
                    panic!("the wire file mark {fields:?} names no role, locator and remedy")
                });
                let (locator, remedy) = rest.split_once('\t').unwrap_or_else(|| {
                    panic!("the wire file mark {fields:?} names no locator and remedy")
                });
                assert!(
                    !role.is_empty() && !locator.is_empty() && !remedy.is_empty(),
                    "the wire file mark {fields:?} has an empty field"
                );
                assert!(
                    !marks.iter().any(|mark| mark.role == role),
                    "the wire file spells the role {role:?} twice"
                );
                marks.push(Mark {
                    role,
                    locator,
                    remedy,
                });
            }
            // The shape of a hash: data, so what tells an input's line from a
            // mark's is the file's to say rather than each reader's to decide.
            "hash" => {
                let (length, alphabet) = fields.split_once('\t').unwrap_or_else(|| {
                    panic!("the wire file hash {fields:?} names no length and alphabet")
                });
                let length: usize = length.parse().unwrap_or_else(|_| {
                    panic!("the wire file names the hash length {length:?}, which is not a number")
                });
                assert!(
                    length > 0 && !alphabet.is_empty(),
                    "the wire file hash {fields:?} has an empty field"
                );
                assert!(hash.is_none(), "the wire file spells the hash shape twice");
                hash = Some((length, alphabet));
            }
            // Which of several marks the file does not name a refusal reports:
            // data, so a reader cannot invent its own order.
            "unknown-order" => assert_eq!(
                fields, "first-in-record",
                "the wire file names the unknown-mark order {fields:?}, which this crate does not \
                 implement"
            ),
            // The encoding the record's bytes are in: data, so both readers decode
            // by the file's own word rather than each assuming one. This crate
            // decodes with `from_utf8`, so a file naming anything else is refused
            // here rather than read as UTF-8 anyway.
            "encoding" => {
                assert_eq!(
                    fields, "UTF-8",
                    "the wire file names the record encoding {fields:?}, which this crate does not \
                     implement"
                );
                assert!(
                    scalars.insert("encoding", fields).is_none(),
                    "the wire file spells \"encoding\" twice"
                );
            }
            name if SCALARS.contains(&name) => {
                assert!(
                    scalars.insert(name, fields).is_none(),
                    "the wire file spells {name:?} twice"
                );
            }
            other => panic!("the wire file has no keyword {other:?}"),
        }
    }
    assert!(!marks.is_empty(), "the wire file holds no mark");
    let (hash_length, hash_alphabet) = hash.expect("the wire file names no hash shape");
    let scalar = |name: &str| {
        *scalars
            .get(name)
            .unwrap_or_else(|| panic!("the wire file names no {name:?}"))
    };
    Wire {
        start: scalar("start"),
        end: scalar("end"),
        encoding: scalar("encoding"),
        environment: scalar("environment"),
        prefix: scalar("prefix"),
        unset: scalar("unset"),
        hash_length,
        hash_alphabet,
        unknown_remedy: scalar("unknown"),
        malformed_remedy: scalar("malformed"),
        undecodable_remedy: scalar("undecodable"),
        unframed_remedy: scalar("unframed"),
        marks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An input line's reading, for a test table: the locator and the content the
    /// record spells for it.
    fn input<'a>(locator: &'a str, content: &'a str) -> Line<'a> {
        Line::Input(locator, content)
    }

    #[test]
    fn the_embedded_file_spells_the_whole_wire() {
        let wire = wire();
        assert_eq!(wire.start, "symbiote-source-record:[");
        assert_eq!(wire.end, "]symbiote-source-record");
        assert_eq!(wire.environment, "CARGO");
        assert_eq!(wire.prefix, "env:");
        assert_eq!(wire.unset, "unset");
        assert_eq!(
            wire.encoding, "UTF-8",
            "the file states what a record's bytes are, so both readers decode them the same \
             way"
        );
        assert_eq!(
            (wire.hash_length, wire.hash_alphabet),
            (64, "0123456789abcdef"),
            "a record gives an input a sha256, and what tells one from a mark is the file's \
             to say"
        );
        assert!(
            wire.unknown_remedy.contains("the wire does not name")
                && wire
                    .unknown_remedy
                    .contains("Rebuild it from the workspace"),
            "a mark the file does not name still gets the file's remedy: {}",
            wire.unknown_remedy
        );
        assert!(
            wire.malformed_remedy
                .contains("a line the wire cannot read")
                && wire
                    .malformed_remedy
                    .contains("Rebuild the binary from the workspace"),
            "and a line the file cannot read gets its own: {}",
            wire.malformed_remedy
        );
        let undecodable = wire.undecodable(7);
        assert!(
            undecodable.contains(&format!("are not the {} the wire spells", wire.encoding))
                && undecodable.contains("at byte 7 of the record")
                && undecodable.contains("Rebuild the binary from the workspace"),
            "and so does a record whose bytes are not the encoding it states, with the byte they \
             stop being it at filled in: {undecodable}"
        );
        assert!(
            wire.undecodable_remedy.contains("{byte}")
                && wire.undecodable_remedy.contains("{encoding}"),
            "the file states where the offset and the encoding are spelled, so neither reader \
             holds that sentence either: {}",
            wire.undecodable_remedy
        );
        assert!(
            wire.marks.len() >= 2,
            "every mark the crate writes is here: {:?}",
            wire.marks.iter().map(|mark| mark.role).collect::<Vec<_>>()
        );
        // The order is what a refusal reports first, so the cargo — which can
        // take the resolution with it — is read before the resolution.
        assert_eq!(wire.marks[0].role, "unasked-cargo");
        assert_eq!(
            wire.mark("unasked-cargo").locator,
            "cargo-not-asked",
            "the crate writes this locator, so the file spells it"
        );
        assert_eq!(
            wire.mark("unnamed-resolution").locator,
            "resolution-not-named"
        );
        for mark in &wire.marks {
            assert!(
                !mark.remedy.is_empty() && !mark.locator.is_empty(),
                "a mark a refusal cannot print is not a mark: {mark:?}"
            );
        }
    }

    #[test]
    fn a_record_is_divided_into_lines_the_way_the_file_states() {
        let hash = "0".repeat(64);
        let line = format!("{hash}\tsrc/lib.rs");
        assert_eq!(
            record_lines(&format!("{line}\n")),
            [line.as_str()],
            "the LF the record's last line ends with is where the record stops, not a line"
        );
        assert_eq!(
            record_lines(&format!("{line}\n{line}")),
            [line.as_str(), line.as_str()],
            "and a record whose last line does not end with one still has that last line"
        );
        assert_eq!(record_lines(""), Vec::<&str>::new());
        assert_eq!(
            record_lines(&format!("{line}\n\n")),
            [line.as_str(), ""],
            "a blank line in the middle is a line of its own, which the rule reads as malformed"
        );
        assert_eq!(
            record_lines(&format!("{line}\r\n")),
            [format!("{line}\r").as_str()],
            "the division trims nothing else, so a CR stays in the line — which the rule then \
             reads as malformed rather than as a locator's last character"
        );
    }

    #[test]
    fn the_record_a_binary_carries_is_taken_out_and_decoded_by_the_wire() {
        let wire = wire();
        let hash = "0".repeat(wire.hash_length);
        let record = format!("{hash}\tsrc/lib.rs\n");
        let binary = format!("noise{}{}{}noise", wire.start, record, wire.end);
        assert_eq!(
            embedded_record(binary.as_bytes()).expect("a record the wire framed"),
            Some(record.as_str()),
            "the framing is the file's, wherever it falls in the bytes"
        );
        assert_eq!(
            embedded_record(b"noise without a record").expect("no record is not an error"),
            None
        );
        assert_eq!(
            embedded_record(format!("{}{record}", wire.start).as_bytes())
                .expect("an unterminated record is one that is not there"),
            None,
            "without the end marker there is no framed record to decode"
        );
        // The end marker ends a record only where it follows one of its lines —
        // the bytes the wire frames always hold at least the environment line a
        // build writes — so bytes no `end` follows a line of hold no record at
        // all, and reach a check as a binary carrying none rather than as an empty
        // record a reader would call clean.
        assert_eq!(
            embedded_record(format!("{}{hash}\tsrc/lib.rs{}", wire.start, wire.end).as_bytes())
                .expect("an end marker that follows no line ends nothing"),
            None,
            "the last line of a record ends with the LF its `end` follows"
        );
        assert_eq!(
            embedded_record(format!("{}{}", wire.start, wire.end).as_bytes())
                .expect("markers with no line between them frame no record"),
            None
        );
        // The encoding is the file's, and a record whose bytes are not it is
        // refused in the file's own remedy rather than raised over or reported as
        // a binary carrying no record — which is what the two readers each did.
        let mut unreadable = wire.start.as_bytes().to_vec();
        unreadable.extend_from_slice(b"0\t");
        unreadable.extend_from_slice(&[0xff, 0xfe]);
        unreadable.extend_from_slice(b"x\n");
        unreadable.extend_from_slice(wire.end.as_bytes());
        let problem = embedded_record(&unreadable).expect_err("bytes that are not the encoding");
        assert_eq!(
            problem,
            wire.undecodable(2),
            "the refusal is the file's own text for one, with the byte it stops at filled in"
        );
    }

    #[test]
    fn a_record_whose_own_bytes_spell_a_marker_is_refused_rather_than_cut_short() {
        // Measured: a record naming a file whose name ends with the end marker —
        // legal, and what a build then writes — was framed at that inner marker by
        // both readers, which read a record missing every line after it and
        // reported a binary as matching a tree it was not built from.
        let wire = wire();
        let hash = "0".repeat(wire.hash_length);
        let spelled = format!("src/a.rs{}", wire.end);
        let body = format!("{hash}\t{spelled}\n{hash}\tsrc/b.rs\n");
        let binary = format!("noise{}{body}{}noise", wire.start, wire.end);
        let problem = embedded_record(binary.as_bytes())
            .expect_err("a record spelling a marker cannot be read whole");
        assert_eq!(
            problem, wire.unframed_remedy,
            "the refusal is the file's own text for a record it cannot frame"
        );
        // The line after the marker it spells is part of the bytes the wire framed
        // — the record is refused, not read up to the marker and short.
        assert!(
            body.contains(&format!("{hash}\tsrc/b.rs")),
            "the record a reader that took the first marker it met would drop: {body:?}"
        );
    }

    #[test]
    fn a_locator_no_record_can_hold_is_named_by_the_wires_own_rule() {
        let wire = wire();
        assert_eq!(
            wire.unholdable("src/lib.rs"),
            None,
            "an ordinary locator is one a record reads back as it was written"
        );
        // The separator itself is not what a record cannot hold: the locator is
        // what follows the first TAB, so a TAB in one survives the round trip.
        assert_eq!(
            wire.unholdable("src/a\tb.rs"),
            None,
            "a TAB in a locator is read back by both readers as it was written"
        );
        // The byte a record's lines are divided at: a locator holding one would end
        // the line it is spelled in, and the rest of it would be read as a line of
        // its own. Measured, such a path built successfully and stamped a record
        // both readers then refused.
        let holds = wire
            .unholdable("src/a\nb.rs")
            .expect("a line feed ends the line it is spelled in");
        assert!(holds.contains("a line feed"), "{holds}");
        // And the byte the rule reads as malformed.
        let holds = wire
            .unholdable("src/c\rd.rs")
            .expect("a carriage return makes a line unreadable");
        assert!(holds.contains("a carriage return"), "{holds}");
        // And the markers the record is framed with.
        let holds = wire
            .unholdable(&format!("src/lib.rs{}", wire.end))
            .expect("a marker would leave the record unframed");
        assert!(
            holds.contains(wire.end),
            "the refusal names the marker it holds: {holds}"
        );
        let holds = wire
            .unholdable(&format!("{}/src/lib.rs", wire.start))
            .expect("so would the other marker");
        assert!(holds.contains(wire.start), "{holds}");
    }

    #[test]
    fn a_line_is_read_by_the_rule_the_file_states() {
        // The one rule, shape by shape; the shapes whose comment says
        // "measured" are the ones the two readers read differently while each
        // decided a line's kind for itself.
        let wire = wire();
        let mark = wire.mark("unasked-cargo").locator;
        let hash = "0".repeat(64);
        assert_eq!(
            wire.line(&format!("{hash}\tsrc/lib.rs")),
            input("src/lib.rs", &hash)
        );
        assert_eq!(
            wire.line(&format!("{hash}\tenv:CARGO")),
            input("env:CARGO", &hash)
        );
        assert_eq!(
            wire.line(&format!("{}\tenv:CARGO", wire.unset)),
            input("env:CARGO", wire.unset),
            "a variable that was not set at all is an input too"
        );
        // Measured: a mark's own locator under a hash content was a mark to this
        // crate and an input, and so a file the record said it had read, to the
        // checker, which reported the record clean.
        assert_eq!(wire.line(&format!("{hash}\t{mark}")), Line::Named(mark));
        assert_eq!(
            wire.line(&format!("{}\t{mark}", wire.unset)),
            Line::Named(mark)
        );
        // A mark an older version wrote is read by its content, not its name.
        assert_eq!(
            wire.line(&format!("{}\trun-directory-not-named", wire.unset)),
            Line::Unknown("run-directory-not-named")
        );
        // Measured: a line with no tab was a malformed record to this crate and a
        // mark named by an empty locator to the checker.
        assert_eq!(wire.line("no tab at all"), Line::Malformed);
        assert_eq!(wire.line(&format!("{hash}\t")), Line::Malformed);
        assert_eq!(
            wire.line(""),
            Line::Malformed,
            "a blank line has no tab either, so it is refused rather than passed over: measured, \
             both readers used to skip one"
        );
        // Measured: a content that is neither a hash nor `unset` was a mark to
        // both readers, when neither can say what it is.
        assert_eq!(wire.line("abc\tsrc/lib.rs"), Line::Malformed);
        // A CR is not part of a locator. Measured: a CRLF record was read as
        // inputs whose locators each ended in a CR, so a line carried the CR
        // rather than the record being refused for one.
        assert_eq!(wire.line(&format!("{hash}\tsrc/lib.rs\r")), Line::Malformed);
        assert_eq!(wire.line("\r"), Line::Malformed);
        assert_eq!(
            wire.line(&format!("{}\tsrc/lib.rs", "0".repeat(63))),
            Line::Malformed,
            "a content as long as a hash but not one is not a hash"
        );
        assert_eq!(
            wire.line(&format!("{}\tsrc/lib.rs", "g".repeat(64))),
            Line::Malformed,
            "and neither is one outside the alphabet the file spells"
        );
    }
}
