// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Grammar, Unicode, nesting, and allocation-free JSON evidence.
// - Must-Not:
//   - Decode typed manifests or confer ZIP/file authority.
// - Allows:
//   - Inputs: JSON byte vectors and explicit parser resource limits.
//   - Outputs: Exact borrowed bytes and typed rejection outcomes.
//   - Side effects: Test-process allocations only.
// - Split-When:
//   - ZIP root adapter integration gains independent fixtures.
// - Merge-When:
//   - The final typed manifest codec subsumes this syntax evidence.
// - Summary:
//   - Bounds JSON syntax validation without field interpretation.
// - Description:
//   - Invalid syntax cannot masquerade as a parsed manifest.
// - Usage:
//   - Run in the Rust workspace tests.
// - Defaults:
//   - Caller resource ceilings remain mandatory.
//

//! Strict and independently bounded JSON grammar regression checks.

use atrament_profile_json_syntax::{
    MAXIMUM_JSON_NESTING, ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
    validate_profile_json_object,
};

const fn limits() -> ProfileJsonSyntaxLimits {
    ProfileJsonSyntaxLimits {
        maximum_bytes: 1_024,
        maximum_nesting: 12,
        maximum_values: 64,
    }
}

#[test]
fn valid_json_objects_keep_exact_source_bytes() {
    let valid: &[&str] = &[
        "{}",
        " { } ",
        "{\n\t\r\"a\" : [] }",
        "{\"a\":0}",
        "{\"a\":-0}",
        "{\"a\":-1.23E+49}",
        "{\"a\":1e-8}",
        "{\"a\":0.0}",
        "{\"a\":true,\"b\":false,\"c\":null}",
        "{\"a\":[1,2,3]}",
        "{\"a\":{\"b\":1}}",
        "{\"a\":[[],{},[{}]]}",
        r#"{"a":"a\"b\\c\/d"}"#,
        r#"{"a":"\n\t\b\f\r"}"#,
        r#"{"a":"\u0061"}"#,
        r#"{"a":"\uD834\uDD1E"}"#,
        r#"{"a":"\ud834\udd1e"}"#,
        r#"{"a":"\u0000"}"#,
        r#"{"a":"\uFFFF"}"#,
        "{\"ñ\":\"明日📓\"}",
        "{\"combining\":\"ñ\"}",
        "{\"a\":{\"a\":{\"a\":{\"a\":{}}}}}",
        "{\"a\":1,\"a\":2}",
    ];
    for text in valid {
        let admitted = validate_profile_json_object(text.as_bytes(), limits())
            .expect("independent JSON fixture is valid");
        assert_eq!(admitted.bytes(), text.as_bytes());
        assert!(admitted.value_count() >= 1);
    }
}

#[test]
fn invalid_json_grammar_escapes_numbers_and_trailers_reject() {
    let invalid: &[&str] = &[
        "",
        " ",
        "{",
        "}",
        "{ } { }",
        "{}xyz",
        "{\"a\"}",
        "{a:1}",
        "{'a':1}",
        "{\"a\" 1}",
        "{\"a\":}",
        "{\"a\":1,}",
        "{,}",
        "{,\"a\":1}",
        "{\"a\":1 \"b\":2}",
        "{\"a\":[1,]}",
        "{\"a\":[,]}",
        "{\"a\":[1 2]}",
        "{\"a\":+1}",
        "{\"a\":01}",
        "{\"a\":-01}",
        "{\"a\":.1}",
        "{\"a\":1.}",
        "{\"a\":1e}",
        "{\"a\":1e+}",
        "{\"a\":--1}",
        "{\"a\":NaN}",
        "{\"a\":Infinity}",
        "{\"a\":TRUE}",
        "{\"a\":tru}",
        "{\"a\":falsee}",
        "{\"a\":\"abc}",
        r#"{"a":"\x00"}"#,
        r#"{"a":"\u123"}"#,
        r#"{"a":"\u12xz"}"#,
        r#"{"a":"\uD800"}"#,
        r#"{"a":"\uDC00"}"#,
        r#"{"a":"\uD800\uD800"}"#,
        r#"{"a":"\uD800\u0041"}"#,
        "{\"a\":\"\n\"}",
        "{\"a\":\"\t\"}",
        "{\"a\":\"\0\"}",
        "{\"a\": 1\u{000b}}",
        "{\"a\":1\u{00a0}}",
        "{\"a\":truefalse}",
    ];
    for text in invalid {
        assert!(
            validate_profile_json_object(text.as_bytes(), limits()).is_err(),
            "should reject {text:?}"
        );
    }
}

#[test]
fn root_nonobjects_are_refused() {
    for root in ["null", "[]", "1", "true", "\"ok\""] {
        assert_eq!(
            validate_profile_json_object(root.as_bytes(), limits()),
            Err(ProfileJsonSyntaxError::RootNotObject),
        );
    }
}

