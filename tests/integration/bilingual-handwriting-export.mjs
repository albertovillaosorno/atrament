// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// A real browser-authored geometry -> TSV -> Rust page export regression.
// Run after the frontend TypeScript build: node --test <this file>.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import test from 'node:test';
import {
    composeAccentedGlyph, encodeGlyphLibrary, geometricStarter,
    reverseCandidatePath, moveCandidatePath, translateCandidatePath,
    candidatePenPaths,
} from '../../src/browser/workspace/adapter-inbound/generated/glyph-strokes.js';

const root = fileURLToPath(new URL('../../', import.meta.url));
const starter = [1550, 2450, 1100, 250, '#243647'];
const accents = [
    ['a', 'á'], ['e', 'é'], ['i', 'í'], ['o', 'ó'],
    ['u', 'ú'], ['n', 'ñ'], ['u', 'ü'],
];

function bilingualStarter() {
    const sections = [];
    const strokes = [];
    const seed = (letters) => {
        sections.push(letters);
        strokes.push(...letters.flatMap((g) =>
            geometricStarter(g, ...starter)));
    };
    const compose = (uppercase, normalization) => {
        const created = [];
        for (const [base, accent] of accents) {
            const source = uppercase ? base.toUpperCase() : base;
            const target = (uppercase ? accent.toUpperCase() : accent)
                .normalize(normalization);
            strokes.push(...composeAccentedGlyph(
                strokes, source, target, '#243647'));
            created.push(target);
        }
        sections.push(created);
    };
    seed([...'abcdefghijklmnopqrstuvwxyz']);
    compose(false, 'NFC');
    compose(false, 'NFD');
    seed(['.', ',', '!', '¡', '?', '¿']);
    seed(['«', '»', '‘', '’', '“', '”']);
    seed(['+', '-', '−', '=', '×', '÷']);
    seed([...'0123456789']);
    seed([...'ABCDEFGHIJKLMNOPQRSTUVWXYZ']);
    compose(true, 'NFC');
    compose(true, 'NFD');
    seed([';', ':', '"', "'", '…', '–', '—', '(', ')', '[', ']']);
    return { strokes, sections };
}

