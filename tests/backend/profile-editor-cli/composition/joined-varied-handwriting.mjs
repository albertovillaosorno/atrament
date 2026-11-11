// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: MIT
// Real Rust glyph + notebook subprocess acceptance, without a printer.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { inflateSync } from 'node:zlib';
import {
    candidatePenPaths, encodeGlyphLibrary, geometricStarter,
} from '../../../../src/browser/workspace/adapter-inbound/generated/glyph-strokes.js';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const nfd = 'a\u0301';
const nfc = 'ó';
const text = `${nfd}${nfc}${nfd}${nfc}\n${nfd}${nfc}${nfd}${nfc}`;
const variation = [
    '--glyph-variation', '42', '2', '2', '0', '1', '0', '8',
];

function cli(subcommand, args) {
    const result = spawnSync('cargo', [
        'run', '--offline', '--quiet', '-p', 'atrament_profile_editor_cli',
        '--bin', 'atrament-profiles', '--', subcommand, ...args,
    ], { cwd: root, timeout: 120000, maxBuffer: 16 * 1024 * 1024 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function authoredLibrary(output) {
    const rows = [
        [nfd, 250, 900, 1500, 2600],
        [nfd, 1500, 2600, 2900, 2400],
        [nfd, 2850, 1350, 3250, 900],
        [nfc, 250, 900, 1500, 2600],
        [nfc, 1500, 2600, 2900, 2400],
        [nfc, 2850, 1350, 3250, 900],
    ];
    const flags = rows.flatMap((row) => [
        '--glyph-stroke', ...row.map(String), '250', '#102030',
    ]);
    const result = cli('glyphs', [...flags, '--output', output]);
    assert.equal(result.status, 0, result.stderr.toString());
    return readFileSync(output);
}

function pdfPenStarts(bytes) {
    const pdf = bytes.toString('latin1');
    assert.ok(pdf.startsWith('%PDF-1.4'), 'native vector PDF');
    return [...pdf.matchAll(/([0-9.]+) ([0-9.]+) m\n/g)]
        .map((match) => [Number(match[1]), Number(match[2])]);
}

function pngPigment(bytes) {
    assert.deepEqual(bytes.subarray(0, 8),
        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
    const compressed = [];
    for (let index = 8; index + 12 <= bytes.length;) {
        const length = bytes.readUInt32BE(index);
        const type = bytes.toString('ascii', index + 4, index + 8);
        if (type === 'IDAT') {
            compressed.push(bytes.subarray(index + 8, index + 8 + length));
        }
        index += length + 12;
    }
    assert.ok(compressed.length, 'real PNG image data');
    return inflateSync(Buffer.concat(compressed))
        .includes(Buffer.from([16, 32, 48, 255]));
}

test('real two-line accented handwriting joins and varies seeded pen paths',
    () => {
        const dir = mkdtempSync(join(root, '.temp/joined-varied-'));
        try {
            const library = join(dir, 'authored.tsv');
            const pdfPath = join(dir, 'joined.pdf');
            const source = authoredLibrary(library);
            const base = [
                '--printed-sheet', 'true', '--glyph-file', library,
                '--flow-text', text, '--join-text-pair', nfd, nfc,
            ];
            const render = (format, extra = []) => cli('notebook', [
                ...base, ...extra, '--export-paper', format,
                ...(format === 'png' ? ['--dpi', '72'] : []),
            ]);
            const ideal = render('pdf');
            assert.equal(ideal.status, 0, ideal.stderr.toString());
            const sampled = render('pdf', variation);
            assert.equal(sampled.status, 0, sampled.stderr.toString());
            const starts = pdfPenStarts(ideal.stdout);
            assert.equal(starts.length, 28,
                '24 authored glyph segments plus four joins');
            assert.notDeepEqual(starts.slice(0, 14), starts.slice(14),
                'each accented row has distinct physical y positions');
            assert.notDeepEqual(sampled.stdout, ideal.stdout,
                'seeded variation visibly changes pen trajectories');
            assert.deepEqual(sampled.stdout, render('pdf', variation).stdout,
                'seed and text deterministically replay the same ink');
            const newSeed = [...variation];
            newSeed[1] = '43';
            assert.notDeepEqual(sampled.stdout, render('pdf', newSeed).stdout,
                'the selected seed changes the occurrence samples');
            assert.equal(pdfPenStarts(sampled.stdout).length,
                28, 'seeded joins never duplicate or omit ink segments');
            const unjoined = cli('notebook', [
                '--printed-sheet', 'true', '--glyph-file', library,
                '--flow-text', text, ...variation, '--export-paper', 'pdf',
            ]);
            assert.equal(unjoined.status, 0, unjoined.stderr.toString());
            assert.equal(pdfPenStarts(unjoined.stdout).length,
                24, 'no inferred join without an explicit pair');
            const png = render('png', variation);
            assert.equal(png.status, 0, png.stderr.toString());
            assert.ok(pngPigment(png.stdout),
                'transparent PNG contains the authored dark pen pigment');
            assert.deepEqual(readFileSync(library), source,
                'variation and row joins cannot modify canonical pen data');
            // The final destination is replaced only after complete admission.
            const accepted = cli('notebook', [
                ...base, ...variation, '--export-paper', 'pdf',
                '--output', pdfPath,
            ]);
            assert.equal(accepted.status, 0, accepted.stderr.toString());
            const preserved = readFileSync(pdfPath);
            for (const invalid of [
                ['--glyph-variation', '42', '21', '2', '0', '1', '0', '8'],
                [...variation, '--join-text-pair', nfc, nfc],
                [...variation, '--join-text-pair', 'ñ', nfc],
            ]) {
                const refused = cli('notebook', [
                    ...base, ...invalid, '--export-paper', 'pdf',
                    '--output', pdfPath, '--replace',
                ]);
                assert.notEqual(refused.status, 0,
                    `unsafe join or spread must refuse: ${invalid}`);
                assert.deepEqual(readFileSync(pdfPath), preserved,
                    'refusal preserves previously accepted PDF bytes');
            }
            // Stroke ordering and direction are explicit and reversible.
            // Breaking the selected pen exit must refuse the cursive join.
            for (const [flag, ordinal] of [
                ['--reverse-stroke', '3'],
                ['--move-stroke', '3'],
            ]) {
                const edited = join(dir, `${flag.slice(2)}.tsv`);
                const edit = [
                    '--profile', library, flag, nfd, ordinal,
                    ...(flag === '--move-stroke' ? ['1'] : []),
                ];
                const changed = cli('glyphs', [
                    ...edit, '--output', edited,
                ]);
                assert.equal(changed.status, 0, changed.stderr.toString());
                assert.notDeepEqual(readFileSync(edited), source);
                const recovery = cli('glyphs', [
                    '--profile', edited, flag, nfd,
                    ...(flag === '--move-stroke' ? ['1', '3'] : ['3']),
                ]);
                assert.equal(recovery.status, 0,
                    recovery.stderr.toString());
                assert.deepEqual(recovery.stdout, source,
                    'explicit inverse edit recovers exact authored TSV');
                const broken = cli('notebook', [
                    '--printed-sheet', 'true', '--glyph-file', edited,
                    '--flow-text', text, '--join-text-pair', nfd, nfc,
                    ...variation, '--export-paper', 'pdf',
                    '--output', pdfPath, '--replace',
                ]);
                assert.notEqual(broken.status, 0,
                    'unsafe physical join after edit refuses rendering');
                assert.deepEqual(readFileSync(pdfPath), preserved);
                assert.deepEqual(readFileSync(library), source);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('manually selected cursive o/a accents render exact physical native ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/cursive-native-'));
        try {
            const file = join(dir, 'cursive.tsv');
            const output = join(dir, 'cursive.pdf');
            const variants = ['ó', 'a'].flatMap((glyph) =>
                geometricStarter(glyph, 1550, 2450, 700,
                    250, '#102030', undefined, 'cursive'));
            assert.deepEqual(variants.map(({ grapheme }) => grapheme)
                .filter((value, i, all) => i === 0 || value !== all[i - 1]),
            ['ó', 'a']);
            assert.equal(candidatePenPaths(variants, 'ó').length, 2,
                'cursive acute still lifts for an explicit accent');
            assert.equal(candidatePenPaths(variants, 'a').length, 1,
                'cursive a connects entry, circle, stem and exit');
            const saved = Buffer.from(encodeGlyphLibrary(variants));
            writeFileSync(file, saved);
            const opened = cli('glyphs', ['--profile', file]);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.deepEqual(opened.stdout, saved,
                'native Rust preserves every exact TSV stroke and pen order');
            const flags = ['--printed-sheet', 'false', '--glyph-file', file,
                '--flow-text', 'óa\nóa'];
            const pdf = cli('notebook', [...flags, '--export-paper', 'pdf',
                '--output', output]);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = readFileSync(output);
            const vector = accepted.toString('latin1');
            assert.match(vector, /^%PDF-1\.4/);
            assert.equal((vector.match(/0\.063 0\.125 0\.188 RG/g)
                ?? []).length, 302,
            'two rows of 75+76 exact colored pen segments');
            const png = cli('notebook', [...flags, '--export-paper', 'png',
                '--dpi', '72']);
            assert.equal(png.status, 0, png.stderr.toString());
            assert.equal(pngPigment(png.stdout), true,
                'native transparent PNG contains the selected cursive ink');
            const repeated = cli('notebook', [...flags, '--export-paper', 'pdf']);
            assert.equal(repeated.status, 0, repeated.stderr.toString());
            assert.deepEqual(repeated.stdout, accepted,
                'manual variant exports deterministically without changes');
            const missing = cli('notebook', ['--printed-sheet', 'false',
                '--glyph-file', file, '--flow-text', 'óñ',
                '--export-paper', 'pdf', '--output', output, '--replace']);
            assert.notEqual(missing.status, 0,
                'unwritten neighbor cannot become an implicit cursive join');
            assert.match(missing.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(readFileSync(output), accepted,
                'missing glyph refuses without replacing selected export');
            assert.deepEqual(readFileSync(file), saved,
                'native export cannot mutate the browser stroke draft');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('selected join anchors bypass detached accents', () => {
    const dir = mkdtempSync(join(root, '.temp/accent-join-'));
    try {
        const destination = join(dir, 'joined.pdf');
        const shapes = [
            ['ó', 300, 2500, 3200, 2500],
            ['ó', 1700, 1100, 2200, 800],
            ['a', 100, 800, 400, 900],
            ['a', 350, 2500, 1900, 2500],
        ].flatMap((row) => [
            '--glyph-stroke', ...row.map(String), '200', '#102030',
        ]);
        const base = ['--printed-sheet', 'true', ...shapes,
            '--place-text', '1', '25000', '40000', 'óa'];
        const print = (...extra) => cli('notebook', [
            ...base, ...extra, '--export-paper', 'pdf',
        ]);
        const ordinary = print('--join-text-pair', 'ó', 'a');
        assert.notEqual(ordinary.status, 0,
            'the default last stroke is a detached accent, not a pen exit');
        assert.match(ordinary.stderr.toString(), /join gap/i);
        const selected = ['--join-text-anchors', 'ó', 'a', '1', '2'];
        const joined = print(...selected);
        assert.equal(joined.status, 0, joined.stderr.toString());
        const starts = pdfPenStarts(joined.stdout);
        assert.equal(starts.length, 5,
            'four original lines plus exactly one selected bridge');
        assert.ok(Math.abs((starts[2][0] - starts[0][0])
            - (2900 * 72 / 25400)) < 0.01,
        'bridge starts at the first base-stroke exit, not the acute');
        assert.equal(starts[2][1], starts[0][1],
            'selected exit remains on the unaccented pen baseline');
        const segments = [...joined.stdout.toString('latin1').matchAll(
            /([0-9.]+) ([0-9.]+) m\n([0-9.]+) ([0-9.]+) l\n/g,
        )];
        assert.equal(segments.length, 5);
        assert.deepEqual(segments[2].slice(3, 5),
            segments[4].slice(1, 3),
            'bridge ends at selected second a stroke, not first lifted mark');
        const png = cli('notebook', [
            ...base, ...selected, '--export-paper', 'png', '--dpi', '72',
        ]);
        assert.equal(png.status, 0, png.stderr.toString());
        assert.ok(pngPigment(png.stdout),
            'transparent PNG renders the same opt-in anchored ink');
        const plain = print();
        assert.equal(plain.status, 0, plain.stderr.toString());
        assert.equal(pdfPenStarts(plain.stdout).length, 4);
        assert.deepEqual(print(...selected).stdout, joined.stdout,
            'explicit anchors replay identically without editing the glyphs');
        const accepted = cli('notebook', [...base, ...selected,
            '--export-paper', 'pdf', '--output', destination]);
        assert.equal(accepted.status, 0, accepted.stderr.toString());
        const original = readFileSync(destination);
        for (const bad of [
            ['--join-text-anchors', 'ó', 'a', '2', '2'],
            ['--join-text-anchors', 'ó', 'a', '3', '2'],
            ['--join-text-anchors', 'ó', 'a', '0', '2'],
            ['--join-text-anchors', 'ó', 'a', '1', '3'],
            ['--join-text-anchors', 'ó', 'a', '1', '1'],
            [...selected, '--join-text-pair', 'ó', 'a'],
            ['--join-text-anchors', 'a', 'ó', '1', '1'],
        ]) {
            const refused = cli('notebook', [
                ...base, ...bad, '--export-paper', 'pdf',
                '--output', destination, '--replace',
            ]);
            assert.notEqual(refused.status, 0,
                `bad or absent anchors must refuse: ${bad}`);
            assert.deepEqual(readFileSync(destination), original,
                'refusal preserves the accepted destination PDF');
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});
