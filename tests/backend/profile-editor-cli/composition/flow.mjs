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
];
const headingOptions = [
    '--heading-style', 'marker', '--heading-color', '#F8DE91',
];

function render(dir, source, format, extra = [],
    flowFlag = '--flow-markdown', pngPage = 1) {
    const output = join(dir, `flow-${format}.output`);
    const heading = flowFlag === '--flow-markdown' ? headingOptions : [];
    const args = [...base, ...heading, ...extra, flowFlag, source,
        '--export-paper', format, '--output', output, '--replace'];
    if (format === 'png') {
        args.push('--png-page', String(pngPage), '--dpi', '72');
    }
    const result = spawnSync('cargo', [
        'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
        ...args,
    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
    assert.equal(result.error, undefined, result.error?.message);
    return { result, output };
}

test('code fences keep literal Unicode ink and physical hard line breaks',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-'));
        try {
            const code = '# ó\n- a\n\n> o\u0301';
            const samples = [
                `\`\`\`rust\n${code}\n\`\`\``,
                `   ~~~~ note\n${code}\n ~~~~  `,
            ];
            for (const format of ['pdf', 'png']) {
                const plain = render(dir, code, format, punctuationInk,
                    '--flow-text');
                assert.equal(plain.result.status, 0, plain.result.stderr);
                const expected = readFileSync(plain.output);
                if (format === 'pdf') {
                    assert.equal(expected.toString('latin1')
                        .split('0.067 0.133 0.200 RG').length - 1, 3,
                        'code markers are caller-authored literal glyphs');
                }
                for (const source of samples) {
                    const candidate = render(dir, source, format,
                        punctuationInk);
                    assert.equal(candidate.result.status, 0,
                        candidate.result.stderr);
                    assert.deepEqual(readFileSync(candidate.output),
                        expected, 'fence must match literal plain handwriting');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('fenced code paginates literal rows and selected page-two PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-'));
        try {
            const lines = Array.from({ length: 35 }, () => 'a');
            const code = lines.join('\n');
            const fenced = `\`\`\`\n${code}\n\`\`\``;
            const plain = render(dir, code, 'pdf', [], '--flow-text');
            assert.equal(plain.result.status, 0, plain.result.stderr);
            const expectedPdf = readFileSync(plain.output);
            assert.match(expectedPdf.toString('latin1'), /\/Type \/Page\b/);
            const candidate = render(dir, fenced, 'pdf');
            assert.equal(candidate.result.status, 0,
                candidate.result.stderr);
            assert.deepEqual(readFileSync(candidate.output), expectedPdf);
            for (const page of [1, 2]) {
                const native = render(dir, code, 'png', [],
                    '--flow-text', page);
                assert.equal(native.result.status, 0, native.result.stderr);
                const expected = readFileSync(native.output);
                const marked = render(dir, fenced, 'png', [],
                    '--flow-markdown', page);
                assert.equal(marked.result.status, 0,
                    marked.result.stderr);
                assert.deepEqual(readFileSync(marked.output), expected,
                    `code page ${page} must use identical RGBA ink`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('fences end before ordinary Markdown headings and bullet ornaments',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-'));
        try {
            const fenced = 'a\n\`\`\`\na\n\`\`\`\n# a\n- a';
            const reference = 'a  \na  \n# a\n- a';
            for (const format of ['pdf', 'png']) {
                const expectedResult = render(dir, reference, format);
                assert.equal(expectedResult.result.status, 0,
                    expectedResult.result.stderr);
                const expected = readFileSync(expectedResult.output);
                const actual = render(dir, fenced, format);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                assert.deepEqual(readFileSync(actual.output), expected,
                    'closing the code fence restores heading/list semantics');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('unclosed, unapproved and unsupported code fences refuse atomically',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-'));
        try {
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('selected preexisting PDF');
            const cases = [
                ['\`\`\`\na', /unclosed Markdown code fence refused/],
                [' ~~~lang\na', /unclosed Markdown code fence refused/],
                ['\`\`\`\na\n~~~~', /unclosed Markdown code fence refused/],
                ['\`\`\`\`\na\n\`\`\`',
                    /unclosed Markdown code fence refused/],
                ['\`\`\`\n# ó\n\`\`\`',
                    /missing authored glyph shapes/],
                ['    \`\`\`\na', /missing authored glyph shapes/],
                ['\`\`\`\n\`\`\`',
                    /flow requires at least one visible text block/],
            ];
            for (const [source, diagnostic] of cases) {
                writeFileSync(output, sentinel);
                const result = render(dir, source, 'pdf');
                assert.notEqual(result.result.status, 0, source);
                assert.match(result.result.stderr, diagnostic, source);
                assert.deepEqual(readFileSync(output), sentinel);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('empty fenced syntax after final heading does not force blank page',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-orphan-'));
        try {
            const heading = 'a  \n'.repeat(27) + '# a';
            const emptyFence = `${heading}\n\`\`\`\n\`\`\``;
            for (const format of ['pdf', 'png']) {
                const baseline = render(dir, heading, format);
                assert.equal(baseline.result.status, 0,
                    baseline.result.stderr);
                const expected = readFileSync(baseline.output);
                const candidate = render(dir, emptyFence, format);
                assert.equal(candidate.result.status, 0,
                    candidate.result.stderr);
                assert.ok(readFileSync(candidate.output).equals(expected),
                    `an empty fence after the heading cannot move ${format}`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('heading keeps first actual code row after fence syntax',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-orphan-'));
        try {
            const prefix = 'a  \n'.repeat(27);
            const baseline = `${prefix}# a\na`;
            const alternatives = [
                `${prefix}# a\n\`\`\`\na\n\`\`\``,
                `${prefix}# a\n~~~\n~~~\na`,
            ];
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, baseline, format, [],
                    '--flow-markdown', 2);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                for (const candidate of alternatives) {
                    const result = render(dir, candidate, format, [],
                        '--flow-markdown', 2);
                    assert.equal(result.result.status, 0,
                        result.result.stderr);
                    assert.ok(readFileSync(result.output).equals(expected),
                        `heading+next actual row needs page-two ${format}`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('ordinary Markdown paragraphs ignore up to three indentation spaces',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-body-indent-'));
        try {
            const ordinary = 'ó a\no\u0301 a\n# ó\nó\na';
            const indented = '   ó a\n  o\u0301 a\n# ó\n ó\n   a';
            for (const format of ['pdf', 'png']) {
                const baseline = render(dir, ordinary, format);
                assert.equal(baseline.result.status, 0,
                    baseline.result.stderr);
                const expected = readFileSync(baseline.output);
                const result = render(dir, indented, format);
                assert.equal(result.result.status, 0,
                    result.result.stderr);
                assert.deepEqual(readFileSync(result.output), expected,
                    `normal paragraph indent must not affect ${format}`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('four-space paragraphs and fenced code retain literal indentation',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-body-indent-'));
        try {
            for (const format of ['pdf', 'png']) {
                const unindented = render(dir, 'a', format);
                assert.equal(unindented.result.status, 0,
                    unindented.result.stderr);
                const noIndent = readFileSync(unindented.output);
                for (const [markdown, plain] of [
                    ['    a', '    a'],
                    ['\`\`\`\n  a\n\`\`\`', '  a'],
                ]) {
                    const reference = render(dir, plain, format, [],
                        '--flow-text');
                    assert.equal(reference.result.status, 0,
                        reference.result.stderr);
                    const expected = readFileSync(reference.output);
                    const result = render(dir, markdown, format);
                    assert.equal(result.result.status, 0,
                        result.result.stderr);
                    assert.deepEqual(readFileSync(result.output), expected,
                        `literal indentation must survive in ${format}`);
                    assert.notDeepEqual(expected, noIndent,
                        'authored spaces change physical handwriting offset');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('heading marker stroke weight is selectable in native PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/heading-weight-'));
        try {
            const source = '# ó\n## a\n### ó\na';
            for (const format of ['pdf', 'png']) {
                const defaultRun = render(dir, source, format);
                assert.equal(defaultRun.result.status, 0,
                    defaultRun.result.stderr);
                const standard = readFileSync(defaultRun.output);
                const hundred = render(dir, source, format,
                    ['--heading-marker-scale', '100']);
                assert.equal(hundred.result.status, 0,
                    hundred.result.stderr);
                assert.deepEqual(readFileSync(hundred.output), standard,
                    'default 100% must preserve the original marker output');
                const narrow = render(dir, source, format,
                    ['--heading-marker-scale', '50']);
                assert.equal(narrow.result.status, 0, narrow.result.stderr);
                const thin = readFileSync(narrow.output);
                const wide = render(dir, source, format,
                    ['--heading-marker-scale', '105']);
                assert.equal(wide.result.status, 0, wide.result.stderr);
                const thick = readFileSync(wide.output);
                assert.notDeepEqual(thin, standard, `${format} changes at 50%`);
                assert.notDeepEqual(thick, standard,
                    `${format} changes at 105%`);
                assert.notDeepEqual(thin, thick,
                    `${format} weight is distinct`);
                if (format === 'pdf') {
                    const native = thick.toString('latin1');
                    assert.equal(native.split('0.973 0.871 0.569 RG')
                        .length - 1,
                        9, 'only marker strokes retain pastel highlighter');
                    assert.equal(native.split('0.071 0.204 0.337 RG')
                        .length - 1,
                        4, 'body and heading use unchanged authored ink');
                } else {
                    assert.deepEqual(thick.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('invalid marker weights refuse without replacing accepted PDF',
    () => {
        const dir = mkdtempSync(join(root, '.temp/heading-weight-'));
        try {
            const output = join(dir, 'flow-pdf.output');
            const saved = Buffer.from('already approved physical notes');
            for (const values of [
                ['49'], ['106'], ['-1'], ['0'], ['100.5'],
                ['9999999999999999999999'], [''], ['80',
                    '--heading-marker-scale', '110'],
            ]) {
                writeFileSync(output, saved);
                const candidate = render(dir, '# ó', 'pdf',
                    ['--heading-marker-scale', ...values]);
                assert.notEqual(candidate.result.status, 0, values.join(' '));
                assert.match(candidate.result.stderr, /marker scale|heading/);
                assert.deepEqual(readFileSync(output), saved);
            }
            for (const style of [undefined, 'none', 'underline']) {
                writeFileSync(output, saved);
                const args = [
                    ...base, '--heading-marker-scale', '100',
                    ...(style ? ['--heading-style', style] : []),
                    '--flow-markdown', '# ó', '--export-paper', 'pdf',
                    '--output', output, '--replace',
                ];
                const refused = spawnSync('cargo', [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...args,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.notEqual(refused.status, 0, String(style));
                assert.match(refused.stderr, /requires --heading-style marker/);
                assert.deepEqual(readFileSync(output), saved);
            }
            const missing = render(dir, '# ó', 'pdf', [
                '--heading-marker-scale',
            ]);
            assert.notEqual(missing.result.status, 0);
            assert.deepEqual(readFileSync(output), saved);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

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

test('fenced code respects repeated fixed regions and native page ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-region-'));
        try {
            const plain = 'a\n# ó\n\n> a\no\u0301';
            const markdown = `\`\`\`\n${plain}\n\`\`\``;
            const region = [
                '--flow-region', '30000', '40000', '50000', '30000',
            ];
            const extra = [...punctuationInk, ...region];
            for (const [format, pages] of [
                ['pdf', [1]], ['png', [1, 2, 3]],
            ]) {
                for (const page of pages) {
                    const expectedResult = render(dir, plain, format,
                        extra, '--flow-text', page);
                    assert.equal(expectedResult.result.status, 0,
                        expectedResult.result.stderr);
                    const expected = readFileSync(expectedResult.output);
                    const candidate = render(dir, markdown, format,
                        extra, '--flow-markdown', page);
                    assert.equal(candidate.result.status, 0,
                        candidate.result.stderr);
                    assert.ok(readFileSync(candidate.output).equals(expected),
                        `page ${page} fenced code in ${format} matches ink`);
                }
            }
            const pngOutput = join(dir, 'flow-png.output');
            const pngSentinel = Buffer.from('keep the selected PNG');
            writeFileSync(pngOutput, pngSentinel);
            const beyond = render(dir, markdown, 'png', extra,
                '--flow-markdown', 4);
            assert.notEqual(beyond.result.status, 0);
            assert.match(beyond.result.stderr, /page|range/);
            assert.deepEqual(readFileSync(pngOutput), pngSentinel);
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('keep the existing PDF');
            writeFileSync(output, sentinel);
            const invalid = render(dir, markdown, 'pdf', [
                ...punctuationInk, '--flow-region',
                '30000', '40000', '50000', '9000',
            ]);
            assert.notEqual(invalid.result.status, 0);
            assert.match(invalid.result.stderr, /vertical|region/);
            assert.deepEqual(readFileSync(output), sentinel);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

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

const fenceLiteralInk = ['`', '~'].flatMap((glyph) => [
    '--glyph-stroke', glyph, '650', '800', '2400', '3100',
    '250', '#112233',
]);

test('escaped fence-like source prints exact authored PDF and PNG glyphs',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-escape-'));
        try {
            const literal = '``` a\n~~~ a';
            const escaped = '\\``` a  \n\\~~~ a';
            const indented = '  \\``` a  \n   \\~~~ a';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, literal, format,
                    fenceLiteralInk, '--flow-text');
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                if (format === 'pdf') {
                    const pdf = expected.toString('latin1');
                    assert.equal(pdf.split('0.067 0.133 0.200 RG').length - 1,
                        6, 'six literal backticks and tildes are authored');
                }
                for (const markdown of [escaped, indented]) {
                    const candidate = render(dir, markdown, format,
                        fenceLiteralInk);
                    assert.equal(candidate.result.status, 0,
                        candidate.result.stderr);
                    assert.ok(readFileSync(candidate.output).equals(expected),
                        `escaped fence marks must be literal in ${format}`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('literal fence marks without authored glyphs refuse atomically',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-fence-escape-'));
        try {
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('preserve selected document');
            for (const [source, codepoint] of [
                ['\\``` a', /U\+0060/],
                ['\\~~~ a', /U\+007E/],
            ]) {
                writeFileSync(output, sentinel);
                const failed = render(dir, source, 'pdf');
                assert.notEqual(failed.result.status, 0, source);
                assert.match(failed.result.stderr,
                    /missing authored glyph shapes/);
                assert.match(failed.result.stderr, codepoint);
                assert.deepEqual(readFileSync(output), sentinel);
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

test('spaced thematic rules render identical native PDF and PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-spaced-rule-'));
        try {
            const canonical = 'a\n---\na\n***\na\n___\na';
            const spaced = 'a\n- - -  \na\n *  * *\na\n  _ _   _\na';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, canonical, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                if (format === 'pdf') {
                    assert.equal(expected.toString('latin1')
                        .split('0.118 0.188 0.290 RG').length - 1, 3,
                        'three native horizontal pen rules');
                }
                const candidate = render(dir, spaced, format);
                assert.equal(candidate.result.status, 0,
                    candidate.result.stderr);
                assert.deepEqual(readFileSync(candidate.output), expected,
                    `spaced syntax must not alter physical ${format}`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('near-miss thematic spacing stays literal and refuses replacement',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-spaced-rule-'));
        try {
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('selected PDF is unchanged');
            for (const source of [
                'a\n- -', 'a\n- - *', 'a\n- - x',
                'a\n    - - -', 'a\n\\- - -',
            ]) {
                writeFileSync(output, sentinel);
                const candidate = render(dir, source, 'pdf');
                assert.notEqual(candidate.result.status, 0, source);
                assert.match(candidate.result.stderr,
                    /missing authored glyph shapes/);
                assert.deepEqual(readFileSync(output), sentinel,
                    `refusal must not overwrite PDF: ${source}`);
            }
            writeFileSync(output, sentinel);
            const empty = render(dir, ' - - -', 'pdf');
            assert.notEqual(empty.result.status, 0);
            assert.match(empty.result.stderr,
                /flow requires at least one visible text block/);
            assert.deepEqual(readFileSync(output), sentinel);
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

test('task lists draw checked and unchecked page-local pen boxes',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-checkbox-'));
        try {
            const source = '- [ ] a\n* [x] ó\n+ [X] a\na';
            const indented = ' - [ ] a\n  * [x] ó\n   + [X] a\na';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, source, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                if (format === 'pdf') {
                    const pdf = expected.toString('latin1');
                    assert.equal(pdf.split('0.118 0.188 0.290 RG').length - 1,
                        16, 'three checkbox outlines, two with check marks');
                    assert.equal(pdf.split('0.071 0.204 0.337 RG').length - 1,
                        4, 'four actual authored glyph strokes');
                } else {
                    assert.deepEqual(expected.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                }
                const candidate = render(dir, indented, format);
                assert.equal(candidate.result.status, 0,
                    candidate.result.stderr);
                assert.deepEqual(readFileSync(candidate.output), expected,
                    `syntactic checkbox indentation must not alter ${format}`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('wrapped task checkbox stays on its first physical page',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-checkbox-'));
        try {
            const source = '- [x] ' + Array(31).fill('a').join(' ');
            const region = [
                '--flow-region', '30000', '40000', '50000', '30000',
            ];
            const pdf = render(dir, source, 'pdf', region);
            assert.equal(pdf.result.status, 0, pdf.result.stderr);
            const vector = readFileSync(pdf.output).toString('latin1');
            assert.match(vector, /\/Count [2-9]\b/,
                'long task body paginates across physical pages');
            assert.equal(vector.split('0.118 0.188 0.290 RG').length - 1,
                6, 'wrapped task emits exactly one checked box');
            const nextPage = render(dir, source, 'png', region,
                '--flow-markdown', 2);
            assert.equal(nextPage.result.status, 0,
                nextPage.result.stderr);
            assert.deepEqual(readFileSync(nextPage.output).subarray(0, 8),
                Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

const checkboxLiteralInk = ['-', '[', ']', 'x'].flatMap((glyph) => [
    '--glyph-stroke', glyph, '650', '800', '2400', '3100',
    '250', '#112233',
]);

test('escaped task source stays literal and invalid boxes refuse atomically',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-checkbox-'));
        try {
            const literal = '- [x] a';
            const escaped = '\\- [x] a';
            for (const format of ['pdf', 'png']) {
                const plain = render(dir, literal, format,
                    checkboxLiteralInk, '--flow-text');
                assert.equal(plain.result.status, 0, plain.result.stderr);
                const expected = readFileSync(plain.output);
                const markdown = render(dir, escaped, format,
                    checkboxLiteralInk);
                assert.equal(markdown.result.status, 0,
                    markdown.result.stderr);
                assert.deepEqual(readFileSync(markdown.output), expected,
                    'escaped task mark is caller-authored handwriting');
                const fenced = render(dir,
                    `\`\`\`\n${literal}\n\`\`\``,
                    format, checkboxLiteralInk);
                assert.equal(fenced.result.status, 0,
                    fenced.result.stderr);
                assert.deepEqual(readFileSync(fenced.output), expected,
                    'literal fenced task marks keep authored punctuation');
            }
            const sentinel = Buffer.from('saved notebook output');
            const output = join(dir, 'flow-pdf.output');
            for (const source of [
                '- [ ]', '- [x]', '- [y] a', '    - [x] a',
            ]) {
                writeFileSync(output, sentinel);
                const candidate = render(dir, source, 'pdf');
                assert.notEqual(candidate.result.status, 0, source);
                assert.match(candidate.result.stderr,
                    /empty Markdown block|missing authored glyph shapes/);
                assert.deepEqual(readFileSync(output), sentinel);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

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