function native(...args) {
    const result = spawnSync('cargo', [
        'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--', ...args,
    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function pngHasTransparentCorner(png) {
    assert.deepEqual(png.subarray(0, 8),
        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
    assert.equal(png.readUInt32BE(16), 596);
    assert.equal(png.readUInt32BE(20), 842);
    assert.equal(png[25], 6, 'RGBA instead of opaque RGB');
    const compressed = [];
    let offset = 8;
    while (offset + 12 <= png.length) {
        const length = png.readUInt32BE(offset);
        const kind = png.toString('ascii', offset + 4, offset + 8);
        if (kind === 'IDAT') {
            compressed.push(png.subarray(offset + 8, offset + 8 + length));
        }
        offset += length + 12;
    }
    assert.ok(compressed.length > 0, 'PNG has raster payload');
    const raw = inflateSync(Buffer.concat(compressed));
    assert.equal(raw[0], 0, 'first row uses an unfiltered RGBA encoding');
    assert.equal(raw[4], 0, 'page top-left alpha is transparent');
}

const flowRegion = [
    '--flow-region', '30000', '40000', '140000', '120000',
];

test('entire bilingual starter library renders and fails closed', () => {
    const dir = mkdtempSync(join(tmpdir(), 'atrament-bilingual-'));
    try {
        const { strokes, sections } = bilingualStarter();
        assert.equal(strokes.length, 2738);
        assert.equal(new Set(strokes.map((stroke) => stroke.grapheme)).size,
            119);
        const library = join(dir, 'ink.tsv');
        const source = join(dir, 'source.txt');
        const pdfPath = join(dir, 'baseline.pdf');
        const pngPath = join(dir, 'baseline.png');
        writeFileSync(library, encodeGlyphLibrary(strokes));
        const original = sections.map((part) => part.join('')).join('\n');
        writeFileSync(source, `${original}\n`);
        const audit = native('glyphs', '--profile', library,
            '--audit-baseline');
        assert.equal(audit.status, 0, audit.stderr);
        assert.match(audit.stdout, /Distinct visible graphemes: 114/);
        assert.match(audit.stdout, /Missing authored glyphs: 0/);
        // The browser-produced TSV, not a synthetic Rust-only fixture,
        // determines the exact native pen sequence. NFC and NFD stay apart.
        const originalTsv = readFileSync(library);
        for (const key of ['ó', 'o\u0301', 'ñ', 'n\u0303']) {
            const authored = strokes.filter((stroke) =>
                stroke.grapheme === key);
            assert.ok(authored.length > 0, `authored ${key}`);
            const report = native('glyphs', '--profile', library,
                '--inspect-glyph', key);
            assert.equal(report.status, 0, report.stderr);
            const lines = report.stdout.trimEnd().split('\n');
            const identity = [...key].map((ch) =>
                `U+${ch.codePointAt(0).toString(16)
                    .toUpperCase().padStart(4, '0')}`).join(' ');
            assert.equal(lines[0],
                'Exact authored stroke sequence (not inferred cursive)');
            assert.equal(lines[1], `Glyph: "${key}" [${identity}]`);
            assert.equal(lines[2], `Strokes: ${authored.length}`);
            assert.equal(lines.length, authored.length + 3);
            for (const [index, stroke] of authored.entries()) {
                assert.equal(lines[index + 3],
                    `${index + 1}: (${stroke.x1}, ${stroke.y1}) -> `
                    + `(${stroke.x2}, ${stroke.y2}) width ${stroke.width} `
                    + `um color ${stroke.color.toUpperCase()}`);
            }
        }
        assert.deepEqual(readFileSync(library), originalTsv,
            'stroke inspection does not change browser-authored TSV');
        const notebookBase = ['notebook', '--printed-sheet', 'true',
            '--glyph-file', library];
        const args = [...notebookBase, '--flow-text-file', source,
            ...flowRegion];
        const pdfRun = native(...args, '--export-paper', 'pdf',
            '--output', pdfPath);
        assert.equal(pdfRun.status, 0, pdfRun.stderr);
        const pdf = readFileSync(pdfPath);
        assert.ok(pdf.subarray(0, 8).toString().startsWith('%PDF-1.4'));
        assert.equal(pdf.toString('latin1').split(' m\n').length - 1,
            2738, 'all starter segments reach vector PDF without truncation');
        assert.equal(pdf.toString('latin1').split('\nstream\n').length - 1,
            1, 'fixed bilingual region fits one physical page');
        const pngRun = native(...args, '--export-paper', 'png',
            '--dpi', '72', '--png-page', '1', '--output', pngPath);
        assert.equal(pngRun.status, 0, pngRun.stderr);
        pngHasTransparentCorner(readFileSync(pngPath));
        const pdfReplay = native(...args, '--export-paper', 'pdf',
            '--output', pdfPath, '--replace');
        assert.equal(pdfReplay.status, 0, pdfReplay.stderr);
        assert.deepEqual(readFileSync(pdfPath), pdf);

        const fallbackSource = join(dir, 'unwritten.txt');
        const preserved = join(dir, 'preserve.pdf');
        const sentinel = Buffer.from('last accepted notebook');
        writeFileSync(preserved, sentinel);
        writeFileSync(fallbackSource, `${original}\n∞\n`);
        const withMissing = [...notebookBase,
            '--flow-text-file', fallbackSource, ...flowRegion];
        const missing = native(...withMissing, '--export-paper', 'pdf',
            '--output', preserved, '--replace');
        assert.notEqual(missing.status, 0,
            'no unapproved missing Unicode glyph substitution');
        assert.deepEqual(readFileSync(preserved), sentinel);
        const explicit = [...withMissing, '--fallback-glyph', '∞', '8'];
        const substituted = native(...explicit, '--export-paper', 'pdf',
            '--output', preserved, '--replace');
        assert.equal(substituted.status, 0, substituted.stderr);
        assert.ok(readFileSync(preserved).subarray(0, 8)
            .toString().startsWith('%PDF-1.4'));
        writeFileSync(preserved, sentinel);
        writeFileSync(fallbackSource, `${original}\n${'∞'.repeat(15)}\n`);
        const overCap = native(...explicit, '--export-paper', 'pdf',
            '--output', preserved, '--replace');
        assert.notEqual(overCap.status, 0,
            'more than 4096 expanded strokes must refuse');
        assert.match(overCap.stderr, /4096 total pen strokes/);
        assert.deepEqual(readFileSync(preserved), sentinel);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('browser loop shapes derive independent accented glyphs in Rust', () => {
    const dir = mkdtempSync(join(tmpdir(), 'atrament-derived-'));
    try {
        const { strokes } = bilingualStarter();
        const source = join(dir, 'browser.tsv');
        const derived = join(dir, 'derived.tsv');
        const comparison = join(dir, 'equivalent.tsv');
        const preserve = join(dir, 'preserve.tsv');
        const original = encodeGlyphLibrary(strokes);
        writeFileSync(source, original);
        const sourceO = strokes.filter((stroke) => stroke.grapheme === 'o');
        assert.ok(sourceO.length > 1, 'browser o is multi-segment ink');
        assert.ok(!strokes.some((stroke) => stroke.grapheme === 'ö'),
            'the target is not an already-authored Unicode glyph');
        const dots = [
            { grapheme: 'ö', x1: 1200, y1: 350, x2: 1370, y2: 450,
                width: 180, color: '#243647' },
            { grapheme: 'ö', x1: 2150, y1: 350, x2: 2320, y2: 450,
                width: 180, color: '#243647' },
        ];
        const expected = encodeGlyphLibrary([
            ...strokes,
            ...sourceO.map((stroke) => ({ ...stroke, grapheme: 'ö' })),
            ...dots,
        ]);
        writeFileSync(comparison, expected);
        const saved = native('glyphs', '--profile', source,
            '--copy-glyph', 'o', 'ö',
            '--glyph-stroke', 'ö', '1200', '350', '1370', '450',
            '180', '#243647',
            '--glyph-stroke', 'ö', '2150', '350', '2320', '450',
            '180', '#243647', '--output', derived);
        assert.equal(saved.status, 0, saved.stderr);
        assert.deepEqual(readFileSync(derived), Buffer.from(expected),
            'Rust copies browser-authored o in order before two user dots');
        assert.deepEqual(readFileSync(source), Buffer.from(original),
            'base glyph and its browser TSV stay unchanged');
        const inspected = native('glyphs', '--profile', derived,
            '--inspect-glyph', 'ö');
        assert.equal(inspected.status, 0, inspected.stderr);
        assert.match(inspected.stdout, /Glyph: "ö" \[U\+00F6\]/);
        assert.match(inspected.stdout,
            new RegExp(`Strokes: ${sourceO.length + dots.length}`));
        const audit = native('glyphs', '--profile', derived,
            '--audit-text', 'o ö ó');
        assert.equal(audit.status, 0, audit.stderr);
        assert.match(audit.stdout, /Missing authored glyphs: 0/);
        writeFileSync(preserve, Buffer.from('previous accepted library'));
        const duplicate = native('glyphs', '--profile', source,
            '--copy-glyph', 'o', 'ó', '--output', preserve, '--replace');
        assert.notEqual(duplicate.status, 0,
            'copy must not overwrite a browser-authored accented glyph');
        assert.deepEqual(readFileSync(preserve),
            Buffer.from('previous accepted library'));
        for (const format of ['pdf', 'png']) {
            const flags = ['--export-paper', format];
            if (format === 'png') flags.push('--dpi', '72');
            const artifact = join(dir, `derived.${format}`);
            const equal = join(dir, `manual.${format}`);
            const make = (library, output) => native('notebook',
                '--printed-sheet', 'true', '--glyph-file', library,
                '--place-text', '1', '30000', '45000', 'oö',
                ...flags, '--output', output);
            const generated = make(derived, artifact);
            const manuallyComposed = make(comparison, equal);
            assert.equal(generated.status, 0, generated.stderr);
            assert.equal(manuallyComposed.status, 0,
                manuallyComposed.stderr);
            assert.deepEqual(readFileSync(artifact), readFileSync(equal),
                `native ${format} preserves browser geometry and added dots`);
            if (format === 'png') {
                pngHasTransparentCorner(readFileSync(artifact));
            } else {
                assert.ok(readFileSync(artifact).subarray(0, 8).toString()
                    .startsWith('%PDF-1.4'));
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});


test('browser reversed path has native inspection and PDF/PNG parity', () => {
    const dir = mkdtempSync(join(tmpdir(), 'atrament-reverse-path-'));
    try {
        const nfd = 'n\u0303';
        const line = (grapheme, x1, y1, x2, y2) => ({
            grapheme, x1, y1, x2, y2,
            width: 250, color: '#243647',
        });
        const original = [
            line(nfd, 500, 600, 1100, 1400),
            line('ñ', 300, 500, 2000, 3000),
            line(nfd, 1100, 1400, 1800, 2100),
            line('∞', 700, 700, 1700, 2100),
            line(nfd, 1800, 2100, 2200, 2600),
        ];
        const input = encodeGlyphLibrary(original);
        const result = reverseCandidatePath(original, nfd, 2);
        assert.equal(result.selectedIndex, 2);
        const library = join(dir, 'browser-reversed.tsv');
        writeFileSync(library, encodeGlyphLibrary(result.strokes));
        const reverseInk = [
            line(nfd, 2200, 2600, 1800, 2100),
            line(nfd, 1800, 2100, 1100, 1400),
            line(nfd, 1100, 1400, 500, 600),
        ];
        assert.deepEqual(result.strokes, [reverseInk[0], original[1],
            reverseInk[1], original[3], reverseInk[2]]);
        assert.equal(encodeGlyphLibrary(original), input,
            'the browser keeps its original authored TSV unmodified');

        const report = native('glyphs', '--profile', library,
            '--inspect-paths', nfd);
        assert.equal(report.status, 0, report.stderr);
        assert.match(report.stdout, /Strokes: 3\nPaths: 1\nPen lifts: 0/);
        assert.ok(report.stdout.includes(
            'Path 1: strokes 1..3 (3 segments) (2200, 2600) '
            + '-> (500, 600) width 250 um color #243647'));
        const originalGlyph = native('glyphs', '--profile', library,
            '--inspect-glyph', 'ñ');
        assert.equal(originalGlyph.status, 0, originalGlyph.stderr);
        assert.match(originalGlyph.stdout,
            /1: \(300, 500\) -> \(2000, 3000\)/);

        for (const format of ['pdf', 'png']) {
            const flags = ['--export-paper', format];
            if (format === 'png') flags.push('--dpi', '72');
            const actual = join(dir, `reversed.${format}`);
            const equivalent = join(dir, `direct.${format}`);
            const run = (args, path) => native('notebook',
                '--printed-sheet', 'true', ...args,
                '--place-text', '1', '30000', '45000', nfd,
                ...flags, '--output', path);
            const generated = run(['--glyph-file', library], actual);
            const ink = reverseInk.flatMap((stroke) => [
                '--glyph-stroke', nfd, String(stroke.x1), String(stroke.y1),
                String(stroke.x2), String(stroke.y2),
                String(stroke.width), stroke.color,
            ]);
            const manual = run(ink, equivalent);
            assert.equal(generated.status, 0, generated.stderr);
            assert.equal(manual.status, 0, manual.stderr);
            assert.deepEqual(readFileSync(actual), readFileSync(equivalent),
                `${format} must preserve exact reversed pen order`);
            if (format === 'png') {
                pngHasTransparentCorner(readFileSync(actual));
            } else {
                assert.ok(readFileSync(actual).subarray(0, 8).toString()
                    .startsWith('%PDF-1.4'));
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});


test('moved complete browser paths preserve native PDF/PNG stroke order',
    () => {
    const dir = mkdtempSync(join(tmpdir(), 'atrament-move-path-'));
    try {
        const nfd = 'n\u0303';
        const line = (grapheme, x1, y1, x2, y2, color) => ({
            grapheme, x1, y1, x2, y2, width: 250, color,
        });
        const a = line(nfd, 500, 600, 1100, 1400, '#243647');
        const b = line(nfd, 1100, 1400, 1800, 2100, '#243647');
        const c = line(nfd, 1500, 2500, 2200, 3200, '#B03040');
        const d = line(nfd, 2200, 3200, 2500, 3600, '#B03040');
        const original = [a, line('ñ', 500, 500, 700, 700,
            '#123456'), b, c, line('∞', 100, 100, 500, 900,
            '#123456'), d];
        const input = encodeGlyphLibrary(original);
        const moved = moveCandidatePath(original, nfd, 2, 1);
        const expected = [c, original[1], d, a, original[4], b];
        assert.deepEqual(moved.strokes, expected);
        assert.equal(moved.selectedIndex, 5);
        assert.equal(encodeGlyphLibrary(original), input);
        const target = join(dir, 'moved.tsv');
        writeFileSync(target, encodeGlyphLibrary(moved.strokes));
        const inspected = native('glyphs', '--profile', target,
            '--inspect-paths', nfd);
        assert.equal(inspected.status, 0, inspected.stderr);
        assert.match(inspected.stdout,
            /Strokes: 4\nPaths: 2\nPen lifts: 1/);
        assert.ok(inspected.stdout.includes(
            'Path 1: strokes 1..2 (2 segments) (1500, 2500) '
            + '-> (2500, 3600) width 250 um color #B03040'));
        assert.ok(inspected.stdout.includes(
            'Path 2: strokes 3..4 (2 segments) (500, 600) '
            + '-> (1800, 2100) width 250 um color #243647'));
        for (const format of ['pdf', 'png']) {
            const flags = ['--export-paper', format];
            if (format === 'png') flags.push('--dpi', '72');
            const actual = join(dir, `moved.${format}`);
            const reference = join(dir, `reference.${format}`);
            const run = (args, path) => native('notebook',
                '--printed-sheet', 'true', ...args,
                '--place-text', '1', '30000', '45000', nfd,
                ...flags, '--output', path);
            const manual = [c, d, a, b].flatMap((stroke) => [
                '--glyph-stroke', nfd, String(stroke.x1), String(stroke.y1),
                String(stroke.x2), String(stroke.y2),
                String(stroke.width), stroke.color,
            ]);
            const output = run(['--glyph-file', target], actual);
            const independent = run(manual, reference);
            assert.equal(output.status, 0, output.stderr);
            assert.equal(independent.status, 0, independent.stderr);
            assert.deepEqual(readFileSync(actual), readFileSync(reference),
                `${format} retains the browser's exact moved path order`);
            if (format === 'png') {
                pngHasTransparentCorner(readFileSync(actual));
            } else {
                assert.ok(readFileSync(actual).subarray(0, 8).toString()
                    .startsWith('%PDF-1.4'));
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('Rust path-level edits exactly match browser-authored candidate paths',
    () => {
    const dir = mkdtempSync(join(tmpdir(), 'atrament-native-path-'));
    try {
        const nfd = 'n\u0303';
        const line = (grapheme, x1, y1, x2, y2, color) => ({
            grapheme, x1, y1, x2, y2, width: 250, color,
        });
        const first = [
            line(nfd, 500, 600, 900, 1000, '#243647'),
            line(nfd, 900, 1000, 1300, 1500, '#243647'),
        ];
        const second = [
            line(nfd, 1800, 1800, 2200, 2400, '#B03040'),
            line(nfd, 2200, 2400, 2500, 3000, '#B03040'),
        ];
        const original = [
            first[0], line('ñ', 100, 100, 200, 200, '#101010'),
            first[1], second[0],
            line('∞', 100, 100, 200, 200, '#101010'),
            second[1], line(nfd, 1000, 3500, 1500, 4000, '#303030'),
        ];
        const originalTsv = encodeGlyphLibrary(original);
        const source = join(dir, 'original.tsv');
        const rustOutput = join(dir, 'edited.tsv');
        const browserOutput = join(dir, 'browser.tsv');
        writeFileSync(source, originalTsv);
        const moved = moveCandidatePath(original, nfd, 0, 1);
        const reversed = reverseCandidatePath(
            moved.strokes, nfd, moved.selectedIndex);
        const expected = encodeGlyphLibrary(reversed.strokes);
        writeFileSync(browserOutput, expected);
        const nativeEdit = native('glyphs', '--profile', source,
            '--move-path', nfd, '1', '2',
            '--reverse-path', nfd, '2',
            '--output', rustOutput);
        assert.equal(nativeEdit.status, 0, nativeEdit.stderr);
        assert.deepEqual(readFileSync(rustOutput), readFileSync(browserOutput),
            'native and browser use identical candidate path semantics');
        assert.equal(readFileSync(source, 'utf8'), originalTsv);
        const inspected = native('glyphs', '--profile', rustOutput,
            '--inspect-paths', nfd);
        assert.equal(inspected.status, 0, inspected.stderr);
        assert.match(inspected.stdout,
            /Strokes: 5\nPaths: 3\nPen lifts: 2/);
        assert.ok(inspected.stdout.includes(
            'Path 2: strokes 3..4 (2 segments) (1300, 1500) '
            + '-> (500, 600) width 250 um color #243647'));

        const sentinel = join(dir, 'sentinel.tsv');
        writeFileSync(sentinel, 'preserve');
        const refused = native('glyphs', '--profile', source,
            '--move-path', nfd, '1', '3',
            '--output', sentinel, '--replace');
        assert.notEqual(refused.status, 0,
            'nonadjacent paths require explicitly bounded moves');
        assert.equal(readFileSync(sentinel, 'utf8'), 'preserve');
        assert.equal(readFileSync(source, 'utf8'), originalTsv);

        for (const format of ['pdf', 'png']) {
            const flags = ['--export-paper', format];
            if (format === 'png') flags.push('--dpi', '72');
            const expectedPath = join(dir, 'browser.' + format);
            const actualPath = join(dir, 'rust.' + format);
            const render = (library, output) => native('notebook',
                '--printed-sheet', 'true', '--glyph-file', library,
                '--place-text', '1', '30000', '45000', nfd,
                ...flags, '--output', output);
            const browser = render(browserOutput, expectedPath);
            const rust = render(rustOutput, actualPath);
            assert.equal(browser.status, 0, browser.stderr);
            assert.equal(rust.status, 0, rust.stderr);
            assert.deepEqual(readFileSync(actualPath),
                readFileSync(expectedPath),
                'physical ' + format + ' matches browser exact pen-path order');
            if (format === 'png') {
                pngHasTransparentCorner(readFileSync(actualPath));
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});


test('browser whole-path translation has native PDF/PNG ink parity', () => {
    const dir = mkdtempSync(join(tmpdir(), 'atrament-shift-path-'));
    try {
        const nfd = 'n\u0303';
        const ink = (grapheme, x1, y1, x2, y2, color) => ({
            grapheme, x1, y1, x2, y2,
            color, width: 250,
        });
        const a = ink(nfd, 500, 650, 850, 950, '#243647');
        const b = ink(nfd, 850, 950, 1300, 1400, '#243647');
        const accent = ink(nfd, 1600, 2500, 2000, 2900, '#B03040');
        const original = [
            a, ink('ñ', 100, 200, 300, 400, '#121212'),
            b, ink('∞', 500, 600, 700, 800, '#121212'), accent,
        ];
        const originalBytes = encodeGlyphLibrary(original);
        const shifted = translateCandidatePath(
            original, nfd, 2, 300, -250);
        const movedAccent = translateCandidatePath(
            shifted.strokes, nfd, 4, -100, 200);
        const library = join(dir, 'shifted.tsv');
        writeFileSync(library, encodeGlyphLibrary(movedAccent.strokes));
        const independentlyAuthored = [
            ink(nfd, 800, 400, 1150, 700, '#243647'),
            ink(nfd, 1150, 700, 1600, 1150, '#243647'),
            ink(nfd, 1500, 2700, 1900, 3100, '#B03040'),
        ];
        assert.deepEqual(movedAccent.strokes, [
            independentlyAuthored[0], original[1],
            independentlyAuthored[1], original[3],
            independentlyAuthored[2],
        ]);
        assert.equal(encodeGlyphLibrary(original), originalBytes);
        const source = join(dir, 'source.tsv');
        const nativeOutput = join(dir, 'native.tsv');
        writeFileSync(source, originalBytes);
        const nativeEdit = native('glyphs', '--profile', source,
            '--translate-path', nfd, '1', '300', '-250',
            '--translate-path', nfd, '2', '-100', '200',
            '--output', nativeOutput);
        assert.equal(nativeEdit.status, 0, nativeEdit.stderr);
        assert.deepEqual(readFileSync(nativeOutput), readFileSync(library),
            'native and browser signed offsets produce byte-exact TSV');
        assert.equal(readFileSync(source, 'utf8'), originalBytes);
        const sentinel = join(dir, 'sentinel.tsv');
        writeFileSync(sentinel, 'preserve');
        for (const offset of [
            ['1', '0', '0'], ['1', '-500', '0'],
            ['1', '1.5', '0'], ['1', '3501', '0'],
            ['3', '100', '0'], ['1', '+1', '0'],
        ]) {
            const refused = native('glyphs', '--profile', source,
                '--translate-path', nfd, ...offset,
                '--output', sentinel, '--replace');
            assert.notEqual(refused.status, 0,
                'invalid native offsets must refuse');
            assert.equal(readFileSync(sentinel, 'utf8'), 'preserve');
        }
        assert.equal(readFileSync(source, 'utf8'), originalBytes);
        const inspection = native('glyphs', '--profile', library,
            '--inspect-paths', nfd);
        assert.equal(inspection.status, 0, inspection.stderr);
        assert.match(inspection.stdout,
            /Strokes: 3\nPaths: 2\nPen lifts: 1/);
        assert.ok(inspection.stdout.includes(
            'Path 1: strokes 1..2 (2 segments) (800, 400) '
            + '-> (1600, 1150) width 250 um color #243647'));
        assert.ok(inspection.stdout.includes(
            'Path 2: strokes 3..3 (1 segment) (1500, 2700) '
            + '-> (1900, 3100) width 250 um color #B03040'));

        for (const format of ['pdf', 'png']) {
            const flags = ['--export-paper', format];
            if (format === 'png') flags.push('--dpi', '72');
            const actual = join(dir, 'translated.' + format);
            const reference = join(dir, 'reference.' + format);
            const run = (args, target) => native('notebook',
                '--printed-sheet', 'true', ...args,
                '--place-text', '1', '30000', '45000', nfd,
                ...flags, '--output', target);
            const rendered = run(['--glyph-file', library], actual);
            const independent = run(independentlyAuthored.flatMap(
                (shape) => [
                    '--glyph-stroke', nfd,
                    String(shape.x1), String(shape.y1),
                    String(shape.x2), String(shape.y2),
                    String(shape.width), shape.color,
                ]), reference);
            assert.equal(rendered.status, 0, rendered.stderr);
            assert.equal(independent.status, 0, independent.stderr);
            assert.deepEqual(readFileSync(actual), readFileSync(reference),
                'translated path physical ' + format + ' parity');
            if (format === 'png') {
                pngHasTransparentCorner(readFileSync(actual));
            }
        }
        assert.equal(encodeGlyphLibrary(original), originalBytes);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});


test('combined pen-path edit script matches native and browser ink', () => {
    const dir = mkdtempSync(join(tmpdir(), 'atrament-path-script-'));
    try {
        const nfd = 'n\u0303';
        const line = (glyph, x1, y1, x2, y2, width, color) => ({
            grapheme: glyph, x1, y1, x2, y2, width, color,
        });
        const a = [
            line(nfd, 500, 700, 800, 1100, 250, '#243647'),
            line(nfd, 800, 1100, 1200, 1500, 250, '#243647'),
            line(nfd, 1200, 1500, 1500, 1900, 250, '#243647'),
        ];
        const b = [
            line(nfd, 1600, 2400, 1900, 2700, 300, '#334455'),
            line(nfd, 1900, 2700, 2300, 3000, 300, '#334455'),
        ];
        const c = line(nfd, 2500, 3500, 3000, 3900, 350, '#667788');
        const original = [
            a[0], line('ñ', 200, 300, 400, 500, 250, '#121212'),
            a[1], line('∞', 300, 500, 600, 800, 250, '#121212'),
            a[2], b[0], line('n', 400, 500, 700, 900,
                250, '#121212'), b[1], c,
        ];
        const sourceBytes = encodeGlyphLibrary(original);
        const steps = [
            ['reverse', 1],
            ['translate', 1, 250, -100],
            ['move', 1, 2],
            ['translate', 1, -100, 150],
            ['reverse', 2],
            ['move', 2, 3],
            ['translate', 3, 100, 200],
            ['move', 1, 2],
            ['reverse', 2],
            ['translate', 2, 50, -100],
            ['move', 2, 1],
            ['reverse', 3],
        ];
        let state = original.slice();
        const cli = [];
        for (const [verb, number, other, y] of steps) {
            const positions = state.flatMap((stroke, index) =>
                stroke.grapheme === nfd ? [index] : []);
            const paths = candidatePenPaths(state, nfd);
            assert.equal(paths.length, 3);
            const path = paths[number - 1];
            assert.ok(path);
            const selected = positions[path.first - 1];
            assert.notEqual(selected, undefined);
            let next;
            if (verb === 'reverse') {
                next = reverseCandidatePath(state, nfd, selected);
                cli.push('--reverse-path', nfd, String(number));
            } else if (verb === 'move') {
                const direction = Math.sign(other - number);
                assert.ok(direction === -1 || direction === 1);
                next = moveCandidatePath(state, nfd, selected, direction);
                cli.push('--move-path', nfd, String(number), String(other));
            } else {
                next = translateCandidatePath(
                    state, nfd, selected, other, y);
                cli.push('--translate-path', nfd,
                    String(number), String(other), String(y));
            }
            state = next.strokes;
            assert.equal(candidatePenPaths(state, nfd).length, 3,
                'whole paths stay pen-separated through each operation');
        }
        const browserTsv = encodeGlyphLibrary(state);
        assert.notEqual(browserTsv, sourceBytes);
        assert.equal(encodeGlyphLibrary(original), sourceBytes);
        for (const [position, stroke] of original.entries()) {
            if (stroke.grapheme !== nfd) {
                assert.deepEqual(state[position], stroke,
                    'non-target exact Unicode slot stays immutable');
            }
        }
        const source = join(dir, 'original.tsv');
        const nativeOutput = join(dir, 'native.tsv');
        writeFileSync(source, sourceBytes);
        const generated = native('glyphs', '--profile', source, ...cli,
            '--output', nativeOutput);
        assert.equal(generated.status, 0, generated.stderr);
        assert.equal(readFileSync(nativeOutput, 'utf8'), browserTsv,
            'twelve native operations exactly match the browser');
        assert.equal(readFileSync(source, 'utf8'), sourceBytes);
        const report = native('glyphs', '--profile', nativeOutput,
            '--inspect-paths', nfd);
        assert.equal(report.status, 0, report.stderr);
        assert.match(report.stdout,
            /Strokes: 6\nPaths: 3\nPen lifts: 2/);

        for (const format of ['pdf', 'png']) {
            const flags = ['--export-paper', format];
            if (format === 'png') flags.push('--dpi', '72');
            const actual = join(dir, 'script.' + format);
            const reference = join(dir, 'manual.' + format);
            const run = (args, target) => native('notebook',
                '--printed-sheet', 'true', ...args,
                '--place-text', '1', '30000', '45000', nfd,
                ...flags, '--output', target);
            const selected = state.filter((stroke) =>
                stroke.grapheme === nfd);
            const manual = selected.flatMap((shape) => [
                '--glyph-stroke', nfd,
                String(shape.x1), String(shape.y1),
                String(shape.x2), String(shape.y2),
                String(shape.width), shape.color,
            ]);
            const output = run(['--glyph-file', nativeOutput], actual);
            const expected = run(manual, reference);
            assert.equal(output.status, 0, output.stderr);
            assert.equal(expected.status, 0, expected.stderr);
            assert.deepEqual(readFileSync(actual), readFileSync(reference),
                'scripted ink has physical ' + format + ' parity');
            if (format === 'png') {
                pngHasTransparentCorner(readFileSync(actual));
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});
