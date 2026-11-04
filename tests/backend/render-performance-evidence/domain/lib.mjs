// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: LicenseRef-Proprietary
// Repeatable Linux CPU measurements, not cross-machine performance budgets.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { cpus, arch, platform, tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import test from 'node:test';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const pngSignature = Buffer.from('89504e470d0a1a0a', 'hex');

function subprocess(executable, args) {
    const result = spawnSync(executable, args, {
        cwd: root, encoding: 'utf8', timeout: 120000,
        env: { ...process.env, LC_ALL: 'C' },
    });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function builtBinaries() {
    const build = subprocess('cargo', [
        'build', '--offline', '--quiet',
        '-p', 'atrament_notebook_paper_cli', '--bin', 'atrament-paper',
        '-p', 'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
    ]);
    assert.equal(build.status, 0, build.stderr);
    const metadata = subprocess('cargo', [
        'metadata', '--offline', '--no-deps', '--format-version', '1',
    ]);
    assert.equal(metadata.status, 0, metadata.stderr);
    const target = JSON.parse(metadata.stdout).target_directory;
    return {
        paper: join(target, 'debug', 'atrament-paper'),
        profiles: join(target, 'debug', 'atrament-profiles'),
    };
}

function pngPixelBytes(png) {
    const compressed = [];
    for (let at = 8; at + 12 <= png.length;) {
        const length = png.readUInt32BE(at);
        const end = at + 12 + length;
        assert.ok(end <= png.length, 'PNG chunk remains bounded');
        if (png.toString('ascii', at + 4, at + 8) === 'IDAT') {
            compressed.push(png.subarray(at + 8, at + 8 + length));
        }
        at = end;
    }
    assert.ok(compressed.length > 0, 'PNG contains raster data');
    return inflateSync(Buffer.concat(compressed));
}

function measuredPng(binary, args, path, log) {
    // GNU time records the native CLI, excluding cargo, node and test startup.
    const result = subprocess('/usr/bin/time', [
        '--format', '%e|%U|%S|%M', '--output', log,
        binary, ...args, '--output', path,
    ]);
    assert.equal(result.status, 0, result.stderr);
    const source = readFileSync(path);
    assert.deepEqual(source.subarray(0, 8), pngSignature);
    assert.equal(source[25], 6, 'RGBA PNG transparency is preserved');
    assert.ok(source.length <= 24_000_000, 'bounded output file');
    const fields = readFileSync(log, 'utf8').trim().split('|').map(Number);
    assert.equal(fields.length, 4, 'GNU time reports four metrics');
    assert.ok(fields.every(Number.isFinite), 'measurements must be finite');
    const [wallSeconds, userSeconds, systemSeconds, peakRssKiB] = fields;
    assert.ok(wallSeconds >= 0 && userSeconds >= 0);
    assert.ok(systemSeconds >= 0 && peakRssKiB > 0);
    const quality = args[args.indexOf('--quality') + 1];
    const evidence = {
        machine: { platform: platform(), arch: arch(),
            cpu: cpus()[0]?.model ?? 'unknown' },
        renderer: binary.endsWith('atrament-paper') ? 'paper' : 'authored-ink',
        quality,
        widthPixels: source.readUInt32BE(16),
        heightPixels: source.readUInt32BE(20),
        outputBytes: source.length,
        sha256: createHash('sha256').update(source).digest('hex'),
        wallSeconds, userSeconds, systemSeconds, peakRssKiB,
    };
    console.log(`CPU_PNG_MEASUREMENT ${JSON.stringify(evidence)}`);
    return { evidence, source };
}

test('real CPU Preview/Final ink output has inspectable costs and bounds',
    { skip: platform() !== 'linux' }, () => {
        const dir = mkdtempSync(join(tmpdir(), 'atrament-cpu-png-quality-'));
        try {
            const binaries = builtBinaries();
            for (const [renderer, binary, options] of [
                ['paper', binaries.paper, [
                    '--style', 'grid', '--paper-size', 'a4', '--format', 'png',
                ]],
                ['authored-ink', binaries.profiles, [
                    'notebook', '--printed-sheet', 'true',
                    '--export-paper', 'png',
                    '--ink-line', '1', '25400', '25400',
                    '50800', '50800', '700', '#DE1144',
                ]],
            ]) {
                const measured = [];
                for (const quality of ['preview', 'final']) {
                    const output = join(dir, `${renderer}-${quality}.png`);
                    const log = join(dir, `${renderer}-${quality}.time`);
                    measured.push(measuredPng(binary,
                        [...options, '--quality', quality], output, log));
                }
                const [preview, final] = measured;
                const coloredStroke = Buffer.from([222, 17, 68, 255]);
                for (const { source } of measured) {
                    const containsStroke = pngPixelBytes(source)
                        .includes(coloredStroke);
                    assert.equal(containsStroke, renderer === 'authored-ink',
                        'only authored ink must contain the selected color');
                }
                assert.ok(final.evidence.widthPixels
                    > preview.evidence.widthPixels);
                assert.ok(final.evidence.heightPixels
                    > preview.evidence.heightPixels);
                assert.notDeepEqual(preview.source, final.source,
                    'Preview and Final use different native raster resolution');
            }
            const reviewed = join(dir, 'reviewed.png');
            const original = Buffer.from('existing reviewed output');
            writeFileSync(reviewed, original);
            const refused = subprocess(binaries.paper, [
                '--paper-size', 'a3', '--format', 'png', '--dpi', '300',
                '--output', reviewed, '--replace',
            ]);
            assert.notEqual(refused.status, 0,
                'oversized A3 300-DPI raster must fail closed');
            assert.match(refused.stderr, /PNG pixel budget exceeded/,
                'overflow must fail at the native raster budget');
            assert.deepEqual(readFileSync(reviewed), original,
                'invalid request never replaces reviewed output');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
