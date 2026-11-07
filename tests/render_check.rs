//! SPEC-015: the independent checkers tested (test the testers before using
//! them). `fuzz/shared/render_check.rs` holds a strict JSON parser, a strict
//! CSV parser and a naive `safe_text`; each is run over a table of valid and
//! invalid documents, so a lenient checker is itself shown red.

include!("../fuzz/shared/render_check.rs");

use render_check::{naive_safe_text, parse_csv, parse_json, J};

fn json_ok(text: &str) -> J {
    parse_json(text.as_bytes()).unwrap_or_else(|e| panic!("{text:?} should parse: {e}"))
}

fn json_bad(text: &[u8], why: &str) {
    assert!(
        parse_json(text).is_err(),
        "the JSON checker accepted {why}: {:?}",
        String::from_utf8_lossy(text)
    );
}

#[test]
fn json_checker_accepts_valid_documents_and_rejects_each_invalid_one() {
    // Valid documents.
    for text in [
        "null",
        "true",
        "false",
        "0",
        "-0",
        "10",
        "-12.5e+3",
        "1E5",
        "\"\"",
        "\"a\\\"b\\\\c\\/d\\b\\f\\n\\r\\t\"",
        "\"\\u00e9\\u0000\"",
        "\"\\ud83d\\ude00\"",
        "\"\u{e9}\u{65e5}\"",
        "[]",
        "{}",
        "[1,2,3]",
        "{\"a\":1,\"b\":[true,null,{\"c\":\"d\"}]}",
        "  \n\t{ \"a\" : [ 1 , 2 ] }\r\n",
    ] {
        json_ok(text);
    }
    // Depth 32 is the limit.
    let deep32 = format!("{}{}", "[".repeat(32), "]".repeat(32));
    json_ok(&deep32);
    // The parsed tree keeps key order.
    let v = json_ok("{\"b\":1,\"a\":2}");
    assert_eq!(v.keys(), vec!["b", "a"]);
    assert_eq!(v.get("a").and_then(J::as_uint), Some(2));
    assert_eq!(json_ok("\"\\ud83d\\ude00\"").as_str(), Some("\u{1F600}"));

    // Invalid documents, one reason each.
    json_bad(b"[1,2,]", "a trailing comma in an array");
    json_bad(b"{\"a\":1,}", "a trailing comma in an object");
    json_bad(b"[01]", "a leading zero");
    json_bad(b"[-01]", "a negative leading zero");
    json_bad(b"[1.]", "a number with a bare point");
    json_bad(b"[.5]", "a number without an integer part");
    json_bad(b"[1e]", "an empty exponent");
    json_bad(b"[+1]", "a plus sign");
    json_bad(b"\"a\x01b\"", "a raw control character in a string");
    json_bad(b"\"a\nb\"", "a raw newline in a string");
    json_bad(b"\"a\\xb\"", "a bad escape");
    json_bad(b"\"a\\", "an unterminated escape");
    json_bad(b"\"\\u12\"", "a short \\u escape");
    json_bad(b"\"\\ud800\"", "a lone high surrogate escape");
    json_bad(b"\"\\udc00\"", "a lone low surrogate escape");
    json_bad(
        b"\"\\ud800\\u0041\"",
        "a high surrogate with a non-surrogate",
    );
    json_bad(b"\"\xff\"", "invalid UTF-8");
    json_bad(b"\"\xc3\"", "a truncated UTF-8 sequence");
    json_bad(b"{\"a\":1,\"a\":2}", "a duplicate key");
    json_bad(b"[1] x", "trailing data");
    json_bad(b"[1][2]", "two documents");
    json_bad(b"", "empty input");
    json_bad(b"   ", "blank input");
    json_bad(b"{'a':1}", "single quotes");
    json_bad(b"{a:1}", "an unquoted key");
    json_bad(b"[1 2]", "a missing comma");
    json_bad(b"nul", "a truncated literal");
    json_bad(b"True", "a capitalised literal");
    json_bad(b"\"abc", "an unterminated string");
    json_bad(b"[1", "an unterminated array");
    let deep33 = format!("{}{}", "[".repeat(33), "]".repeat(33));
    json_bad(deep33.as_bytes(), "nesting of depth 33");
    let deep33_obj = format!("{}1{}", "{\"a\":".repeat(33), "}".repeat(33));
    json_bad(deep33_obj.as_bytes(), "objects nested 33 deep");
}

