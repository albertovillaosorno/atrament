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
