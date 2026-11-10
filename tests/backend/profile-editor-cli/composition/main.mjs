// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: LicenseRef-Proprietary
// Verify real native CLI selected-source resolution and no-clobber refusal.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync,
    writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = fileURLToPath(new URL(
    '../../../../', import.meta.url,
));
const cargo = join(root, '.dependencies/rust/1.99.1/bin/cargo');

function profiles(...args) {
    const result = spawnSync(cargo, [
        'run', '--offline', '--quiet', '-p', 'atrament_profile_editor_cli',
        '--bin', 'atrament-profiles', '--', ...args,
    ], { cwd: root, timeout: 45_000, maxBuffer: 24 * 1024 * 1024 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function accepted(...args) {
    const result = profiles(...args);
    assert.equal(result.status, 0,
        `${args.join(' ')}: ${result.stderr.toString('utf8')}`);
    return result.stdout;
}

function refused(args, expected, preserved) {
    const before = readFileSync(preserved);
    const result = profiles(...args);
    assert.notEqual(result.status, 0,
        `unexpected acceptance: ${args.join(' ')}`);
    assert.match(result.stderr.toString('utf8'), expected,
        `unexpected refusal for ${args.join(' ')}`);
    assert.deepEqual(readFileSync(preserved), before,
        'selected destination must remain identical after refusal');
}

test('native CLI reads selected regular files but never traverses input links',
    { skip: process.platform !== 'linux' }, () => {
        const dir = mkdtempSync(join(root, '.temp/source-open-'));
        try {
            const handwriting = join(dir, 'handwriting.json');
            const notebook = join(dir, 'notebook.json');
            const paper = join(dir, 'paper.json');
            const glyph = join(dir, 'glyph.tsv');
            const flow = join(dir, 'flow.md');
            const destination = join(dir, 'accepted.json');
            accepted('handwriting', '--seed', '11', '--output', handwriting);
            accepted('notebook', '--output', notebook);
            accepted('paper', '--emit-profile', '--output', paper);
            accepted('glyphs', '--glyph-stroke', 'a', '500', '500', '1800',
                '3000', '250', '#123456', '--output', glyph);
            writeFileSync(flow, 'a\n');
            const sources = [handwriting, notebook, paper, glyph, flow];
            const expected = sources.map((file) => readFileSync(file));

            assert.deepEqual(accepted('handwriting', '--profile', handwriting),
                expected[0]);
            assert.deepEqual(accepted('notebook', '--profile', notebook),
                expected[1]);
            assert.deepEqual(accepted('notebook', '--paper-profile', paper),
                expected[1]);
            assert.deepEqual(accepted('glyphs', '--profile', glyph),
                expected[3]);
            assert.ok(accepted('notebook', '--glyph-file', glyph,
                '--place-text', '1', '30000', '40000', 'a',
                '--export-paper', 'pdf')
                .toString('latin1').startsWith('%PDF-'));
            assert.ok(accepted('notebook', '--flow-markdown-file', flow,
                '--glyph-file', glyph, '--export-paper', 'pdf')
                .toString('latin1').startsWith('%PDF-'));
            assert.ok(accepted('notebook', '--handwriting-profile',
                handwriting, '--glyph-file', glyph, '--place-text', '1',
                '30000', '40000', 'a', '--export-paper', 'pdf')
                .toString('latin1').startsWith('%PDF-'));

            writeFileSync(destination, 'previous accepted export');
            const leafLinks = sources.map((source, index) => {
                const link = join(dir, `link-${index}`);
                symlinkSync(source, link);
                return link;
            });
            const realParent = join(dir, 'real');
            const parentLink = join(dir, 'linked-parent');
            mkdirSync(realParent);
            symlinkSync(realParent, parentLink, 'dir');
            const linkedParentFiles = sources.map((source, index) => {
                const basename = `source-${index}`;
                writeFileSync(join(realParent, basename), readFileSync(source));
                return join(parentLink, basename);
            });
            const cases = (sourcePaths) => [
                ['handwriting', '--profile', sourcePaths[0]],
                ['notebook', '--profile', sourcePaths[1]],
                ['notebook', '--paper-profile', sourcePaths[2]],
                ['glyphs', '--profile', sourcePaths[3]],
                ['notebook', '--glyph-file', sourcePaths[3]],
                ['notebook', '--handwriting-profile', sourcePaths[0]],
                ['notebook', '--flow-markdown-file', sourcePaths[4]],
                ['notebook', '--flow-text-file', sourcePaths[4]],
            ];
            for (const paths of [leafLinks, linkedParentFiles]) {
                for (const [mode, ...flags] of cases(paths)) {
                    refused([mode, ...flags, '--output', destination,
                        '--replace'], /SymlinkForbidden/, destination);
                }
            }
            refused(['notebook', '--profile',
                `${dir}/real/../notebook.json`, '--output',
                destination, '--replace'], /ParentTraversal/, destination);
            for (const [source, before] of sources.map((item, index) =>
                [item, expected[index]])) {
                assert.deepEqual(readFileSync(source), before,
                    'read-only refusal must preserve actual source bytes');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('native notebook PNG quality presets match exact authored glyph ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/notebook-quality-'));
        try {
            const glyph = join(dir, 'glyph.tsv');
            const output = join(dir, 'prior.png');
            accepted('glyphs', '--glyph-stroke', 'a', '500', '500',
                '1800', '3000', '250', '#123456', '--output', glyph);
            const base = [
                'notebook', '--printed-sheet', 'true',
                '--export-paper', 'png', '--glyph-file', glyph,
                '--place-text', '1', '30000', '40000', 'a',
            ];
            for (const [quality, dpi] of [
                ['preview', '72'], ['final', '150'],
            ]) {
                const named = accepted(...base, '--quality', quality);
                const numeric = accepted(...base, '--dpi', dpi);
                assert.deepEqual(named, numeric,
                    `${quality} must preserve native glyph pixels`);
                assert.deepEqual(named.subarray(0, 8),
                    Buffer.from('89504e470d0a1a0a', 'hex'));
                assert.equal(named[25], 6, 'transparent RGBA PNG');
            }
            writeFileSync(output, 'existing reviewed bytes');
            refused([...base, '--quality', 'preview', '--dpi', '72',
                '--output', output, '--replace'],
            /mutually exclusive/, output);
            refused(['notebook', '--export-paper', 'pdf',
                '--quality', 'final', '--output', output, '--replace'],
            /require --export-paper png/, output);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('native flow validates final ink against printable margins before commit',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-margin-'));
        try {
            const shifted = join(dir, 'shifted.json');
            accepted('handwriting', '--set-glyph', 'a', 'baseline_offset',
                '-20000', '-20000', '-20000', '-20000', '0',
                '--output', shifted);
            const authored = ['--glyph-stroke', 'a', '600', '500',
                '1800', '3000', '250', '#123456'];
            for (const [flag, source] of [
                ['--flow-text', 'a'],
                ['--flow-markdown', '# a'],
            ]) {
                for (const format of ['pdf', 'png']) {
                    const output = join(dir,
                        `accepted-${flag.slice(2)}-${format}.output`);
                    const options = ['notebook', '--printed-sheet', 'false',
                        ...authored, flag, source, '--export-paper', format,
                        ...(format === 'png' ? ['--dpi', '72'] : []),
                        '--output', output];
                    accepted(...options);
                    const previous = readFileSync(output);
                    assert.ok(previous.length > 100, 'real native export');
                    const rejected = profiles(...options,
                        '--handwriting-profile', shifted, '--replace');
                    assert.notEqual(rejected.status, 0,
                        'baseline-shifted pen crosses the writable top');
                    assert.match(rejected.stderr.toString('utf8'),
                        /page 1 ink needs printable endpoints and valid width/);
                    assert.equal(rejected.stdout.length, 0,
                        'failed physical export cannot emit partial bytes');
                    assert.deepEqual(readFileSync(output), previous,
                        'invalid physical ink cannot replace accepted export');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('selected PNG page rejects invalid ink on a later flow page',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-offpage-'));
        try {
            const shifted = join(dir, 'shifted.json');
            const output = join(dir, 'reviewed.png');
            accepted('handwriting', '--set-glyph', 'a', 'baseline_offset',
                '-20000', '-20000', '-20000', '-20000', '0',
                '--output', shifted);
            const source = `${'b\n'.repeat(9)}a`;
            const options = ['notebook', '--printed-sheet', 'false',
                '--glyph-stroke', 'a', '600', '500', '1800', '3000',
                '250', '#123456', '--glyph-stroke', 'b', '600', '500',
                '1800', '3000', '250', '#123456', '--flow-text', source,
                '--export-paper', 'png', '--png-page', '1', '--dpi', '72',
                '--output', output];
            accepted(...options);
            const prior = readFileSync(output);
            const failed = profiles(...options, '--handwriting-profile',
                shifted, '--replace');
            assert.notEqual(failed.status, 0);
            assert.match(failed.stderr.toString('utf8'),
                /page [2-9] ink needs printable endpoints and valid width/,
                'unselected later pages must be validated before PNG export');
            assert.equal(failed.stdout.length, 0);
            assert.deepEqual(readFileSync(output), prior,
                'invalid later-page ink cannot replace selected PNG');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('native flow requires an export and cannot be saved as profile JSON',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-export-only-'));
        try {
            const source = join(dir, 'source.md');
            const destination = join(dir, 'previous.json');
            writeFileSync(source, '# a\n\na\n');
            writeFileSync(destination, 'previous accepted bytes');
            const glyph = ['--glyph-stroke', 'a', '500', '500',
                '1800', '3000', '250', '#123456'];
            const sources = [
                ['--flow-text', 'a'],
                ['--flow-markdown', '# a'],
                ['--flow-text-file', source],
                ['--flow-markdown-file', source],
            ];
            for (const selected of sources) {
                const result = profiles('notebook', ...glyph, ...selected,
                    '--output', destination, '--replace');
                assert.notEqual(result.status, 0);
                assert.match(result.stderr.toString('utf8'),
                    /flow source needs --export-paper pdf or png/);
                assert.equal(result.stdout.length, 0);
                assert.equal(readFileSync(destination, 'utf8'),
                    'previous accepted bytes');
            }
            // Even a source without glyphs must not be mistaken for an
            // editable notebook profile or a successful inspection.
            for (const flags of [[], ['--inspect']]) {
                const result = profiles('notebook', '--flow-text', 'a',
                    ...flags);
                assert.notEqual(result.status, 0);
                assert.match(result.stderr.toString('utf8'),
                    /flow source needs --export-paper pdf or png/);
                assert.equal(result.stdout.length, 0);
            }
            const piped = spawnSync(cargo, [
                'run', '--offline', '--quiet', '-p',
                'atrament_profile_editor_cli', '--bin', 'atrament-profiles',
                '--', 'notebook', ...glyph, '--flow-markdown-file', '-',
                '--output', destination, '--replace',
            ], { cwd: root, input: '# a\n', timeout: 45_000 });
            assert.equal(piped.error, undefined, piped.error?.message);
            assert.notEqual(piped.status, 0);
            assert.match(piped.stderr.toString('utf8'),
                /flow source needs --export-paper pdf or png/);
            assert.equal(piped.stdout.length, 0);
            assert.equal(readFileSync(destination, 'utf8'),
                'previous accepted bytes');

            // The explicit profile-only command still emits canonical JSON.
            const profile = accepted('notebook', '--printed-sheet', 'true');
            assert.match(profile.toString('utf8'),
                /"printed_sheet":\s*true/);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
