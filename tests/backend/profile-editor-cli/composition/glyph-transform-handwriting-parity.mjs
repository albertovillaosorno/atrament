// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: MIT
// Compare the actual native glyph CLI with the built browser editor, not an
// independently rewritten transform oracle. Reproduction seed is fixed.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import {
    encodeGlyphLibrary, italicizeGlyphStrokes, rotateGlyphStrokes,
    scaleGlyphHeight, scaleGlyphInk, scaleGlyphWidth,
    translateGlyphStrokes, reflectGlyphStrokes, subdivideGlyphStroke,
// jig-ignore-next-line: static ESM import needs a literal path.
} from '../../../../src/browser/workspace/adapter-inbound/generated/glyph-strokes.js';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const cargo = join(root, '.dependencies/rust/1.99.1/bin/cargo');
const binaryName = process.platform === 'win32'
    ? 'atrament-profiles.exe' : 'atrament-profiles';
const binary = join(root, '.cache/cargo-target/debug', binaryName);
const nfc = 'ó';
const nfd = 'o\u0301';
const colors = ['#345678', '#A1B2C3'];

function generator(seed) {
    let state = seed >>> 0;
    const next = () => {
        state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
        return state;
    };
    return (minimum, maximum) => minimum + next() % (maximum - minimum + 1);
}

const operations = [
    {
        name: 'rotate',
        options: (pick) => [pick(-20, 20), pick(0, 3500), pick(0, 4500)],
        browser: rotateGlyphStrokes,
        native: '--rotate-glyph',
    },
    {
        name: 'shear',
        options: (pick) => [pick(0, 20), pick(0, 4500)],
        browser: italicizeGlyphStrokes,
        native: '--shear-glyph',
    },
    {
        name: 'height',
        options: (pick) => [pick(50, 150), pick(0, 4500)],
        browser: scaleGlyphHeight,
        native: '--scale-glyph-height',
    },
    {
        name: 'width',
        options: (pick) => [pick(50, 150), pick(0, 3500)],
        browser: scaleGlyphWidth,
        native: '--scale-glyph-width',
    },
    {
        name: 'thickness',
        options: (pick) => [pick(50, 150)],
        browser: scaleGlyphInk,
        native: '--scale-glyph-ink',
    },
    {
        name: 'translation',
        options: (pick) => [pick(-900, 900), pick(-900, 900)],
        browser: translateGlyphStrokes,
        native: '--translate-glyph',
    },
    {
        name: 'reflection',
        options: (pick) => {
            const axis = pick(0, 1) === 0 ? 'x' : 'y';
            return [axis, pick(0, axis === 'x' ? 3500 : 4500)];
        },
        browser: reflectGlyphStrokes,
        native: '--reflect-glyph',
    },
];

function authoredStroke(pick, grapheme, color) {
    const stroke = {
        grapheme, x1: pick(40, 3460), y1: pick(40, 4460),
        x2: pick(40, 3460), y2: pick(40, 4460),
        width: pick(80, 600), color,
    };
    if (stroke.x1 === stroke.x2 && stroke.y1 === stroke.y2) {
        stroke.x2 = stroke.x1 === 3460 ? 3459 : stroke.x1 + 1;
    }
    return stroke;
}

function cliStroke(stroke) {
    return ['--glyph-stroke', stroke.grapheme,
        String(stroke.x1), String(stroke.y1),
        String(stroke.x2), String(stroke.y2),
        String(stroke.width), stroke.color];
}

// All 1,792 checked cases use the real Rust binary, direct UTF-8 TSV output,
// and the browser's production encoder. Neither implementation is mirrored
// with ad hoc arithmetic in the test. Unselected interleaved Unicode shapes
// must remain byte-identical when the selected glyph changes.
test('seven native glyph edits match browser decisions and exact TSV bytes',
    { timeout: 120_000 }, () => {
        const built = spawnSync(cargo, [
            'build', '--locked', '--offline', '--quiet',
            '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
        ], { cwd: root, encoding: 'utf8', timeout: 90_000 });
        assert.equal(built.error, undefined, built.error?.message);
        assert.equal(built.status, 0, built.stderr);

        for (const [family, operation] of operations.entries()) {
            const pick = generator(0x6F74_AC31 + family);
            let approved = 0;
            let rejected = 0;
            for (let sample = 0; sample < 256; sample += 1) {
                const target = sample % 2 === 0 ? nfc : nfd;
                const neighbor = target === nfc ? nfd : nfc;
                const source = [
                    authoredStroke(pick, target, colors[0]),
                    authoredStroke(pick, neighbor, colors[1]),
                    authoredStroke(pick, target, colors[0]),
                ];
                const values = operation.options(pick);
                let expected;
                try {
                    expected = encodeGlyphLibrary(operation.browser(
                        source, target, ...values,
                    ));
                } catch (error) {
                    expected = error;
                }
                const args = ['glyphs', ...source.flatMap(cliStroke),
                    operation.native, target, ...values.map(String)];
                const actual = spawnSync(binary, args, {
                    cwd: root, encoding: 'utf8', timeout: 2_000,
                    maxBuffer: 512 * 1024,
                });
                const diagnostic = `${operation.name} sample ${sample}`
                    + ` (${target}) ${JSON.stringify(values)}`;
                assert.equal(actual.error, undefined,
                    `${diagnostic}: ${actual.error?.message}`);
                if (expected instanceof Error) {
                    rejected += 1;
                    assert.notEqual(actual.status, 0, diagnostic
                        + JSON.stringify(source) + String(expected.message));
                    assert.match(actual.stderr, /^Error: /, diagnostic);
                    assert.equal(actual.stdout, '', diagnostic);
                } else {
                    approved += 1;
                    assert.equal(actual.status, 0,
                        `${diagnostic}: ${actual.stderr}`);
                    assert.equal(actual.stdout, expected, diagnostic);
                }
            }
            assert.ok(approved > 0, `${operation.name} needs accepted edits`);
            assert.ok(rejected > 0, `${operation.name} needs refused edits`);
        }
    });