#[test]
fn syntax_budgets_count_values_and_nesting() {
    let bytes = b"{\"a\":[1,2,3]}";
    assert_eq!(
        validate_profile_json_object(
            bytes,
            ProfileJsonSyntaxLimits {
                maximum_values: 4,
                ..limits()
            }
        ),
        Err(ProfileJsonSyntaxError::MaximumValuesExceeded),
    );
    let admitted = validate_profile_json_object(
        bytes,
        ProfileJsonSyntaxLimits {
            maximum_values: 5,
            ..limits()
        },
    )
    .expect("root, array and three numbers");
    assert_eq!(admitted.value_count(), 5);
    for nesting in [0, 1, 2] {
        assert_eq!(
            validate_profile_json_object(
                b"{\"a\":[{}]}",
                ProfileJsonSyntaxLimits {
                    maximum_nesting: nesting,
                    ..limits()
                }
            ),
            Err(ProfileJsonSyntaxError::MaximumNestingExceeded),
        );
    }
    assert!(
        validate_profile_json_object(
            b"{\"a\":[{}]}",
            ProfileJsonSyntaxLimits {
                maximum_nesting: 3,
                ..limits()
            }
        )
        .is_ok()
    );
    let deep = format!(
        "{{\"a\":{}{} }}",
        "[".repeat(MAXIMUM_JSON_NESTING + 3),
        "]".repeat(MAXIMUM_JSON_NESTING + 3),
    );
    assert_eq!(
        validate_profile_json_object(
            deep.as_bytes(),
            ProfileJsonSyntaxLimits {
                maximum_bytes: 4_096,
                maximum_nesting: usize::MAX,
                maximum_values: usize::MAX,
            }
        ),
        Err(ProfileJsonSyntaxError::MaximumNestingExceeded),
    );
}

#[test]
fn malformed_utf8_is_rejected_before_syntax() {
    for bytes in [
        &[123u8, 255u8, 125u8][..],
        &[123u8, 192u8, 175u8, 125u8][..],
        &[123u8, 237u8, 160u8, 128u8, 125u8][..],
    ] {
        assert_eq!(
            validate_profile_json_object(bytes, limits()),
            Err(ProfileJsonSyntaxError::InvalidUtf8),
        );
    }
    assert_eq!(
        validate_profile_json_object(
            &[123u8, 255u8, 125u8],
            ProfileJsonSyntaxLimits {
                maximum_bytes: 1,
                ..limits()
            },
        ),
        Err(ProfileJsonSyntaxError::MaximumBytesExceeded),
    );
}

#[test]
fn single_byte_mutation_corpus_returns_only_borrowed_source_bytes() {
    let original = b"{\"part\":[1,true,null,{\"pages\":2}]}";
    let mut cases = 0usize;
    for offset in 0..original.len() {
        for replacement in [0u8, 1, 34, 92, 123, 125, 255] {
            let mut changed = original.to_vec();
            if let Some(byte) = changed.get_mut(offset) {
                *byte = replacement;
            }
            if let Ok(evidence) =
                validate_profile_json_object(&changed, limits())
            {
                assert_eq!(evidence.bytes(), changed.as_slice());
            }
            cases = cases.saturating_add(1);
        }
    }
    assert!(cases > 200);
}

#[test]
fn exactly_the_hard_nesting_ceiling_is_accepted_but_one_more_rejects() {
    for (arrays, accepted) in [
        (MAXIMUM_JSON_NESTING.saturating_sub(1), true),
        (MAXIMUM_JSON_NESTING, false),
    ] {
        let input =
            format!("{{\"a\":{}0{}}}", "[".repeat(arrays), "]".repeat(arrays),);
        let budget = ProfileJsonSyntaxLimits {
            maximum_bytes: 1_024,
            maximum_nesting: usize::MAX,
            maximum_values: usize::MAX,
        };
        let observed = validate_profile_json_object(input.as_bytes(), budget);
        if accepted {
            assert!(observed.is_ok(), "arrays={arrays}");
        } else {
            assert_eq!(
                observed,
                Err(ProfileJsonSyntaxError::MaximumNestingExceeded)
            );
        }
    }
}

#[test]
fn zero_value_and_exact_byte_budgets_fail_with_stable_precedence() {
    let input = b"{\"a\":null}";
    assert_eq!(
        validate_profile_json_object(
            input,
            ProfileJsonSyntaxLimits {
                maximum_values: 0,
                ..limits()
            }
        ),
        Err(ProfileJsonSyntaxError::MaximumValuesExceeded),
    );
    assert_eq!(
        validate_profile_json_object(
            input,
            ProfileJsonSyntaxLimits {
                maximum_bytes: 0,
                ..limits()
            }
        ),
        Err(ProfileJsonSyntaxError::MaximumBytesExceeded),
    );
    let accepted = validate_profile_json_object(
        input,
        ProfileJsonSyntaxLimits {
            maximum_bytes: input.len(),
            maximum_values: 2,
            maximum_nesting: 1,
        },
    )
    .expect("exact byte, two-value, single-object bounds succeed");
    assert_eq!(accepted.value_count(), 2);
}
