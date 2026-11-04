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

function render(dir, markdown, format, extra = []) {
    const output = join(dir, `flow-${format}.output`);
    const args = [...base, ...extra, '--flow-markdown', markdown,
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

const punctuation = ['#', '>', '-', '*', '+', '_'];
const punctuationInk = punctuation.flatMap((glyph) => [
    '--glyph-stroke', glyph, '650', '800', '2400', '3100',
    '250', '#112233',
]);

test('escaped Markdown structural marks stay literal on physical pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-escape-'));
        try {
            const source = punctuation.map((mark) =>
                `\\${mark} ó`).join('\n');
            const indented = punctuation.map((mark, index) =>
                `${' '.repeat(index % 4)}\\${mark} ó`).join('\n');
            for (const format of ['pdf', 'png']) {
                const ordinary = render(dir, source, format,
                    punctuationInk);
                assert.equal(ordinary.result.status, 0,
                    ordinary.result.stderr);
                const expected = readFileSync(ordinary.output);
                if (format === 'pdf') {
                    const pdf = expected.toString('latin1');
                    assert.equal(pdf.split('0.067 0.133 0.200 RG').length - 1,
                        6, 'six caller-authored punctuation glyphs');
                    assert.equal(pdf.split('0.071 0.204 0.337 RG').length - 1,
                        6, 'six exact accented handwriting glyphs');
                } else {
                    assert.deepEqual(expected.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                }
                const alternative = render(dir, indented, format,
                    punctuationInk);
                assert.equal(alternative.result.status, 0,
                    alternative.result.stderr);
                assert.deepEqual(readFileSync(alternative.output), expected);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('unsupported backslashes and unauthored literal marks refuse atomically',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-escape-'));
        try {
            const sentinel = Buffer.from('preexisting PDF remains immutable');
            const output = join(dir, 'flow-pdf.output');
            for (const source of ['\\a', '\\# ó', '    \\# ó']) {
                writeFileSync(output, sentinel);
                const rendered = render(dir, source, 'pdf');
                assert.notEqual(rendered.result.status, 0, source);
                assert.match(rendered.result.stderr,
                    /missing authored glyph shapes/);
                assert.deepEqual(readFileSync(output), sentinel);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('indented thematic rules preserve PDF and PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-rule-'));
        try {
            const canonical = 'a\n---\na\n***\na\n___\na';
            const indented = 'a\n ---\na\n  ***\na\n   ___\na';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, canonical, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                if (format === 'pdf') {
                    const pdf = expected.toString('latin1');
                    assert.equal(pdf.split('0.118 0.188 0.290 RG').length - 1,
                        3, 'exactly three decorative rules');
                }
                const candidate = render(dir, indented, format);
                assert.equal(candidate.result.status, 0,
                    candidate.result.stderr);
                assert.deepEqual(readFileSync(candidate.output), expected,
                    `syntactic rule indentation must not change ${format}`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('four-space and separator-only Markdown refuse replacement',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-rule-'));
        try {
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('immutable selected PDF');
            for (const [source, diagnostic] of [
                ['a\n    ---', /missing authored glyph shapes/],
                ['   ---', /flow requires at least one visible text block/],
                ['  ***\n ___',
                    /flow requires at least one visible text block/],
            ]) {
                writeFileSync(output, sentinel);
                const result = render(dir, source, 'pdf');
                assert.notEqual(result.result.status, 0, source);
                assert.match(result.result.stderr, diagnostic);
                assert.deepEqual(readFileSync(output), sentinel);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

const numberedInk = ['1', '2', '.', ')'].flatMap((grapheme) => [
    '--glyph-stroke', grapheme, '550', '850', '2000', '2750',
    '250', '#2277AA',
]);

test('indented bullet quote and ordered blocks preserve actual PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-list-'));
        try {
            const source = '- ó\n* o\u0301\n+ a\n> ó\n> o\u0301\n1. a\n2) ó';
            const indented = ' - ó\n  * o\u0301\n   + a\n > ó\n  > o\u0301'
                + '\n   1. a\n  2) ó';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, source, format, numberedInk);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                if (format === 'pdf') {
                    assert.equal(expected.toString('latin1')
                        .split('0.133 0.467 0.667 RG').length - 1,
                        4, 'authored numerals and punctuation remain visible');
                }
                const candidate = render(dir, indented, format,
                    numberedInk);
                assert.equal(candidate.result.status, 0,
                    candidate.result.stderr);
                assert.deepEqual(readFileSync(candidate.output), expected,
                    `physical ${format} must ignore structural indentation`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('four-space list and quote marks remain literal refused glyphs',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-list-'));
        try {
            const sentinel = Buffer.from('original physical output');
            const output = join(dir, 'flow-pdf.output');
            for (const prefix of ['- ó', '* ó', '+ ó', '> ó']) {
                writeFileSync(output, sentinel);
                const candidate = render(dir, `    ${prefix}`, 'pdf');
                assert.notEqual(candidate.result.status, 0, prefix);
                assert.match(candidate.result.stderr,
                    /missing authored glyph shapes/);
                assert.deepEqual(readFileSync(output), sentinel);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
