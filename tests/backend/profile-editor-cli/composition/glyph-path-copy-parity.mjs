// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Verify native CLI and browser copy the same explicitly authored pen paths.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { candidatePenPaths, copyCandidatePath, encodeGlyphLibrary }
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