// Exact TSV agreement also has to survive the independently selected ink
// source path, through both physical output serializers. This does not infer
// new shapes: the notebook receives only the two explicit Unicode keys.
test('browser and native transformed shapes render identical PDF and PNG',
    { timeout: 90_000 }, () => {
        const source = [
            { grapheme: nfc, x1: 700, y1: 1000, x2: 1300, y2: 1700,
                width: 250, color: colors[0] },
            { grapheme: nfd, x1: 600, y1: 1100, x2: 1200, y2: 1550,
                width: 251, color: colors[1] },
            { grapheme: nfc, x1: 1300, y1: 1700, x2: 1650, y2: 2050,
                width: 250, color: colors[0] },
        ];
        const valuesByOperation = [
            [8, 1750, 2250], [9, 2250], [115, 2250],
            [120, 1750], [120], [90, -75], ['x', 1750],
        ];
        for (const [index, operation] of operations.entries()) {
            const values = valuesByOperation[index];
            const browserTsv = encodeGlyphLibrary(operation.browser(
                source, nfc, ...values,
            ));
            const transformed = spawnSync(binary, [
                'glyphs', ...source.flatMap(cliStroke), operation.native,
                nfc, ...values.map(String),
            ], { cwd: root, encoding: 'utf8', timeout: 2_000 });
            assert.equal(transformed.error, undefined);
            assert.equal(transformed.status, 0,
                `${operation.name}: ${transformed.stderr}`);
            assert.equal(transformed.stdout, browserTsv, operation.name);
            for (const format of ['pdf', 'png']) {
                const render = (input) => spawnSync(binary, [
                    'notebook', '--glyph-file', '-', '--place-text',
                    '1', '25000', '30000', `${nfc}${nfd}${nfc}`,
                    '--export-paper', format,
                ], { cwd: root, input: Buffer.from(input, 'utf8'),
                    timeout: 30_000, maxBuffer: 16 * 1024 * 1024 });
                const fromBrowser = render(browserTsv);
                const fromNative = render(transformed.stdout);
                for (const result of [fromBrowser, fromNative]) {
                    assert.equal(result.error, undefined,
                        `${operation.name} ${format}: `
                        + `${result.error?.message}`);
                    assert.equal(result.status, 0,
                        `${operation.name} ${format}: ${result.stderr}`);
                }
                assert.deepEqual(fromNative.stdout, fromBrowser.stdout,
                    `${operation.name} ${format} differs`);
                if (format === 'pdf') {
                    assert.ok(fromNative.stdout.subarray(0, 5)
                        .equals(Buffer.from('%PDF-')), operation.name);
                } else {
                    assert.equal(fromNative.stdout.subarray(0, 8)
                        .toString('hex'), '89504e470d0a1a0a');
                }
            }
        }
    });


