// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Verify native CLI and browser copy the same explicitly authored pen paths.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { candidatePenPaths, copyCandidatePath, copyShiftedCandidatePath,
    encodeGlyphLibrary, styleCandidatePath, removeCandidatePath,
    scaleCandidatePath, reflectCandidatePath, rotateCandidatePath }
// jig-ignore-next-line: static ESM import needs a literal path.
    from '../../../../src/browser/workspace/adapter-inbound/generated/glyph-strokes.js';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const cargo = join(root, '.dependencies/rust/1.99.1/bin/cargo');
const binary = join(root, '.cache/cargo-target/debug',
    process.platform === 'win32'
        ? 'atrament-profiles.exe' : 'atrament-profiles');

function random(seed) {
    let state = seed >>> 0;
    return (minimum, maximum) => {
        state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
        return minimum + state % (maximum - minimum + 1);
    };
}

function stroke(pick, grapheme, color) {
    const x1 = pick(500, 2100);
    const y1 = pick(500, 2500);
    return { grapheme, x1, y1,
        x2: x1 + pick(25, 400), y2: y1 + pick(25, 400),
        width: 250, color };
}

function strokeFlags(line) {
    return ['--glyph-stroke', line.grapheme,
        String(line.x1), String(line.y1),
        String(line.x2), String(line.y2),
        String(line.width), line.color];
}

// Deterministically vary Unicode normalization, interleaving, path width,
// pen continuity, selected path, existing-target ink and typed refusals.
// Compare real native stdout and the production browser's full TSV output,
// not a handwritten replacement path-selection implementation.
test('256 native and browser path copies agree on exact TSV and refusals',
    { timeout: 90_000 }, () => {
        const built = spawnSync(cargo, [
            'build', '--locked', '--offline', '--quiet',
            '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
        ], { cwd: root, encoding: 'utf8', timeout: 90_000,
            env: { ...process.env, TMPDIR: join(root, '.temp') } });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);

        const pick = random(0xC0F1_2026);
        let accepted = 0;
        let refused = 0;
        for (let sample = 0; sample < 256; sample += 1) {
            const sourceKey = sample % 2 === 0 ? 'ñ' : 'n\u0303';
            const other = sourceKey === 'ñ' ? 'n\u0303' : 'ñ';
            const targetKey = sample % 3 === 0 ? other : 'á';
            const mode = sample % 8;
            const first = stroke(pick, sourceKey, '#123456');
            const second = stroke(pick, sourceKey, '#123456');
            if (sample % 3 !== 0) {
                second.x1 = first.x2;
                second.y1 = first.y2;
            }
            if (sample % 5 === 0) second.width = 275;
            const third = stroke(pick, sourceKey, '#567890');
            const source = [first, stroke(pick, other, '#AABBCC'),
                second, stroke(pick, '∞', '#667788'), third];
            if (sample % 4 === 0) {
                source.push(stroke(pick, targetKey, '#778899'));
            }
            const chosenIndex = mode % 3 === 0 ? 0
                : mode % 3 === 1 ? 2 : 4;
            const chosenOrdinal = [0, 2, 4].indexOf(chosenIndex) + 1;
            const path = candidatePenPaths(source, sourceKey).findIndex(
                (item) => chosenOrdinal >= item.first
                    && chosenOrdinal <= item.last);
            let browserIndex = chosenIndex;
            let nativeIndex = path + 1;
            let target = targetKey;
            if (mode === 4) {
                browserIndex = -1;
                nativeIndex = 0;
            } else if (mode === 5) {
                browserIndex = 9999;
                nativeIndex = 4097;
            } else if (mode === 6) {
                target = sourceKey;
            } else if (mode === 7) {
                target = 'ab';
            }
            const original = encodeGlyphLibrary(source);
            let expected;
            try {
                expected = encodeGlyphLibrary(copyCandidatePath(
                    source, sourceKey, browserIndex, target));
            } catch (error) {
                expected = error;
            }
            const actual = spawnSync(binary, [
                'glyphs', ...source.flatMap(strokeFlags),
                '--copy-path', sourceKey, String(nativeIndex), target,
            ], { cwd: root, encoding: 'utf8', timeout: 2_000,
                maxBuffer: 512 * 1024 });
            const context = `copy sample=${sample} mode=${mode} ${sourceKey}`;
            assert.equal(actual.error, undefined, context);
            if (expected instanceof Error) {
                refused += 1;
                assert.notEqual(actual.status, 0, context);
                assert.equal(actual.stdout, '', context);
                assert.match(actual.stderr, /^Error: /, context);
            } else {
                accepted += 1;
                assert.equal(actual.status, 0,
                    context + ': ' + actual.stderr);
                assert.equal(actual.stdout, expected, context);
                assert.ok(expected.startsWith(original), context);
            }
            assert.equal(encodeGlyphLibrary(source), original,
                `sample ${sample} mutated original input`);
        }
        assert.ok(accepted > 80, `expected accepted copies: ${accepted}`);
        assert.ok(refused > 80, `expected safe refusals: ${refused}`);
    });

