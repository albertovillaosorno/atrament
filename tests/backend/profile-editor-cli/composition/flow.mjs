// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
// Real native PDF/PNG acceptance for bounded Markdown heading syntax.
// Run: node --test tests/backend/profile-editor-cli/composition/flow.mjs
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const base = [
    'notebook', '--printed-sheet', 'false',
    '--glyph-stroke', 'a', '600', '500', '1800', '3000', '250', '#123456',
    '--glyph-stroke', 'ó', '900', '600', '1700', '3300', '250', '#123456',
    '--glyph-stroke', 'o\u0301', '1100', '600', '1900', '3300',
    '250', '#123456',
    '--heading-style', 'marker', '--heading-color', '#F8DE91',
];

function render(dir, markdown, format) {
    const output = join(dir, `flow-${format}.output`);
    const args = [...base, '--flow-markdown', markdown,
        '--export-paper', format, '--output', output, '--replace'];
    if (format === 'png') {
        args.push('--png-page', '1', '--dpi', '72');
    }
    const result = spawnSync('cargo', [
        'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
        ...args,
    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
    assert.equal(result.error, undefined, result.error?.message);
    return { result, output };
}

test('closing ATX markers and up-to-three-space headings emit identical ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-flow-'));
        try {
            const canonical = '# ó\n## o\u0301\n### a\na';
            const decorated = '   # ó ###  \n ## o\u0301 ##\n  ### a #\na';
            for (const format of ['pdf', 'png']) {
                const standard = render(dir, canonical, format);
                assert.equal(standard.result.status, 0,
                    standard.result.stderr);
                const expected = readFileSync(standard.output);
                if (format === 'pdf') {
                    assert.ok(expected.toString('latin1').startsWith('%PDF-'));
                } else {
                    assert.deepEqual(expected.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                }
                const alternative = render(dir, decorated, format);
                assert.equal(alternative.result.status, 0,
                    alternative.result.stderr);
                assert.deepEqual(readFileSync(alternative.output), expected,
                    `real ${format} ink must ignore heading syntax`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('literal hashes and empty headings refuse without replacing output',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-flow-'));
        try {
            const sentinel = Buffer.from('existing saved document');
            for (const source of [
                '# ó#', '# ó #not-a-closer', '    # ó', '# ###',
                '##    ###   ',
            ]) {
                const output = join(dir, 'flow-pdf.output');
                writeFileSync(output, sentinel);
                const rendered = render(dir, source, 'pdf');
                assert.notEqual(rendered.result.status, 0, source);
                if (source.trimStart().startsWith('##')
                    || source === '# ###') {
                    assert.match(rendered.result.stderr,
                        /empty Markdown block refused/);
                } else {
                    assert.match(rendered.result.stderr,
                        /missing authored glyph shapes/);
                }
                assert.deepEqual(readFileSync(rendered.output), sentinel,
                    `failed source must not replace PDF: ${source}`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
