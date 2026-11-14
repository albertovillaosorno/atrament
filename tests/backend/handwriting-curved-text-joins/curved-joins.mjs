// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: MIT
// Native CLI acceptance for explicit curved connections between glyphs.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
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

test('physical TSV saved from curved text reproduces exact vector and raster pages', () => {
    const directory = mkdtempSync(join(root, '.temp', 'atrament-curved-ink-parity-'));
    try {
        const filename = join(directory, 'joined.page-ink.tsv');
        const ink = admitted([...source, ...curve, '--emit-ink-file']);
        writeFileSync(filename, ink);
        for (const format of ['pdf', 'png']) {
            const option = ['--export-paper', format,
                ...(format === 'png' ? ['--dpi', '72'] : [])];
            const direct = admitted([...source, ...curve, ...option]);
            const imported = admitted(['--printed-sheet', 'true', '--ink-file',
                filename, ...option]);
            assert.deepEqual(imported, direct,
                format + ' must draw byte-identical native physical pen segments');
        }
        assert.deepEqual(readFileSync(filename), ink,
            'replaying page ink must not modify source bytes');
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});

test('128 independent quadratic bends match a whole-um numeric reference oracle', () => {
    const start = [33200, 47200];
    const end = [34200, 47200];
    for (let seed = 0; seed < 128; seed++) {
        const bx = (seed * 97) % 1201 - 600;
        const by = (seed * 89) % 1199 - 599;
        if (by === 0) continue;
        const command = ['--join-text-curved', 'a', 'o', '1', '1',
            String(bx), String(by)];
        const actual = admitted([...source, ...command, '--emit-ink-file'])
            .toString().trimEnd().split('\n').slice(2, 6)
            .map((line) => line.split('\t').slice(1, 5).map(Number));
        let from = start;
        for (let quarter = 1; quarter <= 4; quarter++) {
            const t = quarter / 4;
            const to = start.map((a, axis) => Math.floor(
                (1 - t) * (1 - t) * a
                + 2 * t * (1 - t) * ((a + end[axis]) / 2
                    + [bx, by][axis]) + t * t * end[axis] + 0.5));
            assert.deepEqual(actual[quarter - 1], [...from, ...to],
                'seed=' + seed + ', quarter=' + quarter);
            from = to;
        }
    }
});

test('cross-row, out-of-range gap and incompatible pen links fail closed', () => {
    const cases = [
        [...source.slice(0, -5), '--flow-text', 'a\no',
            ...curve, '--emit-ink-file'],
        [...source.map((token, index) => index === 6 ? '2999' : token),
            ...curve, '--emit-ink-file'],
        [...source.map((token, index) => index === 16 ? '251' : token),
            ...curve, '--emit-ink-file'],
        [...source.map((token, index) => index === 17 ? '#304050' : token),
            ...curve, '--emit-ink-file'],
    ];
    for (const flags of cases) {
        const result = notebook(flags);
        assert.notEqual(result.status, 0, result.stderr.toString());
        assert.equal(result.stdout.length, 0);
    }
});

test('multiple curved pairs retain independent bends in reverse option order', () => {
    const pairs = [
        '--printed-sheet', 'true',
        '--glyph-stroke', 'a', '200', '1000', '3200', '2200', '250', '#102030',
        '--glyph-stroke', 'o', '200', '2200', '3200', '1000', '250', '#102030',
        '--glyph-stroke', 'n', '200', '1000', '1200', '2400', '250', '#102030',
        '--place-text', '1', '30000', '45000', 'aon',
        '--join-text-curved', 'o', 'n', '1', '1', '0', '350',
        '--join-text-curved', 'a', 'o', '1', '1', '0', '-400',
    ];
    const rows = admitted([...pairs, '--emit-ink-file']).toString()
        .trimEnd().split('\n').slice(1);
    assert.equal(rows.length, 11);
    const points = rows.map((row) => row.split('\t').slice(1, 5).map(Number));
    assert.deepEqual(points[1], [33200, 47200, 33450, 47050]);
    assert.deepEqual(points[4], [33950, 47050, 34200, 47200]);
    assert.deepEqual(points[6], [37200, 46000, 37450, 46131]);
    assert.deepEqual(points[9], [37950, 46131, 38200, 46000]);
    assert.deepEqual(admitted([...pairs, '--emit-ink-file']),
        admitted([...pairs, '--emit-ink-file']));
    const duplicate = notebook([...pairs, '--join-text-pair', 'a', 'o',
        '--emit-ink-file']);
    assert.notEqual(duplicate.status, 0);
    assert.equal(duplicate.stdout.length, 0);
});

test('128 oblique glyph gaps match signed integer quadratic samples', () => {
    let checked = 0;
    for (let seed = 0; seed < 128; seed++) {
        const lx = 3200 + seed % 101;
        const ly = 2000 + seed * 31 % 401;
        const rx = 150 + seed * 17 % 101;
        const ry = ly + seed * 53 % 501 - 250;
        const bx = seed * 97 % 1201 - 600;
        const by = seed * 193 % 1201 - 600;
        const from = [30000 + lx, 45000 + ly];
        const to = [34000 + rx, 45000 + ry];
        const dx = to[0] - from[0];
        const dy = to[1] - from[1];
        if (bx * dy === by * dx) continue;
        const args = [
            '--printed-sheet', 'true',
            '--glyph-stroke', 'a', '200', '1000', String(lx), String(ly),
            '250', '#102030',
            '--glyph-stroke', 'o', String(rx), String(ry),
            '1200', '1000', '250', '#102030',
            '--place-text', '1', '30000', '45000', 'ao',
            '--join-text-curved', 'a', 'o', '1', '1', String(bx), String(by),
            '--emit-ink-file',
        ];
        const actual = admitted(args).toString().trimEnd().split('\n')
            .slice(2, 6).map((row) => row.split('\t').slice(1, 5).map(Number));
        let previous = from;
        for (let quarter = 1; quarter <= 4; quarter++) {
            const i = BigInt(quarter);
            const c = 4n - i;
            const next = from.map((start, axis) => Number((
                c * c * BigInt(start)
                + i * c * (BigInt(start) + BigInt(to[axis])
                    + 2n * BigInt([bx, by][axis]))
                + i * i * BigInt(to[axis]) + 8n) / 16n));
            assert.deepEqual(actual[quarter - 1], [...previous, ...next],
                `seed=${seed}, bend=(${bx},${by}), quarter=${quarter}`);
            previous = next;
        }
        assert.deepEqual(previous, to);
        checked++;
    }
    assert.ok(checked >= 120, 'independent off-axis bend fixtures exercised');
});

test('curved pen overshoot outside printable margins refuses a valid straight bridge', () => {
    // Default A4 right ink limit is 195875 um with the 250-um pen.
    // Both original glyphs and the straight bridge fit. Its curved
    // quarter samples protrude past that physical boundary.
    const nearEdge = [
        '--printed-sheet', 'true',
        '--glyph-stroke', 'a', '200', '1000', '3350', '2200',
        '250', '#102030',
        '--glyph-stroke', 'o', '130', '2200', '125', '1000',
        '250', '#102030',
        '--place-text', '1', '191730', '45000', 'ao',
    ];
    const straight = admitted([...nearEdge, '--join-text-anchors',
        'a', 'o', '1', '1', '--emit-ink-file']);
    assert.equal(straight.toString().trimEnd().split('\n').length, 4);
    const directory = mkdtempSync(join(root, '.temp', 'atrament-curve-margin-'));
    try {
        for (const format of ['pdf', 'png']) {
            const filename = join(directory, `existing.${format}`);
            const prior = Buffer.from('prior accepted physical page');
            writeFileSync(filename, prior);
            const refused = notebook([...nearEdge,
                '--join-text-curved', 'a', 'o', '1', '1', '600', '120',
                '--export-paper', format,
                ...(format === 'png' ? ['--dpi', '72'] : []),
                '--output', filename, '--replace']);
            assert.notEqual(refused.status, 0);
            assert.equal(refused.stdout.length, 0);
            assert.match(refused.stderr.toString(), /printable endpoints/);
            assert.deepEqual(readFileSync(filename), prior);
        }
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});

test('two accented text rows join after correlated variation and honor pen reversal', () => {
    const nfd = 'n\u0303';
    const nfc = 'ó';
    const glyph = [
        '--glyph-stroke', nfd, '200', '2200', '3200', '1200',
        '250', '#102030',
        '--glyph-stroke', nfd, '1500', '500', '2000', '800',
        '250', '#102030',
        '--glyph-stroke', nfc, '200', '1200', '1200', '2200',
        '250', '#102030',
        '--glyph-stroke', nfc, '1400', '500', '1600', '800',
        '250', '#102030',
    ];
    const text = `${nfd}${nfc} ${nfd}${nfc}\n${nfd}${nfc} ${nfd}${nfc}`;
    const rhythm = [
        '--glyph-variation', '42', '1', '1', '0', '1', '0', '4',
        '--glyph-line-drift', '20',
        '--glyph-word-rise', '20',
        '--glyph-word-tracking', '30',
    ];
    const directory = mkdtempSync(join(root, '.temp', 'atrament-accent-curves-'));
    const glyphCommand = (options) => spawnSync('cargo', [
        'run', '--locked', '--offline', '--quiet',
        '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
        '--', 'glyphs', ...options,
    ], { cwd: root, timeout: 120000, maxBuffer: 16 * 1024 * 1024 });
    try {
        const library = join(directory, 'author.tsv');
        const reversed = join(directory, 'reverse.tsv');
        const saved = glyphCommand([...glyph, '--output', library]);
        assert.equal(saved.status, 0, saved.stderr.toString());
        const original = readFileSync(library);
        const source = [
            '--printed-sheet', 'true', '--glyph-file', library,
            '--flow-text', text,
            '--join-text-curved', nfd, nfc, '1', '1', '0', '-350',
        ];
        const ideal = admitted([...source, '--emit-ink-file']);
        const sampled = admitted([...source, ...rhythm, '--emit-ink-file']);
        const lines = sampled.toString().trimEnd().split('\n');
        assert.equal(lines.length, 33,
            '16 original accent/body segments and 16 curved pieces');
        assert.notDeepEqual(ideal, sampled, 'variation changes physical ink');
        assert.deepEqual(sampled, admitted([...source, ...rhythm,
            '--emit-ink-file']), 'all rows replay the same pen coordinates');
        assert.notEqual(lines[1], lines[17], 'rows occupy different baselines');
        const pageInk = join(directory, 'page-ink.tsv');
        writeFileSync(pageInk, sampled);
        for (const format of ['pdf', 'png']) {
            const option = ['--export-paper', format,
                ...(format === 'png' ? ['--dpi', '72'] : [])];
            assert.deepEqual(admitted([...source, ...rhythm, ...option]),
                admitted(['--printed-sheet', 'true', '--ink-file',
                    pageInk, ...option]),
                'imported accented page ink exactly replays native rendering');
        }
        const edit = glyphCommand([
            '--profile', library, '--reverse-stroke', nfd, '1',
            '--output', reversed,
        ]);
        assert.equal(edit.status, 0, edit.stderr.toString());
        assert.notDeepEqual(readFileSync(reversed), original);
        const restored = glyphCommand([
            '--profile', reversed, '--reverse-stroke', nfd, '1',
        ]);
        assert.equal(restored.status, 0, restored.stderr.toString());
        assert.deepEqual(restored.stdout, original,
            'the deliberate stroke reversal has an exact inverse');
        const target = join(directory, 'approved.pdf');
        const approved = admitted([...source, ...rhythm,
            '--export-paper', 'pdf']);
        writeFileSync(target, approved);
        const invalid = notebook([
            '--printed-sheet', 'true', '--glyph-file', reversed,
            '--flow-text', text,
            '--join-text-curved', nfd, nfc, '1', '1', '0', '-350',
            ...rhythm, '--export-paper', 'pdf',
            '--output', target, '--replace',
        ]);
        assert.notEqual(invalid.status, 0,
            'reversed exit must not silently bridge a long physical gap');
        assert.equal(invalid.stdout.length, 0);
        assert.deepEqual(readFileSync(target), approved);
        assert.deepEqual(readFileSync(library), original);
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});

test('four curved segments consume the exact 4096-line physical budget', () => {
    const directory = mkdtempSync(join(root, '.temp', 'atrament-curve-budget-'));
    const glyphRow = 'a\t200\t1000\t3200\t2200\t250\t#102030';
    const nextRow = 'o\t200\t2200\t1200\t1000\t250\t#102030';
    const filename = join(directory, 'glyphs.tsv');
    const flags = [
        '--printed-sheet', 'true', '--glyph-file', filename,
        '--place-text', '1', '30000', '45000', 'ao',
    ];
    const writeLibrary = (count) => {
        const text = ['atrament.glyph-strokes.v1',
            ...Array(count).fill(glyphRow), nextRow, ''].join('\n');
        writeFileSync(filename, text);
        return Buffer.from(text);
    };
    try {
        const admittedLibrary = writeLibrary(4091);
        const joined = admitted([...flags, '--join-text-curved',
            'a', 'o', '4091', '1', '0', '-400', '--emit-ink-file']);
        assert.equal(joined.toString().trimEnd().split('\n').length, 4097,
            'header plus 4092 source lines and four curved pieces');
        assert.deepEqual(readFileSync(filename), admittedLibrary);
        assert.deepEqual(joined, admitted([...flags, '--join-text-curved',
            'a', 'o', '4091', '1', '0', '-400', '--emit-ink-file']));
        const excessLibrary = writeLibrary(4092);
        const target = join(directory, 'previous.pdf');
        const previous = Buffer.from('accepted page must remain unchanged');
        writeFileSync(target, previous);
        const refused = notebook([...flags, '--join-text-curved',
            'a', 'o', '4092', '1', '0', '-400',
            '--export-paper', 'pdf', '--output', target, '--replace']);
        assert.notEqual(refused.status, 0);
        assert.match(refused.stderr.toString(), /4096 total pen strokes/);
        assert.equal(refused.stdout.length, 0);
        assert.deepEqual(readFileSync(target), previous);
        assert.deepEqual(readFileSync(filename), excessLibrary);
        const straight = admitted([...flags, '--join-text-anchors',
            'a', 'o', '4092', '1', '--emit-ink-file']);
        assert.equal(straight.toString().trimEnd().split('\n').length, 4095,
            'a one-piece connector stays under the same bound');
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});

test('malformed curved text joins refuse before opening an unclosed glyph stdin', async () => {
    const invalid = [
        [['a', 'o', '1', '1', '0', '0'], /requires nonzero bend/],
        [['a', 'o', '1', '1', '601', '0'], /outside native bounds/],
        [['a', 'o', '1', '1', '0', '-601'], /outside native bounds/],
        [['a', 'o', '0', '1', '0', '-400'], /join exit stroke/],
        [['ab', 'o', '1', '1', '0', '-400'], /exact graphemes/],
        [['a', 'o', '1', '1', 'NaN', '0'], /signed whole/],
        [['a', 'o', '1', '1', '0'], /signed whole/],
    ];
    for (const [operands, diagnostic] of invalid) {
        const result = await new Promise((resolve, reject) => {
            const child = spawn('cargo', [
                'run', '--locked', '--offline', '--quiet',
                '-p', 'atrament_profile_editor_cli', '--bin',
                'atrament-profiles', '--', 'notebook',
                '--glyph-file', '-', '--join-text-curved', ...operands,
                '--emit-ink-file',
            ], { cwd: root, stdio: ['pipe', 'pipe', 'pipe'] });
            let stderr = '';
            let stdout = '';
            const timer = setTimeout(() => {
                child.kill('SIGKILL');
                reject(new Error('curved join waited for unopened stdin'));
            }, 8000);
            child.stdout.setEncoding('utf8').on('data', (chunk) => {
                stdout += chunk;
            });
            child.stderr.setEncoding('utf8').on('data', (chunk) => {
                stderr += chunk;
            });
            child.once('error', (error) => {
                clearTimeout(timer);
                reject(error);
            });
            child.once('close', (status) => {
                clearTimeout(timer);
                resolve({ status, stdout, stderr });
            });
            // Deliberately never close or write to child.stdin.
        });
        assert.notEqual(result.status, 0);
        assert.equal(result.stdout, '');
        assert.match(result.stderr, diagnostic);
    }
});