// Compare full production TSV for accepted and refused shifted copies,
// including exact NFC/NFD source/target identity and self-copy.
test("256 native and browser shifted path copies agree on full ink",
    { timeout: 90_000 }, () => {
        const built = spawnSync(cargo, [
            "build", "--locked", "--offline", "--quiet",
            "-p", "atrament_profile_editor_cli", "--bin", "atrament-profiles",
        ], { cwd: root, encoding: "utf8", timeout: 90_000,
            env: { ...process.env, TMPDIR: join(root, ".temp") } });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);
        const pick = random(0xC0F1_5517);
        let accepted = 0;
        let refused = 0;
        for (let sample = 0; sample < 256; sample += 1) {
            const sourceKey = sample % 2 === 0 ? "ñ" : "n\u0303";
            const other = sourceKey === "ñ" ? "n\u0303" : "ñ";
            const targetKey = sample % 4 === 0 ? sourceKey
                : sample % 3 === 0 ? other : "á";
            const first = stroke(pick, sourceKey, "#123456");
            const second = { ...first, x1: first.x2, y1: first.y2,
                x2: first.x2 + pick(5, 100),
                y2: first.y2 + pick(5, 100) };
            const detached = stroke(pick, sourceKey, "#556677");
            const rows = [first, stroke(pick, other, "#AABBCC"),
                second, detached, stroke(pick, "∞", "#667788")];
            const chosenIndex = sample % 2 === 0 ? 0 : 3;
            const ordinal = [0, 2, 3].indexOf(chosenIndex) + 1;
            const path = candidatePenPaths(rows, sourceKey).findIndex(
                ({ first: from, last: to }) =>
                    ordinal >= from && ordinal <= to) + 1;
            const mode = sample % 16;
            let dx = mode === 0 ? 0 : mode === 1 ? 3500
                : mode === 2 ? -3500 : pick(-250, 250);
            let dy = mode === 0 ? 0 : mode === 3 ? 4500
                : mode === 4 ? -4500 : pick(-250, 250);
            let target = targetKey;
            let browserIndex = chosenIndex;
            let nativeIndex = path;
            if (mode === 5) {
                target = "ab";
            } else if (mode === 6) {
                dx = 3501;
            } else if (mode === 7) {
                browserIndex = -1;
                nativeIndex = 0;
            } else if (mode === 8) {
                dy = -4501;
            } else if (mode === 9) {
                dx = 123;
                dy = 34;
            }
            const original = encodeGlyphLibrary(rows);
            let expected;
            try {
                expected = encodeGlyphLibrary(copyShiftedCandidatePath(
                    rows, sourceKey, browserIndex, target, dx, dy));
            } catch (error) {
                expected = error;
            }
            const actual = spawnSync(binary, [
                "glyphs", ...rows.flatMap(strokeFlags),
                "--copy-path-shifted", sourceKey, String(nativeIndex),
                target, String(dx), String(dy),
            ], { cwd: root, encoding: "utf8", timeout: 2_000,
                maxBuffer: 512 * 1024 });
            const context = `shifted sample ${sample} mode ${mode}`;
            assert.equal(actual.error, undefined, context);
            if (expected instanceof Error) {
                refused += 1;
                assert.notEqual(actual.status, 0, context);
                assert.equal(actual.stdout, "", context);
                assert.match(actual.stderr, /^Error: /, context);
            } else {
                accepted += 1;
                assert.equal(actual.status, 0,
                    context + ": " + actual.stderr);
                assert.equal(actual.stdout, expected, context);
                assert.ok(expected.startsWith(original), context);
            }
            assert.equal(encodeGlyphLibrary(rows), original,
                `${context} must not mutate source`);
        }
        assert.ok(accepted > 40, `accepted samples: ${accepted}`);
        assert.ok(refused > 80, `refused samples: ${refused}`);
    });

