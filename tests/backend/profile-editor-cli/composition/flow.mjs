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
    const heading = flowFlag.startsWith('--flow-markdown')
        && !extra.includes('--heading-style') ? headingOptions : [];
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

function coloredHeadingWidths(bytes) {
    return [...bytes.toString('latin1').matchAll(
        /0\.973 0\.871 0\.569 RG\n([0-9.]+) w/g,
    )].map((match) => match[1]);
}

test('notebook CLI help advertises real Markdown task shapes', () => {
    const help = spawnSync('cargo', [
        'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
        'notebook', '--help',
    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
    assert.equal(help.status, 0, help.stderr);
    assert.match(help.stdout, /\[ \]\/\[x\]\/\[X\] checkbox tasks/);
    assert.match(help.stdout, /--heading-marker-scale 50\.\.105/);
    assert.match(help.stdout, /# through ###### headings/);
});

test('trailing invisible rows never create extra native PDF or PNG pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-trailing-blank-'));
        try {
            for (const [flag, source] of [
                ['--flow-text', `a\n${'\n'.repeat(55)}`],
                ['--flow-markdown',
                    `a\n~~~txt\n${'\n'.repeat(55)}~~~`],
            ]) {
                for (const format of ['pdf', 'png']) {
                    const pageLimit = format === 'pdf'
                        ? ['--pages', '1'] : [];
                    const clean = render(dir, 'a', format,
                        pageLimit, flag);
                    assert.equal(clean.result.status, 0,
                        clean.result.stderr);
                    const expected = readFileSync(clean.output);
                    const trailing = render(dir, source, format,
                        pageLimit, flag);
                    assert.equal(trailing.result.status, 0,
                        trailing.result.stderr);
                    assert.deepEqual(readFileSync(trailing.output), expected,
                        `trailing blank ${flag} must not export empty pages`);
                    if (format === 'png') {
                        const pageTwo = render(dir, source, format,
                            [], flag, 2);
                        assert.notEqual(pageTwo.result.status, 0,
                            'page two must not exist without printable ink');
                        assert.deepEqual(readFileSync(pageTwo.output),
                            expected, 'refusal preserves selected PNG');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('blank-only fenced block cannot orphan the final Markdown heading',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-last-heading-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '100000', '23000'];
            const clean = 'a\n# a';
            const trailing = `${clean}\n~~~txt\n${'\n'.repeat(55)}~~~`;
            for (const format of ['pdf', 'png']) {
                const options = format === 'pdf'
                    ? [...region, '--pages', '1'] : region;
                const expected = render(dir, clean, format, options);
                assert.equal(expected.result.status, 0,
                    expected.result.stderr);
                const bytes = readFileSync(expected.output);
                const actual = render(dir, trailing, format, options);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                assert.deepEqual(readFileSync(actual.output), bytes,
                    'fenced blank rows cannot move the last heading');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('adjacent heading levels travel with their first body row in PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-heading-chain-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '100000', '45000'];
            const prelude = 'a  \na';
            const group = '# a\n## ó\na';
            const combined = `${prelude}\n${group}`;
            const firstPage = render(dir, combined, 'png', region);
            assert.equal(firstPage.result.status, 0,
                firstPage.result.stderr);
            const firstInk = readFileSync(firstPage.output);
            const alonePrelude = render(dir, prelude, 'png', region);
            assert.equal(alonePrelude.result.status, 0,
                alonePrelude.result.stderr);
            assert.deepEqual(firstInk, readFileSync(alonePrelude.output),
                'page one must contain only the prelude');

            const secondPage = render(dir, combined, 'png', region,
                '--flow-markdown', 2);
            assert.equal(secondPage.result.status, 0,
                secondPage.result.stderr);
            const secondInk = readFileSync(secondPage.output);
            const aloneGroup = render(dir, group, 'png', region);
            assert.equal(aloneGroup.result.status, 0,
                aloneGroup.result.stderr);
            assert.deepEqual(secondInk, readFileSync(aloneGroup.output),
                'page two must retain both heading markers and body ink');

            const accepted = render(dir, combined, 'pdf',
                [...region, '--pages', '2']);
            assert.equal(accepted.result.status, 0,
                accepted.result.stderr);
            const bytes = readFileSync(accepted.output);
            assert.match(bytes.toString('latin1'), /\/Count 2/,
                'actual native PDF must contain two physical pages');
            const refused = render(dir, combined, 'pdf',
                [...region, '--pages', '1']);
            assert.notEqual(refused.result.status, 0);
            assert.match(refused.result.stderr, /page/i);
            assert.deepEqual(readFileSync(refused.output), bytes,
                'insufficient page selection preserves accepted PDF bytes');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('ATX and Setext heading chains keep native PDF and PNG ink together',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-mixed-headings-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '100000', '45000'];
            const prelude = 'a  \na  ';
            for (const group of [
                '# a\nó\n===\na',
                'a\n===\nó\n---\na',
                '# a\nó\n===\n## a\na',
                '# a\nó\na\n===\na',
                'ó\na\n===\na\nó\n---\na',
            ]) {
                const combined = `${prelude}\n${group}`;
                const first = render(dir, combined, 'png', region);
                assert.equal(first.result.status, 0,
                    first.result.stderr);
                const firstInk = readFileSync(first.output);
                const standalonePrelude = render(dir, prelude,
                    'png', region);
                assert.equal(standalonePrelude.result.status, 0,
                    standalonePrelude.result.stderr);
                assert.deepEqual(firstInk,
                    readFileSync(standalonePrelude.output),
                    'page one must not orphan a mixed heading');

                const second = render(dir, combined, 'png', region,
                    '--flow-markdown', 2);
                assert.equal(second.result.status, 0,
                    second.result.stderr);
                const secondInk = readFileSync(second.output);
                const standaloneGroup = render(dir, group, 'png', region);
                assert.equal(standaloneGroup.result.status, 0,
                    standaloneGroup.result.stderr);
                assert.deepEqual(secondInk,
                    readFileSync(standaloneGroup.output),
                    'page two has every heading stroke and body mark');

                const pdf = render(dir, combined, 'pdf',
                    [...region, '--pages', '2']);
                assert.equal(pdf.result.status, 0, pdf.result.stderr);
                const accepted = readFileSync(pdf.output);
                assert.match(accepted.toString('latin1'), /\/Count 2/);
                const refused = render(dir, combined, 'pdf',
                    [...region, '--pages', '1']);
                assert.notEqual(refused.result.status, 0);
                assert.match(refused.result.stderr, /page/i);
                assert.deepEqual(readFileSync(refused.output), accepted,
                    'refused page limit cannot replace the accepted PDF');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('blank-separated heading groups preserve native PDF and PNG pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-title-blank-'));
        try {
            for (const [height, group] of [
                ['41000', '# a\n\n## ó\na'],
                ['41000', '# a\n\nó\n===\na'],
                ['50000', '# a\n\n## ó\n\n### a\na'],
            ]) {
                const region = ['--flow-region', '30000', '40000',
                    '100000', height];
                const prelude = 'a  ';
                const combined = `${prelude}\n${group}`;
                const first = render(dir, combined, 'png', region);
                assert.equal(first.result.status, 0, first.result.stderr);
                const actualFirstInk = readFileSync(first.output);
                const expectedFirst = render(dir, prelude, 'png', region);
                assert.equal(expectedFirst.result.status, 0,
                    expectedFirst.result.stderr);
                assert.deepEqual(actualFirstInk,
                    readFileSync(expectedFirst.output),
                    'initial sheet must contain only the prelude');

                const second = render(dir, combined, 'png', region,
                    '--flow-markdown', 2);
                assert.equal(second.result.status, 0, second.result.stderr);
                const actualSecondInk = readFileSync(second.output);
                const expectedSecond = render(dir, group, 'png', region);
                assert.equal(expectedSecond.result.status, 0,
                    expectedSecond.result.stderr);
                assert.deepEqual(actualSecondInk,
                    readFileSync(expectedSecond.output),
                    'the next sheet retains every title, gap and body');

                const pdf = render(dir, combined, 'pdf',
                    [...region, '--pages', '2']);
                assert.equal(pdf.result.status, 0, pdf.result.stderr);
                const accepted = readFileSync(pdf.output);
                assert.match(accepted.toString('latin1'), /\/Count 2\b/);
                const rejected = render(dir, combined, 'pdf',
                    [...region, '--pages', '1']);
                assert.notEqual(rejected.result.status, 0);
                assert.match(rejected.result.stderr, /page/i);
                assert.deepEqual(readFileSync(rejected.output), accepted,
                    'insufficient page selection preserves the prior PDF');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('code fence underline ink cannot promote a preceding Markdown row',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-fence-setext-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '100000', '45000'];
            const equal = ['--glyph-stroke', '=',
                '600', '500', '1800', '3000', '250', '#123456'];
            const firstPage = 'a  \na\n# a\nó';
            for (const fence of ['~~~', '```']) {
                const combined = `${firstPage}\n${fence}txt\n===\n`
                    + `${fence}\na`;
                const accepted = render(dir, firstPage, 'png',
                    [...region, ...equal]);
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const expected = readFileSync(accepted.output);
                const actual = render(dir, combined, 'png',
                    [...region, ...equal]);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                assert.deepEqual(readFileSync(actual.output), expected,
                    'fenced equals signs cannot move preceding heading');

                const pdf = render(dir, combined, 'pdf',
                    [...region, ...equal, '--pages', '2']);
                assert.equal(pdf.result.status, 0, pdf.result.stderr);
                assert.match(readFileSync(pdf.output).toString('latin1'),
                    /\/Count 2/, 'code ink starts on physical page two');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('saved BOM/CRLF Markdown task file exports like inline PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-checkbox-file-'));
        try {
            const source = '- [ ] a\n* [x] ó\n+ [X] o\u0301';
            const path = join(dir, 'tasks.md');
            const saved = Buffer.concat([
                Buffer.from([0xef, 0xbb, 0xbf]),
                Buffer.from(source.replaceAll('\n', '\r\n'), 'utf8'),
            ]);
            writeFileSync(path, saved);
            for (const format of ['pdf', 'png']) {
                const inline = render(dir, source, format);
                assert.equal(inline.result.status, 0,
                    inline.result.stderr);
                const expected = readFileSync(inline.output);
                const reopened = render(dir, path, format, [],
                    '--flow-markdown-file');
                assert.equal(reopened.result.status, 0,
                    reopened.result.stderr);
                assert.deepEqual(readFileSync(reopened.output), expected,
                    `${format} same task syntax from saved Markdown`);
            }
            assert.deepEqual(readFileSync(path), saved,
                'source Markdown file remains exactly unchanged');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('pasted tab stops preserve physical text and Markdown ink parity',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-tabs-'));
        try {
            const samples = [
                {
                    format: '--flow-text',
                    tabbed: 'o\u0301\tó\n\tó',
                    expanded: 'o\u0301   ó\n    ó',
                },
                {
                    format: '--flow-markdown',
                    tabbed: '- [x]\tó\n~~~rs\nó\t a\n~~~\n# a',
                    expanded: '- [x]   ó\n~~~rs\nó    a\n~~~\n# a',
                },
            ];
            for (const { format, tabbed, expanded } of samples) {
                for (const outputFormat of ['pdf', 'png']) {
                    const reference = render(dir, expanded, outputFormat,
                        [], format);
                    assert.equal(reference.result.status, 0,
                        reference.result.stderr);
                    const expected = readFileSync(reference.output);
                    const pasted = render(dir, tabbed, outputFormat,
                        [], format);
                    assert.equal(pasted.result.status, 0,
                        pasted.result.stderr);
                    assert.deepEqual(readFileSync(pasted.output), expected,
                        'tab stops preserve actual physical ink bytes');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('saved BOM/CRLF tabs preserve Unicode and physical export parity',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-tabs-file-'));
        try {
            const tabbed = '- [x]\tó\no\u0301\t a';
            const expanded = '- [x]   ó\no\u0301    a';
            const path = join(dir, 'tabs.md');
            const saved = Buffer.from('\uFEFF' +
                tabbed.replaceAll('\n', '\r\n'), 'utf8');
            writeFileSync(path, saved);
            for (const format of ['pdf', 'png']) {
                const inline = render(dir, expanded, format);
                assert.equal(inline.result.status, 0, inline.result.stderr);
                const expected = readFileSync(inline.output);
                const opened = render(dir, path, format, [],
                    '--flow-markdown-file');
                assert.equal(opened.result.status, 0,
                    opened.result.stderr);
                assert.deepEqual(readFileSync(opened.output), expected);
            }
            assert.deepEqual(readFileSync(path), saved,
                'source tabs and decomposed accents remain byte-identical');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('pasted Unicode paragraph separators become physical rows',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-separators-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '50000', '30000'];
            for (const { flag, pasted, lines } of [
                {
                    flag: '--flow-text',
                    pasted: 'ó\u2028a\u2029o\u0301',
                    lines: 'ó\na\n\no\u0301',
                },
                {
                    flag: '--flow-markdown',
                    pasted: '# a\u2028- [x] ó\u2029o\u0301',
                    lines: '# a\n- [x] ó\n\no\u0301',
                },
            ]) {
                for (const format of ['pdf', 'png']) {
                    const canonical = render(dir, lines, format,
                        region, flag, 2);
                    assert.equal(canonical.result.status, 0,
                        canonical.result.stderr);
                    const expected = readFileSync(canonical.output);
                    const separated = render(dir, pasted, format,
                        region, flag, 2);
                    assert.equal(separated.result.status, 0,
                        separated.result.stderr);
                    assert.deepEqual(readFileSync(separated.output),
                        expected, 'separator codepoints are physical breaks');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('Unicode line separator keeps a hard Markdown row',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-separators-'));
        try {
            for (const format of ['pdf', 'png']) {
                const hard = render(dir, 'a  \nó', format);
                assert.equal(hard.result.status, 0, hard.result.stderr);
                const expected = readFileSync(hard.output);
                const unicode = render(dir, 'a\u2028ó', format);
                assert.equal(unicode.result.status, 0,
                    unicode.result.stderr);
                assert.deepEqual(readFileSync(unicode.output), expected);
                const soft = render(dir, 'a\nó', format);
                assert.equal(soft.result.status, 0, soft.result.stderr);
                assert.notDeepEqual(readFileSync(soft.output), expected,
                    'Unicode line break must not soft-wrap a paragraph');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('paragraph separator produces a real measured gap in both modes',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-separators-'));
        try {
            for (const flag of ['--flow-text', '--flow-markdown']) {
                for (const format of ['pdf', 'png']) {
                    const joined = render(dir, 'a\u2028ó', format,
                        [], flag);
                    assert.equal(joined.result.status, 0,
                        joined.result.stderr);
                    const oneLineBreak = readFileSync(joined.output);
                    const separated = render(dir, 'a\u2029ó', format,
                        [], flag);
                    assert.equal(separated.result.status, 0,
                        separated.result.stderr);
                    assert.notDeepEqual(readFileSync(separated.output),
                        oneLineBreak, 'paragraph must add physical space');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('saved mixed Unicode separators keep physical PDF and PNG parity',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-separators-'));
        try {
            const source = 'ó\u2028a\r\no\u0301\u2029a';
            const canonical = 'ó\na\no\u0301\n\na';
            const path = join(dir, 'unicode-lines.txt');
            const saved = Buffer.from('\uFEFF' + source, 'utf8');
            writeFileSync(path, saved);
            for (const format of ['pdf', 'png']) {
                const inline = render(dir, canonical, format, [],
                    '--flow-text');
                assert.equal(inline.result.status, 0, inline.result.stderr);
                const expected = readFileSync(inline.output);
                const file = render(dir, path, format, [],
                    '--flow-text-file');
                assert.equal(file.result.status, 0, file.result.stderr);
                assert.deepEqual(readFileSync(file.output), expected);
            }
            assert.deepEqual(readFileSync(path), saved,
                'native source read never rewrites authored bytes');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('expanded tab source stays bounded and refuses atomic replacements',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-tabs-'));
        try {
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('accepted physical PDF');
            for (const source of [
                'a\t'.repeat(1025), 'a\vó', 'a\u0085ó',
                '\u2028\u2029',
            ]) {
                writeFileSync(output, sentinel);
                const candidate = render(dir, source, 'pdf', [],
                    '--flow-text');
                assert.notEqual(candidate.result.status, 0);
                assert.match(candidate.result.stderr,
                    /4096 UTF-8 bytes|control characters/);
                assert.deepEqual(readFileSync(output), sentinel);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('indented glyph cannot spill into an invisible physical row',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-indent-'));
        try {
            const wide = ['--flow-region', '30000', '40000',
                '30000', '50000'];
            const narrow = ['--flow-region', '30000', '40000',
                '20000', '50000'];
            for (const format of ['pdf', 'png']) {
                for (const flag of ['--flow-text', '--flow-markdown']) {
                    const accepted = render(dir, '    a', format, wide,
                        flag);
                    assert.equal(accepted.result.status, 0,
                        accepted.result.stderr);
                    const original = readFileSync(accepted.output);
                    for (const input of flag === '--flow-markdown'
                        ? ['    a', '```\n    a\n```']
                        : ['    a', 'a\n    a']) {
                        const refused = render(dir, input, format,
                            narrow, flag);
                        assert.notEqual(refused.result.status, 0,
                            'spaces-only physical rows must be refused');
                        assert.match(refused.result.stderr,
                            /indentation cannot fit first visible glyph/);
                        assert.deepEqual(readFileSync(refused.output),
                            original, 'failed layout preserves export');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

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

test('fenced blank rows keep heading and following ink on page two',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-heading-blank-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '100000', '36000'];
            // Both sources put the same authored ink on the same two
            // physical pages. The first needs a heading orphan guard;
            // the second fills page one before the heading naturally.
            const cases = [
                ['a\n# a\n~~~txt\n\nó\n~~~',
                    'a\n~~~txt\n\n\n~~~\n# a\n~~~txt\n\nó\n~~~'],
                ['a\n# a\n\n~~~txt\n\nó\n~~~',
                    'a\n~~~txt\n\n\n~~~\n# a\n\n~~~txt\n\nó\n~~~'],
            ];
            for (const [candidate, expected] of cases) {
                for (const format of ['pdf', 'png']) {
                    for (const page of format === 'pdf' ? [1] : [1, 2]) {
                        const reference = render(dir, expected, format,
                            region, '--flow-markdown', page);
                        assert.equal(reference.result.status, 0,
                            reference.result.stderr);
                        const bytes = readFileSync(reference.output);
                        const actual = render(dir, candidate, format,
                            region, '--flow-markdown', page);
                        assert.equal(actual.result.status, 0,
                            actual.result.stderr);
                        assert.deepEqual(readFileSync(actual.output), bytes,
                            `${format} page ${page} must keep physical ink`);
                        if (format === 'pdf') {
                            assert.equal((bytes.toString('latin1')
                                .match(/\/Type \/Page\b/g) || []).length,
                                2);
                        }
                    }
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

test('four-space literal Markdown rows preserve source line placement',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-literal-rows-'));
        try {
            const region = [
                '--flow-region', '30000', '40000', '50000', '30000',
            ];
            for (const source of [
                'a\n    ó\n    o\u0301',
                '    a\n    ó\n    o\u0301',
            ]) {
                const explicit = source.replaceAll('\n', '  \n');
                for (const format of ['pdf', 'png']) {
                    const plain = render(dir, source, format, region,
                        '--flow-text', 2);
                    assert.equal(plain.result.status, 0,
                        plain.result.stderr);
                    const reference = readFileSync(plain.output);
                    const literal = render(dir, source, format, region,
                        '--flow-markdown', 2);
                    assert.equal(literal.result.status, 0,
                        literal.result.stderr);
                    assert.deepEqual(readFileSync(literal.output), reference,
                        `literal Markdown row placement in ${format}`);
                    const broken = render(dir, explicit, format, region,
                        '--flow-markdown', 2);
                    assert.equal(broken.result.status, 0,
                        broken.result.stderr);
                    assert.deepEqual(readFileSync(broken.output), reference,
                        `explicit hard breaks agree with literal ${format}`);
                    if (format === 'pdf') {
                        assert.match(reference.toString('latin1'),
                            /\/Count 2\b/, 'three rows occupy two sheets');
                    } else {
                        assert.deepEqual(reference.subarray(0, 8),
                            Buffer.from('89504e470d0a1a0a', 'hex'));
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('four-space literal Markdown retains trailing authored backslash ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-literal-slash-'));
        try {
            const slash = String.fromCharCode(92);
            const slashInk = [
                '--glyph-stroke', slash, '600', '500', '1800', '3000',
                '250', '#123456',
            ];
            for (const format of ['pdf', 'png']) {
                for (const prefix of ['', 'a\n']) {
                    const input = `${prefix}    ó${slash}`;
                    const plain = render(dir, input, format, slashInk,
                        '--flow-text');
                    assert.equal(plain.result.status, 0,
                        plain.result.stderr);
                    const reference = readFileSync(plain.output);
                    const literal = render(dir, input, format, slashInk);
                    assert.equal(literal.result.status, 0,
                        literal.result.stderr);
                    assert.deepEqual(readFileSync(literal.output), reference,
                        `literal backslash remains ink in ${format}`);
                    const missing = render(dir, input, format);
                    assert.notEqual(missing.result.status, 0);
                    assert.match(missing.result.stderr,
                        /missing authored glyph shapes.*U\+005C/);
                    assert.deepEqual(readFileSync(missing.output), reference,
                        'missing literal slash must not replace accepted ink');
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


test('ATX heading separator spaces and pasted tabs are syntax not ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/atx-heading-spacing-'));
        try {
            const canonical = '# ó\n## a\n### o\u0301\n#### a'
                + '\n##### ó\n###### a\nó';
            const alternate = '  #   ó ##  \n  ##\ta\n ###     o\u0301   ###'
                + '\n ####    a\n#####    ó #'
                + '\n  ######\ta ###\nó';
            for (const format of ['pdf', 'png']) {
                const standard = render(dir, canonical, format);
                assert.equal(standard.result.status, 0,
                    standard.result.stderr);
                const accepted = readFileSync(standard.output);
                const spaced = render(dir, alternate, format);
                assert.equal(spaced.result.status, 0,
                    spaced.result.stderr);
                assert.deepEqual(readFileSync(spaced.output), accepted,
                    `${format} marker delimiter whitespace cannot shift ink`);
                for (const invalid of ['#   ', '###\t', '######       ']) {
                    const result = render(dir, invalid, format);
                    assert.notEqual(result.result.status, 0, invalid);
                    assert.match(result.result.stderr,
                        /empty Markdown block refused/);
                    assert.deepEqual(readFileSync(result.output), accepted,
                        'empty heading must preserve previously exported ink');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('Setext heading underlines match ATX heading PDF and PNG exactly',
    () => {
        const dir = mkdtempSync(join(root, '.temp/setext-heading-'));
        try {
            const baseline = '# ó\n## a\nó';
            const setext = 'ó\n===\na\n---\nó';
            const indented = 'ó\n  ==   \na\n ---   \nó';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, baseline, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                for (const source of [setext, indented]) {
                    const result = render(dir, source, format);
                    assert.equal(result.result.status, 0,
                        result.result.stderr);
                    assert.deepEqual(readFileSync(result.output), expected,
                        `${format} Setext syntax is not authored pen ink`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('multiline Setext title wraps as one heading; invalid marks refuse',
    () => {
        const dir = mkdtempSync(join(root, '.temp/setext-atomic-'));
        try {
            const canonical = '# ó a\na';
            const setext = 'ó\na\n=\na';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, canonical, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const accepted = readFileSync(reference.output);
                const promoted = render(dir, setext, format);
                assert.equal(promoted.result.status, 0,
                    promoted.result.stderr);
                assert.deepEqual(readFileSync(promoted.output), accepted,
                    'soft-line title becomes one highlighted heading');
                for (const source of [
                    '===', 'ó\n== =', 'ó\n    ===',
                    'ó\n~~~\n===\n~~~', 'ó\n\\===',
                ]) {
                    const refused = render(dir, source, format);
                    assert.notEqual(refused.result.status, 0, source);
                    assert.match(refused.result.stderr,
                        /missing authored glyph|no authored ink|visible/i,
                        `unsupported Setext ${JSON.stringify(source)}`);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        'refusal never replaces the accepted sheet');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('Setext heading keeps its first body row inside a fixed page region',
    () => {
        const dir = mkdtempSync(join(root, '.temp/setext-page-keep-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '50000', '30000'];
            const canonical = 'a  \n# ó\na';
            const setext = 'a  \nó\n===\na';
            for (const format of ['pdf', 'png']) {
                const page = format === 'pdf' ? 1 : 2;
                const standard = render(dir, canonical, format, region,
                    '--flow-markdown', page);
                assert.equal(standard.result.status, 0,
                    standard.result.stderr);
                const expected = readFileSync(standard.output);
                const underlined = render(dir, setext, format, region,
                    '--flow-markdown', page);
                assert.equal(underlined.result.status, 0,
                    underlined.result.stderr);
                assert.deepEqual(readFileSync(underlined.output), expected,
                    'H1 and first body row paginate together');
                if (format === 'pdf') {
                    const pdf = expected.toString('latin1');
                    assert.equal((pdf.match(/\/Type \/Page\b/g) || [])
                        .length, 2, 'heading and body must advance to page 2');
                } else {
                    assert.deepEqual(expected.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('ATX H4 through H6 retain tiered marker strokes in PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-h4-h6-'));
        try {
            const canonical = '#### ó\n##### a\n###### o\u0301\na';
            const indented = '   #### ó ##  \n ##### a #\n'
                + '  ###### o\u0301 ###\na';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, canonical, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                const decorated = render(dir, indented, format);
                assert.equal(decorated.result.status, 0,
                    decorated.result.stderr);
                assert.deepEqual(readFileSync(decorated.output), expected,
                    `${format} H4/H5/H6 closing syntax is not ink`);
                if (format === 'pdf') {
                    const content = expected.toString('latin1');
                    assert.ok(content.startsWith('%PDF-'));
                    assert.equal(content.split('0.973 0.871 0.569 RG')
                        .length - 1, 4,
                    'H4 two marker strokes, H5 and H6 one each');
                    assert.equal(content.split('0.071 0.204 0.337 RG')
                        .length - 1, 4,
                    'only four caller-authored text glyphs remain');
                    assert.deepEqual(coloredHeadingWidths(expected), [
                        '2.154', '2.154', '1.757', '1.417',
                    ], 'H4/H5/H6 marker widths are physically distinct');
                } else {
                    assert.deepEqual(expected.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('deeper headings stay styled and reject unapproved shapes atomically',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-h4-styles-'));
        try {
            const source = '#### ó\n##### a\n###### o\u0301';
            for (const format of ['pdf', 'png']) {
                const marker = render(dir, source, format);
                assert.equal(marker.result.status, 0, marker.result.stderr);
                const markerBytes = readFileSync(marker.output);
                for (const style of ['underline', 'double-underline',
                    'none']) {
                    const options = ['--heading-style', style];
                    if (style !== 'none') {
                        options.push('--heading-color', '#F8DE91');
                    }
                    const result = render(dir, source, format, options);
                    assert.equal(result.result.status, 0,
                        result.result.stderr);
                    const ink = readFileSync(result.output);
                    assert.notDeepEqual(ink, markerBytes,
                        `${format} ${style} not marker`);
                    if (format === 'pdf') {
                        const descending = ['0.680', '0.510', '0.340'];
                        const widths = style === 'none' ? []
                            : style === 'underline' ? descending
                                : descending.flatMap((value) => [value, value]);
                        assert.deepEqual(coloredHeadingWidths(ink), widths,
                            `${style} has precise per-level underline widths`);
                    }
                }
            }
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('keep accepted native PDF');
            for (const source of [
                '####### a', '    #### a', '#### ', '##### ###',
                '###### ',
            ]) {
                writeFileSync(output, sentinel);
                const result = render(dir, source, 'pdf');
                assert.notEqual(result.result.status, 0, source);
                assert.match(result.result.stderr,
                    /missing authored glyph shapes|empty Markdown block/);
                assert.deepEqual(readFileSync(output), sentinel,
                    'unsupported or empty heading cannot replace PDF');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('H4 H5 H6 keep first authored row on the next physical page',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-h4-keep-'));
        try {
            const prefix = 'a  \n'.repeat(27);
            for (const [hashes, markerLines] of [
                ['####', 2], ['#####', 1], ['######', 1],
            ]) {
                const source = `${prefix}${hashes} ó\na`;
                const fenced = `${prefix}${hashes} ó\n~~~\na\n~~~`;
                for (const format of ['pdf', 'png']) {
                    const page = format === 'pdf' ? 1 : 2;
                    const direct = render(dir, source, format, [],
                        '--flow-markdown', page);
                    assert.equal(direct.result.status, 0,
                        direct.result.stderr);
                    const expected = readFileSync(direct.output);
                    const codeRow = render(dir, fenced, format, [],
                        '--flow-markdown', page);
                    assert.equal(codeRow.result.status, 0,
                        codeRow.result.stderr);
                    assert.deepEqual(readFileSync(codeRow.output), expected,
                        `${format} ${hashes} and next real row stay together`);
                    if (format === 'pdf') {
                        const pdf = expected.toString('latin1');
                        assert.equal((pdf.match(/\/Type \/Page\b/g) || [])
                            .length, 2, 'the heading requires page two');
                        const streams = [...pdf.matchAll(
                            /stream\n([\s\S]*?)\nendstream/g,
                        )].map((match) => match[1]);
                        assert.equal(streams.length, 2);
                        assert.ok(!streams[0].includes('0.973 0.871'));
                        assert.equal(streams[1].split('0.973 0.871')
                            .length - 1, markerLines);
                    } else {
                        assert.deepEqual(expected.subarray(0, 8),
                            Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                    }
                }
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

const orderedLiteralInk = ['1', '2', '.', ')'].flatMap((glyph) => [
    '--glyph-stroke', glyph, '700', '800', '2350', '3100',
    '250', '#112233',
]);

test('escaped ordered prefixes stay ordinary native handwriting in PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-number-escape-'));
        try {
            const plain = '1. a\n2) ó';
            const escaped = '1\\. a  \n2\\) ó';
            const indented = '   1\\. a  \n  2\\) ó';
            for (const format of ['pdf', 'png']) {
                const ordinary = render(dir, plain, format,
                    orderedLiteralInk, '--flow-text');
                assert.equal(ordinary.result.status, 0,
                    ordinary.result.stderr);
                const expected = readFileSync(ordinary.output);
                for (const markdown of [escaped, indented]) {
                    const literal = render(dir, markdown, format,
                        orderedLiteralInk);
                    assert.equal(literal.result.status, 0,
                        literal.result.stderr);
                    assert.deepEqual(readFileSync(literal.output), expected,
                        `${format} escaped labels remain ordinary pen ink`);
                }
                const numbered = render(dir, plain, format,
                    orderedLiteralInk);
                assert.equal(numbered.result.status, 0,
                    numbered.result.stderr);
                assert.notDeepEqual(readFileSync(numbered.output), expected,
                    'unescaped Markdown numbering must still be a list');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('escaped 9-digit labels remain authored and 10-digit escapes refuse',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-number-limit-'));
        try {
            const numberInk = [...'0123456789.'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '700', '800', '2350', '3100',
                '250', '#112233',
            ]);
            for (const format of ['pdf', 'png']) {
                const plain = render(dir, '123456789. a', format,
                    numberInk, '--flow-text');
                assert.equal(plain.result.status, 0, plain.result.stderr);
                const expected = readFileSync(plain.output);
                const escaped = render(dir, '123456789\\. a', format,
                    numberInk);
                assert.equal(escaped.result.status, 0,
                    escaped.result.stderr);
                assert.deepEqual(readFileSync(escaped.output), expected,
                    'maximum admitted list label escape is ordinary ink');
            }
            const output = join(dir, 'flow-pdf.output');
            const sentinel = Buffer.from('retain previously exported ink');
            writeFileSync(output, sentinel);
            const rejected = render(dir, '1234567890\\. a', 'pdf',
                numberInk);
            assert.notEqual(rejected.result.status, 0,
                'a 10-digit prefix is not admitted as an ordered escape');
            assert.match(rejected.result.stderr,
                /missing authored glyph shapes/);
            assert.match(rejected.result.stderr, /U\+005C/);
            assert.deepEqual(readFileSync(output), sentinel,
                'an unsupported escape cannot replace the selected PDF');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('malformed and unauthored number escapes preserve selected PDFs',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-number-escape-'));
        try {
            const sentinel = Buffer.from('original approved PDF');
            const output = join(dir, 'flow-pdf.output');
            for (const [source, ink, missing] of [
                ['1\\. a', [], /U\+0031/],
                ['2\\) ó', [], /U\+0032/],
                ['    1\\. a', orderedLiteralInk, /U\+005C/],
                ['1\\; a', orderedLiteralInk, /U\+005C/],
            ]) {
                writeFileSync(output, sentinel);
                const failed = render(dir, source, 'pdf', ink);
                assert.notEqual(failed.result.status, 0, source);
                assert.match(failed.result.stderr,
                    /missing authored glyph shapes/);
                assert.match(failed.result.stderr, missing);
                assert.deepEqual(readFileSync(output), sentinel,
                    'invalid escaped number never changes accepted PDF');
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
            const canonical = 'a\n\n---\na\n***\na\n___\na';
            const indented = 'a\n\n ---\na\n  ***\na\n   ___\na';
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
            const canonical = 'a\n\n---\na\n***\na\n___\na';
            const spaced = 'a\n\n- - -  \na\n *  * *\na\n  _ _   _\na';
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

test('bullet and task marker padding up to four spaces is not ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-bullet-padding-'));
        try {
            const canonical = '- ó\n* a\n+ o\u0301\n- [x] a'
                + '\n* [ ] ó\n+ [X] o\u0301\na';
            const separated = '-   ó\n*    a\n+ \to\u0301'
                + '\n-   [x] a\n*    [ ] ó\n+  [X] o\u0301\na';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, canonical, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                const padded = render(dir, separated, format);
                assert.equal(padded.result.status, 0,
                    padded.result.stderr);
                assert.deepEqual(readFileSync(padded.output), expected,
                    `${format} 1..4 list delimiter spaces are syntax only`);
                const deep = render(dir, '-      a', format);
                assert.equal(deep.result.status, 0, deep.result.stderr);
                const indentedInk = readFileSync(deep.output);
                const shallow = render(dir, '- a', format);
                assert.equal(shallow.result.status, 0,
                    shallow.result.stderr);
                assert.notDeepEqual(indentedInk,
                    readFileSync(shallow.output),
                    'spaces beyond fourth separator remain authored ink gap');
                for (const empty of ['-   [x]   ', '*    [ ]', '+    ']) {
                    const result = render(dir, empty, format);
                    assert.notEqual(result.result.status, 0, empty);
                    assert.match(result.result.stderr,
                        /empty Markdown block refused/);
                    assert.deepEqual(readFileSync(result.output),
                        readFileSync(shallow.output),
                        'empty bullet/task refuses without output replacement');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('extra checkbox separator spaces do not shift physical handwriting',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-checkbox-gap-'));
        try {
            const source = '- [x] a\n* [ ] ó\n+ [X] a';
            const separated = '- [x]   a\n* [ ]    ó\n+ [X]  a';
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, source, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                const alternate = render(dir, separated, format);
                assert.equal(alternate.result.status, 0,
                    alternate.result.stderr);
                assert.deepEqual(readFileSync(alternate.output), expected,
                    `task separator spaces are not ${format} pen advances`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('empty task items refuse even with literal bracket glyphs available',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-checkbox-gap-'));
        try {
            const output = join(dir, 'flow-pdf.output');
            const saved = Buffer.from('existing validated PDF');
            for (const source of ['- [ ]', '* [x]', '+ [X]', '- [ ]   ']) {
                writeFileSync(output, saved);
                const candidate = render(dir, source, 'pdf', [
                    ...checkboxLiteralInk,
                    '--glyph-stroke', 'X', '650', '800', '2400', '3100',
                    '250', '#112233',
                ]);
                assert.notEqual(candidate.result.status, 0, source);
                assert.match(candidate.result.stderr,
                    /empty Markdown block refused/);
                assert.deepEqual(readFileSync(output), saved);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('bare Markdown headings and list labels refuse before file replacement',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-empty-markers-'));
        try {
            const punctuationInk = ['#', '-', '*', '+', '1', '2', '7',
                '.', ')'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '550', '850', '2000', '2750',
                '250', '#2277AA',
            ]);
            for (const format of ['pdf', 'png']) {
                const accepted = render(dir, 'a', format, punctuationInk);
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const original = readFileSync(accepted.output);
                for (const source of [
                    '#', '##', '######', ' -', '   *', '+', '1.', '2)',
                    '  7.', 'a\n\n+', 'a\n\n#',
                ]) {
                    const invalid = render(dir, source, format,
                        punctuationInk);
                    assert.notEqual(invalid.result.status, 0,
                        JSON.stringify(source));
                    assert.match(invalid.result.stderr,
                        /empty Markdown block refused/,
                        `bare marker ${JSON.stringify(source)}`);
                    assert.deepEqual(readFileSync(invalid.output), original,
                        'rejected structure must not replace native output');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('escaped bare Markdown marks remain literal authored PDF and PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-bare-escape-'));
        try {
            const punctuationInk = ['#', '-', '*', '+', '1', '2',
                '.', ')'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '550', '850', '2000', '2750',
                '250', '#2277AA',
            ]);
            const plain = ['#', '-', '*', '+', '1.', '2)'].join('\n');
            const escaped = ['\\#', '\\-', '\\*', '\\+',
                '1\\.', '2\\)'].join('  \n');
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, plain, format,
                    punctuationInk, '--flow-text');
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                const markdown = render(dir, escaped, format,
                    punctuationInk);
                assert.equal(markdown.result.status, 0,
                    markdown.result.stderr);
                assert.deepEqual(readFileSync(markdown.output), expected,
                    'literal escape keeps exact handwritten punctuation');
                const literalIndent = render(dir, '    -', format,
                    punctuationInk);
                assert.equal(literalIndent.result.status, 0,
                    literalIndent.result.stderr);
                const plainIndent = render(dir, '    -', format,
                    punctuationInk, '--flow-text');
                assert.equal(plainIndent.result.status, 0,
                    plainIndent.result.stderr);
                assert.deepEqual(readFileSync(literalIndent.output),
                    readFileSync(plainIndent.output),
                    'four-space indentation stays literal authored text');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('single Setext dash and fenced bare punctuation retain their context',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-context-marker-'));
        try {
            const authoredHash = [
                '--glyph-stroke', '#', '550', '850', '2000', '2750',
                '250', '#2277AA',
            ];
            for (const format of ['pdf', 'png']) {
                const atx = render(dir, '## a', format);
                assert.equal(atx.result.status, 0, atx.result.stderr);
                const expectedHeading = readFileSync(atx.output);
                const setext = render(dir, 'a\n-', format);
                assert.equal(setext.result.status, 0,
                    setext.result.stderr);
                assert.deepEqual(readFileSync(setext.output), expectedHeading,
                    'one hyphen after text stays a Setext heading underline');
                const plain = render(dir, '#', format, authoredHash,
                    '--flow-text');
                assert.equal(plain.result.status, 0,
                    plain.result.stderr);
                const expectedInk = readFileSync(plain.output);
                const fenced = render(dir, '```\n#\n```', format,
                    authoredHash);
                assert.equal(fenced.result.status, 0,
                    fenced.result.stderr);
                assert.deepEqual(readFileSync(fenced.output), expectedInk,
                    'fenced marker preserves exact authored ink');
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

test('indented quote lines retain physical PDF and page-local PNG bars',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-quote-indent-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '24000', '23000'];
            const canonical = 'a  \na\n> a ó a a';
            const variants = [
                'a  \na\n> a\n  ó a a',
                'a  \na\n> a\n   ó a a',
                'a  \na\n> a\n  ó a\n   a',
            ];
            const pageOne = render(dir, canonical, 'png', region);
            assert.equal(pageOne.result.status, 0, pageOne.result.stderr);
            const expectedOne = readFileSync(pageOne.output);
            const pageTwo = render(dir, canonical, 'png', region,
                '--flow-markdown', 2);
            assert.equal(pageTwo.result.status, 0, pageTwo.result.stderr);
            const expectedTwo = readFileSync(pageTwo.output);
            assert.notDeepEqual(expectedOne, expectedTwo,
                'quote rule and ink must appear on page two');
            const pdf = render(dir, canonical, 'pdf',
                [...region, '--pages', '2']);
            assert.equal(pdf.result.status, 0, pdf.result.stderr);
            const expectedPdf = readFileSync(pdf.output);
            assert.match(expectedPdf.toString('latin1'), /\/Count 2/);
            for (const continued of variants) {
                const outputPdf = render(dir, continued, 'pdf',
                    [...region, '--pages', '2']);
                assert.equal(outputPdf.result.status, 0,
                    outputPdf.result.stderr);
                assert.deepEqual(readFileSync(outputPdf.output), expectedPdf,
                    'indented quote continuation preserves native vector ink');
                for (const [page, expected] of [
                    [1, expectedOne], [2, expectedTwo],
                ]) {
                    const outputPng = render(dir, continued, 'png', region,
                        '--flow-markdown', page);
                    assert.equal(outputPng.result.status, 0,
                        outputPng.result.stderr);
                    assert.deepEqual(readFileSync(outputPng.output), expected,
                        `page ${page} keeps exact transparent quote ink`);
                }
            }
            const refused = render(dir, variants[0], 'pdf',
                [...region, '--pages', '1']);
            assert.notEqual(refused.result.status, 0);
            assert.match(refused.result.stderr, /page/i);
            assert.deepEqual(readFileSync(refused.output), expectedPdf,
                'invalid page selection cannot replace a good PDF');
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


test('authored Unicode space glyphs render actual PDF and PNG flow ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-unicode-space-'));
        try {
            const extra = ['\u00a0', '\u2009', '\u202f', '1', '.']
                .flatMap((glyph) => ['--glyph-stroke', glyph,
                    '700', '800', '1700', '2900', '250', '#123456']);
            const cases = [
                ['--flow-text', '\u00a0'],
                ['--flow-text', 'a\n\u00a0'],
                ['--flow-markdown', '\u00a0'],
                ['--flow-markdown', 'a\n\u00a0'],
                ['--flow-markdown', '- \u00a0'],
                ['--flow-markdown', '> \u2009'],
                ['--flow-markdown', '1. \u202f'],
                ['--flow-markdown', '# \u00a0'],
                ['--flow-markdown', '\u00a0\n==='],
            ];
            for (const format of ['pdf', 'png']) {
                const baseline = render(dir, 'a', format, extra,
                    '--flow-text');
                assert.equal(baseline.result.status, 0,
                    baseline.result.stderr);
                const noUnicodeInk = readFileSync(baseline.output);
                for (const [flag, source] of cases) {
                    const candidate = render(dir, source, format,
                        extra, flag);
                    assert.equal(candidate.result.status, 0,
                        `${flag} ${JSON.stringify(source)}: ` +
                        candidate.result.stderr);
                    const ink = readFileSync(candidate.output);
                    if (format === 'pdf') {
                        assert.match(ink.toString('latin1'), /^%PDF-1\.4/);
                        assert.match(ink.toString('latin1'), / m\n/,
                            'Unicode glyph supplies native vector strokes');
                    } else {
                        assert.deepEqual(ink.subarray(0, 8),
                            Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
                            'Unicode glyph has transparent PNG output');
                    }
                    assert.notDeepEqual(ink, noUnicodeInk,
                        'Unicode whitespace is not discarded as blank ink');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('unapproved Unicode spaces refuse before replacing real PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-unicode-refuse-'));
        try {
            const sources = [
                ['--flow-text', '\u00a0'],
                ['--flow-text', 'a\n\u00a0'],
                ['--flow-markdown', '- \u00a0'],
                ['--flow-markdown', '> \u2009'],
                ['--flow-markdown', '1. \u202f'],
                ['--flow-markdown', '\u00a0\n==='],
            ];
            const numerals = ['1', '.'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '700', '800', '1700',
                '2900', '250', '#123456',
            ]);
            for (const format of ['pdf', 'png']) {
                const accepted = render(dir, 'a', format, numerals,
                    '--flow-text');
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const original = readFileSync(accepted.output);
                for (const [flag, source] of sources) {
                    const refused = render(dir, source, format,
                        numerals, flag);
                    assert.notEqual(refused.result.status, 0,
                        JSON.stringify(source));
                    assert.match(refused.result.stderr,
                        /missing authored glyph/i,
                        'exact Unicode ink requires authored strokes');
                    assert.deepEqual(readFileSync(refused.output), original,
                        'a rejected Unicode glyph cannot replace valid output');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('two/three-space Markdown list continuations keep one native marker',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-list-continue-'));
        try {
            const cases = [
                ['- a\n  ó\n   o\u0301', '- a ó o\u0301'],
                ['* a\n   ó', '* a ó'],
                ['+ a\n  ó', '+ a ó'],
                ['- [x] a\n  ó', '- [x] a ó'],
                ['- [ ] a\n   o\u0301', '- [ ] a o\u0301'],
                ['1. a\n   ó\n  o\u0301', '1. a ó o\u0301'],
                ['2) a\n  ó', '2) a ó'],
            ];
            for (const format of ['pdf', 'png']) {
                for (const [source, singleLine] of cases) {
                    const one = render(dir, singleLine, format, numberedInk);
                    assert.equal(one.result.status, 0, one.result.stderr);
                    const expected = readFileSync(one.output);
                    const continued = render(dir, source, format, numberedInk);
                    assert.equal(continued.result.status, 0,
                        JSON.stringify(source) + ': '
                            + continued.result.stderr);
                    assert.deepEqual(readFileSync(continued.output), expected,
                        `one ${format} list marker and same authored glyphs`);
                }
                const separate = render(dir, '- a\nó', format);
                assert.equal(separate.result.status, 0,
                    separate.result.stderr);
                const distinct = readFileSync(separate.output);
                const hanging = render(dir, '- a\n  ó', format);
                assert.equal(hanging.result.status, 0,
                    hanging.result.stderr);
                const hangingBytes = readFileSync(hanging.output);
                assert.notDeepEqual(hangingBytes, distinct,
                    'unindented row remains a separate list/body block');
                const literal = render(dir, '- a\n    ó', format);
                assert.equal(literal.result.status, 0,
                    literal.result.stderr);
                assert.notDeepEqual(readFileSync(literal.output),
                    hangingBytes,
                    'four-space indentation remains physically authored');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('continued list wraps across sheets and missing glyph refuses atomically',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-list-pages-'));
        try {
            const region = [
                '--flow-region', '30000', '40000', '50000', '30000',
            ];
            const longWords = Array(34).fill('a');
            const continued = '- [x] a\n  ' + longWords.slice(1).join('\n  ');
            const single = '- [x] ' + longWords.join(' ');
            for (const format of ['pdf', 'png']) {
                const page = format === 'pdf' ? 1 : 2;
                const expected = render(dir, single, format, region,
                    '--flow-markdown', page);
                assert.equal(expected.result.status, 0,
                    expected.result.stderr);
                const bytes = readFileSync(expected.output);
                const result = render(dir, continued, format, region,
                    '--flow-markdown', page);
                assert.equal(result.result.status, 0,
                    result.result.stderr);
                assert.deepEqual(readFileSync(result.output), bytes,
                    `continuation keeps identical physical ${format} pages`);
                if (format === 'pdf') {
                    assert.match(bytes.toString('latin1'), /\/Count [2-9]\b/);
                    assert.equal(bytes.toString('latin1')
                        .split('0.118 0.188 0.290 RG').length - 1,
                        6, 'one checked-box ornament despite many rows');
                }
                const missing = render(dir, '- a\n  x', format);
                assert.notEqual(missing.result.status, 0);
                assert.match(missing.result.stderr,
                    /missing authored glyph shapes/);
                assert.deepEqual(readFileSync(missing.output), bytes,
                    'refusal preserves existing complete native export');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('continued list stops at heading quote fence and paragraph boundaries',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-list-boundary-'));
        try {
            const cases = [
                ['- a\n  ó\n# a', '- a ó\n# a'],
                ['1. a\n   ó\n- o\u0301', '1. a ó\n- o\u0301'],
                ['- [x] a\n  ó\n> o\u0301', '- [x] a ó\n> o\u0301'],
                ['- a\n  ó\n\na', '- a ó\n\na'],
                ['- a\n  ó\n~~~\na\n~~~', '- a ó\n~~~\na\n~~~'],
                ['- a\n  ó  \n   o\u0301', '- a ó\no\u0301'],
            ];
            for (const format of ['pdf', 'png']) {
                for (const [continued, canonical] of cases) {
                    const one = render(dir, canonical, format, numberedInk);
                    assert.equal(one.result.status, 0, one.result.stderr);
                    const expected = readFileSync(one.output);
                    const actual = render(dir, continued, format, numberedInk);
                    assert.equal(actual.result.status, 0, actual.result.stderr);
                    assert.deepEqual(readFileSync(actual.output), expected,
                        `list boundary remains canonical in ${format}`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('hard line separators end a list before an indented ordinary row',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-list-hard-'));
        try {
            for (const format of ['pdf', 'png']) {
                for (const marker of ['- a', '- [x] a', '1. a']) {
                    const reference = render(dir, `${marker}\nó`, format,
                        numberedInk);
                    assert.equal(reference.result.status, 0,
                        reference.result.stderr);
                    const separatedBytes = readFileSync(reference.output);
                    const soft = render(dir, `${marker}\n  ó`, format,
                        numberedInk);
                    assert.equal(soft.result.status, 0,
                        soft.result.stderr);
                    assert.notDeepEqual(readFileSync(soft.output),
                        separatedBytes, 'soft continuation stays a list item');
                    const slash = String.fromCharCode(92);
                    for (const hard of [
                        `${marker}  \n  ó`, `${marker}\u2028  ó`,
                        `${marker}${slash}\n  ó`,
                    ]) {
                        const actual = render(dir, hard, format, numberedInk);
                        assert.equal(actual.result.status, 0,
                            actual.result.stderr);
                        assert.deepEqual(readFileSync(actual.output),
                            separatedBytes,
                            'authored hard break must end the list item');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('literal paired list backslashes still require authored glyph ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-list-slashes-'));
        try {
            const slash = String.fromCharCode(92);
            for (const format of ['pdf', 'png']) {
                const accepted = render(dir, '- a\nó', format);
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const before = readFileSync(accepted.output);
                for (const marker of ['- a', '- [x] a', '1. a']) {
                    const invalid = render(dir,
                        `${marker}${slash}${slash}\n  ó`, format,
                        numberedInk);
                    assert.notEqual(invalid.result.status, 0);
                    assert.match(invalid.result.stderr,
                        /missing authored glyph shapes.*U\+005C/);
                    assert.deepEqual(readFileSync(invalid.output), before,
                        'literal paired slash refusal preserves export');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('four-space Markdown row before equals measures literal glyph ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/setext-literal-pitch-'));
        try {
            const stats = join(dir, 'tall-equal.json');
            const edited = spawnSync('cargo', [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', '--set-glyph', '=', 'height',
                '25000', '25000', '25000', '25000', '0',
            ], { cwd: root, timeout: 120000 });
            assert.equal(edited.status, 0,
                edited.stderr?.toString() || edited.error?.message);
            writeFileSync(stats, edited.stdout);
            const ink = [
                '--handwriting-profile', stats, '--glyph-stroke', '=',
                '600', '500', '1800', '3000', '250', '#123456',
            ];
            for (const [inline, split] of [
                ['    a\n= a', '    a\n=\na'],
                ['- a\n  a\n= a', '- a\n  a\n=\na'],
                ['- a\n   a\n= a', '- a\n   a\n=\na'],
            ]) {
                for (const format of ['pdf', 'png']) {
                    const expected = render(dir, inline, format, ink);
                    assert.equal(expected.result.status, 0,
                        expected.result.stderr);
                    const reference = readFileSync(expected.output);
                    const actual = render(dir, split, format, ink);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    assert.deepEqual(readFileSync(actual.output), reference,
                        `literal Setext-like mark keeps ${format} row pitch`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('Markdown backslash hard break survives Unicode line separators',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-slash-linesep-'));
        try {
            const slash = String.fromCharCode(92);
            const authoredSlash = [
                '--glyph-stroke', slash, '650', '700', '1800', '3200',
                '250', '#123456',
            ];
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, `a${slash}\na`, format);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const hardBreak = readFileSync(reference.output);
                for (const alternate of [
                    `a${slash}\u2028a`, `a${slash}  \na`,
                ]) {
                    const replay = render(dir, alternate, format);
                    assert.equal(replay.result.status, 0,
                        replay.result.stderr);
                    assert.deepEqual(readFileSync(replay.output), hardBreak,
                        'single slash is syntactic before either hard break');
                }
                const paired = `a${slash}${slash}\u2028a`;
                const literal = render(dir, paired, format, authoredSlash);
                assert.equal(literal.result.status, 0,
                    literal.result.stderr);
                const literalBytes = readFileSync(literal.output);
                const explicitSpaces = render(dir,
                    `a${slash}${slash}  \na`, format, authoredSlash);
                assert.equal(explicitSpaces.result.status, 0,
                    explicitSpaces.result.stderr);
                assert.deepEqual(readFileSync(explicitSpaces.output),
                    literalBytes, 'paired slashes remain authored ink');
                const refused = render(dir, paired, format);
                assert.notEqual(refused.result.status, 0);
                assert.match(refused.result.stderr,
                    /missing authored glyph shapes.*U\+005C/);
                assert.deepEqual(readFileSync(refused.output),
                    literalBytes, 'missing slash preserves selected export');
                const plain = render(dir, `a${slash}\na`, format,
                    authoredSlash, '--flow-text');
                assert.equal(plain.result.status, 0, plain.result.stderr);
                const plainBytes = readFileSync(plain.output);
                const plainSeparator = render(dir, `a${slash}\u2028a`,
                    format, authoredSlash, '--flow-text');
                assert.equal(plainSeparator.result.status, 0,
                    plainSeparator.result.stderr);
                assert.deepEqual(readFileSync(plainSeparator.output),
                    plainBytes, 'plain text preserves its authored slash');
                assert.notDeepEqual(plainBytes, hardBreak);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });


test('fenced blank row keeps adjacent titles and body on page two', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-fenced-title-chain-'));
    try {
        const region = ['--flow-region', '30000', '40000',
            '100000', '41000'];
        const prelude = 'a';
        for (const group of [
            '# a\n~~~txt\n\n~~~\n## ó\na',
            '# a\n~~~txt\n\n~~~\nó\n===\na',
        ]) {
            const combined = `a  \n${group}`;
            const first = render(dir, combined, 'png', region);
            assert.equal(first.result.status, 0, first.result.stderr);
            const standalonePrelude = render(dir, prelude, 'png', region);
            assert.equal(standalonePrelude.result.status, 0,
                standalonePrelude.result.stderr);
            assert.deepEqual(readFileSync(first.output),
                readFileSync(standalonePrelude.output),
                'the prelude must be the sole first-page ink');

            const second = render(dir, combined, 'png', region,
                '--flow-markdown', 2);
            assert.equal(second.result.status, 0, second.result.stderr);
            const standaloneGroup = render(dir, group, 'png', region);
            assert.equal(standaloneGroup.result.status, 0,
                standaloneGroup.result.stderr);
            assert.deepEqual(readFileSync(second.output),
                readFileSync(standaloneGroup.output),
                'both titles, code gap and body must stay together');

            const pdf = render(dir, combined, 'pdf',
                [...region, '--pages', '2']);
            assert.equal(pdf.result.status, 0, pdf.result.stderr);
            const bytes = readFileSync(pdf.output);
            assert.match(bytes.toString('latin1'), /\/Count 2/,
                'native PDF must contain two physical sheets');
            const refused = render(dir, combined, 'pdf',
                [...region, '--pages', '1']);
            assert.notEqual(refused.result.status, 0);
            assert.deepEqual(readFileSync(refused.output), bytes,
                'page-budget refusal preserves the accepted PDF');
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('terminal heading chains retain interior fenced blank rows in PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-terminal-titles-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '100000', '36000'];
            const prelude = 'a';
            for (const group of [
                '# a\n~~~txt\n\n~~~\n## ó',
                '# a\n~~~txt\n\n~~~\nó\n===',
                '# a\n~~~txt\n\n~~~\n## ó\n~~~txt\n\n~~~',
            ]) {
                const combined = `${prelude}\n${group}`;
                const first = render(dir, combined, 'png', region);
                assert.equal(first.result.status, 0, first.result.stderr);
                const alonePrelude = render(dir, prelude, 'png', region);
                assert.equal(alonePrelude.result.status, 0,
                    alonePrelude.result.stderr);
                assert.deepEqual(readFileSync(first.output),
                    readFileSync(alonePrelude.output),
                    'page one must contain only the prelude');

                const second = render(dir, combined, 'png', region,
                    '--flow-markdown', 2);
                assert.equal(second.result.status, 0, second.result.stderr);
                const aloneGroup = render(dir, group, 'png', region);
                assert.equal(aloneGroup.result.status, 0,
                    aloneGroup.result.stderr);
                assert.deepEqual(readFileSync(second.output),
                    readFileSync(aloneGroup.output),
                    'page two must retain both titles and interior blank');

                const accepted = render(dir, combined, 'pdf',
                    [...region, '--pages', '2']);
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const pdf = readFileSync(accepted.output);
                assert.match(pdf.toString('latin1'), /\/Count 2/);
                const refused = render(dir, combined, 'pdf',
                    [...region, '--pages', '1']);
                assert.notEqual(refused.result.status, 0);
                assert.match(refused.result.stderr, /page/i);
                assert.deepEqual(readFileSync(refused.output), pdf,
                    'page-budget refusal preserves accepted PDF bytes');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('quoted continuation keeps literal Setext-like ink in native PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-quote-equal-pitch-'));
        try {
            const statistics = join(dir, 'tall-equal.json');
            const configured = spawnSync('cargo', [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', '--set-glyph', '=', 'height',
                '25000', '25000', '25000', '25000', '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(statistics, configured.stdout);
            const ink = [
                '--handwriting-profile', statistics,
                '--glyph-stroke', '=', '600', '500', '1800', '3000',
                '250', '#123456',
            ];
            for (const format of ['pdf', 'png']) {
                const canonical = render(dir, '> a a\n= a', format, ink);
                assert.equal(canonical.result.status, 0,
                    canonical.result.stderr);
                const expected = readFileSync(canonical.output);
                for (const source of [
                    '> a\n  a\n=\na', '> a\n   a\n=\na',
                ]) {
                    const joined = render(dir, source, format, ink);
                    assert.equal(joined.result.status, 0,
                        joined.result.stderr);
                    assert.deepEqual(readFileSync(joined.output), expected,
                        'literal equal glyph must set physical row pitch');
                }
                const missing = render(dir, '> a\n  a\n=\na', format,
                    ['--handwriting-profile', statistics]);
                assert.notEqual(missing.result.status, 0);
                assert.match(missing.result.stderr,
                    /missing authored glyph shapes.*U\+003D/);
                assert.deepEqual(readFileSync(missing.output), expected,
                    'refused literal equals must preserve existing export');
                const heading = render(dir, 'a\n=\na', format,
                    ['--handwriting-profile', statistics]);
                assert.equal(heading.result.status, 0,
                    heading.result.stderr);
                assert.notDeepEqual(readFileSync(heading.output), expected,
                    'actual Setext syntax still has no equals-sign ink');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('escaped Setext equals exports literal native ink, not a title', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-escaped-equals-'));
    try {
        const equals = [
            '--glyph-stroke', '=', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        for (const format of ['pdf', 'png']) {
            const expected = render(dir, 'a === a', format, equals);
            assert.equal(expected.result.status, 0,
                expected.result.stderr);
            const bytes = readFileSync(expected.output);
            for (const source of ['a\n\\===\na', 'a\n  \\===\na']) {
                const actual = render(dir, source, format, equals);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                assert.deepEqual(readFileSync(actual.output), bytes,
                    'escaped equals line must be exact authored paragraph');
            }
            const missing = render(dir, 'a\n\\===\na', format);
            assert.notEqual(missing.result.status, 0);
            assert.match(missing.result.stderr,
                /missing authored glyph shapes.*U\+003D/);
            assert.deepEqual(readFileSync(missing.output), bytes,
                'refusal cannot replace accepted page ink');
            const title = render(dir, 'a\n===\na', format);
            assert.equal(title.result.status, 0, title.result.stderr);
            assert.notDeepEqual(readFileSync(title.output), bytes,
                'an actual Setext underline remains structural');
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('Unicode breaks before Setext marks retain two-page native ink', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-unicode-setext-'));
    try {
        const statistics = join(dir, 'tall-equal.json');
        const configured = spawnSync('cargo', [
            'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
            'handwriting', '--set-glyph', '=', 'height',
            '25000', '25000', '25000', '25000', '0',
        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
        assert.equal(configured.status, 0, configured.stderr);
        writeFileSync(statistics, configured.stdout);
        const region = ['--flow-region', '30000', '40000',
            '100000', '35000'];
        const ink = [
            ...region, '--handwriting-profile', statistics,
            '--glyph-stroke', '=', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        const source = 'a\u2028===\na';
        const paragraph = 'a\u2029===\na';
        const canonical = 'a  \n===\na';
        const plainParagraph = 'a\n\n===\na';
        for (const format of ['pdf', 'png']) {
            const selected = format === 'pdf'
                ? [...ink, '--pages', '2'] : ink;
            const accepted = render(dir, source, format, selected);
            assert.equal(accepted.result.status, 0,
                accepted.result.stderr);
            const bytes = readFileSync(accepted.output);
            if (format === 'pdf') {
                assert.match(bytes.toString('latin1'), /\/Count 2/);
            } else {
                const first = render(dir, 'a', 'png', ink);
                assert.equal(first.result.status, 0, first.result.stderr);
                assert.deepEqual(bytes, readFileSync(first.output),
                    'page one must contain only the original body row');
                const second = render(dir, source, 'png', ink,
                    '--flow-markdown', 2);
                assert.equal(second.result.status, 0,
                    second.result.stderr);
                const pageTwo = readFileSync(second.output);
                const alone = render(dir, '=== a', 'png', ink);
                assert.equal(alone.result.status, 0, alone.result.stderr);
                assert.deepEqual(pageTwo, readFileSync(alone.output),
                    'page two must write the exact literal equals glyphs');
            }
            for (const equivalent of [canonical, paragraph, plainParagraph]) {
                const actual = render(dir, equivalent, format, selected);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                assert.deepEqual(readFileSync(actual.output), bytes,
                    'Unicode breaks must match explicit source boundaries');
            }
            const missing = render(dir, source, format,
                [...region, '--handwriting-profile', statistics]);
            assert.notEqual(missing.result.status, 0);
            assert.match(missing.result.stderr,
                /missing authored glyph shapes.*U\+003D/);
            assert.deepEqual(readFileSync(missing.output), bytes,
                'missing literal punctuation cannot replace accepted ink');
            if (format === 'pdf') {
                const short = render(dir, source, 'pdf',
                    [...ink, '--pages', '1']);
                assert.notEqual(short.result.status, 0);
                assert.match(short.result.stderr, /page/i);
                assert.deepEqual(readFileSync(short.output), bytes,
                    'one-page selection cannot replace two-page PDF');
            }
            const actualSetext = render(dir, 'a\n===\na', format,
                [...region, '--handwriting-profile', statistics]);
            assert.equal(actualSetext.result.status, 0,
                actualSetext.result.stderr);
            assert.notDeepEqual(readFileSync(actualSetext.output), bytes,
                'only adjacent ordinary lines permit Setext promotion');
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('mixed BOM CRLF tabs and Unicode breaks keep literal native ink', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-mixed-transport-'));
    try {
        const statistics = join(dir, 'tall-equal.json');
        const configured = spawnSync('cargo', [
            'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
            'handwriting', '--set-glyph', '=', 'height',
            '25000', '25000', '25000', '25000', '0',
        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
        assert.equal(configured.status, 0, configured.stderr);
        writeFileSync(statistics, configured.stdout);
        const glyphs = [
            '--glyph-stroke', 'b', '600', '500', '1800', '3000',
            '250', '#123456',
            '--glyph-stroke', 'c', '900', '500', '1800', '3000',
            '250', '#123456',
        ];
        const equals = [
            '--glyph-stroke', '=', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        const ink = ['--handwriting-profile', statistics,
            ...glyphs, ...equals];
        const transported = '\uFEFFa\r\n\tb\u2028===\r\nc';
        const canonical = 'a\n    b  \n===\nc';
        for (const format of ['pdf', 'png']) {
            const clean = render(dir, canonical, format, ink);
            assert.equal(clean.result.status, 0, clean.result.stderr);
            const expected = readFileSync(clean.output);
            const actual = render(dir, transported, format, ink);
            assert.equal(actual.result.status, 0, actual.result.stderr);
            assert.deepEqual(readFileSync(actual.output), expected,
                'combined source transports preserve literal physical ink');
            const missing = render(dir, transported, format,
                ['--handwriting-profile', statistics, ...glyphs]);
            assert.notEqual(missing.result.status, 0);
            assert.match(missing.result.stderr,
                /missing authored glyph shapes.*U\+003D/);
            assert.deepEqual(readFileSync(missing.output), expected,
                'missing literal equals cannot replace accepted ink');
            const invalid = render(dir, `${transported}\r`, format, ink);
            assert.notEqual(invalid.result.status, 0);
            assert.deepEqual(readFileSync(invalid.output), expected,
                'unpaired carriage return cannot replace accepted ink');
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('plain Unicode grapheme wraps preserve authored PDF and PNG ink', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-unicode-wrapped-'));
    try {
        const glyphs = Array.from({ length: 6 }, () =>
            ['a', 'ó', 'o\u0301']).flat();
        const source = glyphs.join('');
        const rows = [glyphs.slice(0, 7), glyphs.slice(7, 14),
            glyphs.slice(14)].map((row) => row.join(''));
        const region = ['--flow-region', '30000', '40000',
            '33000', '25000'];
        const manualRuns = [
            ['1', '32500', '42500', rows[0]],
            ['1', '32500', '51500', rows[1]],
            ['2', '32500', '42500', rows[2]],
        ].flatMap((values) => ['--place-text', ...values]);
        const manual = (format, page) => {
            const output = join(dir, `manual-${format}-${page}.output`);
            const args = [...base, ...manualRuns,
                ...(format === 'pdf' ? ['--pages', '2'] : []),
                '--export-paper', format, '--output', output];
            if (format === 'png') {
                args.push('--png-page', String(page), '--dpi', '72');
            }
            const result = spawnSync('cargo', ['run', '--quiet', '-p',
                'atrament_profile_editor_cli', '--', ...args], {
                cwd: root, encoding: 'utf8', timeout: 120000,
            });
            assert.equal(result.error, undefined, result.error?.message);
            assert.equal(result.status, 0, result.stderr);
            return readFileSync(output);
        };
        for (const format of ['pdf', 'png']) {
            const pages = format === 'png' ? [1, 2] : [1];
            for (const page of pages) {
                const generated = render(dir, source, format,
                    region, '--flow-text', page);
                assert.equal(generated.result.status, 0,
                    generated.result.stderr);
                const actual = readFileSync(generated.output);
                assert.deepEqual(actual, manual(format, page),
                    `physical ${format} page ${page} must preserve all `
                    + 'NFC and NFD source graphemes across row breaks');
                if (format === 'pdf') {
                    assert.match(actual.toString('latin1'), /\/Count 2/);
                }
            }
        }
        const accepted = readFileSync(join(dir, 'flow-pdf.output'));
        const rejected = render(dir, source, 'pdf',
            [...region, '--pages', '1'], '--flow-text');
        assert.notEqual(rejected.result.status, 0);
        assert.match(rejected.result.stderr, /requires 2 physical pages/);
        assert.deepEqual(readFileSync(rejected.output), accepted,
            'a selected one-page export must preserve the existing PDF');
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('joined Unicode fallback keeps Markdown glyph ink on exact pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-emoji-fallback-'));
        try {
            const joined = '👩‍🏫';
            const source = `# a\n- ${joined} ó\n> o\u0301 a`;
            const authored = '# a\n- a ó\n> o\u0301 a';
            const region = ['--flow-region', '30000', '40000',
                '100000', '14000'];
            const fallback = ['--fallback-glyph', joined, 'a'];
            const singleRows = ['# a', '- a ó', '> o\u0301 a'];
            for (let page = 1; page <= 3; page += 1) {
                const projected = render(dir, source, 'png',
                    [...region, ...fallback], '--flow-markdown', page);
                assert.equal(projected.result.status, 0,
                    projected.result.stderr);
                const pageInk = readFileSync(projected.output);
                const independent = render(dir, singleRows[page - 1],
                    'png', region);
                assert.equal(independent.result.status, 0,
                    independent.result.stderr);
                assert.deepEqual(pageInk, readFileSync(independent.output),
                    `page ${page} must use the exact selected pen contours`);
            }
            const accepted = render(dir, source, 'pdf',
                [...region, ...fallback, '--pages', '3']);
            assert.equal(accepted.result.status, 0,
                accepted.result.stderr);
            const pdf = readFileSync(accepted.output);
            assert.match(pdf.toString('latin1'), /\/Count 3/);
            const substituted = render(dir, authored, 'pdf',
                [...region, '--pages', '3']);
            assert.equal(substituted.result.status, 0,
                substituted.result.stderr);
            assert.deepEqual(readFileSync(substituted.output), pdf,
                'joined grapheme fallback must equal authored replacement');

            const file = join(dir, 'original.md');
            const transported = `\uFEFF${source.replaceAll('\n', '\r\n')}`;
            writeFileSync(file, transported, 'utf8');
            const loaded = render(dir, file, 'pdf',
                [...region, ...fallback, '--pages', '3'],
                '--flow-markdown-file');
            assert.equal(loaded.result.status, 0,
                loaded.result.stderr);
            assert.deepEqual(readFileSync(loaded.output), pdf,
                'BOM/CRLF file transport must not change Unicode page ink');
            assert.equal(readFileSync(file, 'utf8'), transported,
                'caller-selected input remains unchanged');

            for (const options of [
                [...region, '--pages', '3'],
                [...region, ...fallback, '--pages', '2'],
            ]) {
                const refused = render(dir, source, 'pdf', options);
                assert.notEqual(refused.result.status, 0);
                if (options.includes('--fallback-glyph')) {
                    assert.match(refused.result.stderr,
                        /requires 3 physical pages/);
                } else {
                    assert.match(refused.result.stderr,
                        /missing authored glyph shapes/);
                    assert.match(refused.result.stderr, /U\+1F469/);
                }
                assert.deepEqual(readFileSync(refused.output), pdf,
                    'refusal must preserve the previously exported PDF');
            }
            const priorPng = readFileSync(join(dir, 'flow-png.output'));
            const empty = render(dir, source, 'png',
                [...region, ...fallback], '--flow-markdown', 4);
            assert.notEqual(empty.result.status, 0);
            assert.match(empty.result.stderr, /selected PNG page is empty/);
            assert.deepEqual(readFileSync(empty.output), priorPng,
                'invalid page selection preserves the accepted PNG');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('statistical advances wrap native Unicode ink on measured pages', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-mixed-advances-'));
    try {
        const statistics = join(dir, 'mixed-width.json');
        const edits = [
            ['a', 'width', '6400'],
            ['a', 'letter_spacing', '-600'],
            ['ó', 'width', '3300'],
            ['ó', 'letter_spacing', '700'],
        ].flatMap(([glyph, dimension, value]) => [
            '--set-glyph', glyph, dimension,
            value, value, value, value, '0',
        ]);
        const configured = spawnSync('cargo', [
            'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
            'handwriting', ...edits,
        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
        assert.equal(configured.status, 0, configured.stderr);
        writeFileSync(statistics, configured.stdout);
        const region = [
            '--flow-region', '30000', '40000', '34000', '25000',
        ];
        const profile = ['--handwriting-profile', statistics];
        const source = 'aó'.repeat(6) + 'a';
        const expectedRows = ['aó'.repeat(3), 'aó'.repeat(3), 'a'];
        const manualRuns = [
            ['1', '32500', '42500', expectedRows[0]],
            ['1', '32500', '51500', expectedRows[1]],
            ['2', '32500', '42500', expectedRows[2]],
        ].flatMap((values) => ['--place-text', ...values]);
        const manual = (format, page) => {
            const output = join(dir, `manual-${format}-${page}.output`);
            const args = [...base, ...profile, ...manualRuns,
                ...(format === 'pdf' ? ['--pages', '2'] : []),
                '--export-paper', format, '--output', output];
            if (format === 'png') {
                args.push('--png-page', String(page), '--dpi', '72');
            }
            const result = spawnSync('cargo', [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', ...args,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(result.error, undefined, result.error?.message);
            assert.equal(result.status, 0, result.stderr);
            return readFileSync(output);
        };
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const laidOut = render(dir, source, format,
                    [...region, ...profile], '--flow-text', page);
                assert.equal(laidOut.result.status, 0,
                    laidOut.result.stderr);
                const ink = readFileSync(laidOut.output);
                assert.deepEqual(ink, manual(format, page),
                    'statistical glyph advances must match individually '
                    + `placed ink on physical ${format} page ${page}`);
                if (format === 'pdf') {
                    assert.match(ink.toString('latin1'), /\/Count 2/);
                }
            }
        }
        const acceptedPdf = readFileSync(join(dir, 'flow-pdf.output'));
        const narrow = render(dir, source, 'pdf',
            [...region, ...profile, '--pages', '1'], '--flow-text');
        assert.notEqual(narrow.result.status, 0);
        assert.match(narrow.result.stderr, /requires 2 physical pages/);
        assert.deepEqual(readFileSync(narrow.output), acceptedPdf,
            'an insufficient page request cannot replace accepted ink');
        const baseline = render(dir, source, 'pdf', region,
            '--flow-text');
        assert.equal(baseline.result.status, 0, baseline.result.stderr);
        const pageCount = readFileSync(baseline.output)
            .toString('latin1').match(/\/Count (\d+)/)?.[1];
        assert.equal(pageCount, '1',
            'nominal advances should use only one page');
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});
