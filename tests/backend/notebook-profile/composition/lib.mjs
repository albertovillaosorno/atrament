// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: MIT
// Exercises real Rust file admission without contacting localhost or CUPS.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync,
    writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = fileURLToPath(new URL('../../../../', import.meta.url));

function notebook(...args) {
    const result = spawnSync('cargo', [
        'run', '--offline', '--quiet', '-p', 'atrament_profile_editor_cli',
        '--bin', 'atrament-profiles', '--', 'notebook', ...args,
    ], { cwd: root, timeout: 120000, maxBuffer: 24 * 1024 * 1024 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function accepted(result, context) {
    assert.equal(result.status, 0,
        `${context}: ${result.stderr.toString('utf8')}`);
    return result.stdout;
}

test('legacy seven-field notebook v1 explicitly upgrades without data loss',
    () => {
        const dir = mkdtempSync(join(root, '.temp/legacy-notebook-'));
        try {
            const modern = join(dir, 'modern.json');
            const legacy = join(dir, 'legacy.json');
            const migrated = join(dir, 'migrated.json');
            const candidate = join(dir, 'candidate.json');
            accepted(notebook('--date-style', 'line',
                '--date-placement', 'top-left',
                '--page-numbers', 'true',
                '--border-radius-um', '5000',
                '--printed-sheet', 'true',
                '--output', modern), 'create native modern profile');
            const expected = readFileSync(modern);
            const modernText = expected.toString('utf8');
            const oldText = modernText.replace('  "date_text": "",\n', '');
            assert.notEqual(oldText, modernText,
                'only optional date is absent');
            assert.equal(oldText.split('"date_text"').length, 1);
            const oldBytes = Buffer.from(oldText);
            writeFileSync(legacy, oldBytes);

            const readOnly = accepted(notebook('--profile', legacy),
                'read legacy seven-field notebook');
            assert.deepEqual(readOnly, expected,
                'missing optional date defaults to empty canonical field');
            assert.deepEqual(readdirSync(dir).sort(),
                ['legacy.json', 'modern.json'],
                'a read-only migration cannot create a file');
            accepted(notebook('--profile', legacy, '--output', migrated),
                'explicitly save upgraded notebook');
            const saved = readFileSync(migrated);
            assert.deepEqual(saved, expected,
                'page geometry and all existing metadata survive upgrade');
            assert.deepEqual(readFileSync(legacy), oldBytes,
                'source remains byte-for-byte original seven-field JSON');

            const inPlace = join(dir, 'explicit-upgrade.json');
            writeFileSync(inPlace, oldBytes);
            const implicitOverwrite = notebook('--profile', inPlace,
                '--output', inPlace);
            assert.notEqual(implicitOverwrite.status, 0,
                'migration cannot silently overwrite its selected source');
            assert.deepEqual(readFileSync(inPlace), oldBytes,
                'no-clobber leaves original legacy bytes intact');
            accepted(notebook('--profile', inPlace, '--output', inPlace,
                '--replace'), 'user-authorized in-place upgrade');
            assert.deepEqual(readFileSync(inPlace), expected,
                'only an explicit --replace performs the in-place upgrade');
            assert.deepEqual(readFileSync(legacy), oldBytes,
                'an independent legacy source remains unchanged');

            for (const args of [
                ['--export-paper', 'pdf', '--pages', '2'],
                ['--export-paper', 'png', '--png-page', '2', '--dpi', '72'],
            ]) {
                const oldInk = accepted(notebook('--profile', legacy,
                    ...args), 'legacy physical export');
                const newInk = accepted(notebook('--profile', migrated,
                    ...args), 'upgraded physical export');
                assert.deepEqual(newInk, oldInk,
                    'old/new profile must render identical physical pages');
                if (args[1] === 'pdf') {
                    assert.ok(oldInk.toString('latin1').startsWith('%PDF-'));
                    assert.match(oldInk.toString('latin1'), /\/Count 2\b/,
                        'actual two-page physical PDF');
                } else {
                    assert.deepEqual(oldInk.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                    assert.equal(oldInk.readUInt32BE(16), 596);
                    assert.equal(oldInk.readUInt32BE(20), 842);
                    assert.equal(oldInk[25], 6,
                        'page-two PNG is real transparent RGBA');
                }
            }

            const invalid = [
                Buffer.from(oldText.replace('atrament.notebook.v1',
                    'atrament.notebook.v0')),
                Buffer.from(oldText.replace('"printed_sheet": true',
                    '"unsupported": true')),
                Buffer.from(oldText.replace('"page_numbers": true',
                    '"page_numbers": 1')),
                Buffer.alloc(8193, 32),
                Buffer.from([0xff, 0xfe, 0xfd]),
            ];
            for (const bytes of invalid) {
                writeFileSync(candidate, bytes);
                const refused = notebook('--profile', candidate,
                    '--output', migrated, '--replace');
                assert.notEqual(refused.status, 0,
                    'untrusted or unsupported saved input must refuse');
                assert.deepEqual(readFileSync(candidate), bytes,
                    'refusal must not alter the selected source');
                assert.deepEqual(readFileSync(migrated), saved,
                    'refusal must not replace accepted output');
                assert.deepEqual(readFileSync(legacy), oldBytes,
                    'unsupported versions cannot mutate the original');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