// Rust's --style-path and browser restyling agree at the byte boundary.
// Covers interleaved Unicode, multi-stroke paths, odd pen widths and refusal.
test("256 Rust/browser whole-path pen styles match exact TSV or refusal",
    { timeout: 90_000 }, () => {
        const built = spawnSync(cargo, [
            "build", "--locked", "--offline", "--quiet",
            "-p", "atrament_profile_editor_cli", "--bin", "atrament-profiles",
        ], { cwd: root, encoding: "utf8", timeout: 90_000,
            env: { ...process.env, TMPDIR: join(root, ".temp") } });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);
        const pick = random(0x51A7_2026);
        let accepted = 0;
        let refused = 0;
        for (let sample = 0; sample < 256; sample += 1) {
            const glyph = sample % 2 === 0 ? "ñ" : "n\u0303";
            const other = glyph === "ñ" ? "n\u0303" : "ñ";
            const first = stroke(pick, glyph, "#123456");
            const next = { ...first, x1: first.x2, y1: first.y2,
                x2: first.x2 + pick(5, 100),
                y2: first.y2 + pick(5, 100) };
            if (sample % 5 === 0) next.color = "#445566";
            const detached = stroke(pick, glyph, "#778899");
            const rows = [first, stroke(pick, other, "#ABCD01"),
                next, detached, stroke(pick, "∞", "#556677")];
            const chosen = sample % 2 === 0 ? 0 : 3;
            const ordinal = [0, 2, 3].indexOf(chosen) + 1;
            const path = candidatePenPaths(rows, glyph).findIndex(
                ({ first: from, last: to }) =>
                    ordinal >= from && ordinal <= to) + 1;
            const mode = sample % 16;
            let width = mode === 0 ? 250 : mode === 1 ? 80
                : mode === 2 ? 1500 : mode === 3 ? 251
                    : mode === 4 ? 1501 : mode === 5 ? 79
                        : pick(80, 800);
            let color = mode === 6 ? "123456"
                : mode === 7 ? "#GGGGGG"
                    : mode === 8 ? "#123456" : "#aBcDef";
            let source = glyph;
            let browserIndex = chosen;
            let nativeIndex = path;
            if (mode === 9) {
                browserIndex = -1;
                nativeIndex = 0;
            } else if (mode === 10) {
                browserIndex = 9999;
                nativeIndex = 4097;
            } else if (mode === 11) {
                source = "ab";
            } else if (mode === 12) {
                browserIndex = 0;
                nativeIndex = 1;
                width = 250;
                color = "#123456";
            } else if (mode === 13) {
                rows[0] = { ...rows[0], x1: 125 };
                browserIndex = 0;
                nativeIndex = 1;
                width = 251;
                color = "#aBcDef";
            }
            const original = encodeGlyphLibrary(rows);
            let expected;
            try {
                expected = encodeGlyphLibrary(styleCandidatePath(
                    rows, source, browserIndex, width, color).strokes);
            } catch (error) {
                expected = error;
            }
            const actual = spawnSync(binary, [
                "glyphs", ...rows.flatMap(strokeFlags),
                "--style-path", source, String(nativeIndex),
                String(width), color,
            ], { cwd: root, encoding: "utf8", timeout: 2_000,
                maxBuffer: 512 * 1024 });
            const context = "path style sample=" + sample + " mode=" + mode;
            assert.equal(actual.error, undefined, context);
            if (expected instanceof Error) {
                refused += 1;
                assert.notEqual(actual.status, 0, context);
                assert.equal(actual.stdout, "", context);
                assert.match(actual.stderr, /^Error: /, context);
            } else {
                accepted += 1;
                assert.equal(actual.status, 0,
                    context + ": " + actual.stderr);
                assert.equal(actual.stdout, expected, context);
            }
            assert.equal(encodeGlyphLibrary(rows), original,
                context + ": original source must stay untouched");
        }
        assert.ok(accepted > 50, "accepted styles: " + accepted);
        assert.ok(refused > 70, "refused styles: " + refused);
    });

