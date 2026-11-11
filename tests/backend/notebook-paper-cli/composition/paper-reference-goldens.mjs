// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: LicenseRef-Proprietary
// Black-box native paper CLI parity with viewable candidate paper files.
// No real browser or printer is started.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { inflateSync } from 'node:zlib';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const referenceDir = fileURLToPath(new URL('./', import.meta.url));
const baselineDigests = {
    ruled: {
        pdf: 'fa59a88a53ffba9a11b8f7dd1416101bf3cd9b6d63693fed6b5692b5899bad8e',
        png: 'd775db518f8cba371e5af6cf1608aef7ad38c33d1ec8d37ab6bda71700c50aaf',
    },
    grid: {
        pdf: '6bbf7d74ded9f62210b45f04fb1b5c48a51422cc4f315019de1b45c16e10b773',
        png: '46e2ff82bda2db65a685836a6a3fa96f8b73e883aabc0571e28127308f270631',
    },
};

function nativePaper(...args) {
    const result = spawnSync('cargo', [
        'run', '--offline', '--quiet', '-p', 'atrament_notebook_paper_cli',
        '--bin', 'atrament-paper', '--', ...args,
    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function digest(bytes) {
    return createHash('sha256').update(bytes).digest('hex');
}

test('native CLI creates exact ruled and squared PDF/PNG references', () => {
    const directory = mkdtempSync(join(tmpdir(), 'atrament-paper-goldens-'));
    try {
        for (const style of ['ruled', 'grid']) {
            for (const format of ['pdf', 'png']) {
                const filename = `a4-${style}-72dpi.${format}`;
                const output = join(directory, filename);
                const flags = format === 'png'
                    ? ['--format', 'png', '--dpi', '72'] : [];
                const run = nativePaper('--style', style, ...flags,
                    '--output', output);
                assert.equal(run.status, 0, run.stderr);
                const actual = readFileSync(output);
                const expected = readFileSync(join(referenceDir, filename));
                assert.deepEqual(actual, expected,
                    `${style} ${format} must match the inspectable candidate`);
                assert.equal(digest(expected), baselineDigests[style][format],
                    'fixture updates need independent hash review');
                assert.equal(digest(actual), baselineDigests[style][format]);
                if (format === 'pdf') {
                    assert.ok(actual.subarray(0, 8)
                        .toString().startsWith('%PDF-1.4'));
                    assert.match(actual.toString('latin1'),
                        /\/MediaBox \[0 0 595\.276 841\.890\]/);
                } else {
                    assert.deepEqual(actual.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                    assert.equal(actual.readUInt32BE(16), 596);
                    assert.equal(actual.readUInt32BE(20), 842);
                    assert.equal(actual[25], 6, 'RGBA rather than RGB');
                }

                const noReplace = nativePaper('--style', style,
                    ...flags, '--output', output);
                assert.notEqual(noReplace.status, 0,
                    'explicit replacement is necessary for existing output');
                assert.deepEqual(readFileSync(output), actual);
                const invalid = nativePaper('--style', 'invalid',
                    ...flags, '--output', output, '--replace');
                assert.notEqual(invalid.status, 0,
                    'unknown styles must refuse before file replacement');
                assert.deepEqual(readFileSync(output), actual,
                    'refusal must preserve previously accepted page bytes');
            }
        }
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});

// Validate native PNG file framing independently of the Rust encoder.
// PNG CRC-32 covers each chunk type and payload, not the length header.
function pngCrc32(bytes) {
    let crc = 0xffffffff;
    for (const byte of bytes) {
        crc ^= byte;
        for (let bit = 0; bit < 8; bit += 1) {
            crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
        }
    }
    return (crc ^ 0xffffffff) >>> 0;
}

function decodeNativePng(bytes, dpi, widthUm, heightUm,
    style = 'blank', color = [197, 212, 231]) {
    assert.deepEqual(bytes.subarray(0, 8),
        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
    const tags = [];
    const chunks = new Map();
    let position = 8;
    while (position < bytes.length) {
        assert.ok(position + 12 <= bytes.length,
            'each PNG chunk needs length, type and checksum');
        const length = bytes.readUInt32BE(position);
        const end = position + length + 12;
        assert.ok(end <= bytes.length, 'complete bounded PNG chunk');
        const tagBytes = bytes.subarray(position + 4, position + 8);
        const data = bytes.subarray(position + 8, end - 4);
        const storedCrc = bytes.readUInt32BE(end - 4);
        assert.equal(storedCrc,
            pngCrc32(Buffer.concat([tagBytes, data])),
            `valid PNG chunk checksum for ${tagBytes.toString('ascii')}`);
        const tag = tagBytes.toString('ascii');
        assert.ok(!chunks.has(tag), `one ${tag} chunk is expected`);
        tags.push(tag);
        chunks.set(tag, data);
        position = end;
    }
    assert.equal(position, bytes.length, 'no junk after terminal chunk');
    assert.deepEqual(tags, ['IHDR', 'pHYs', 'IDAT', 'IEND']);
    assert.equal(chunks.get('IEND').length, 0);
    const header = chunks.get('IHDR');
    const width = Math.ceil(widthUm * dpi / 25400);
    const height = Math.ceil(heightUm * dpi / 25400);
    assert.equal(header.readUInt32BE(0), width);
    assert.equal(header.readUInt32BE(4), height);
    assert.deepEqual(header.subarray(8), Buffer.from([8, 6, 0, 0, 0]));
    const resolution = chunks.get('pHYs');
    const pixelsPerMetre = Math.round(dpi * 10000 / 254);
    assert.equal(resolution.readUInt32BE(0), pixelsPerMetre);
    assert.equal(resolution.readUInt32BE(4), pixelsPerMetre);
    assert.equal(resolution[8], 1, 'metre resolution units');
    const raster = inflateSync(chunks.get('IDAT'));
    const stride = width * 4 + 1;
    assert.equal(raster.length, stride * height);
    for (let row = 0; row < height; row += 1) {
        assert.equal(raster[row * stride], 0, 'unfiltered RGBA row');
    }
    // Sample physical millimetre positions independently of the Rust paper
    // geometry and rasterizer. The first rule begins 22 mm from the top;
    // 5-mm grid columns begin at 20 mm from the left.
    const pixelAtMm = (xMm, yMm) => {
        const x = Math.floor(xMm * dpi / 25.4);
        const y = Math.floor(yMm * dpi / 25.4);
        assert.ok(x < width && y < height, 'physical sample on sheet');
        const offset = y * stride + 1 + 4 * x;
        return [...raster.subarray(offset, offset + 4)];
    };
    const clear = [0, 0, 0, 0];
    const pigment = [...color, 255];
    assert.deepEqual(pixelAtMm(2, 2), clear,
        'outside the writing region remains transparent');
    assert.deepEqual(pixelAtMm(27, 24), clear,
        'inside a paper cell remains transparent');
    if (style === 'blank') {
        assert.ok(raster.every(value => value === 0));
    } else {
        assert.deepEqual(pixelAtMm(27, 27), pigment,
            'a horizontal line lands at its 27-mm physical anchor');
        assert.deepEqual(pixelAtMm(25, 24),
            style === 'grid' ? pigment : clear,
            'vertical lines exist only on the squared grid');
        assert.equal(raster[4], 0, 'unmarked upper-left remains clear');
    }
}

test('native transparent PNG passes independent CRC and zlib validation',
    () => {
        const directory = mkdtempSync(join(tmpdir(), 'atrament-png-frame-'));
        try {
            const output = join(directory, 'canvas.png');
            // 300-DPI A4 exceeds five million pixels, but A5 remains
            // admissible and independently decodable at the maximum DPI.
            for (const { dpi, size, widthUm, heightUm, style = 'blank',
                color } of [
                { dpi: 72, size: 'a4', widthUm: 210000, heightUm: 297000 },
                { dpi: 150, size: 'a4', widthUm: 210000, heightUm: 297000 },
                { dpi: 225, size: 'a4', widthUm: 210000, heightUm: 297000 },
                { dpi: 300, size: 'a5', widthUm: 148000, heightUm: 210000 },
                { dpi: 300, size: 'a5', widthUm: 148000, heightUm: 210000,
                    style: 'grid', color: '#1A6DD9' },
                { dpi: 300, size: 'a5', widthUm: 148000, heightUm: 210000,
                    style: 'ruled' },
            ]) {
                const run = nativePaper('--style', style,
                    '--paper-size', size, '--format', 'png',
                    '--dpi', String(dpi), '--output', output,
                    ...(color ? ['--line-color', color] : []),
                    ...(dpi === 72 ? [] : ['--replace']));
                assert.equal(run.status, 0, run.stderr);
                const png = readFileSync(output);
                assert.ok(png.length <= 24_000_000,
                    'bounded native PNG output');
                decodeNativePng(png, dpi, widthUm, heightUm, style,
                    color ? [26, 109, 217] : undefined);
            }
            const accepted = readFileSync(output);
            const refused = nativePaper('--style', 'blank',
                '--format', 'png', '--dpi', '300',
                '--output', output, '--replace');
            assert.notEqual(refused.status, 0,
                '5-million-pixel limit refuses oversized A4 PNG');
            assert.deepEqual(readFileSync(output), accepted,
                'over-budget raster cannot replace reviewed output');
        } finally {
            rmSync(directory, { recursive: true, force: true });
        }
    });

// A byte-level size budget must admit its exact inclusive ceiling. These
// millimetre inputs round to precisely 2000 x 2500 physical pixels at 300 DPI.
test('PNG admits exactly five million pixels and preserves accepted bytes',
    () => {
        const directory = mkdtempSync(join(tmpdir(), 'atrament-png-limit-'));
        try {
            const output = join(directory, 'reviewed.png');
            const dimensions = [
                '--width-mm', '169.333', '--height-mm', '211.666',
            ];
            const flags = [
                '--style', 'blank', '--format', 'png', '--dpi', '300',
            ];
            const acceptedRun = nativePaper(...flags, ...dimensions,
                '--output', output);
            assert.equal(acceptedRun.status, 0, acceptedRun.stderr);
            const accepted = readFileSync(output);
            assert.ok(accepted.length <= 24_000_000);
            decodeNativePng(accepted, 300, 169333, 211666);
            for (const larger of [
                ['--width-mm', '169.334', '--height-mm', '211.666'],
                ['--width-mm', '169.333', '--height-mm', '211.667'],
            ]) {
                const refused = nativePaper(...flags, ...larger,
                    '--output', output, '--replace');
                assert.notEqual(refused.status, 0,
                    'one additional pixel column or row exceeds the budget');
                assert.match(refused.stderr, /PNG pixel budget exceeded/);
                assert.equal(refused.stdout, '',
                    'refusal must not emit partial binary output');
                assert.deepEqual(readFileSync(output), accepted,
                    'rejected extra pixels cannot replace approved PNG');
            }
        } finally {
            rmSync(directory, { recursive: true, force: true });
        }
    });

// README examples omit --bin. The package default must remain the harmless
// paper exporter, never the separately opt-in local printer executable.
test('default Cargo package run selects paper rather than local print', () => {
    const directory = mkdtempSync(join(tmpdir(), 'atrament-paper-default-'));
    try {
        for (const format of ['pdf', 'png']) {
            const output = join(directory, `paper.${format}`);
            const formatFlags = format === 'png'
                ? ['--format', 'png', '--dpi', '72'] : [];
            const result = spawnSync('cargo', [
                'run', '--offline', '--quiet', '-p',
                'atrament_notebook_paper_cli', '--', '--style', 'grid',
                ...formatFlags, '--output', output,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(result.error, undefined, result.error?.message);
            assert.equal(result.status, 0, result.stderr);
            assert.deepEqual(readFileSync(output), readFileSync(join(
                referenceDir, `a4-grid-72dpi.${format}`)),
            'unnamed Cargo binary selection is real paper output');
        }
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});

// The documented shell redirection path must emit only exact binary paper,
// without a selected file or accidental progress text on stdout.
test('paper stdout exactly matches reference PDFs and RGBA PNGs', () => {
    for (const style of ['ruled', 'grid']) {
        for (const format of ['pdf', 'png']) {
            const flags = format === 'png'
                ? ['--format', 'png', '--dpi', '72'] : [];
            const result = spawnSync('cargo', [
                'run', '--offline', '--quiet', '-p',
                'atrament_notebook_paper_cli', '--',
                '--style', style, ...flags,
            ], { cwd: root, timeout: 120000,
                maxBuffer: 16 * 1024 * 1024 });
            assert.equal(result.error, undefined, result.error?.message);
            assert.equal(result.status, 0, result.stderr.toString());
            assert.deepEqual(result.stdout, readFileSync(join(
                referenceDir, `a4-${style}-72dpi.${format}`)),
            'binary stdout must be exact native paper without extra text');
        }
    }
});

// The profile opener is never consulted for source-independent bad options,
// even when an earlier file was selected and an output overwrite requested.
test('paper CLI validates late options before loading profile or replacing PDF',
    () => {
        const directory = mkdtempSync(join(tmpdir(),
            'atrament-paper-preflight-'));
        try {
            const output = join(directory, 'reviewed.pdf');
            const baseline = nativePaper('--style', 'ruled',
                '--output', output);
            assert.equal(baseline.status, 0, baseline.stderr);
            const approved = readFileSync(output);
            const absent = join(directory, 'missing-profile.json');
            for (const [flags, diagnostic] of [
                [['--line-color', 'invalid'], /line color must be #RRGGBB/],
                [['--format', 'png', '--pages', '2'],
                    /--pages is only valid for PDF export/],
                [['--dpi', '301', '--format', 'png'],
                    /PNG DPI must be between 72 and 300/],
                [['--emit-profile', '--emit-profile'],
                    /--emit-profile may appear only once/],
            ]) {
                const result = nativePaper('--profile', absent,
                    '--output', output, '--replace', ...flags);
                assert.notEqual(result.status, 0,
                    'invalid late option must refuse selected export');
                assert.match(result.stderr, diagnostic,
                    'argument diagnostic must precede missing-profile error');
                assert.equal(result.stdout, '', 'refusal emits no PDF bytes');
                assert.deepEqual(readFileSync(output), approved,
                    'preflight refusal must preserve the reviewed PDF');
            }
            const help = nativePaper('--profile', absent, '--help');
            assert.equal(help.status, 0, help.stderr);
            assert.match(help.stdout, /^Usage: atrament-paper/);
        } finally {
            rmSync(directory, { recursive: true, force: true });
        }
    });
