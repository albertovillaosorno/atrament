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

function profiles(...args) {
    const result = spawnSync('cargo', [
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