// CLI and browser remove the exact same authored path, not a whole glyph.
test("128 native/browser path removals agree on TSV and safe refusal",
    { timeout: 90_000 }, () => {
        const built = spawnSync(cargo, [
            "build", "--locked", "--offline", "--quiet",
            "-p", "atrament_profile_editor_cli", "--bin", "atrament-profiles",
        ], { cwd: root, encoding: "utf8", timeout: 90_000,
            env: { ...process.env, TMPDIR: join(root, ".temp") } });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);
        const pick = random(0xD311_2026);
        let accepted = 0;
        let refused = 0;
        for (let sample = 0; sample < 128; sample += 1) {
            const glyph = sample % 2 ? "ñ" : "n\u0303";
            const other = glyph === "ñ" ? "n\u0303" : "ñ";
            const first = stroke(pick, glyph, "#123456");
            const second = { ...first, x1: first.x2, y1: first.y2,
                x2: first.x2 + pick(5, 100),
                y2: first.y2 + pick(5, 100) };
            const last = stroke(pick, glyph, "#556677");
            const rows = sample % 7 === 0 ? [first, second]
                : [first, stroke(pick, other, "#ABCDEF"),
                    second, stroke(pick, "∞", "#998877"), last];
            const chosen = sample % 3 === 0 ? 0
                : rows.length === 2 ? 1 : 4;
            const ordinal = rows.flatMap((line, index) =>
                line.grapheme === glyph ? [index] : []).indexOf(chosen) + 1;
            const path = candidatePenPaths(rows, glyph).findIndex(
                ({ first: from, last: to }) =>
                    ordinal >= from && ordinal <= to) + 1;
            const mode = sample % 13;
            const index = mode === 1 ? -1 : mode === 2 ? 9999 : chosen;
            const nativeIndex = mode === 1 ? 0
                : mode === 2 ? 4097 : path;
            const requested = mode === 3 ? "ab" : glyph;
            const original = encodeGlyphLibrary(rows);
            let expected;
            try {
                expected = encodeGlyphLibrary(removeCandidatePath(
                    rows, requested, index));
            } catch (error) {
                expected = error;
            }
            const actual = spawnSync(binary, [
                "glyphs", ...rows.flatMap(strokeFlags),
                "--remove-path", requested, String(nativeIndex),
            ], { cwd: root, encoding: "utf8", timeout: 2_000,
                maxBuffer: 512 * 1024 });
            const context = "path remove sample " + sample;
            assert.equal(actual.error, undefined, context);
            if (expected instanceof Error) {
                refused += 1;
                assert.notEqual(actual.status, 0, context);
                assert.equal(actual.stdout, "", context);
                assert.match(actual.stderr, /^Error: /, context);
            } else {
                accepted += 1;
                assert.equal(actual.status, 0,
                    context + ": " + actual.stderr);
                assert.equal(actual.stdout, expected, context);
            }
            assert.equal(encodeGlyphLibrary(rows), original,
                context + ": original library untouched");
        }
        assert.ok(accepted > 60, "accepted " + accepted);
        assert.ok(refused > 30, "refused " + refused);
    });

