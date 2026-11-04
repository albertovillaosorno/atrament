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
import { encodeGlyphLibrary, geometricStarter } from
    '../../src/browser/workspace/adapter-inbound/generated/glyph-strokes.js';

const root = fileURLToPath(new URL('../../', import.meta.url));
const symbols = ['√', 'π', '∞', '≠', '≤', '≥'];
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
}

test('six math symbols roundtrip TSV and render two real ink rows', () => {
    const dir = mkdtempSync(join(root, '.temp/math-native-'));
    try {
        const library = join(dir, 'math.tsv');
        const output = join(dir, 'math.pdf');
        assert.equal(ink.length, 81, 'four quadratic lobes are 64 lines');
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
        assert.match(audit.stdout.toString(), /Distinct visible graphemes: 6/);
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
        assert.equal((vector.match(/ m\n/g) ?? []).length, 162,
            'each of two physical rows uses 81 exact authored pen strokes');
        assert.match(vector, /0\.141 0\.212 0\.278 RG/,
            'vector PDF uses the explicit authored RGB pigment');
        const png = cli('notebook', [
            ...flags, '--export-paper', 'png', '--dpi', '72',
        ]);
        assert.equal(png.status, 0, png.stderr.toString());
        actualPngInk(png.stdout);
        for (const invalid of [
            [...flags, '--flow-text', `${text}∑`],
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
