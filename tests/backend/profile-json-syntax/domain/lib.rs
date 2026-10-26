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
    MAXIMUM_JSON_NESTING, MAXIMUM_JSON_OBJECT_MEMBERS, ProfileJsonSyntaxError,
    ProfileJsonSyntaxLimits, validate_profile_json_object,
};

const fn limits() -> ProfileJsonSyntaxLimits {
    ProfileJsonSyntaxLimits {
        maximum_bytes: 1_024,
        maximum_nesting: 12,
        maximum_object_members: 16,
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
                maximum_object_members: usize::MAX,
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
            maximum_object_members: usize::MAX,
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
            maximum_object_members: 1,
            maximum_nesting: 1,
        },
    )
    .expect("exact byte, two-value, single-object bounds succeed");
    assert_eq!(accepted.value_count(), 2);
}

#[test]
fn duplicate_decoded_keys_fail_at_every_object_depth() {
    let duplicates = [
        r#"{"a":1,"a":2}"#,
        r#"{"":1,"":2}"#,
        r#"{"a":1,"\u0061":2}"#,
        r#"{"f\u006fo":1,"foo":2}"#,
        r#"{"ñ":1,"\u00f1":2}"#,
        r#"{"N":1,"\u004e":2}"#,
        r#"{"/":1,"\/":2}"#,
        r#"{"\n":0,"\u000a":1}"#,
        r#"{"\\":0,"\u005c":1}"#,
        r#"{"😀":1,"\uD83D\uDE00":2}"#,
        r#"{"\uD83d\ude00":1,"😀":2}"#,
        r#"{"a":{"x":1,"x":2}}"#,
        r#"{"a":[{"x":1,"\u0078":2}]}"#,
        r#"{"a":{"b":{"c":1,"c":2}}}"#,
    ];
    for bytes in duplicates {
        assert_eq!(
            validate_profile_json_object(bytes.as_bytes(), limits()),
            Err(ProfileJsonSyntaxError::DuplicateObjectKey),
            "duplicates={bytes:?}",
        );
    }
}

#[test]
fn unicode_scalar_equality_is_not_casefolding_or_normalization() {
    for accepted in [
        r#"{"A":1,"a":2}"#,
        r#"{"ñ":1,"ñ":2}"#,
        r#"{"\u0061":1,"b":2}"#,
        r#"{"a":{"x":1},"b":{"x":2}}"#,
        r#"{"a":[{"x":1},{"x":2}]}"#,
        r#"{"\\":1,"/":2}"#,
    ] {
        assert!(
            validate_profile_json_object(accepted.as_bytes(), limits()).is_ok(),
            "distinct={accepted:?}"
        );
    }
}

#[test]
fn object_member_budget_has_an_exact_bound_and_hard_ceiling() {
    let source = r#"{"a":1,"b":2}"#;
    assert_eq!(
        validate_profile_json_object(
            source.as_bytes(),
            ProfileJsonSyntaxLimits {
                maximum_object_members: 1,
                ..limits()
            }
        ),
        Err(ProfileJsonSyntaxError::MaximumObjectMembersExceeded),
    );
    assert!(
        validate_profile_json_object(
            source.as_bytes(),
            ProfileJsonSyntaxLimits {
                maximum_object_members: 2,
                ..limits()
            }
        )
        .is_ok()
    );
    assert!(
        validate_profile_json_object(
            b"{}",
            ProfileJsonSyntaxLimits {
                maximum_object_members: 0,
                ..limits()
            }
        )
        .is_ok()
    );
    assert_eq!(
        validate_profile_json_object(
            br#"{"a":1}"#,
            ProfileJsonSyntaxLimits {
                maximum_object_members: 0,
                ..limits()
            }
        ),
        Err(ProfileJsonSyntaxError::MaximumObjectMembersExceeded),
    );
}

#[test]
fn fixed_object_member_cap_rejects_the_next_key_without_heap_growth() {
    let build = |members: usize| -> String {
        let items = (0..members)
            .map(|index| format!("\"entry{index}\":0"))
            .collect::<Vec<_>>()
            .join(",");
        format!("{{{items}}}")
    };
    let permissive = ProfileJsonSyntaxLimits {
        maximum_bytes: 8_192,
        maximum_nesting: 4,
        maximum_object_members: usize::MAX,
        maximum_values: usize::MAX,
    };
    let admitted = build(MAXIMUM_JSON_OBJECT_MEMBERS);
    assert_eq!(
        validate_profile_json_object(admitted.as_bytes(), permissive)
            .expect("exact maximum member count must be supported")
            .value_count(),
        MAXIMUM_JSON_OBJECT_MEMBERS.saturating_add(1),
    );
    let over = build(MAXIMUM_JSON_OBJECT_MEMBERS.saturating_add(1));
    assert_eq!(
        validate_profile_json_object(over.as_bytes(), permissive),
        Err(ProfileJsonSyntaxError::MaximumObjectMembersExceeded),
    );
}

#[test]
fn nested_object_keys_are_scoped_to_each_individual_object() {
    let distinct = r#"{"a":{"id":1},"b":{"id":2},"id":3}"#;
    assert!(
        validate_profile_json_object(distinct.as_bytes(), limits()).is_ok()
    );
    let collision = r#"{"a":{"id":1},"b":{"id":2,"\u0069d":3}}"#;
    assert_eq!(
        validate_profile_json_object(collision.as_bytes(), limits()),
        Err(ProfileJsonSyntaxError::DuplicateObjectKey),
    );
}

#[test]
fn generated_escape_positions_preserve_duplicate_key_identity() {
    let mut observed = 0usize;
    for length in 1usize..=64usize {
        for position in 0usize..length {
            let before = "n".repeat(position);
            let after =
                "n".repeat(length.saturating_sub(position).saturating_sub(1));
            let plain = format!("{before}A{after}");
            let alias = format!(r"{before}\u0041{after}");
            let duplicate = format!(r#"{{"{plain}":0,"{alias}":1}}"#);
            assert_eq!(
                validate_profile_json_object(duplicate.as_bytes(), limits()),
                Err(ProfileJsonSyntaxError::DuplicateObjectKey),
                "length={length},position={position}",
            );
            let different = format!(r"{before}\u0042{after}");
            let distinct = format!(r#"{{"{plain}":0,"{different}":1}}"#);
            assert!(
                validate_profile_json_object(distinct.as_bytes(), limits())
                    .is_ok(),
                "length={length},position={position}",
            );
            observed = observed.saturating_add(2);
        }
    }
    assert!(observed > 4_000);
}