// Exercise the actual browser rounding and Rust CLI path-number lookup with
// interleaved NFC/NFD ink, directed joints and both safe/unsafe transforms.
test('256 real Rust and browser selected-path scales agree byte for byte',
    { timeout: 90_000 }, () => {
        const built = spawnSync(cargo, [
            'build', '--locked', '--offline', '--quiet',
            '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
        ], { cwd: root, encoding: 'utf8', timeout: 90_000,
            env: { ...process.env, TMPDIR: join(root, '.temp') } });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);
        const pick = random(0x5CA1_2026);
        let accepted = 0;
        let refused = 0;
        for (let sample = 0; sample < 256; sample += 1) {
            const glyph = sample % 2 === 0 ? 'ñ' : 'n\u0303';
            const other = glyph === 'ñ' ? 'n\u0303' : 'ñ';
            const first = stroke(pick, glyph, '#123456');
            const second = { ...first, x1: first.x2, y1: first.y2,
                x2: first.x2 + pick(1, 220),
                y2: first.y2 + pick(1, 220) };
            const detached = stroke(pick, glyph, '#556677');
            const mode = sample % 16;
            let xPercent = [125, 50, 75, 150][mode % 4];
            let yPercent = [75, 150, 125, 50][mode % 4];
            let selected = mode % 3 === 0 ? 4 : 2;
            let nativeIndex = selected === 4 ? 2 : 1;
            let key = glyph;
            if (mode === 4) {
                xPercent = 100;
                yPercent = 100;
            } else if (mode === 5) {
                xPercent = 49;
            } else if (mode === 6) {
                yPercent = 151;
            } else if (mode === 7) {
                selected = -1;
                nativeIndex = 0;
            } else if (mode === 8) {
                selected = 9999;
                nativeIndex = 4096;
            } else if (mode === 9) {
                key = 'ab';
            } else if (mode === 10) {
                xPercent = 125.5;
            } else if (mode === 11 || mode === 15) {
                selected = 0;
                nativeIndex = 1;
                first.x1 = 500;
                first.y1 = 700;
                first.x2 = 2800;
                first.y2 = 900;
                second.x1 = first.x2;
                second.y1 = first.y2;
                second.x2 = 3100;
                second.y2 = 1100;
                xPercent = 125;
                yPercent = 100;
            } else if (mode === 13) {
                selected = 0;
                nativeIndex = 1;
                first.x1 = 1500;
                first.y1 = 1500;
                first.x2 = 1501;
                first.y2 = 1500;
                second.x1 = 1501;
                second.y1 = 1500;
                second.x2 = 1502;
                second.y2 = 1500;
                xPercent = 101;
                yPercent = 100;
            } else if (mode === 14) {
                selected = 0;
                nativeIndex = 1;
                first.x1 = 1500;
                first.y1 = 1500;
                first.x2 = 1499;
                first.y2 = 1499;
                second.x1 = 1499;
                second.y1 = 1499;
                second.x2 = 1498;
                second.y2 = 1498;
                xPercent = 50;
                yPercent = 125;
            }
            const source = [first, stroke(pick, other, '#AABBCC'),
                second, stroke(pick, '∞', '#667788'), detached];
            const baseline = encodeGlyphLibrary(source);
            let expected;
            try {
                expected = encodeGlyphLibrary(scaleCandidatePath(
                    source, key, selected, xPercent, yPercent).strokes);
            } catch (error) {
                expected = error;
            }
            const actual = spawnSync(binary, [
                'glyphs', ...source.flatMap(strokeFlags),
                '--scale-path', key, String(nativeIndex),
                String(xPercent), String(yPercent),
            ], { cwd: root, encoding: 'utf8', timeout: 2_000,
                maxBuffer: 512 * 1024 });
            const context = `scaled path sample=${sample} mode=${mode}`;
            assert.equal(actual.error, undefined, context);
            if (expected instanceof Error) {
                refused += 1;
                assert.notEqual(actual.status, 0, context);
                assert.equal(actual.stdout, '', context);
                assert.match(actual.stderr, /^Error: /, context);
            } else {
                accepted += 1;
                assert.equal(actual.status, 0, context + actual.stderr);
                assert.equal(actual.stdout, expected, context);
            }
            assert.equal(encodeGlyphLibrary(source), baseline, context);
        }
        assert.ok(accepted > 75, `accepted scales: ${accepted}`);
        assert.ok(refused > 70, `refused scales: ${refused}`);
    });

