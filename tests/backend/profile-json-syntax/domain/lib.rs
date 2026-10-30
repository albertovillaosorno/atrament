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
    MAXIMUM_JSON_NESTING, MAXIMUM_JSON_OBJECT_MEMBERS, ProfileJsonPathSegment,
    ProfileJsonStringCopyError, ProfileJsonSyntaxError,
    ProfileJsonSyntaxLimits, ProfileJsonValueKind,
    validate_profile_json_object,
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

#[test]
fn sealed_root_member_lookup_preserves_exact_json_value_bytes() {
    let bytes = concat!(
        r#" { "\u0061" : [ 1 , {"nested": [false, null]} ] , "#,
        r#""empty": "" , "literal": "\u0000", "last": 1e+2 } "#,
    )
    .as_bytes();
    let admitted = validate_profile_json_object(bytes, limits())
        .expect("every nested value is syntactically valid");
    assert_eq!(
        admitted.root_member_json("a"),
        Some(br#"[ 1 , {"nested": [false, null]} ]"#.as_slice()),
    );
    assert_eq!(
        admitted.root_member_json("empty"),
        Some(br#""""#.as_slice())
    );
    assert_eq!(
        admitted.root_member_json("literal"),
        Some(br#""\u0000""#.as_slice()),
    );
    assert_eq!(admitted.root_member_json("last"), Some(b"1e+2".as_slice()));
    for missing in ["\u{00e1}", "nested", "LAsT", "", "a/", "%61"] {
        assert_eq!(admitted.root_member_json(missing), None);
    }
    let first = admitted.root_member_json("a").expect("admitted member");
    assert!(
        bytes.windows(first.len()).any(|candidate| {
            std::ptr::eq(candidate.as_ptr(), first.as_ptr())
        })
    );
}

#[test]
fn sealed_lookup_only_matches_root_decoded_unicode_scalars() {
    let bytes = r#"{"😀":true,"ñ":0,"ñ":1,"outer":{"id":2},"id":3}"#;
    let admitted = validate_profile_json_object(bytes.as_bytes(), limits())
        .expect("composed and decomposed names are distinct");
    assert_eq!(admitted.root_member_json("😀"), Some(b"true".as_slice()));
    assert_eq!(admitted.root_member_json("ñ"), Some(b"0".as_slice()));
    assert_eq!(admitted.root_member_json("ñ"), Some(b"1".as_slice()));
    assert_eq!(admitted.root_member_json("id"), Some(b"3".as_slice()));
    assert_eq!(
        admitted.root_member_json("outer"),
        Some(br#"{"id":2}"#.as_slice())
    );
    assert_eq!(admitted.root_member_json("ID"), None);
    let empty = validate_profile_json_object(b"{}", limits())
        .expect("empty object is valid");
    assert_eq!(empty.root_member_json("id"), None);
}

#[test]
fn sealed_root_lookup_matches_escaped_astral_unicode_and_empty_names() {
    let encoded = r#"{"\uD83D\uDE00":[1,2],"":null,"\u0062":false}"#;
    let admitted = validate_profile_json_object(encoded.as_bytes(), limits())
        .expect("escaped astral and empty keys are valid JSON");
    assert_eq!(admitted.root_member_json("😀"), Some(b"[1,2]".as_slice()));
    assert_eq!(admitted.root_member_json(""), Some(b"null".as_slice()));
    assert_eq!(admitted.root_member_json("b"), Some(b"false".as_slice()));
    assert_eq!(admitted.root_member_json("\u{1f601}"), None);
    assert_eq!(admitted.root_member_json("B"), None);
}

#[test]
fn sealed_root_lookup_handles_maximum_members_with_nested_values() {
    let fields = (0usize..MAXIMUM_JSON_OBJECT_MEMBERS)
        .map(|index| format!(r#""f{index}":[{index},{{"v":{index}}}]"#))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("{{{fields}}}");
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: 8_192,
        maximum_nesting: 4,
        maximum_object_members: MAXIMUM_JSON_OBJECT_MEMBERS,
        maximum_values: usize::MAX,
    };
    let admitted = validate_profile_json_object(source.as_bytes(), caps)
        .expect("the exact member ceiling includes all nested values");
    assert_eq!(
        admitted.value_count(),
        MAXIMUM_JSON_OBJECT_MEMBERS
            .saturating_mul(4)
            .saturating_add(1),
    );
    for index in [0usize, 1, 32, 64, 127] {
        let key = format!("f{index}");
        let expected = format!(r#"[{index},{{"v":{index}}}]"#);
        assert_eq!(
            admitted.root_member_json(&key),
            Some(expected.as_bytes()),
            "key={key}"
        );
    }
    assert_eq!(admitted.root_member_json("v"), None);
    assert_eq!(admitted.root_member_json("f128"), None);
}

#[test]
fn root_lookup_distinguishes_literal_backslash_u_from_unicode_escape() {
    let source = r#"{"\\u0061":0,"\u0061":1}"#;
    let admitted = validate_profile_json_object(source.as_bytes(), limits())
        .expect("literal slash-u and escaped Unicode are separate keys");
    assert_eq!(admitted.root_member_json(r"\u0061"), Some(b"0".as_slice()));
    assert_eq!(admitted.root_member_json("a"), Some(b"1".as_slice()));
    assert_eq!(admitted.root_member_json("\\\\u0061"), None);
}

#[test]
fn generated_escaped_root_lookup_matches_exact_decoded_names() {
    let mut cases = 0usize;
    for length in 1usize..=48usize {
        for position in 0usize..length {
            let before = "n".repeat(position);
            let after =
                "n".repeat(length.saturating_sub(position).saturating_sub(1));
            let plain = format!("{before}A{after}");
            let escaped = format!(r"{before}\u0041{after}");
            let source = format!(r#"{{"{escaped}":true}}"#);
            let admitted =
                validate_profile_json_object(source.as_bytes(), limits())
                    .expect("escaped name has valid JSON syntax");
            assert_eq!(
                admitted.root_member_json(&plain),
                Some(b"true".as_slice())
            );
            assert_eq!(
                admitted.root_member_json(&format!("{before}B{after}")),
                None,
            );
            cases = cases.saturating_add(1);
        }
    }
    assert!(cases > 1_000);
}

#[test]
fn sealed_json_value_projection_exposes_all_syntax_kinds_without_typing() {
    let input = concat!(
        r#"{"object":{},"array":[],"true":true,"false":false,"null":null,"#,
        r#""number":-1.30e+4,"string":"\u0061"}"#,
    );
    let admitted = validate_profile_json_object(input.as_bytes(), limits())
        .expect("all six ordinary JSON syntactic categories are admitted");
    for (key, expected, original) in [
        ("object", ProfileJsonValueKind::Object, "{}"),
        ("array", ProfileJsonValueKind::Array, "[]"),
        ("true", ProfileJsonValueKind::Boolean, "true"),
        ("false", ProfileJsonValueKind::Boolean, "false"),
        ("null", ProfileJsonValueKind::Null, "null"),
        ("number", ProfileJsonValueKind::Number, "-1.30e+4"),
        ("string", ProfileJsonValueKind::String, r#""\u0061""#),
    ] {
        let view = admitted.root_member_value(key).expect("admitted member");
        assert_eq!(view.kind(), Some(expected), "key={key}");
        assert_eq!(view.bytes(), original.as_bytes(), "key={key}");
        assert_eq!(admitted.root_member_json(key), Some(view.bytes()));
        assert_eq!(view.object_member("anything"), None);
        assert_eq!(view.array_element(0), None);
    }
    assert_eq!(admitted.root_member_value("missing"), None);
}

#[test]
fn sealed_nested_arrays_and_objects_preserve_unmodified_value_slices() {
    let json = concat!(
        r#" { "\u0072oot" : { "list" : [ 1e+2 , {"a" : ["#,
        r#""\uD83D\uDE00", null]} , [] ]"#,
        r#", "empty" : {} } } "#,
    );
    let admitted = validate_profile_json_object(json.as_bytes(), limits())
        .expect("nested unicode object and array syntax is valid");
    let root = admitted
        .root_member_value("root")
        .expect("escaped root name");
    assert_eq!(root.kind(), Some(ProfileJsonValueKind::Object));
    let list = root.object_member("list").expect("nested list");
    assert_eq!(list.kind(), Some(ProfileJsonValueKind::Array));
    assert_eq!(list.array_element(0).expect("number").bytes(), b"1e+2");
    let first_object = list.array_element(1).expect("nested object");
    assert_eq!(first_object.kind(), Some(ProfileJsonValueKind::Object));
    let inner = first_object.object_member("a").expect("inner array");
    assert_eq!(
        inner
            .array_element(0)
            .expect("escaped astral string")
            .bytes(),
        br#""\uD83D\uDE00""#,
    );
    assert_eq!(
        inner.array_element(1).expect("null").kind(),
        Some(ProfileJsonValueKind::Null),
    );
    assert_eq!(inner.array_element(2), None);
    assert_eq!(
        list.array_element(2).expect("empty array").array_element(0),
        None
    );
    assert_eq!(list.array_element(3), None);
    assert_eq!(list.array_element(usize::MAX), None);
    assert_eq!(
        root.object_member("list").expect("list").bytes(),
        list.bytes()
    );
    assert_eq!(root.object_member("a"), None);
    assert_eq!(first_object.object_member("A"), None);
    assert_eq!(
        root.object_member("empty")
            .expect("empty object")
            .object_member("x"),
        None
    );
    assert_eq!(root.array_element(0), None);
    let raw = inner.array_element(0).expect("source value").bytes();
    assert!(
        json.as_bytes()
            .windows(raw.len())
            .any(|candidate| std::ptr::eq(candidate.as_ptr(), raw.as_ptr()))
    );
}

#[test]
fn sealed_array_projection_distinguishes_nested_from_sibling_values() {
    let bytes = br#"{"items":[[{"v":0}],{"v":1},{"v":2}],"v":3}"#;
    let admitted =
        validate_profile_json_object(bytes, limits()).expect("valid");
    let array = admitted.root_member_value("items").expect("root array");
    for index in [1, 2] {
        let observed = array
            .array_element(index)
            .expect("object element")
            .object_member("v")
            .expect("nested value");
        let expected = index.to_string();
        assert_eq!(observed.bytes(), expected.as_bytes());
    }
    let deep = array.array_element(0).expect("first child array");
    assert_eq!(
        deep.array_element(0)
            .expect("nested object")
            .object_member("v")
            .expect("nested value")
            .bytes(),
        b"0"
    );
    assert_eq!(array.object_member("v"), None);
    assert_eq!(
        admitted.root_member_value("v").expect("root value").bytes(),
        b"3"
    );
}

#[test]
fn sealed_array_iterator_preserves_order_and_is_fused() {
    let source = r#"{"list": [true, {"x":1}, [null, 3.0], "\u0061"]}"#;
    let root = validate_profile_json_object(source.as_bytes(), limits())
        .expect("syntax admission");
    let list = root.root_member_value("list").expect("list value");
    let mut values = list.array_elements().expect("array iterator");
    for expected in ["true", r#"{"x":1}"#, "[null, 3.0]", r#""\u0061""#] {
        let next = values.next().expect("next array element");
        assert_eq!(next.bytes(), expected.as_bytes());
    }
    for _ in 0..3 {
        assert_eq!(values.next(), None, "fused after last item");
    }
    let empty = root
        .root_member_value("list")
        .expect("list must exist")
        .array_element(2)
        .expect("inner array");
    let mut nested = empty.array_elements().expect("nested iterator");
    assert_eq!(nested.next().expect("null").bytes(), b"null");
    assert_eq!(nested.next().expect("decimal").bytes(), b"3.0");
    assert_eq!(nested.next(), None);
    assert_eq!(nested.next(), None);
}

#[test]
fn array_iterator_empty_and_large_index_do_not_wrap_or_leak() {
    let source = br#"{"empty":[],"not_array":{},"numbers":[0,1,2]}"#;
    let root = validate_profile_json_object(source, limits()).expect("valid");
    let mut empty = root
        .root_member_value("empty")
        .expect("array")
        .array_elements()
        .expect("empty array iterator");
    assert_eq!(empty.next(), None);
    assert_eq!(empty.next(), None);
    assert_eq!(
        root.root_member_value("empty")
            .expect("array")
            .array_element(0),
        None
    );
    assert_eq!(
        root.root_member_value("numbers")
            .expect("array")
            .array_element(usize::MAX),
        None
    );
    assert!(
        root.root_member_value("not_array")
            .expect("object")
            .array_elements()
            .is_none()
    );
    let values = root
        .root_member_value("numbers")
        .expect("array")
        .array_elements()
        .expect("iterator");
    let decoded: Vec<Vec<u8>> =
        values.map(|value| value.bytes().to_vec()).collect();
    assert_eq!(decoded, [b"0".to_vec(), b"1".to_vec(), b"2".to_vec()]);
}

#[test]
fn bounded_large_arrays_traverse_once_without_hidden_field_mapping() {
    let members = (0..256usize)
        .map(|index| format!("{{\"x\":{index}}}"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("{{\"rows\":[{members}]}}");
    let budget = ProfileJsonSyntaxLimits {
        maximum_bytes: 8_192,
        maximum_nesting: 3,
        maximum_object_members: 2,
        maximum_values: 514,
    };
    let root = validate_profile_json_object(source.as_bytes(), budget)
        .expect("root array with 256 objects and scalar leaves");
    let rows = root.root_member_value("rows").expect("array");
    let mut count = 0usize;
    for (index, row) in rows.array_elements().expect("iterator").enumerate() {
        assert_eq!(row.kind(), Some(ProfileJsonValueKind::Object));
        let expected = index.to_string();
        assert_eq!(
            row.object_member("x").expect("one member").bytes(),
            expected.as_bytes()
        );
        count = count.saturating_add(1);
    }
    assert_eq!(count, 256);
    assert_eq!(
        rows.array_element(255)
            .expect("final row")
            .object_member("x")
            .expect("scalar")
            .bytes(),
        b"255"
    );
    assert_eq!(rows.array_element(256), None);
}

#[test]
fn sealed_scalar_projection_decodes_json_strings_without_rewriting_source() {
    let input = concat!(
        r#"{"escaped":"\"\\\/\b\f\n\r\t\u0061\uD83D\uDE00","#,
        r#""raw":"mañana 📓","empty":"","combined":"ñ","composed":"ñ"}"#,
    );
    let root = validate_profile_json_object(input.as_bytes(), limits())
        .expect("escaped and raw UTF-8 are valid JSON");
    let escaped = root.root_member_value("escaped").expect("escaped string");
    let decoded = escaped
        .string_chars()
        .expect("string iterator")
        .collect::<String>();
    assert_eq!(decoded, "\"\\/\u{0008}\u{000c}\n\r\ta😀");
    assert_eq!(escaped.bytes(), br#""\"\\\/\b\f\n\r\t\u0061\uD83D\uDE00""#);
    for (field, expected) in [
        ("raw", "mañana 📓"),
        ("empty", ""),
        ("combined", "ñ"),
        ("composed", "ñ"),
    ] {
        let got = root
            .root_member_value(field)
            .expect("field")
            .string_chars()
            .expect("string")
            .collect::<String>();
        assert_eq!(got, expected);
    }
    assert_ne!(
        root.root_member_value("combined")
            .expect("combined")
            .string_chars()
            .expect("string")
            .collect::<String>(),
        root.root_member_value("composed")
            .expect("composed")
            .string_chars()
            .expect("string")
            .collect::<String>(),
    );
    let mut empty = root
        .root_member_value("empty")
        .expect("empty")
        .string_chars()
        .expect("iterator");
    assert_eq!(empty.next(), None);
    assert_eq!(empty.next(), None);
    assert!(
        root.root_member_value("empty")
            .expect("string")
            .array_elements()
            .is_none()
    );
}

#[test]
fn sealed_plain_unsigned_integer_never_coerces_numeric_spellings() {
    let input = concat!(
        r#"{"zero":0,"max":18446744073709551615,"#,
        r#""too_large":18446744073709551616,"#,
        r#""negative":-1,"negative_zero":-0,"decimal":1.0,"exponent":1e2,"#,
        r#""big_exp":1e9999,"word":"123","boolean":true,"nil":null}"#,
    );
    let root = validate_profile_json_object(
        input.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: 2_048,
            ..limits()
        },
    )
    .expect("large exponents are valid syntax but not integer values");
    assert_eq!(
        root.root_member_value("zero")
            .expect("zero")
            .unsigned_integer(),
        Some(0)
    );
    assert_eq!(
        root.root_member_value("max")
            .expect("u64 max")
            .unsigned_integer(),
        Some(u64::MAX)
    );
    for field in [
        "too_large",
        "negative",
        "negative_zero",
        "decimal",
        "exponent",
        "big_exp",
        "word",
        "boolean",
        "nil",
    ] {
        assert_eq!(
            root.root_member_value(field)
                .expect("member")
                .unsigned_integer(),
            None,
            "field={field}"
        );
    }
    assert_eq!(
        root.root_member_value("boolean")
            .expect("boolean")
            .boolean(),
        Some(true)
    );
    assert_eq!(
        root.root_member_value("zero").expect("zero").boolean(),
        None
    );
    assert!(root.root_member_value("nil").expect("nil").is_null());
    assert!(!root.root_member_value("zero").expect("zero").is_null());
    assert!(
        root.root_member_value("boolean")
            .expect("boolean")
            .string_chars()
            .is_none()
    );
}

#[test]
fn generated_escaped_json_string_values_decode_exact_unicode_scalars() {
    for code in [
        0x0u32, 0x1, 0x7f, 0x80, 0x7ff, 0x800, 0x2603, 0xd7ff, 0xe000, 0xffff,
    ] {
        let source = format!(r#"{{"field":"\u{code:04x}"}}"#);
        let root = validate_profile_json_object(source.as_bytes(), limits())
            .expect("valid encoded BMP scalar");
        let observed = root
            .root_member_value("field")
            .expect("field")
            .string_chars()
            .expect("string")
            .collect::<String>();
        let expected = char::from_u32(code).expect("not a surrogate");
        assert_eq!(observed, expected.to_string(), "scalar={code:x}");
    }
    for (high, low, expected) in [
        (0xd800, 0xdc00, '\u{10000}'),
        (0xd83d, 0xde00, '😀'),
        (0xdbff, 0xdfff, '\u{10ffff}'),
    ] {
        let source = format!(r#"{{"field":"\u{high:04X}\u{low:04X}"}}"#);
        let root = validate_profile_json_object(source.as_bytes(), limits())
            .expect("valid surrogate pair in JSON string");
        let output = root
            .root_member_value("field")
            .expect("field")
            .string_chars()
            .expect("string")
            .collect::<String>();
        assert_eq!(output, expected.to_string());
    }
}

#[test]
fn sealed_object_field_stream_preserves_order_and_raw_names() {
    let document = concat!(
        r#" { "\u0061" : [ 0 ] , "😀" : { "inner" : false }, "#,
        r#""ñ":3 , "ñ":null , "": "\u0061" } "#,
    );
    let root = validate_profile_json_object(document.as_bytes(), limits())
        .expect("composed and decomposed keys are distinct");
    let mut fields = root.root_fields();
    for (expected_name, expected_json_key, expected_value) in [
        ("a", r#""\u0061""#, "[ 0 ]"),
        ("😀", r#""😀""#, r#"{ "inner" : false }"#),
        ("ñ", r#""ñ""#, "3"),
        ("ñ", r#""ñ""#, "null"),
        ("", r#""""#, r#""\u0061""#),
    ] {
        let field = fields.next().expect("next admitted source member");
        assert_eq!(field.key_json(), expected_json_key.as_bytes());
        assert_eq!(field.value().bytes(), expected_value.as_bytes());
        assert_eq!(
            field.decoded_key_chars().collect::<String>(),
            expected_name
        );
        assert!(field.key_matches(expected_name));
        assert!(!field.key_matches(&format!("{expected_name}x")));
        let original_key = field.key_json();
        assert!(document.as_bytes().windows(original_key.len()).any(
            |candidate| std::ptr::eq(candidate.as_ptr(), original_key.as_ptr())
        ));
    }
    for _ in 0..3 {
        assert_eq!(fields.next(), None);
    }
}

#[test]
fn nested_object_fields_do_not_steal_sibling_or_root_members() {
    let source = br#"{"x":{"one":1,"two":2},"y":[{"one":3},{}]}"#;
    let root = validate_profile_json_object(source, limits()).expect("valid");
    let outer = root
        .root_fields()
        .map(|field| field.decoded_key_chars().collect::<String>())
        .collect::<Vec<_>>();
    assert_eq!(outer, ["x", "y"]);
    let x = root.root_member_value("x").expect("object");
    let children = x
        .object_fields()
        .expect("fields")
        .map(|field| {
            (
                field.decoded_key_chars().collect::<String>(),
                field.value().bytes(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        children,
        [
            (String::from("one"), b"1".as_slice()),
            (String::from("two"), b"2".as_slice())
        ]
    );
    let y = root.root_member_value("y").expect("array");
    assert!(y.object_fields().is_none());
    let inner = y
        .array_element(0)
        .expect("first object")
        .object_fields()
        .expect("nested fields")
        .collect::<Vec<_>>();
    assert_eq!(inner.len(), 1);
    assert!(inner[0].key_matches("one"));
    assert_eq!(inner[0].value().bytes(), b"3");
    let mut empty = y
        .array_element(1)
        .expect("second object")
        .object_fields()
        .expect("empty member iterator");
    assert_eq!(empty.next(), None);
    assert_eq!(empty.next(), None);
    let empty_root = validate_profile_json_object(b" { } ", limits())
        .expect("valid empty root");
    assert_eq!(empty_root.root_fields().next(), None);
}

#[test]
fn maximum_object_members_can_be_streamed_without_field_name_policy() {
    let entries = (0..MAXIMUM_JSON_OBJECT_MEMBERS)
        .map(|index| format!(r#""key{index}":{index}"#))
        .collect::<Vec<_>>()
        .join(",");
    let document = format!("{{{entries}}}");
    let root = validate_profile_json_object(
        document.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: 8_192,
            maximum_nesting: 2,
            maximum_object_members: usize::MAX,
            maximum_values: usize::MAX,
        },
    )
    .expect("exact member ceiling supported");
    let mut count = 0usize;
    for (index, field) in root.root_fields().enumerate() {
        assert!(field.key_matches(&format!("key{index}")));
        assert_eq!(field.value().unsigned_integer(), Some(index as u64));
        count = count.saturating_add(1);
    }
    assert_eq!(count, MAXIMUM_JSON_OBJECT_MEMBERS);
}

#[test]
fn generated_object_fields_preserve_decoded_names_and_source_provenance() {
    let fields = (0..MAXIMUM_JSON_OBJECT_MEMBERS)
        .map(|index| {
            let key = if index % 2 == 0 {
                format!(r#""\u0061{index}""#)
            } else {
                format!("\"a{index}\"")
            };
            format!(r#"{key}:[{index},{{"n":{index}}}]"#)
        })
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("{{{fields}}}");
    let admitted = validate_profile_json_object(
        source.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: 16_384,
            maximum_nesting: 3,
            maximum_object_members: MAXIMUM_JSON_OBJECT_MEMBERS,
            maximum_values: 1_024,
        },
    )
    .expect("all 128 decoded names are unique");
    let mut visited = 0usize;
    for (index, field) in admitted.root_fields().enumerate() {
        let name = format!("a{index}");
        assert!(field.key_matches(&name));
        assert_eq!(field.decoded_key_chars().collect::<String>(), name);
        let selected = admitted.root_member_value(&name).expect("same value");
        assert_eq!(selected.bytes(), field.value().bytes());
        let child = field.value().array_element(1).expect("object in array");
        assert_eq!(
            child.object_member("n").expect("number").unsigned_integer(),
            Some(index as u64)
        );
        let raw = field.value().bytes();
        assert!(
            admitted
                .bytes()
                .windows(raw.len())
                .any(|part| std::ptr::eq(part.as_ptr(), raw.as_ptr()))
        );
        let key_raw = field.key_json();
        assert!(
            admitted
                .bytes()
                .windows(key_raw.len())
                .any(|part| std::ptr::eq(part.as_ptr(), key_raw.as_ptr()))
        );
        visited = visited.saturating_add(1);
    }
    assert_eq!(visited, MAXIMUM_JSON_OBJECT_MEMBERS);
}

#[test]
fn admitted_byte_mutations_never_project_values_outside_original_input() {
    let original = br#"{"items":[1,{"x":2},true],"s":"\u0061"}"#;
    let mut checked = 0usize;
    for position in 0..original.len() {
        for byte in [0u8, 9, 10, 34, 44, 58, 92, 123, 125, 255] {
            let mut changed = original.to_vec();
            changed[position] = byte;
            if let Ok(root) = validate_profile_json_object(&changed, limits()) {
                let mut elements = 0usize;
                for field in root.root_fields() {
                    let raw = field.value().bytes();
                    assert!(root.bytes().windows(raw.len()).any(|part| {
                        std::ptr::eq(part.as_ptr(), raw.as_ptr())
                    }));
                    if let Some(items) = field.value().array_elements() {
                        for element in items {
                            let raw = element.bytes();
                            assert!(root.bytes().windows(raw.len()).any(
                                |part| std::ptr::eq(
                                    part.as_ptr(),
                                    raw.as_ptr()
                                )
                            ));
                            elements = elements.saturating_add(1);
                        }
                    }
                }
                assert!(elements <= 3);
            }
            checked = checked.saturating_add(1);
        }
    }
    assert!(checked > 300);
}

#[test]
fn sealed_member_source_spans_include_colon_spacing_not_sibling_delimiters() {
    let source = concat!(
        " \n {  ",
        r#""\u0061" : [ 1 , 2 ] , "#,
        "\n\t",
        r#""plain": { "id" : null },"#,
        "   ",
        r#""empty" : "" } "#,
    );
    let admitted = validate_profile_json_object(source.as_bytes(), limits())
        .expect("root with mixed insignificant spacing is valid");
    let expected = [
        (r#""\u0061" : [ 1 , 2 ]"#, "a", b"[ 1 , 2 ]".as_slice()),
        (
            r#""plain": { "id" : null }"#,
            "plain",
            br#"{ "id" : null }"#.as_slice(),
        ),
        (r#""empty" : """#, "empty", br#""""#.as_slice()),
    ];
    for (field, (original, name, value)) in admitted.root_fields().zip(expected)
    {
        assert_eq!(field.source_member_json(), original.as_bytes());
        assert!(field.key_matches(name));
        assert_eq!(field.value().bytes(), value);
        let source_span = field.source_member_json();
        assert!(admitted.bytes().windows(source_span.len()).any(|bytes| {
            std::ptr::eq(bytes.as_ptr(), source_span.as_ptr())
        }));
        assert!(source_span.starts_with(field.key_json()));
        assert!(source_span.ends_with(value));
    }
    let inside = admitted.root_member_value("plain").expect("nested object");
    let child = inside
        .object_fields()
        .expect("nested fields")
        .next()
        .expect("nested member");
    assert_eq!(child.source_member_json(), br#""id" : null"#);
    assert!(
        inside
            .bytes()
            .windows(child.source_member_json().len())
            .any(|bytes| std::ptr::eq(
                bytes.as_ptr(),
                child.source_member_json().as_ptr()
            ))
    );
}

#[test]
fn sealed_member_source_spans_remain_distinct_for_aliasing_key_values() {
    let input = br#"{"\u0061" : "a", "a\u0062":"\u0061", "key": {"\u0061":1}}"#;
    let admitted = validate_profile_json_object(input, limits())
        .expect("all decoded keys at one scope are distinct");
    let fields = admitted.root_fields().collect::<Vec<_>>();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].source_member_json(), br#""\u0061" : "a""#);
    assert_eq!(fields[1].source_member_json(), br#""a\u0062":"\u0061""#);
    assert_eq!(fields[2].source_member_json(), br#""key": {"\u0061":1}"#);
    assert_eq!(fields[0].value().bytes(), br#""a""#);
    assert_eq!(fields[1].value().bytes(), br#""\u0061""#);
    let nested = fields[2]
        .value()
        .object_fields()
        .expect("object")
        .next()
        .expect("member");
    assert_eq!(nested.source_member_json(), br#""\u0061":1"#);
    assert!(nested.key_matches("a"));
}

#[test]
fn sealed_named_field_lookup_retains_member_bytes_at_every_scope() {
    let source = concat!(
        r#" { "\u0061" : { "path" : [{ "a" : 1 }, {"a":2}] },"#,
        r#" "path" : false, "ñ": 0, "ñ": 1 } "#,
    );
    let document = validate_profile_json_object(source.as_bytes(), limits())
        .expect("distinct Unicode field names and inner aliases");
    let field = document.root_field("a").expect("root escaped name");
    assert_eq!(field.key_json(), br#""\u0061""#);
    assert_eq!(
        field.source_member_json(),
        br#""\u0061" : { "path" : [{ "a" : 1 }, {"a":2}] }"#
    );
    let array = field
        .value()
        .object_field("path")
        .expect("nested field retaining punctuation");
    assert_eq!(
        array.source_member_json(),
        br#""path" : [{ "a" : 1 }, {"a":2}]"#
    );
    let items = array.value();
    assert_eq!(
        items
            .array_element(0)
            .expect("first object")
            .object_field("a")
            .expect("first field")
            .source_member_json(),
        br#""a" : 1"#
    );
    assert_eq!(
        items
            .array_element(1)
            .expect("second object")
            .object_field("a")
            .expect("second field")
            .source_member_json(),
        br#""a":2"#
    );
    assert_eq!(
        document
            .root_field("path")
            .expect("root path")
            .value()
            .boolean(),
        Some(false)
    );
    assert_eq!(
        document
            .root_field("ñ")
            .expect("composed")
            .value()
            .unsigned_integer(),
        Some(0)
    );
    assert_eq!(
        document
            .root_field("ñ")
            .expect("decomposed")
            .value()
            .unsigned_integer(),
        Some(1)
    );
    assert_eq!(document.root_field("PATH"), None);
    assert_eq!(field.value().object_field("PATH"), None);
    assert_eq!(array.value().object_field("a"), None);
    assert_eq!(
        document
            .root_member_value("a")
            .expect("compatibility")
            .bytes(),
        field.value().bytes()
    );
    assert_eq!(
        field
            .value()
            .object_member("path")
            .expect("legacy lookup")
            .bytes(),
        array.value().bytes()
    );
    let original = array.source_member_json();
    assert!(
        document
            .bytes()
            .windows(original.len())
            .any(|candidate| std::ptr::eq(
                candidate.as_ptr(),
                original.as_ptr()
            ))
    );
}

#[test]
fn sealed_named_lookup_fails_without_reinterpreting_other_value_kinds() {
    let doc = validate_profile_json_object(
        br#"{"object":{},"array":[],"string":"key","nil":null}"#,
        limits(),
    )
    .expect("valid root");
    for name in ["array", "string", "nil"] {
        let value = doc.root_field(name).expect("root field").value();
        assert_eq!(value.object_field("key"), None);
        assert_eq!(value.object_member("key"), None);
    }
    assert_eq!(
        doc.root_field("object")
            .expect("empty object")
            .value()
            .object_field("key"),
        None
    );
    assert_eq!(doc.root_field(""), None);
    assert_eq!(doc.root_field("Object"), None);
}

#[test]
fn all_high_surrogates_preserve_scalar_identity_at_low_code_unit_boundaries() {
    let mut checked = 0usize;
    for high in 0xd800u32..=0xdbffu32 {
        for low in [0xdc00u32, 0xdc01, 0xdc7f, 0xde00, 0xdffe, 0xdfff] {
            let scalar = 0x10000u32 + ((high - 0xd800) << 10) + (low - 0xdc00);
            let expected = char::from_u32(scalar).expect("admitted scalar");
            let input = format!(
                r#"{{"\u{high:04X}\u{low:04X}":"\u{high:04x}\u{low:04x}"}}"#,
            );
            let root = validate_profile_json_object(input.as_bytes(), limits())
                .expect("every valid surrogate pair is accepted");
            let field = root
                .root_field(&expected.to_string())
                .expect("escaped field decoded exactly");
            assert!(field.key_matches(&expected.to_string()));
            assert_eq!(
                field.decoded_key_chars().collect::<String>(),
                expected.to_string()
            );
            assert_eq!(
                field
                    .value()
                    .string_chars()
                    .expect("string")
                    .collect::<String>(),
                expected.to_string()
            );
            checked = checked.saturating_add(1);
        }
    }
    assert_eq!(checked, 6 * 1_024);
}

#[test]
fn all_high_surrogates_reject_non_low_followers_in_keys_and_values() {
    let mut checked = 0usize;
    for high in 0xd800u32..=0xdbffu32 {
        for invalid in [0x0000u32, 0x0041, 0xd7ff, 0xd800, 0xdbff, 0xe000] {
            let value = format!(r#"{{"k":"\u{high:04X}\u{invalid:04X}"}}"#);
            let key = format!(r#"{{"\u{high:04X}\u{invalid:04X}":0}}"#);
            for input in [value, key] {
                assert_eq!(
                    validate_profile_json_object(input.as_bytes(), limits()),
                    Err(ProfileJsonSyntaxError::Malformed),
                    "high={high:04X}, follower={invalid:04X}"
                );
                checked = checked.saturating_add(1);
            }
        }
    }
    assert_eq!(checked, 12 * 1_024);
}

#[test]
fn low_surrogates_alone_never_become_unicode_scalars() {
    for low in 0xdc00u32..=0xdfffu32 {
        let value = format!(r#"{{"value":"\u{low:04X}"}}"#);
        let key = format!(r#"{{"\u{low:04X}":0}}"#);
        for input in [value, key] {
            assert_eq!(
                validate_profile_json_object(input.as_bytes(), limits()),
                Err(ProfileJsonSyntaxError::Malformed),
                "isolated low surrogate {low:04X}"
            );
        }
    }
}

#[test]
fn sealed_nested_object_projection_reaches_exact_maximum_nesting() {
    let levels = MAXIMUM_JSON_NESTING;
    let source = format!("{}0{}", "{\"k\":".repeat(levels), "}".repeat(levels));
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: source.len(),
        maximum_nesting: levels,
        maximum_object_members: 1,
        maximum_values: levels + 1,
    };
    let root = validate_profile_json_object(source.as_bytes(), caps)
        .expect("root plus 63 nested objects fit the hard nesting cap");
    assert_eq!(root.value_count(), levels + 1);
    let mut current = root.root_field("k").expect("first member").value();
    for _ in 1..levels {
        current = current
            .object_field("k")
            .expect("inner object at a validated depth")
            .value();
    }
    assert_eq!(current.kind(), Some(ProfileJsonValueKind::Number));
    assert_eq!(current.unsigned_integer(), Some(0));
    assert_eq!(current.object_field("k"), None);
    assert_eq!(current.array_element(0), None);
    assert!(
        root.bytes()
            .windows(current.bytes().len())
            .any(|slice| std::ptr::eq(
                slice.as_ptr(),
                current.bytes().as_ptr()
            ))
    );

    let too_deep = format!(
        "{}0{}",
        "{\"k\":".repeat(levels + 1),
        "}".repeat(levels + 1)
    );
    let rejected = validate_profile_json_object(
        too_deep.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: too_deep.len(),
            maximum_nesting: usize::MAX,
            maximum_object_members: usize::MAX,
            maximum_values: usize::MAX,
        },
    );
    assert_eq!(
        rejected,
        Err(ProfileJsonSyntaxError::MaximumNestingExceeded)
    );
}

#[test]
fn sealed_nested_array_projection_reaches_exact_maximum_nesting() {
    let levels = MAXIMUM_JSON_NESTING - 1;
    let source = format!(
        "{{\"chain\":{}true{}}}",
        "[".repeat(levels),
        "]".repeat(levels)
    );
    let root = validate_profile_json_object(
        source.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: source.len(),
            maximum_nesting: MAXIMUM_JSON_NESTING,
            maximum_object_members: 1,
            maximum_values: levels + 2,
        },
    )
    .expect("root object and nested arrays meet hard ceiling");
    let mut current = root.root_member_value("chain").expect("array");
    for _ in 0..levels {
        assert_eq!(current.kind(), Some(ProfileJsonValueKind::Array));
        assert_eq!(
            current.array_elements().expect("array iterator").count(),
            1
        );
        assert_eq!(current.array_element(1), None);
        current = current.array_element(0).expect("nested one-item array");
    }
    assert_eq!(current.kind(), Some(ProfileJsonValueKind::Boolean));
    assert_eq!(current.boolean(), Some(true));
    assert_eq!(current.array_element(0), None);
    assert_eq!(current.bytes(), b"true");
}

#[test]
fn sealed_structural_path_traversal_retains_exact_nested_source() {
    use ProfileJsonPathSegment::{Index, Key};
    let json = concat!(
        r#" { "\u0072oot" : { "rows" : [ {"id": 0},"#,
        r#"{"id": ["\uD83D\uDE00", {"value":-1.50e+3}]}] } } "#,
    );
    let root = validate_profile_json_object(json.as_bytes(), limits())
        .expect("nested document syntax admitted");
    let path = [
        Key("root"),
        Key("rows"),
        Index(1),
        Key("id"),
        Index(1),
        Key("value"),
    ];
    let selected = root.value_at_path(&path).expect("escaped root path");
    assert_eq!(selected.kind(), Some(ProfileJsonValueKind::Number));
    assert_eq!(selected.bytes(), b"-1.50e+3");
    assert_eq!(selected.unsigned_integer(), None);
    assert_eq!(selected.at_path(&[]), Some(selected));
    assert_eq!(selected.at_path(&[Index(0)]), None);
    assert_eq!(
        root.value_at_path(&[Key("root")]),
        root.root_member_value("root")
    );
    assert_eq!(
        root.value_at_path(&[Key("root"), Key("rows")]),
        root.root_member_value("root")
            .expect("object")
            .object_member("rows")
    );
    let string_path = [Key("root"), Key("rows"), Index(1), Key("id"), Index(0)];
    let decoded = root
        .value_at_path(&string_path)
        .expect("string leaf")
        .string_chars()
        .expect("scalar stream")
        .collect::<String>();
    assert_eq!(decoded, "😀");
    let source_slice = selected.bytes();
    assert!(
        root.bytes()
            .windows(source_slice.len())
            .any(|span| std::ptr::eq(span.as_ptr(), source_slice.as_ptr()))
    );
}

#[test]
fn sealed_structural_path_never_coerces_name_into_index_or_reverse() {
    use ProfileJsonPathSegment::{Index, Key};
    let source = br#"{"0":[{"0":42}],"rows":[0,true,null,{}]}"#;
    let root = validate_profile_json_object(source, limits()).expect("valid");
    let valid = [Key("0"), Index(0), Key("0")];
    assert_eq!(
        root.value_at_path(&valid)
            .expect("exact path")
            .unsigned_integer(),
        Some(42)
    );
    let invalid: &[&[ProfileJsonPathSegment<'_>]] = &[
        &[],
        &[Index(0)],
        &[Key("missing")],
        &[Key("0"), Key("0")],
        &[Key("0"), Index(1)],
        &[Key("0"), Index(usize::MAX)],
        &[Key("0"), Index(0), Index(0)],
        &[Key("rows"), Index(1), Key("anything")],
        &[Key("rows"), Index(2), Index(0)],
        &[Key("rows"), Index(3), Key("unknown")],
        &[Key("rows"), Index(4)],
        &[Key("rows"), Index(usize::MAX)],
    ];
    for path in invalid {
        assert_eq!(root.value_at_path(path), None, "path={path:?}");
    }
    let object = root
        .root_member_value("rows")
        .expect("array")
        .array_element(3)
        .expect("object");
    assert_eq!(object.at_path(&[]), Some(object));
    assert_eq!(object.at_path(&[Index(0)]), None);
    assert_eq!(object.at_path(&[Key("missing")]), None);
}

#[test]
fn sealed_path_traverses_limit_depth_without_allocating_a_json_tree() {
    use ProfileJsonPathSegment::{Index, Key};
    let depth = MAXIMUM_JSON_NESTING - 1;
    let document =
        format!("{{\"rows\":{}0{}}}", "[".repeat(depth), "]".repeat(depth));
    let root = validate_profile_json_object(
        document.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: document.len(),
            maximum_nesting: MAXIMUM_JSON_NESTING,
            maximum_object_members: 1,
            maximum_values: depth + 2,
        },
    )
    .expect("hard nesting limit admitted");
    let mut path = Vec::with_capacity(depth + 1);
    path.push(Key("rows"));
    path.extend((0..depth).map(|_| Index(0)));
    assert_eq!(
        root.value_at_path(&path).expect("deep scalar").bytes(),
        b"0"
    );
    path.push(Index(0));
    assert_eq!(root.value_at_path(&path), None);
}

#[test]
fn bounded_json_string_copy_decodes_escapes_without_partial_writes() {
    let original = concat!(
        r#"{"mixed":"a\u0061\u0000\n\t\/\"\\"#,
        r#"\uD83D\uDE00ñ","empty":"","number":0}"#,
    );
    let admitted = validate_profile_json_object(original.as_bytes(), limits())
        .expect("source string escapes are all valid");
    let value = admitted.root_member_value("mixed").expect("JSON string");
    let expected = "aa\0\n\t/\"\\😀ñ";
    let mut destination = [0xabu8; 64];
    for capacity in [0usize, 1, expected.len() - 1] {
        let observed = value.copy_string_utf8(&mut destination[..capacity]);
        assert_eq!(
            observed,
            Err(ProfileJsonStringCopyError::BufferTooSmall {
                required: expected.len()
            })
        );
        assert_eq!(destination, [0xab; 64], "failure left buffer alone");
    }
    let written = value
        .copy_string_utf8(&mut destination[..expected.len()])
        .expect("exact buffer size admits all Unicode scalars");
    assert_eq!(written, expected.len());
    assert_eq!(&destination[..written], expected.as_bytes());
    assert_eq!(&destination[written..], &[0xab; 64][written..]);
    assert!(std::str::from_utf8(&destination[..written]).is_ok());
    assert!(original.as_bytes().windows(value.bytes().len()).any(
        |candidate| std::ptr::eq(candidate.as_ptr(), value.bytes().as_ptr())
    ));

    let mut zero = [];
    let empty = admitted.root_member_value("empty").expect("empty string");
    assert_eq!(empty.copy_string_utf8(&mut zero), Ok(0));
    let mut wrong_kind = [0xabu8; 8];
    let number = admitted.root_member_value("number").expect("number");
    assert_eq!(
        number.copy_string_utf8(&mut wrong_kind),
        Err(ProfileJsonStringCopyError::NotString)
    );
    assert_eq!(wrong_kind, [0xab; 8]);
}

#[test]
fn bounded_string_copy_handles_unicode_encoding_thresholds_atomically() {
    for code in [
        0x0u32, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xe000, 0xffff, 0x10000,
        0x1f600, 0x10ffff,
    ] {
        let scalar = char::from_u32(code).expect("scalar is not a surrogate");
        let bytes = scalar.to_string();
        let source = format!(
            "{{\"field\":{}}}",
            if code <= 0xffff {
                format!(r#""\u{code:04x}""#)
            } else {
                let upper = 0xd800 + ((code - 0x10000) >> 10);
                let lower = 0xdc00 + ((code - 0x10000) & 0x03ff);
                format!(r#""\u{upper:04X}\u{lower:04X}""#)
            }
        );
        let admitted =
            validate_profile_json_object(source.as_bytes(), limits())
                .expect("escaped Unicode scalar is a valid string");
        let value = admitted.root_member_value("field").expect("string");
        let mut insufficient = [0x5au8; 4];
        let len = bytes.len();
        assert_eq!(
            value.copy_string_utf8(&mut insufficient[..len - 1]),
            Err(ProfileJsonStringCopyError::BufferTooSmall { required: len })
        );
        assert_eq!(insufficient, [0x5a; 4]);
        assert_eq!(value.copy_string_utf8(&mut insufficient[..len]), Ok(len));
        assert_eq!(&insufficient[..len], bytes.as_bytes());
        assert_eq!(&insufficient[len..], &[0x5a; 4][len..]);
    }
}

#[test]
fn bounded_string_copy_preserves_non_normalized_unicode_sequences() {
    let source = r#"{"decomposed":"ñ","composed":"\u00f1"}"#;
    let admitted = validate_profile_json_object(source.as_bytes(), limits())
        .expect("both names are valid and distinct");
    let mut decomposed = [0xff; 8];
    let mut composed = [0xff; 8];
    let first = admitted
        .root_member_value("decomposed")
        .expect("string")
        .copy_string_utf8(&mut decomposed)
        .expect("bounded copy");
    let second = admitted
        .root_member_value("composed")
        .expect("string")
        .copy_string_utf8(&mut composed)
        .expect("bounded copy");
    assert_eq!(&decomposed[..first], "ñ".as_bytes());
    assert_eq!(&composed[..second], "ñ".as_bytes());
    assert_ne!(&decomposed[..first], &composed[..second]);
}

#[test]
fn decoded_object_name_copy_preserves_exact_unicode_and_original_tokens() {
    let source = concat!(
        r#" {"\u0061":0, "ñ":1, "ñ":2, "#,
        r#""\uD83D\uDE00":3, "":4} "#,
    );
    let admitted = validate_profile_json_object(source.as_bytes(), limits())
        .expect("all keys have distinct exact Unicode scalar identities");
    for (name, original, value) in [
        ("a", br#""\u0061""#.as_slice(), 0),
        ("ñ", "\"ñ\"".as_bytes(), 1),
        ("ñ", "\"ñ\"".as_bytes(), 2),
        ("😀", br#""\uD83D\uDE00""#.as_slice(), 3),
        ("", br#""""#.as_slice(), 4),
    ] {
        let field = admitted.root_field(name).expect("decoded key");
        assert_eq!(field.key_json(), original);
        assert_eq!(field.value().unsigned_integer(), Some(value));
        let mut output = [0xb5u8; 16];
        let capacity = name.len();
        if capacity > 0 {
            assert_eq!(
                field.copy_key_utf8(&mut output[..capacity - 1]),
                Err(ProfileJsonStringCopyError::BufferTooSmall {
                    required: capacity,
                })
            );
            assert_eq!(output, [0xb5; 16]);
        }
        assert_eq!(field.copy_key_utf8(&mut output[..capacity]), Ok(capacity));
        assert_eq!(&output[..capacity], name.as_bytes());
        assert_eq!(&output[capacity..], &[0xb5; 16][capacity..]);
        assert!(field.key_matches(name));
        assert!(admitted.bytes().windows(field.key_json().len()).any(
            |segment| {
                std::ptr::eq(segment.as_ptr(), field.key_json().as_ptr())
            }
        ));
    }
    assert_ne!(
        admitted.root_field("ñ").expect("composed").key_json(),
        admitted.root_field("ñ").expect("decomposed").key_json()
    );
}

#[test]
fn decoded_field_names_and_values_share_atomic_utf8_copy_behavior() {
    let root = validate_profile_json_object(
        br#"{"\u0061":"\u0061","\u0000":"\u0000"}"#,
        limits(),
    )
    .expect("distinct escaped names and values");
    for (name, expected) in [("a", b"a".as_slice()), ("\0", b"\0".as_slice())] {
        let field = root.root_field(name).expect("key");
        let mut key_out = [0xdd; 2];
        let mut value_out = [0xdd; 2];
        assert_eq!(
            field.copy_key_utf8(&mut key_out[..0]),
            Err(ProfileJsonStringCopyError::BufferTooSmall { required: 1 })
        );
        assert_eq!(
            field.value().copy_string_utf8(&mut value_out[..0]),
            Err(ProfileJsonStringCopyError::BufferTooSmall { required: 1 })
        );
        assert_eq!(key_out, [0xdd; 2]);
        assert_eq!(value_out, [0xdd; 2]);
        assert_eq!(field.copy_key_utf8(&mut key_out), Ok(1));
        assert_eq!(field.value().copy_string_utf8(&mut value_out), Ok(1));
        assert_eq!(&key_out[..1], expected);
        assert_eq!(&value_out[..1], expected);
        assert_eq!(key_out[1], 0xdd);
        assert_eq!(value_out[1], 0xdd);
    }
}

#[test]
fn max_member_keys_copy_into_bounded_buffers_without_heap_ownership() {
    let entries = (0..MAXIMUM_JSON_OBJECT_MEMBERS)
        .map(|index| format!(r#""\u006b{index}":{index}"#))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("{{{entries}}}");
    let admitted = validate_profile_json_object(
        source.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: source.len(),
            maximum_nesting: 2,
            maximum_object_members: MAXIMUM_JSON_OBJECT_MEMBERS,
            maximum_values: MAXIMUM_JSON_OBJECT_MEMBERS + 1,
        },
    )
    .expect("at fixed key ceiling");
    for (index, field) in admitted.root_fields().enumerate() {
        let expected = format!("k{index}");
        let mut output = [0xab; 16];
        let written = field.copy_key_utf8(&mut output).expect("bounded name");
        assert_eq!(&output[..written], expected.as_bytes());
        assert_eq!(&output[written..], &[0xab; 16][written..]);
        assert_eq!(field.value().unsigned_integer(), Some(index as u64));
    }
}

#[test]
fn generated_unicode_name_and_value_copies_agree_on_scalar_bytes() {
    let mut exercised = 0usize;
    for index in 0..5_120u32 {
        // Coprime stride crosses BMP and supplementary planes without
        // allocating a corpus or ever treating surrogate code units as chars.
        let scalar_value = index.wrapping_mul(271) % 0x110000;
        let Some(scalar) = char::from_u32(scalar_value) else {
            continue;
        };
        let name = scalar.to_string();
        let escaped = if scalar_value <= 0xffff {
            format!("\\u{scalar_value:04X}")
        } else {
            let value = scalar_value - 0x10000;
            let high = 0xd800 + (value >> 10);
            let low = 0xdc00 + (value & 0x3ff);
            format!("\\u{high:04x}\\u{low:04X}")
        };
        let source = format!("{{\"{escaped}\":\"{escaped}\"}}");
        let admitted =
            validate_profile_json_object(source.as_bytes(), limits())
                .expect("valid scalar escapes in key and value");
        let field = admitted
            .root_field(&name)
            .expect("exact scalar lookup matches escaped key");
        assert_eq!(field.decoded_key_chars().collect::<String>(), name);
        let mut key = [0xddu8; 7];
        let mut value = [0xddu8; 7];
        let size = name.len();
        assert_eq!(
            field.copy_key_utf8(&mut key[..size - 1]),
            Err(ProfileJsonStringCopyError::BufferTooSmall { required: size })
        );
        assert_eq!(
            field.value().copy_string_utf8(&mut value[..size - 1]),
            Err(ProfileJsonStringCopyError::BufferTooSmall { required: size })
        );
        assert_eq!(key, [0xdd; 7]);
        assert_eq!(value, [0xdd; 7]);
        assert_eq!(field.copy_key_utf8(&mut key[..size]), Ok(size));
        assert_eq!(
            field.value().copy_string_utf8(&mut value[..size]),
            Ok(size)
        );
        assert_eq!(&key[..size], name.as_bytes());
        assert_eq!(&value[..size], name.as_bytes());
        assert_eq!(&key[size..], &[0xdd; 7][size..]);
        assert_eq!(&value[size..], &[0xdd; 7][size..]);
        exercised = exercised.saturating_add(1);
    }
    assert!(exercised > 4_800, "crosses all relevant UTF-8 widths");
}

#[test]
fn admitted_json_debug_redacts_source_keys_values_and_escapes() {
    let source = concat!(
        r#"{"private-key-7391":{"secret":"value-8127", "#,
        r#""items":["\u0070rivate-4631",true]}}"#,
    )
    .as_bytes();
    let root = validate_profile_json_object(source, limits())
        .expect("source contains a valid nested object");
    let object_fields = root.root_fields();
    let field = root.root_field("private-key-7391").expect("private key");
    let value = field.value();
    let nested_fields = value.object_fields().expect("nested object");
    let string = value.object_member("secret").expect("private value");
    let chars = string.string_chars().expect("string scalar iterator");
    let array = value.object_member("items").expect("private array");
    let elements = array.array_elements().expect("array iterator");
    let array_value = array.array_element(0).expect("first array item");

    for printed in [
        format!("{root:?} {root:#?}"),
        format!("{object_fields:?} {object_fields:#?}"),
        format!("{field:?} {field:#?}"),
        format!("{value:?} {value:#?}"),
        format!("{nested_fields:?} {nested_fields:#?}"),
        format!("{string:?} {string:#?}"),
        format!("{chars:?} {chars:#?}"),
        format!("{array:?} {array:#?}"),
        format!("{elements:?} {elements:#?}"),
        format!("{array_value:?} {array_value:#?}"),
    ] {
        for private in [
            "private-key-7391",
            "value-8127",
            "private-4631",
            "0070rivate-4631",
            "secret",
            "items",
        ] {
            assert!(
                !printed.contains(private),
                "leaked {private} in {printed}"
            );
        }
    }
    // Redacting incidental formatting must not change the explicit source
    // accessors or decoded Unicode scalar iteration.
    assert_eq!(root.bytes(), source);
    assert_eq!(field.value().bytes(), value.bytes());
    assert_eq!(
        string.string_chars().unwrap().collect::<String>(),
        "value-8127"
    );
    assert_eq!(
        array_value.string_chars().unwrap().collect::<String>(),
        "private-4631"
    );
}

#[test]
fn json_selection_segment_debug_omits_private_member_key() {
    let private = "private-member-key-6251";
    let key = ProfileJsonPathSegment::Key(private);
    for displayed in [format!("{key:?}"), format!("{key:#?}")] {
        assert!(displayed.contains("Key"));
        assert!(!displayed.contains(private));
    }
    assert!(matches!(
        key,
        ProfileJsonPathSegment::Key(original) if original == private
    ));
    let index = ProfileJsonPathSegment::Index(37);
    assert_eq!(format!("{index:?}"), "Index(37)");
    assert_eq!(index, ProfileJsonPathSegment::Index(37));
}
