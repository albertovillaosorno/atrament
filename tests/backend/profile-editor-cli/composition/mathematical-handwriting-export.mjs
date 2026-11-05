// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: LicenseRef-Proprietary
// Native Rust PDF/PNG acceptance of independently authored browser math ink.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import test from 'node:test';
import {
    candidatePenPaths, encodeGlyphLibrary, geometricStarter,
    reverseCandidatePath, rotateClosedCandidatePath,
} from '../../../../src/browser/workspace/adapter-inbound/generated/glyph-strokes.js';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const symbols = ['√', 'π', '∞', '≠', '≤', '≥', '∑', '∫', 'Δ', '≈'];
const text = symbols.join('');
const ink = symbols.flatMap((symbol) => geometricStarter(symbol,
    1550, 2450, 1100, 250, '#243647'));

function cli(subcommand, args) {
    const result = spawnSync('cargo', [
        'run', '--offline', '--quiet', '-p', 'atrament_profile_editor_cli',
        '--bin', 'atrament-profiles', '--', subcommand, ...args,
    ], { cwd: root, timeout: 120000, maxBuffer: 16 * 1024 * 1024 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function actualPngInk(bytes) {
    assert.deepEqual(bytes.subarray(0, 8),
        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
    assert.equal(bytes.readUInt32BE(16), 596);
    assert.equal(bytes.readUInt32BE(20), 842);
    assert.equal(bytes[25], 6, 'transparent physical RGBA output');
    const chunks = [];
    for (let offset = 8; offset + 12 <= bytes.length;) {
        const length = bytes.readUInt32BE(offset);
        if (bytes.toString('ascii', offset + 4, offset + 8) === 'IDAT') {
            chunks.push(bytes.subarray(offset + 8, offset + 8 + length));
        }
        offset += length + 12;
    }
    assert.ok(chunks.length, 'PNG raster data is present');
    const raw = inflateSync(Buffer.concat(chunks));
    assert.equal(raw[4], 0, 'transparent corner has zero alpha');
    assert.ok(raw.includes(Buffer.from([36, 54, 71, 255])),
        'physical PNG contains caller-authored mathematical ink');
    return raw;
}

test('14 direct NFD accent starters render exact Rust PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/nfd-ink-'));
        try {
            const library = join(dir, 'accents.tsv');
            const output = join(dir, 'accents.pdf');
            const nfc = ['á', 'é', 'í', 'ó', 'ú', 'ñ', 'ü',
                'Á', 'É', 'Í', 'Ó', 'Ú', 'Ñ', 'Ü'];
            const nfd = nfc.map((glyph) => glyph.normalize('NFD'));
            const glyphs = [...nfc, ...nfd];
            const rows = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            const expected = Buffer.from(encodeGlyphLibrary(rows));
            writeFileSync(library, expected);
            const reopened = cli('glyphs', ['--profile', library]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, expected);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', glyphs.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 28/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library,
                '--flow-text', `${nfc.join('')}\n${nfd.join('')}`];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, rows.length,
            'two rows retain exact NFC and NFD directed pen segments');
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const unsupported = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', library,
                '--flow-text', `A\u0300${nfd.join('')}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(unsupported.status, 0);
            assert.match(unsupported.stderr.toString(),
                /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted,
                'unknown exact NFD target cannot replace accepted PDF');
            assert.deepEqual(readFileSync(library), expected,
                'native admission and export never normalize TSV identities');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('styled NFD acute pen paths export exact Rust PDF and PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/nfd-styled-'));
    try {
        const library = join(dir, 'styled.tsv');
        const output = join(dir, 'styled.pdf');
        const styles = [
            ['a\u0301', 'two-storey-italic', 700],
            ['o\u0301', 'cursive', 1100],
        ];
        const rows = styles.flatMap(([glyph, kind, radius]) =>
            geometricStarter(glyph, 1550, 2450, radius, 250,
                '#243647', undefined, kind));
        for (const [glyph, kind, radius] of styles) {
            const selected = rows.filter((line) => line.grapheme === glyph);
            assert.ok(selected.length > 5);
            assert.notDeepEqual(selected, geometricStarter(glyph,
                1550, 2450, radius, 250, '#243647'));
            const paths = candidatePenPaths(rows, glyph);
            assert.ok(paths.length > 0,
                `${kind} must retain directed caller-authored pen paths`);
            assert.equal(paths.at(-1).last, selected.length);
        }
        const canonical = Buffer.from(encodeGlyphLibrary(rows));
        writeFileSync(library, canonical);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, canonical);
        const glyphText = styles.map(([glyph]) => glyph).join('');
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', glyphText]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(),
            /Distinct visible graphemes: 2/);
        const flags = ['--printed-sheet', 'true',
            '--glyph-file', library,
            '--flow-text', `${glyphText}\n${glyphText}`];
        const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
            '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, rows.length * 2);
        const png = cli('notebook', [...flags, '--export-paper', 'png',
            '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        const refused = cli('notebook', [
            '--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', 'a\u0300', '--export-paper', 'pdf',
            '--output', output, '--replace']);
        assert.notEqual(refused.status, 0);
        assert.match(refused.stderr.toString(), /missing authored glyph/i);
        assert.deepEqual(readFileSync(output), accepted);
        assert.deepEqual(readFileSync(library), canonical);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('seven selected uppercase accents render real native PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/uppercase-ink-'));
        try {
            const library = join(dir, 'capitals.tsv');
            const output = join(dir, 'capitals.pdf');
            const symbols = ['Á', 'É', 'Í', 'Ó', 'Ú', 'Ñ', 'Ü'];
            const rows = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(rows.length, 139);
            const canonical = Buffer.from(encodeGlyphLibrary(rows));
            writeFileSync(library, canonical);
            const reopened = cli('glyphs', ['--profile', library]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, canonical);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library,
                '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
            const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
                '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, 278, 'both rows keep all uppercase pen segments');
            const png = cli('notebook', [...flags, '--export-paper', 'png',
                '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'ÁÉÍÓÚÑÜß',
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
            assert.deepEqual(readFileSync(library), canonical);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('one-click Spanish accented ink reaches real native PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/accented-ink-'));
        try {
            const library = join(dir, 'accents.tsv');
            const output = join(dir, 'accents.pdf');
            const symbols = ['é', 'ú', 'ü', 'ñ'];
            const rows = symbols.flatMap((symbol) => geometricStarter(
                symbol, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(rows.length, 146);
            assert.deepEqual(symbols.map((symbol) =>
                candidatePenPaths(rows, symbol).at(-1)?.last),
            [38, 35, 36, 37]);
            const canonical = Buffer.from(encodeGlyphLibrary(rows));
            writeFileSync(library, canonical);
            const opened = cli('glyphs', ['--profile', library]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, canonical);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', 'éúüñ']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'éúüñ\néúüñ'];
            const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
                '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const bytes = readFileSync(output);
            assert.match(bytes.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((bytes.toString('latin1').match(/ m\n/g) ?? [])
                .length, 292, 'two authored rows preserve every pen segment');
            const png = cli('notebook', [...flags, '--export-paper', 'png',
                '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'éúüñß',
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), bytes,
                'unknown Unicode source never overwrites accepted PDF');
            assert.deepEqual(readFileSync(library), canonical,
                'native export never rewrites the browser-authored library');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('peso/dollar and euro manual stroke ink reaches real Rust PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/currency-ink-'));
        try {
            const path = join(dir, 'currency.tsv');
            const output = join(dir, 'currency.pdf');
            const authored = ['$', '€'].flatMap((symbol) =>
                geometricStarter(symbol, 1550, 2450, 1100,
                    250, '#243647'));
            assert.equal(authored.length, 103);
            assert.deepEqual(candidatePenPaths(authored, '$').map(
                ({ first, last }) => [first, last]),
            [[1, 64], [65, 65]]);
            assert.deepEqual(candidatePenPaths(authored, '€').map(
                ({ first, last }) => [first, last]),
            [[1, 36], [37, 37], [38, 38]]);
            const original = Buffer.from(encodeGlyphLibrary(authored));
            writeFileSync(path, original);
            const opened = cli('glyphs', ['--profile', path]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, original);
            const audit = cli('glyphs', ['--profile', path,
                '--audit-text', '$€']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', path, '--flow-text', '$€\n$€'];
            const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
                '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, 206, 'two rows with 103 exact authored pen segments');
            const png = cli('notebook', [...flags, '--export-paper', 'png',
                '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const refused = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', path,
                '--flow-text', '$€£', '--export-paper', 'pdf',
                '--output', output, '--replace',
            ]);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted,
                'unmapped £ cannot replace the last accepted PDF');
            assert.deepEqual(readFileSync(path), original,
                'native export does not mutate browser-authored strokes');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('degree and percent manual rings render native PDF and RGBA PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-percent-'));
        try {
            const path = join(dir, 'degree-percent.tsv');
            const output = join(dir, 'degree-percent.pdf');
            const degrees = geometricStarter('°', 1550, 2450,
                1100, 250, '#243647');
            const percent = geometricStarter('%', 1550, 2450,
                1100, 250, '#243647');
            const authored = [...degrees, ...percent];
            assert.equal(authored.length, 145);
            assert.deepEqual(candidatePenPaths(authored, '%').map(
                ({ first, last }) => [first, last]),
            [[1, 48], [49, 96], [97, 97]]);
            const canonical = Buffer.from(encodeGlyphLibrary(authored));
            writeFileSync(path, canonical);
            const opened = cli('glyphs', ['--profile', path]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, canonical);
            const audit = cli('glyphs', ['--profile', path,
                '--audit-text', '°%']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', path, '--flow-text', '°%\n°%'];
            const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
                '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const expected = readFileSync(output);
            assert.match(expected.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((expected.toString('latin1').match(/ m\n/g) ?? [])
                .length, 290, 'two rows contain 145 directed pen lines');
            const png = cli('notebook', [...flags, '--export-paper', 'png',
                '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const refused = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', path,
                '--flow-text', '°%∝', '--export-paper', 'pdf',
                '--output', output, '--replace',
            ]);
            assert.notEqual(refused.status, 0,
                'missing neighboring math graphemes need their own ink');
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), expected,
                'refused shape cannot replace an accepted physical PDF');
            assert.deepEqual(readFileSync(path), canonical,
                'the independently authored TSV is never mutated');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('distinct ± and ∓ ink reach native PDF/PNG and refuse ∔', () => {
    const dir = mkdtempSync(join(root, '.temp/plus-minus-ink-'));
    try {
        const library = join(dir, 'plus-minus.tsv');
        const output = join(dir, 'plus-minus.pdf');
        const rows = ['±', '∓'].flatMap((symbol) =>
            geometricStarter(symbol, 1550, 2450, 1100,
                250, '#243647'));
        assert.equal(rows.length, 6,
            'each symbol has two bars and an independent central stem');
        for (const symbol of ['±', '∓']) {
            assert.deepEqual(candidatePenPaths(rows, symbol).map(
                ({ first, last }) => [first, last]),
            [[1, 1], [2, 2], [3, 3]],
            `${symbol} has three independent pen lifts`);
        }
        const canonical = Buffer.from(encodeGlyphLibrary(rows));
        writeFileSync(library, canonical);
        const reopen = cli('glyphs', ['--profile', library]);
        assert.equal(reopen.status, 0, reopen.stderr.toString());
        assert.deepEqual(reopen.stdout, canonical);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', '±∓']);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        const flags = ['--printed-sheet', 'true',
            '--glyph-file', library, '--flow-text', '±∓\n∓±'];
        const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
            '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, 12, 'four glyph occurrences retain three strokes each');
        const png = cli('notebook', [...flags, '--export-paper', 'png',
            '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        const plusOnly = join(dir, 'plus-only.tsv');
        writeFileSync(plusOnly, Buffer.from(encodeGlyphLibrary(
            rows.filter((row) => row.grapheme === '±'))));
        const missingCounterpart = cli('notebook', [
            '--printed-sheet', 'true', '--glyph-file', plusOnly,
            '--flow-text', '±∓', '--export-paper', 'pdf',
            '--output', output, '--replace']);
        assert.notEqual(missingCounterpart.status, 0,
            '∓ cannot borrow ± ink without an explicit fallback');
        assert.match(missingCounterpart.stderr.toString(),
            /missing authored glyph/i);
        assert.deepEqual(readFileSync(output), accepted);
        const missing = cli('notebook', [
            '--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', '±∓∔', '--export-paper', 'pdf',
            '--output', output, '--replace']);
        assert.notEqual(missing.status, 0);
        assert.match(missing.stderr.toString(), /missing authored glyph/i);
        assert.deepEqual(readFileSync(output), accepted,
            'unadmitted dot-plus has no fallback or replacement side effect');
        assert.deepEqual(readFileSync(library), canonical,
            'Rust import and export never rewrite authored TSV bytes');
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('arrow and proportionality render independent native PDF/PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-arrow-'));
        try {
            const library = join(dir, 'symbols.tsv');
            const output = join(dir, 'symbols.pdf');
            const strokes = ['→', '∝'].flatMap((symbol) =>
                geometricStarter(symbol, 1550, 2450, 1100,
                    250, '#243647'));
            assert.equal(strokes.length, 67,
                'arrow has three marks; proportionality has 64 chords');
            const canonical = Buffer.from(encodeGlyphLibrary(strokes));
            writeFileSync(library, canonical);
            const opened = cli('glyphs', ['--profile', library]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, canonical);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', '→∝']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', '→∝\n∝→'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, 134, 'each physical row has 67 directed strokes');
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const onlyArrow = join(dir, 'arrow.tsv');
            writeFileSync(onlyArrow, Buffer.from(encodeGlyphLibrary(
                strokes.filter((line) => line.grapheme === '→'))));
            for (const [source, ink] of [
                [onlyArrow, '→∝'],
                [library, '→∝↔'],
            ]) {
                const refused = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', ink, '--export-paper', 'pdf',
                    '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i,
                    'no implicit proportional or bidirectional fallback');
                assert.deepEqual(readFileSync(output), accepted,
                    'refusal preserves the accepted physical PDF');
            }
            assert.deepEqual(readFileSync(library), canonical,
                'the native codec never mutates exact Unicode source ink');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('ten math symbols roundtrip TSV and render two real ink rows', () => {
    const dir = mkdtempSync(join(root, '.temp/math-native-'));
    try {
        const library = join(dir, 'math.tsv');
        const output = join(dir, 'math.pdf');
        assert.equal(ink.length, 200, 'directed curves and straight paths');
        const canonical = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, canonical);
        const reopen = cli('glyphs', ['--profile', library]);
        assert.equal(reopen.status, 0, reopen.stderr.toString());
        assert.deepEqual(reopen.stdout, canonical,
            'native Rust reopens every exact Unicode identity and pen order');
        const audit = cli('glyphs', [
            '--profile', library, '--audit-text', text,
        ]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 10/);
        const flags = [
            '--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${text}\n${text}`,
        ];
        const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
            '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const bytes = readFileSync(output);
        const vector = bytes.toString('latin1');
        assert.ok(vector.startsWith('%PDF-1.4'));
        assert.equal((vector.match(/ m\n/g) ?? []).length, 400,
            'each of two physical rows uses 200 authored pen strokes');
        assert.match(vector, /0\.141 0\.212 0\.278 RG/,
            'vector PDF uses the explicit authored RGB pigment');
        const png = cli('notebook', [
            ...flags, '--export-paper', 'png', '--dpi', '72',
        ]);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const invalid of [
            [...flags, '--flow-text', `${text}∂`],
            [...flags, '--flow-text', '√∞', '--pages', '0'],
        ]) {
            const refused = cli('notebook', [
                ...invalid, '--export-paper', 'pdf',
                '--output', output, '--replace',
            ]);
            assert.notEqual(refused.status, 0,
                'missing math or invalid page geometry refuses');
            assert.deepEqual(readFileSync(output), bytes,
                'refusal never replaces accepted native output');
        }
        assert.deepEqual(readFileSync(library), canonical,
            'native rendering never changes the browser-authored TSV');
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});