// Exercise the production browser transform against actual native output.
// Reversal is an exact involution; no shape, Unicode or row normalization.
test('256 real Rust and browser selected-path reflections agree on TSV',
    { timeout: 90_000 }, () => {
        const built = spawnSync(cargo, [
            'build', '--locked', '--offline', '--quiet',
            '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
        ], { cwd: root, encoding: 'utf8', timeout: 90_000,
            env: { ...process.env, TMPDIR: join(root, '.temp') } });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);
        const pick = random(0xC0F1_78A7);
        let accepted = 0;
        let refused = 0;
        for (let sample = 0; sample < 256; sample += 1) {
            const key = sample % 2 === 0 ? 'ñ' : 'n\u0303';
            const other = key === 'ñ' ? 'n\u0303' : 'ñ';
            const mode = sample % 16;
            const first = stroke(pick, key, '#123456');
            const second = {
                ...first, x1: first.x2, y1: first.y2,
                x2: first.x2 + pick(1, 200),
                y2: first.y2 + pick(1, 200),
            };
            const detached = stroke(pick, key, '#667788');
            const source = [first, stroke(pick, other, '#AABBCC'),
                second, stroke(pick, '∞', '#CCDDEE'), detached];
            let selected = mode % 3 === 0 ? 4 : mode % 3 === 1 ? 2 : 0;
            const pathOrdinal = candidatePenPaths(source, key).findIndex(
                ({ first: from, last: to }) => {
                    const ordinal = [0, 2, 4].indexOf(selected) + 1;
                    return ordinal >= from && ordinal <= to;
                }) + 1;
            let nativeIndex = pathOrdinal;
            let axis = mode % 2 === 0 ? 'x' : 'y';
            let pivot = axis === 'x' ? 1750 : 2250;
            if (mode === 2) {
                // Clipping near the physical edge must reject atomically.
                pivot = 0;
            } else if (mode === 3) {
                pivot = 4501;
            } else if (mode === 4) {
                axis = 'z';
            } else if (mode === 5) {
                selected = -1;
                nativeIndex = 0;
            } else if (mode === 6) {
                selected = 9999;
                nativeIndex = 4097;
            } else if (mode === 7) {
                pivot = 3501;
                axis = 'x';
            } else if (mode === 8) {
                pivot = -1;
            } else if (mode === 9) {
                pivot = 1.5;
            } else if (mode === 10) {
                first.x1 = 1750;
                first.x2 = 1750;
                first.y1 = 700;
                first.y2 = 900;
                second.x1 = 1750;
                second.x2 = 1750;
                second.y1 = 900;
                second.y2 = 1200;
                selected = 0;
                nativeIndex = 1;
                axis = 'x';
                pivot = 1750;
            } else if (mode === 11) {
                // Odd pen width plus half-micrometre round cap at edge.
                first.x1 = 126;
                first.x2 = 900;
                first.width = 251;
                second.width = 251;
                selected = 0;
                nativeIndex = 1;
                axis = 'y';
                pivot = 2250;
            } else if (mode === 12) {
                // Reflection must not infer a join through a different pen.
                second.color = '#FF7733';
                selected = 2;
                nativeIndex = 2;
            }
            const original = encodeGlyphLibrary(source);
            let expected;
            try {
                expected = encodeGlyphLibrary(reflectCandidatePath(
                    source, key, selected, axis, pivot).strokes);
            } catch (error) {
                expected = error;
            }
            const actual = spawnSync(binary, [
                'glyphs', ...source.flatMap(strokeFlags),
                '--reflect-path', key, String(nativeIndex), axis,
                String(pivot),
            ], { cwd: root, encoding: 'utf8', timeout: 2_000,
                maxBuffer: 512 * 1024 });
            const context = `reflected path sample=${sample} mode=${mode}`;
            assert.equal(actual.error, undefined, context);
            if (expected instanceof Error) {
                refused += 1;
                assert.notEqual(actual.status, 0, context);
                assert.equal(actual.stdout, '', context);
                assert.match(actual.stderr, /^Error: /, context);
            } else {
                accepted += 1;
                assert.equal(actual.status, 0, context + actual.stderr);
                assert.equal(actual.stdout, expected, context);
            }
            assert.equal(encodeGlyphLibrary(source), original, context);
        }
        assert.ok(accepted > 65, `accepted mirrors: ${accepted}`);
        assert.ok(refused > 65, `refused mirrors: ${refused}`);
    });