fn csv_bad(text: &[u8], why: &str) {
    assert!(
        parse_csv(text).is_err(),
        "the CSV checker accepted {why}: {:?}",
        String::from_utf8_lossy(text)
    );
}

#[test]
fn csv_checker_accepts_valid_documents_and_rejects_each_invalid_one() {
    let ok = |text: &str| -> Vec<Vec<String>> {
        parse_csv(text.as_bytes()).unwrap_or_else(|e| panic!("{text:?} should parse: {e}"))
    };
    assert_eq!(ok("a,b\r\n"), vec![vec!["a", "b"]]);
    assert_eq!(ok("a,b\r\n1,2\r\n"), vec![vec!["a", "b"], vec!["1", "2"]]);
    // Empty fields, including the last.
    assert_eq!(
        ok("a,,\r\n,,\r\n"),
        vec![vec!["a", "", ""], vec!["", "", ""]]
    );
    // Quoted: comma, doubled quote, CR, LF, an empty quoted field.
    assert_eq!(
        ok("a,b,c\r\n\"x,y\",\"say \"\"hi\"\"\",\"\"\r\n"),
        vec![vec!["a", "b", "c"], vec!["x,y", "say \"hi\"", ""]]
    );
    assert_eq!(
        ok("a,b\r\n\"l1\r\nl2\",\"x\ny\rz\"\r\n"),
        vec![vec!["a", "b"], vec!["l1\r\nl2", "x\ny\rz"]]
    );
    // A single column.
    assert_eq!(ok("a\r\nb\r\n"), vec![vec!["a"], vec!["b"]]);
    // Non-ASCII.
    assert_eq!(ok("\u{e9}\r\n"), vec![vec!["\u{e9}"]]);

    csv_bad(b"a,b\r\n1,2,3\r\n", "a ragged row (too long)");
    csv_bad(b"a,b\r\n1\r\n", "a ragged row (too short)");
    csv_bad(b"a,b\r\n1,2\r\n3\r\n", "a ragged last row");
    csv_bad(b"a,b\r\nx\"y,2\r\n", "a bare quote in an unquoted field");
    csv_bad(b"a,b\r\n\"x\"y,2\r\n", "junk after a closing quote");
    csv_bad(b"a,b\r\n\"x,2\r\n", "an unterminated quote");
    csv_bad(b"a,b\r\n1,2", "a missing final CRLF");
    csv_bad(b"a,b\r\n1,", "a missing final CRLF after a comma");
    csv_bad(b"a,b\n1,2\n", "LF-only line ends");
    csv_bad(b"a,b\r\n1,2\n", "an LF-only last line end");
    csv_bad(b"a,b\r\n1,\r2\r\n", "a bare CR in an unquoted field");
    csv_bad(b"", "empty input");
    csv_bad(b"a,\xff\r\n", "invalid UTF-8");
    csv_bad(b"a,b\r\n\r\n", "a blank line (a row with one field)");
}

#[test]
fn the_naive_safe_text_matches_the_dec_010_examples() {
    assert_eq!(naive_safe_text(b"plain"), "plain");
    assert_eq!(naive_safe_text(b"a\"b\\c"), "a\\\"b\\\\c");
    assert_eq!(
        naive_safe_text(b"\x00\x1b\n\r\t\x7f"),
        "\\x00\\x1b\\n\\r\\t\\x7f"
    );
    assert_eq!(naive_safe_text("\u{85}".as_bytes()), "\\u{0085}");
    assert_eq!(naive_safe_text("\u{202e}".as_bytes()), "\\u{202e}");
    assert_eq!(
        naive_safe_text("\u{2028}\u{2029}".as_bytes()),
        "\\u{2028}\\u{2029}"
    );
    assert_eq!(
        naive_safe_text(&[0xff, b'a', 0xe2, 0x82]),
        "\\xffa\\xe2\\x82"
    );
    assert_eq!(naive_safe_text("caf\u{e9}".as_bytes()), "caf\u{e9}");
}