test('partial derivative ideal curves produce native PDF and transparent PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-derivative-'));
        try {
            const source = join(dir, 'calculus.tsv');
            const output = join(dir, 'calculus.pdf');
            const symbols = ['∂', '∫'];
            const authored = symbols.flatMap((symbol) => geometricStarter(
                symbol, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(authored.length, 112,
                'four partial-derivative and three integral curve groups');
            assert.deepEqual(candidatePenPaths(authored, '∂').map(
                ({ first, last }) => [first, last]), [[1, 64]],
            'partial derivative is a continuous directed ink path');
            const canonical = Buffer.from(encodeGlyphLibrary(authored));
            writeFileSync(source, canonical);
            const reopened = cli('glyphs', ['--profile', source]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, canonical,
                'native Rust reopens exact ∂ and ∫ glyphs unchanged');
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', '∂∫']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', '∂∫\n∂∫'];
            const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
                '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const bytes = readFileSync(output);
            assert.match(bytes.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((bytes.toString('latin1').match(/ m\n/g) ?? [])
                .length, 224, 'two lines of 112 exact vector pen paths');
            const png = cli('notebook', [...flags, '--export-paper', 'png',
                '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const refused = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', source,
                '--flow-text', '∂∫∑', '--export-paper', 'pdf',
                '--output', output, '--replace']);
            assert.notEqual(refused.status, 0,
                'an unauthored summation is never synthesized');
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), bytes,
                'a refused glyph leaves the previously accepted PDF intact');
            assert.deepEqual(readFileSync(source), canonical,
                'rendering cannot mutate browser-authored glyph strokes');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('closed infinity pen-loop entry and reversal match native exact edits',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-loop-'));
        try {
            const sourcePath = join(dir, 'source.tsv');
            const browserPath = join(dir, 'browser.tsv');
            const nativePath = join(dir, 'native.tsv');
            const sentinel = join(dir, 'preserved.tsv');
            const source = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(sourcePath, source);
            writeFileSync(sentinel, 'preserve accepted output');
            const candidate = candidatePenPaths(ink, '∞');
            assert.equal(candidate.length, 1);
            assert.equal(candidate[0].first, 1);
            assert.equal(candidate[0].last, 64);
            assert.deepEqual(candidate[0].start, candidate[0].end,
                'infinity is one authored closed pen-down candidate');
            const inspected = cli('glyphs', [
                '--profile', sourcePath, '--inspect-paths', '∞',
            ]);
            assert.equal(inspected.status, 0, inspected.stderr.toString());
            assert.match(inspected.stdout.toString(),
                /Strokes: 64\nPaths: 1\nPen lifts: 0/);
            assert.match(inspected.stdout.toString(),
                /Path 1: strokes 1\.\.64 \(64 segments\)/);
            const firstInfinity = ink.findIndex((line) =>
                line.grapheme === '∞');
            const browserRotate = rotateClosedCandidatePath(
                ink, '∞', firstInfinity + 16);
            const browserReverse = reverseCandidatePath(
                browserRotate.strokes, '∞', browserRotate.selectedIndex);
            const expected = Buffer.from(encodeGlyphLibrary(
                browserReverse.strokes));
            writeFileSync(browserPath, expected);
            const native = cli('glyphs', [
                '--profile', sourcePath, '--rotate-path', '∞', '1', '17',
                '--reverse-path', '∞', '1', '--output', nativePath,
            ]);
            assert.equal(native.status, 0, native.stderr.toString());
            assert.deepEqual(readFileSync(nativePath), expected,
                'independent browser and native path edit scripts match');
            assert.notDeepEqual(expected, source,
                'explicit entry and direction actually change pen order');
            for (const format of ['pdf', 'png']) {
                const render = (file) => cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', file,
                    '--place-text', '1', '30000', '45000', '∞',
                    '--export-paper', format,
                    ...(format === 'png' ? ['--dpi', '72'] : []),
                ]);
                const nativeInk = render(nativePath);
                const browserInk = render(browserPath);
                assert.equal(nativeInk.status, 0,
                    nativeInk.stderr.toString());
                assert.equal(browserInk.status, 0,
                    browserInk.stderr.toString());
                assert.deepEqual(nativeInk.stdout, browserInk.stdout,
                    `real ${format} ink follows identical rotated pen order`);
            }
            const invalid = cli('glyphs', [
                '--profile', sourcePath, '--rotate-path', '∞', '1', '1',
                '--output', sentinel, '--replace',
            ]);
            assert.notEqual(invalid.status, 0,
                'identity start edit is explicitly refused');
            assert.equal(readFileSync(sentinel).toString(),
                'preserve accepted output');
            const undone = cli('glyphs', [
                '--profile', nativePath, '--reverse-path', '∞', '1',
                '--rotate-path', '∞', '1', '49',
            ]);
            assert.equal(undone.status, 0, undone.stderr.toString());
            assert.deepEqual(undone.stdout, source,
                'exact inverse edit restores original TSV bytes');
            assert.deepEqual(readFileSync(sourcePath), source,
                'editing cannot change the original saved library');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('mathematical Markdown paginates with visible marker and atomic refusal',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-markdown-'));
        try {
            const source = join(dir, 'calculus.md');
            const library = join(dir, 'math.tsv');
            const acceptedPdf = join(dir, 'calculus.pdf');
            const pageOne = join(dir, 'heading.png');
            const pageTwo = join(dir, 'continuation.png');
            const canonical = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, canonical);
            const lines = Array.from({ length: 54 }, () => 'Δ≈  ');
            writeFileSync(source, `# ∑∫\n\n${lines.join('\n')}\n`);
            const flags = [
                '--printed-sheet', 'true', '--glyph-file', library,
                '--flow-markdown-file', source,
                '--heading-style', 'marker',
                '--heading-color', '#F8DE91',
                '--heading-marker-scale', '105',
            ];
            const pdf = cli('notebook', [
                ...flags, '--export-paper', 'pdf', '--output', acceptedPdf,
            ]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const bytes = readFileSync(acceptedPdf);
            const vector = bytes.toString('latin1');
            assert.match(vector, /^%PDF-1\.4/);
            assert.match(vector, /\/Count 2\b/,
                'mathematical hard rows must paginate onto two sheets');
            assert.match(vector, /0\.141 0\.212 0\.278 RG/,
                'all body formulas preserve authored pigment');
            assert.match(vector, /0\.973 0\.871 0\.569 RG/,
                'heading marker preserves its independently selected ink');
            const exportPage = (page, output) => cli('notebook', [
                ...flags, '--export-paper', 'png', '--png-page', page,
                '--dpi', '72', '--output', output,
            ]);
            const cover = exportPage('1', pageOne);
            assert.equal(cover.status, 0, cover.stderr.toString());
            const first = actualPngInk(readFileSync(pageOne));
            assert.ok(first.includes(Buffer.from([248, 222, 145, 255])),
                'first page has authored pastel marker under its heading');
            const continuation = exportPage('2', pageTwo);
            assert.equal(continuation.status, 0,
                continuation.stderr.toString());
            const second = actualPngInk(readFileSync(pageTwo));
            assert.ok(!second.includes(Buffer.from([248, 222, 145, 255])),
                'marker does not silently repeat on page two');
            const overScale = [...flags];
            overScale[overScale.indexOf('105')] = '106';
            for (const [invalid, reason] of [
                [[...flags, '--pages', '1'], /flow requires 2 physical pages/],
                [overScale, /marker scale|heading-marker-scale/],
            ]) {
                const refused = cli('notebook', [
                    ...invalid, '--export-paper', 'pdf',
                    '--output', acceptedPdf, '--replace',
                ]);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(), reason);
                assert.deepEqual(readFileSync(acceptedPdf), bytes,
                    'refusal cannot truncate accepted paginated math ink');
            }
            writeFileSync(source, '# ∑∫\n\nΔ≈∂  \n');
            const missing = cli('notebook', [
                ...flags, '--export-paper', 'pdf',
                '--output', acceptedPdf, '--replace',
            ]);
            assert.notEqual(missing.status, 0,
                'unauthored differential symbol must not be synthesized');
            assert.deepEqual(readFileSync(acceptedPdf), bytes);
            assert.deepEqual(readFileSync(library), canonical,
                'multi-page Markdown never rewrites source glyph geometry');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('manual slash and curly notation survives native PDF PNG and refusal',
    () => {
        const dir = mkdtempSync(join(root, '.temp/notation-native-'));
        try {
            const source = join(dir, 'notation.tsv');
            const output = join(dir, 'notation.pdf');
            const symbols = ['/', '\\', '{', '}'];
            const text = symbols.join('');
            const authored = symbols.flatMap((symbol) => geometricStarter(
                symbol, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(authored.length, 130,
                'two directed slash strokes and two 64-chord curly braces');
            const canonical = Buffer.from(encodeGlyphLibrary(authored));
            writeFileSync(source, canonical);
            const reopened = cli('glyphs', ['--profile', source]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, canonical,
                'Rust accepts exact backslash and brace TSV unchanged');
            const audit = cli('glyphs', [
                '--profile', source, '--audit-text', text,
            ]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 4/);
            const flags = [
                '--printed-sheet', 'true', '--glyph-file', source,
                '--flow-text', `${text}\n${text}`,
            ];
            const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
                '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const bytes = readFileSync(output);
            const vector = bytes.toString('latin1');
            assert.match(vector, /^%PDF-1\.4/);
            assert.equal((vector.match(/ m\n/g) ?? []).length, 260,
                'two physical rows use every directed authored pen segment');
            const png = cli('notebook', [
                ...flags, '--export-paper', 'png', '--dpi', '72',
            ]);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const refused = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', source,
                '--flow-text', `${text}%`, '--export-paper', 'pdf',
                '--output', output, '--replace',
            ]);
            assert.notEqual(refused.status, 0,
                'unapproved percent glyph must not inherit other ink');
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), bytes,
                'a refused notation edit never replaces an accepted PDF');
            assert.deepEqual(readFileSync(source), canonical,
                'native exports never modify caller-owned notation TSV');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('exact comparison chevrons render native PDF PNG and refuse fallback',
    () => {
        const dir = mkdtempSync(join(root, '.temp/comparison-ink-'));
        try {
            const source = join(dir, 'comparisons.tsv');
            const output = join(dir, 'comparisons.pdf');
            const symbols = ['<', '>', '≤', '≥'];
            const rows = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(rows.length, 10);
            const bytes = Buffer.from(encodeGlyphLibrary(rows));
            writeFileSync(source, bytes);
            const reopened = cli('glyphs', ['--profile', source]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, bytes);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 4/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source,
                '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, rows.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const refused = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', source,
                '--flow-text', '<>≪', '--export-paper', 'pdf',
                '--output', output, '--replace',
            ]);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
            assert.deepEqual(readFileSync(source), bytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('left and bidirectional arrow ink renders Rust PDF PNG and refuses',
    () => {
        const dir = mkdtempSync(join(root, '.temp/arrow-ink-'));
        try {
            const source = join(dir, 'arrows.tsv');
            const output = join(dir, 'arrows.pdf');
            const symbols = ['→', '←', '↔'];
            const strokes = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(strokes.length, 11);
            const expected = Buffer.from(encodeGlyphLibrary(strokes));
            writeFileSync(source, expected);
            const reopened = cli('glyphs', ['--profile', source]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, expected);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 3/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source,
                '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, strokes.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            const missing = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', source,
                '--flow-text', '←↔↦', '--export-paper', 'pdf',
                '--output', output, '--replace',
            ]);
            assert.notEqual(missing.status, 0);
            assert.match(missing.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
            assert.deepEqual(readFileSync(source), expected);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('vertical arrows render native PDF PNG with exact refusal', () => {
    const dir = mkdtempSync(join(root, '.temp/vertical-arrow-'));
    try {
        const source = join(dir, 'vertical.tsv');
        const output = join(dir, 'vertical.pdf');
        const symbols = ['↑', '↓', '↕'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 11);
        const canonical = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(source, canonical);
        const opened = cli('glyphs', ['--profile', source]);
        assert.equal(opened.status, 0, opened.stderr.toString());
        assert.deepEqual(opened.stdout, canonical);
        const audit = cli('glyphs', ['--profile', source,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(),
            /Distinct visible graphemes: 3/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', source,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const exported = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(exported.status, 0, exported.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        const refused = cli('notebook', ['--printed-sheet', 'true',
            '--glyph-file', source, '--flow-text', '↑↕↥',
            '--export-paper', 'pdf', '--output', output, '--replace']);
        assert.notEqual(refused.status, 0);
        assert.match(refused.stderr.toString(), /missing authored glyph/i);
        assert.deepEqual(readFileSync(output), accepted);
        assert.deepEqual(readFileSync(source), canonical);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});


test('exact logical wedge ink exports native PDF PNG and refuses alias',
    () => {
        const dir = mkdtempSync(join(root, '.temp/logical-ink-'));
        try {
            const source = join(dir, 'logical.tsv');
            const output = join(dir, 'logical.pdf');
            const symbols = ['∧', '∨'];
            const ink = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 4);
            const bytes = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(source, bytes);
            const reopened = cli('glyphs', ['--profile', source]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, bytes);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source,
                '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['v', '⊕']) {
                const refused = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', `∧∨${missing}`, '--export-paper', 'pdf',
                    '--output', output, '--replace',
                ]);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(source), bytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('intersection and union curves render native PDF PNG and refuse',
    () => {
        const dir = mkdtempSync(join(root, '.temp/set-curves-'));
        try {
            const source = join(dir, 'sets.tsv');
            const output = join(dir, 'sets.pdf');
            const symbols = ['∩', '∪'];
            const ink = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 64);
            const bytes = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(source, bytes);
            const opened = cli('glyphs', ['--profile', source]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, bytes);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source,
                '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const absent of ['∧', '∈']) {
                const refused = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', `∩∪${absent}`, '--export-paper', 'pdf',
                    '--output', output, '--replace',
                ]);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(source), bytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('proper and inclusive subset glyphs render exact native PDF PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/subset-notation-'));
        try {
            const source = join(dir, 'subset.tsv');
            const output = join(dir, 'subset.pdf');
            const symbols = ['⊂', '⊆'];
            const rows = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(rows.length, 129);
            const bytes = Buffer.from(encodeGlyphLibrary(rows));
            writeFileSync(source, bytes);
            const opened = cli('glyphs', ['--profile', source]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, bytes);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            assert.match(audit.stdout.toString(),
                /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', '⊂⊆\n⊆⊂'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, rows.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['∈', '⊊']) {
                const refused = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', '⊂⊆' + missing,
                    '--export-paper', 'pdf', '--output', output, '--replace',
                ]);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(source), bytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('superset signs keep separate identities in native PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/superset-ink-'));
        try {
            const source = join(dir, 'superset.tsv');
            const output = join(dir, 'superset.pdf');
            const symbols = ['⊃', '⊇'];
            const strokes = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(strokes.length, 129);
            const data = Buffer.from(encodeGlyphLibrary(strokes));
            writeFileSync(source, data);
            const reopened = cli('glyphs', ['--profile', source]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, data);
            const audit = cli('glyphs', [
                '--profile', source, '--audit-text', symbols.join(''),
            ]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            assert.match(audit.stdout.toString(),
                /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', '⊃⊇\n⊇⊃'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, strokes.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['⊂', '⊋']) {
                const rejected = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', '⊃⊇' + missing,
                    '--export-paper', 'pdf', '--output', output, '--replace',
                ]);
                assert.notEqual(rejected.status, 0);
                assert.match(rejected.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(source), data);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('negated set relations export physical ink with no Unicode fallback',
    () => {
        const dir = mkdtempSync(join(root, '.temp/negated-sets-'));
        try {
            const source = join(dir, 'sets.tsv');
            const output = join(dir, 'sets.pdf');
            const symbols = ['⊄', '⊈', '⊅', '⊉'];
            const rows = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(rows.length, 262);
            const sourceBytes = Buffer.from(encodeGlyphLibrary(rows));
            writeFileSync(source, sourceBytes);
            const opened = cli('glyphs', ['--profile', source]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, sourceBytes);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 4/);
            assert.match(audit.stdout.toString(),
                /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', '⊄⊈\n⊅⊉'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, rows.length);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['⊂', '⊃', '∉']) {
                const refused = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', '⊄⊈⊅⊉' + missing,
                    '--export-paper', 'pdf', '--output', output, '--replace',
                ]);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(source), sourceBytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('reverse membership ink roundtrips exact Rust PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/contains-ink-'));
        try {
            const source = join(dir, 'contains.tsv');
            const output = join(dir, 'contains.pdf');
            const symbols = ['∋', '∌'];
            const rows = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(rows.length, 67);
            const bytes = Buffer.from(encodeGlyphLibrary(rows));
            writeFileSync(source, bytes);
            const opened = cli('glyphs', ['--profile', source]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, bytes);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            assert.match(audit.stdout.toString(),
                /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', '∋∌\n∌∋'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, rows.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['∈', '∉', '⊂']) {
                const refused = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', '∋∌' + missing,
                    '--export-paper', 'pdf', '--output', output, '--replace',
                ]);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(source), bytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('membership and nonmembership export distinct native PDF PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/membership-'));
        try {
            const source = join(dir, 'membership.tsv');
            const output = join(dir, 'membership.pdf');
            const symbols = ['∈', '∉'];
            const ink = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 67);
            const bytes = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(source, bytes);
            const opened = cli('glyphs', ['--profile', source]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, bytes);
            const audit = cli('glyphs', ['--profile', source,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            assert.match(audit.stdout.toString(),
                /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', source,
                '--flow-text', '∈∉\n∈∉'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['∩', '∋']) {
                const refused = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', source,
                    '--flow-text', '∈∉' + missing,
                    '--export-paper', 'pdf', '--output', output,
                    '--replace',
                ]);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(source), bytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('empty set and identity notation export exact native PDF PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-identity-'));
    try {
        const source = join(dir, 'identity.tsv');
        const output = join(dir, 'identity.pdf');
        const symbols = ['∅', '∇', '≡', '≢'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 75);
        const bytes = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(source, bytes);
        const reopened = cli('glyphs', ['--profile', source]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, bytes);
        const audit = cli('glyphs', ['--profile', source,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', source,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['Ø', '⊕']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', `∅≡${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted,
                'unmatched exact Unicode never replaces printed output');
        }
        assert.deepEqual(readFileSync(source), bytes);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('four school geometry signs render native PDF PNG without aliases', () => {
    const dir = mkdtempSync(join(root, '.temp/math-geometry-'));
    try {
        const source = join(dir, 'geometry.tsv');
        const output = join(dir, 'geometry.pdf');
        const symbols = ['∥', '⊥', '∼', '≅'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 70);
        const bytes = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(source, bytes);
        const reopened = cli('glyphs', ['--profile', source]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, bytes);
        const audit = cli('glyphs', ['--profile', source,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', source,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['=', '≈', '⊤']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', `∥⊥${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(source), bytes);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('quantifier and therefore Unicode ink render native PDF PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-quantifiers-'));
    try {
        const source = join(dir, 'quantifiers.tsv');
        const output = join(dir, 'quantifiers.pdf');
        const symbols = ['∀', '∃', '∄', '∴'];
        const rows = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(rows.length, 15);
        const expected = Buffer.from(encodeGlyphLibrary(rows));
        writeFileSync(source, expected);
        const reopened = cli('glyphs', ['--profile', source]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, expected);
        const audit = cli('glyphs', ['--profile', source,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', source,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, rows.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const absent of ['∵', '⊤', '=']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', `∃∄${absent}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(source), expected);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Logic marks are caller-authored Unicode, never automatic semantic aliases.
test('because and logic turnstiles roundtrip through Rust PDF and PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-logic-'));
    try {
        const source = join(dir, 'logic.tsv');
        const output = join(dir, 'logic.pdf');
        const symbols = ['∵', '⊤', '⊢', '⊨'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 10);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(source, original);
        const reopened = cli('glyphs', ['--profile', source]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', source,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', source,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['∴', '⊥', '⊬']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', source, '--flow-text', `⊢⊨${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(source), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Geometry and logic negations remain exact, caller-authored Unicode keys.
test('angles and negated turnstiles export in native PDF and PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-angle-logic-'));
    try {
        const library = join(dir, 'angle-logic.tsv');
        const output = join(dir, 'angle-logic.pdf');
        const symbols = ['∠', '∟', '⊬', '⊭'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 11);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['∴', '⊢', '⊨']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', `∠∟${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Unicode double comparisons and their negations are independent ink.
test('double and negated inequality ink roundtrips in Rust PDF PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-double-inequality-'));
    try {
        const library = join(dir, 'inequalities.tsv');
        const output = join(dir, 'inequalities.pdf');
        const symbols = ['≪', '≫', '≮', '≯'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 14);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['<', '>', '≤', '≥']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', `≮≯${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Diagonal arrows and barred comparisons never borrow neighboring keys.
test('negated inclusive comparisons and diagonal arrows roundtrip', () => {
    const dir = mkdtempSync(join(root, '.temp/math-diagonal-arrows-'));
    try {
        const library = join(dir, 'arrows.tsv');
        const output = join(dir, 'arrows.pdf');
        const symbols = ['≰', '≱', '↗', '↘'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 14);
        const source = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, source);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, source);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['≤', '≥', '→', '←']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', `≰≱↗↘${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), source);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Converse double arrow and remaining diagonals use distinct exact ink.
test('converse implication and left diagonals export native ink', () => {
    const dir = mkdtempSync(join(root, '.temp/math-left-diagonals-'));
    try {
        const library = join(dir, 'left.tsv');
        const output = join(dir, 'left.pdf');
        const symbols = ['↖', '↙', '⇐'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 10);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopen = cli('glyphs', ['--profile', library]);
        assert.equal(reopen.status, 0, reopen.stderr.toString());
        assert.deepEqual(reopen.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 3/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['↗', '↘', '⇒', '←']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', `↖↙⇐${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Number-theory division and its negation are distinct native glyph keys.
test('exact divides and not-divides strokes export to Rust PDF PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-divides-'));
    try {
        const library = join(dir, 'divides.tsv');
        const output = join(dir, 'divides.pdf');
        const symbols = ['∣', '∤'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 3);
        assert.deepEqual(candidatePenPaths(ink, '∣').map(
            ({ first, last }) => [first, last]), [[1, 1]]);
        assert.deepEqual(candidatePenPaths(ink, '∤').map(
            ({ first, last }) => [first, last]), [[1, 1], [2, 2]]);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 2/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2, 'both pages retain the lifted slash');
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['|', '∥', '⊥']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', `∣∤${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted,
                'unauthored lookalikes cannot replace accepted PDF');
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Exact math prime and centered multiplication signs cannot alias plain
// quotation, punctuation, or ASCII star glyph ink.
test('prime marks and multiplication operators export native PDF PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-primes-'));
    try {
        const library = join(dir, 'primes.tsv');
        const output = join(dir, 'primes.pdf');
        const symbols = ['′', '″', '‴', '⋅', '∗'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 10);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 5/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ["'", '"', '.', '*']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library,
                '--flow-text', `′″‴⋅∗${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Manually selected inner-product delimiters cannot borrow comparison ink.
test('manual mathematical angle brackets export exact native PDF PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-angle-'));
        try {
            const library = join(dir, 'angles.tsv');
            const output = join(dir, 'angles.pdf');
            const symbols = ['⟨', '⟩'];
            const ink = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 4);
            const original = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, original);
            const reopened = cli('glyphs', ['--profile', library]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, original);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 2/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', '⟨⟩\n⟩⟨'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['<', '>', '〈', '〉']) {
                const refused = cli('notebook', ['--printed-sheet', 'true',
                    '--glyph-file', library, '--flow-text', `⟨${missing}⟩`,
                    '--export-paper', 'pdf', '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(library), original);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Selected Greek variables are authored independently of Latin letter ink.
test('manual Greek math ink renders real native PDF PNG and refuses aliases',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-greek-'));
        try {
            const library = join(dir, 'greek.tsv');
            const output = join(dir, 'greek.pdf');
            const symbols = ['α', 'β', 'θ', 'λ'];
            const ink = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 197);
            const original = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, original);
            const reopened = cli('glyphs', ['--profile', library]);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.deepEqual(reopened.stdout, original);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', symbols.join('')]);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 4/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'αβθλ\nλθβα'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['a', 'b', 'o', 'l', 'ν']) {
                const refused = cli('notebook', ['--printed-sheet', 'true',
                    '--glyph-file', library, '--flow-text', `α${missing}β`,
                    '--export-paper', 'pdf', '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(library), original);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Opt-in mu and gamma are never automatic Latin or Greek fallbacks.
test('manual Greek mu gamma render native PDF PNG and refuse lookalikes',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-greek-mu-'));
        try {
            const library = join(dir, 'mu-gamma.tsv');
            const output = join(dir, 'greek.pdf');
            const glyphs = ['μ', 'γ'];
            const ink = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 67);
            const original = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, original);
            const reopen = cli('glyphs', ['--profile', library]);
            assert.equal(reopen.status, 0, reopen.stderr.toString());
            assert.deepEqual(reopen.stdout, original);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', 'μγγμ']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'μγ\nγμ'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['u', 'y', 'ν']) {
                const refused = cli('notebook', ['--printed-sheet', 'true',
                    '--glyph-file', library, '--flow-text', 'μ' + missing + 'γ',
                    '--export-paper', 'pdf', '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(library), original);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Opt-in lowercase delta and sigma retain independent Unicode pen geometry.
test('manual Greek delta sigma render native PDF PNG without fallback',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-greek-delta-'));
        try {
            const library = join(dir, 'delta-sigma.tsv');
            const output = join(dir, 'greek.pdf');
            const glyphs = ['δ', 'σ'];
            const ink = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 130);
            const original = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, original);
            const reopen = cli('glyphs', ['--profile', library]);
            assert.equal(reopen.status, 0, reopen.stderr.toString());
            assert.deepEqual(reopen.stdout, original);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', 'δσσδ']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'δσ\nσδ'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['o', 'θ', 'ν']) {
                const refused = cli('notebook', ['--printed-sheet', 'true',
                    '--glyph-file', library, '--flow-text', 'δ' + missing + 'σ',
                    '--export-paper', 'pdf', '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(library), original);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Opt-in lowercase rho and tau retain independent Unicode pen geometry.
test('manual Greek rho tau render native PDF PNG without fallback',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-greek-rho-'));
        try {
            const library = join(dir, 'rho-tau.tsv');
            const output = join(dir, 'greek.pdf');
            const glyphs = ['ρ', 'τ'];
            const ink = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 68);
            const original = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, original);
            const reopen = cli('glyphs', ['--profile', library]);
            assert.equal(reopen.status, 0, reopen.stderr.toString());
            assert.deepEqual(reopen.stdout, original);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', 'ρττρ']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'ρτ\nτρ'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['p', 't', 'ν']) {
                const refused = cli('notebook', ['--printed-sheet', 'true',
                    '--glyph-file', library, '--flow-text', 'ρ' + missing + 'τ',
                    '--export-paper', 'pdf', '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(library), original);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Opt-in lowercase epsilon and omega retain independent Unicode pen geometry.
test('manual Greek epsilon omega render native PDF PNG without fallback',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-greek-epsilon-'));
        try {
            const library = join(dir, 'epsilon-omega.tsv');
            const output = join(dir, 'greek.pdf');
            const glyphs = ['ε', 'ω'];
            const ink = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 98);
            const original = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, original);
            const reopen = cli('glyphs', ['--profile', library]);
            assert.equal(reopen.status, 0, reopen.stderr.toString());
            assert.deepEqual(reopen.stdout, original);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', 'εωωε']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'εω\nωε'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['e', 'w', 'ν']) {
                const refused = cli('notebook', ['--printed-sheet', 'true',
                    '--glyph-file', library, '--flow-text', 'ε' + missing + 'ω',
                    '--export-paper', 'pdf', '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(library), original);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Manually selected kappa and chi never inherit a Latin or Greek lookalike.
test('manual Greek kappa chi render native PDF PNG without fallback',
    () => {
        const dir = mkdtempSync(join(root, '.temp/math-greek-kappa-'));
        try {
            const library = join(dir, 'kappa-chi.tsv');
            const output = join(dir, 'greek.pdf');
            const glyphs = ['κ', 'χ'];
            const ink = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, '#243647'));
            assert.equal(ink.length, 65);
            const original = Buffer.from(encodeGlyphLibrary(ink));
            writeFileSync(library, original);
            const reopen = cli('glyphs', ['--profile', library]);
            assert.equal(reopen.status, 0, reopen.stderr.toString());
            assert.deepEqual(reopen.stdout, original);
            const audit = cli('glyphs', ['--profile', library,
                '--audit-text', 'κχχκ']);
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const flags = ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', 'κχ\nχκ'];
            const pdf = cli('notebook', [...flags,
                '--export-paper', 'pdf', '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
            assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
                .length, ink.length * 2);
            const png = cli('notebook', [...flags,
                '--export-paper', 'png', '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            actualPngInk(png.stdout);
            for (const missing of ['k', 'x', 'ν']) {
                const refused = cli('notebook', ['--printed-sheet', 'true',
                    '--glyph-file', library, '--flow-text', 'κ' + missing + 'χ',
                    '--export-paper', 'pdf', '--output', output, '--replace']);
                assert.notEqual(refused.status, 0);
                assert.match(refused.stderr.toString(),
                    /missing authored glyph/i);
                assert.deepEqual(readFileSync(output), accepted);
            }
            assert.deepEqual(readFileSync(library), original);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Floor and ceiling delimiters keep their directed corners across Rust PDF/PNG.
test('four exact floor and ceiling corners export native ink', () => {
    const dir = mkdtempSync(join(root, '.temp/math-floors-'));
    try {
        const library = join(dir, 'corners.tsv');
        const output = join(dir, 'corners.pdf');
        const symbols = ['⌊', '⌋', '⌈', '⌉'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 8);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 4/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `⌊⌋⌈⌉\n⌈⌉⌊⌋`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['[', ']', '⟦', '⟧']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library,
                '--flow-text', `⌊⌉${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Set difference and nonparallel are independent exact native glyph keys.
test('set difference and nonparallel export distinct Rust ink', () => {
    const dir = mkdtempSync(join(root, '.temp/math-not-parallel-'));
    try {
        const library = join(dir, 'difference.tsv');
        const output = join(dir, 'difference.pdf');
        const symbols = ['∖', '∦'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 4);
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 2/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['\\', '∥', '∣']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library,
                '--flow-text', `∖∦${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Function mapsto arrows have an upright stop distinct from ordinary arrows.
test('left and right mapsto arrows retain native pen stops and refusal', () => {
    const dir = mkdtempSync(join(root, '.temp/math-mapsto-'));
    try {
        const library = join(dir, 'mapsto.tsv');
        const output = join(dir, 'mapsto.pdf');
        const symbols = ['↦', '↤'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 8);
        for (const glyph of symbols) {
            assert.deepEqual(candidatePenPaths(ink, glyph).map(
                ({ first, last }) => [first, last]),
            [[1, 1], [2, 2], [3, 4]],
            'mapsto stop, shaft and head must have separate pen lifts');
        }
        const original = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, original);
        const reopened = cli('glyphs', ['--profile', library]);
        assert.equal(reopened.status, 0, reopened.stderr.toString());
        assert.deepEqual(reopened.stdout, original);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 2/);
        const flags = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...flags,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2, 'two rows keep every mapsto pen stop');
        const png = cli('notebook', [...flags,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['←', '→', '↪']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', `↦↤${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted,
                'missing arrow cannot overwrite the accepted PDF');
        }
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// Logical implication retains distinct multi-shaft pen order and Unicode.
test('negation and two implications export exactly in native PDF PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/math-implication-'));
    try {
        const library = join(dir, 'implication.tsv');
        const output = join(dir, 'implication.pdf');
        const symbols = ['¬', '⇒', '⇔'];
        const ink = symbols.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, '#243647'));
        assert.equal(ink.length, 12);
        const source = Buffer.from(encodeGlyphLibrary(ink));
        writeFileSync(library, source);
        const readback = cli('glyphs', ['--profile', library]);
        assert.equal(readback.status, 0, readback.stderr.toString());
        assert.deepEqual(readback.stdout, source);
        const audit = cli('glyphs', ['--profile', library,
            '--audit-text', symbols.join('')]);
        assert.equal(audit.status, 0, audit.stderr.toString());
        assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 3/);
        const args = ['--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', `${symbols.join('')}\n${symbols.join('')}`];
        const pdf = cli('notebook', [...args,
            '--export-paper', 'pdf', '--output', output]);
        assert.equal(pdf.status, 0, pdf.stderr.toString());
        const accepted = readFileSync(output);
        assert.match(accepted.toString('latin1'), /^%PDF-1\.4/);
        assert.equal((accepted.toString('latin1').match(/ m\n/g) ?? [])
            .length, ink.length * 2);
        const png = cli('notebook', [...args,
            '--export-paper', 'png', '--dpi', '72']);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const missing of ['→', '↔', '⊢']) {
            const refused = cli('notebook', ['--printed-sheet', 'true',
                '--glyph-file', library, '--flow-text', `⇒⇔${missing}`,
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted);
        }
        assert.deepEqual(readFileSync(library), source);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});