// Compare one selected path's clockwise whole-micrometre rotation with the
// native CLI, rather than trusting only browser and Rust unit-test fixtures.
// Keep interleaved NFC/NFD and same-glyph unselected paths in every sample.
test('256 real Rust and browser selected-path rotations agree on TSV',
    { timeout: 90_000 }, () => {
        const built = spawnSync(cargo, [
            'build', '--locked', '--offline', '--quiet',
            '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
        ], { cwd: root, encoding: 'utf8', timeout: 90_000,
            env: { ...process.env, TMPDIR: join(root, '.temp') } });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);
        const pick = random(0xC0F1_AE17);
        let accepted = 0;
        let refused = 0;
        for (let sample = 0; sample < 256; sample += 1) {
            const key = sample % 2 === 0 ? 'ñ' : 'n\u0303';
            const other = key === 'ñ' ? 'n\u0303' : 'ñ';
            const mode = sample % 16;
            const first = stroke(pick, key, '#123456');
            const second = { ...first, x1: first.x2, y1: first.y2,
                x2: first.x2 + pick(1, 200),
                y2: first.y2 + pick(1, 200) };
            const detached = stroke(pick, key, '#667788');
            let selected = mode === 15 ? 4 : 2;
            let degrees = [10, -10, 20, -20][mode % 4];
            let pivotX = 1750;
            let pivotY = 2250;
            let nativeIndex = mode === 15 ? 2 : 1;
            if (mode === 4) {
                degrees = 0;
            } else if (mode === 5) {
                degrees = 21;
            } else if (mode === 6) {
                degrees = -21;
            } else if (mode === 7) {
                pivotX = 0;
            } else if (mode === 8) {
                pivotY = 4500;
            } else if (mode === 9) {
                selected = -1;
                nativeIndex = 0;
            } else if (mode === 10) {
                selected = 9999;
                nativeIndex = 4097;
            } else if (mode === 11) {
                degrees = 0.5;
            } else if (mode === 12) {
                pivotX = 3501;
            } else if (mode === 13) {
                pivotY = 4501;
            } else if (mode === 14) {
                // A minimal stroke around its pivot must refuse a rounded
                // no-op, rather than claim an accepted transform.
                first.x1 = 1750;
                first.y1 = 2250;
                first.x2 = 1751;
                first.y2 = 2250;
                second.x1 = 1751;
                second.y1 = 2250;
                second.x2 = 1752;
                second.y2 = 2250;
                degrees = 1;
                selected = 0;
                nativeIndex = 1;
            }
            const source = [first, stroke(pick, other, '#AABBCC'),
                second, stroke(pick, '∞', '#CCDDEE'), detached];
            const original = encodeGlyphLibrary(source);
            let expected;
            try {
                expected = encodeGlyphLibrary(rotateCandidatePath(
                    source, key, selected, degrees, pivotX, pivotY).strokes);
            } catch (error) {
                expected = error;
            }
            const actual = spawnSync(binary, [
                'glyphs', ...source.flatMap(strokeFlags),
                '--rotate-path-ink', key, String(nativeIndex),
                String(degrees), String(pivotX), String(pivotY),
            ], { cwd: root, encoding: 'utf8', timeout: 2_000,
                maxBuffer: 512 * 1024 });
            const context = `rotated path sample=${sample} mode=${mode}`;
            assert.equal(actual.error, undefined, context);
            if (expected instanceof Error) {
                refused += 1;
                assert.notEqual(actual.status, 0, context);
                assert.equal(actual.stdout, '', context);
                assert.match(actual.stderr, /^Error: /, context);
            } else {
                accepted += 1;
                assert.equal(actual.status, 0, context + actual.stderr);
                assert.equal(actual.stdout, expected, context);
            }
            assert.equal(encodeGlyphLibrary(source), original, context);
        }
        assert.ok(accepted > 55, `accepted rotations: ${accepted}`);
        assert.ok(refused > 90, `refused rotations: ${refused}`);
    });