// The native path inspector must see the new seam as a continuous directed
// pen-down gesture. The existing Rust notebook binary consumes the exact
// browser TSV, not a separate surrogate or a second shape implementation.
test("browser stroke subdivision exports valid native PDF and RGBA PNG",
    { timeout: 90_000 }, () => {
    const built = spawnSync(cargo, [
        "build", "--locked", "--offline", "--quiet",
        "-p", "atrament_profile_editor_cli", "--bin", "atrament-profiles",
    ], { cwd: root, encoding: "utf8", timeout: 90_000,
        env: { ...process.env, TMPDIR: join(root, ".temp") } });
    assert.equal(built.error, undefined, built.error?.message);
    assert.equal(built.status, 0, built.stderr);

    const source = [
        { grapheme: nfd, x1: 1000, y1: 900, x2: 1601, y2: 1702,
            width: 251, color: colors[0] },
        { grapheme: nfc, x1: 400, y1: 800, x2: 500, y2: 1200,
            width: 251, color: colors[1] },
        { grapheme: nfd, x1: 1601, y1: 1702, x2: 2000, y2: 2200,
            width: 251, color: colors[0] },
    ];
    const divided = subdivideGlyphStroke(source, nfd, 0);
    const tsv = encodeGlyphLibrary(divided);
    const invoke = (args, binaryOutput = false) => spawnSync(binary, args, {
        cwd: root, input: Buffer.from(tsv, "utf8"),
        encoding: binaryOutput ? undefined : "utf8",
        timeout: 30_000, maxBuffer: 16 * 1024 * 1024,
    });
    const readback = invoke(["glyphs", "--profile", "-"]);
    assert.equal(readback.status, 0, readback.stderr);
    assert.equal(readback.stdout, tsv);
    const report = invoke([
        "glyphs", "--profile", "-", "--inspect-paths", nfd,
    ]);
    assert.equal(report.status, 0, report.stderr);
    assert.match(report.stdout, /Strokes: 3\nPaths: 1\nPen lifts: 0/);
    assert.match(report.stdout, /strokes 1\.\.3 \(3 segments\)/);
    const other = invoke([
        "glyphs", "--profile", "-", "--inspect-paths", nfc,
    ]);
    assert.equal(other.status, 0, other.stderr);
    assert.match(other.stdout, /Strokes: 1\nPaths: 1\nPen lifts: 0/);
    for (const format of ["pdf", "png"]) {
        const flags = ["notebook", "--glyph-file", "-",
            "--place-text", "1", "25000", "30000", nfd + nfc,
            "--export-paper", format];
        if (format === "png") flags.push("--dpi", "72");
        const actual = invoke(flags, true);
        assert.equal(actual.error, undefined, actual.error?.message);
        assert.equal(actual.status, 0, String(actual.stderr));
        assert.ok(actual.stdout.length > 100, format);
        const signature = format === "pdf"
            ? Buffer.from("%PDF-")
            : Buffer.from("89504e470d0a1a0a", "hex");
        assert.ok(actual.stdout.subarray(0, signature.length)
            .equals(signature), format);
        const replay = invoke(flags, true);
        assert.equal(replay.status, 0, String(replay.stderr));
        assert.deepEqual(replay.stdout, actual.stdout,
            "native output must be byte-stable after subdivision");
    }
});


// Browser and Rust must agree on selected exact-glyph stroke ordinals even
// when NFC/NFD siblings are interleaved. The browser API selects a global
// zero-based source row; the CLI selects one-based rows within the glyph.
test('browser subdivision matches native exact TSV and short-line refusal',
    { timeout: 90_000 }, () => {
    const built = spawnSync(cargo, [
        'build', '--locked', '--offline', '--quiet',
        '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
    ], { cwd: root, encoding: 'utf8', timeout: 90_000,
        env: { ...process.env, TMPDIR: join(root, '.temp') } });
    assert.equal(built.error, undefined, built.error?.message);
    assert.equal(built.status, 0, built.stderr);

    const pick = generator(0x1D35_8910);
    let approved = 0;
    let rejected = 0;
    for (let sample = 0; sample < 256; sample += 1) {
        const target = sample % 2 === 0 ? nfc : nfd;
        const neighbor = target === nfc ? nfd : nfc;
        const source = [
            authoredStroke(pick, target, colors[0]),
            authoredStroke(pick, neighbor, colors[1]),
            authoredStroke(pick, target, colors[0]),
        ];
        const selected = sample % 4 < 2 ? 0 : 2;
        if (sample % 11 === 0) {
            source[selected].x1 = 1000;
            source[selected].y1 = 1200;
            source[selected].x2 = 1001;
            source[selected].y2 = 1200;
        }
        let expected;
        try {
            expected = encodeGlyphLibrary(subdivideGlyphStroke(
                source, target, selected,
            ));
        } catch (error) {
            expected = error;
        }
        const actual = spawnSync(binary, [
            'glyphs', ...source.flatMap(cliStroke),
            '--split-stroke', target, String(selected === 0 ? 1 : 2),
        ], { cwd: root, encoding: 'utf8', timeout: 2_000,
            maxBuffer: 512 * 1024 });
        const diagnostic = 'subdivision sample ' + sample
            + ' (' + target + ', source row ' + selected + ')';
        assert.equal(actual.error, undefined,
            diagnostic + ': ' + actual.error?.message);
        if (expected instanceof Error) {
            rejected += 1;
            assert.notEqual(actual.status, 0, diagnostic);
            assert.match(actual.stderr, /^Error: /, diagnostic);
            assert.equal(actual.stdout, '', diagnostic);
        } else {
            approved += 1;
            assert.equal(actual.status, 0,
                diagnostic + ': ' + actual.stderr);
            assert.equal(actual.stdout, expected, diagnostic);
        }
    }
    assert.ok(approved > 0, 'subdivision needs accepted segments');
    assert.ok(rejected > 0, 'subdivision needs refused short segments');
});
