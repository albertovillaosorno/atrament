// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: MIT
// Native CLI acceptance for explicit curved connections between glyphs.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const source = [
    '--printed-sheet', 'true',
    '--glyph-stroke', 'a', '200', '1000', '3200', '2200',
    '250', '#102030',
    '--glyph-stroke', 'o', '200', '2200', '1200', '1000',
    '250', '#102030',
    '--place-text', '1', '30000', '45000', 'ao',
];
const curve = ['--join-text-curved', 'a', 'o', '1', '1', '0', '-400'];

function notebook(args) {
    const result = spawnSync('cargo', [
        'run', '--locked', '--offline', '--quiet',
        '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
        '--', 'notebook', ...args,
    ], { cwd: root, timeout: 120000, maxBuffer: 16 * 1024 * 1024 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function admitted(args) {
    const result = notebook(args);
    assert.equal(result.status, 0, result.stderr.toString());
    return result.stdout;
}

const expectedInk = [
    'atrament.page-ink.v1',
    '1\t30200\t46000\t33200\t47200\t250\t#102030',
    '1\t33200\t47200\t33450\t47050\t250\t#102030',
    '1\t33450\t47050\t33700\t47000\t250\t#102030',
    '1\t33700\t47000\t33950\t47050\t250\t#102030',
    '1\t33950\t47050\t34200\t47200\t250\t#102030',
    '1\t34200\t47200\t35200\t46000\t250\t#102030',
    '',
].join('\n');

test('curved join follows adjacent glyph endpoints with four quadratic pieces', () => {
    const bytes = admitted([...source, ...curve, '--emit-ink-file']);
    assert.equal(bytes.toString(), expectedInk);
    assert.deepEqual(bytes, admitted([...source, ...curve, '--emit-ink-file']));
    assert.notDeepEqual(bytes, admitted([...source, '--emit-ink-file']));
});

test('vector PDF and transparent PNG consume the same curved physical ink', () => {
    for (const format of ['pdf', 'png']) {
        const suffix = format === 'png' ? ['--dpi', '72'] : [];
        const curved = admitted([...source, ...curve, '--export-paper', format, ...suffix]);
        const straight = admitted([...source, '--join-text-anchors', 'a', 'o', '1', '1',
            '--export-paper', format, ...suffix]);
        const signature = format === 'png' ? Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])
            : Buffer.from('%PDF-');
        assert.deepEqual(curved.subarray(0, signature.length), signature);
        assert.notDeepEqual(curved, straight);
        assert.deepEqual(curved, admitted([...source, ...curve,
            '--export-paper', format, ...suffix]));
    }
});

test('invalid curves refuse atomically before replacing existing output', () => {
    const directory = mkdtempSync(join(root, '.temp', 'atrament-curved-joins-'));
    try {
        const filename = join(directory, 'accepted.pdf');
        const sentinel = Buffer.from('preserve the earlier printable PDF');
        writeFileSync(filename, sentinel);
        const invalid = [
            ['--join-text-curved', 'a', 'o', '1', '1', '0', '0'],
            ['--join-text-curved', 'a', 'o', '1', '1', '601', '0'],
            ['--join-text-curved', 'a', 'o', '1', '1', '0', '-601'],
            ['--join-text-curved', 'a', 'o', '0', '1', '0', '-400'],
            ['--join-text-curved', 'a', 'o', '2', '1', '0', '-400'],
            ['--join-text-curved', 'a', 'o', '1', '1', '100', '0'],
            [...curve, ...curve],
            ['--join-text-curved', 'a', 'ó', '1', '1', '0', '-400'],
        ];
        for (const choice of invalid) {
            const result = notebook([...source, ...choice, '--export-paper', 'pdf',
                '--output', filename, '--replace']);
            assert.notEqual(result.status, 0, JSON.stringify(choice));
            assert.equal(result.stdout.length, 0);
            assert.deepEqual(readFileSync(filename), sentinel);
        }
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});

test('two independent measured text rows receive separate curved connections', () => {
    const flowSource = source.slice(0, -5);
    const flags = [...flowSource, '--flow-text', 'ao\nao',
        ...curve, '--emit-ink-file'];
    const lines = admitted(flags).toString().trimEnd().split('\n');
    assert.equal(lines[0], 'atrament.page-ink.v1');
    assert.equal(lines.length, 13, 'two glyphs and four curved strokes per row');
    for (const offset of [0, 6]) {
        const decoded = lines.slice(1 + offset, 7 + offset).map((line) => {
            const [, x1, y1, x2, y2, width, color] = line.split('\t');
            return { start: [Number(x1), Number(y1)],
                end: [Number(x2), Number(y2)], width, color };
        });
        for (let index = 1; index < 5; index++) {
            assert.deepEqual(decoded[index].start, decoded[index - 1].end);
            assert.equal(decoded[index].width, '250');
            assert.equal(decoded[index].color, '#102030');
        }
        assert.deepEqual(decoded[5].start, decoded[4].end);
    }
    assert.notDeepEqual(lines[1], lines[7], 'second row uses a distinct physical baseline');
    assert.deepEqual(admitted(flags), admitted(flags), 'replay is exact');
});

test('exact decomposed Unicode and explicit stroke anchors remain distinct', () => {
    const original = [
        '--printed-sheet', 'true',
        '--glyph-stroke', 'n\u0303', '200', '1200', '1500', '1600',
        '250', '#102030',
        '--glyph-stroke', 'n\u0303', '1500', '1600', '3200', '2200',
        '250', '#102030',
        '--glyph-stroke', 'ó', '200', '2200', '1200', '1000',
        '250', '#102030',
        '--glyph-stroke', 'ó', '1200', '1000', '1400', '800',
        '250', '#102030',
        '--place-text', '1', '30000', '45000', 'n\u0303ó',
    ];
    const curved = [...original, '--join-text-curved',
        'n\u0303', 'ó', '2', '1', '0', '-300', '--emit-ink-file'];
    const lines = admitted(curved).toString().trimEnd().split('\n');
    assert.equal(lines.length, 9);
    const coordinates = lines.slice(1).map((line) => line.split('\t').slice(1, 5).map(Number));
    assert.deepEqual(coordinates[1].slice(2), [33200, 47200]);
    assert.deepEqual(coordinates[2].slice(0, 2), [33200, 47200]);
    assert.deepEqual(coordinates[5].slice(2), [34200, 47200]);
    assert.deepEqual(coordinates[6].slice(0, 2), [34200, 47200]);
    const unjoined = admitted([...original, '--emit-ink-file']);
    assert.equal(unjoined.toString().trimEnd().split('\n').length, 5);
    assert.notDeepEqual(admitted(curved), unjoined);
});

test('seeded per-occurrence geometry is joined after native transforms', () => {
    const withVariation = [...source, ...curve, '--glyph-variation',
        '42', '1', '1', '0', '1', '0', '5', '--emit-ink-file'];
    const ink = admitted(withVariation).toString().trimEnd().split('\n')
        .slice(1).map((line) => line.split('\t').slice(1, 5).map(Number));
    assert.equal(ink.length, 6);
    assert.deepEqual(ink[1].slice(0, 2), ink[0].slice(2));
    for (let index = 2; index < 6; index++) {
        assert.deepEqual(ink[index].slice(0, 2), ink[index - 1].slice(2));
    }
    assert.notDeepEqual(admitted(withVariation),
        admitted([...source, ...curve, '--emit-ink-file']));
    assert.deepEqual(admitted(withVariation), admitted(withVariation));
});
