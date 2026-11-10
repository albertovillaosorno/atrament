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

test('native flow includes explicit manual ink on later PDF/PNG pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-manual-page-'));
        try {
            const ink = ['--ink-line', '2', '70000', '40000',
                '71000', '41000', '250', '#123456'];
            const base = ['notebook', '--printed-sheet', 'true',
                '--glyph-stroke', 'a', '500', '500', '1800', '3000',
                '250', '#234567', '--flow-text', 'a',
                '--flow-region', '30000', '40000', '30000', '14000',
                ...ink];
            const png = ['--export-paper', 'png', '--dpi', '72',
                '--png-page', '2'];
            const reference = accepted('notebook', '--printed-sheet',
                'true', ...ink, ...png);
            const later = accepted(...base, ...png);
            assert.deepEqual(later, reference,
                'manual-only later page has identical native RGBA ink');
            assert.deepEqual(later.subarray(0, 8),
                Buffer.from('89504e470d0a1a0a', 'hex'));
            assert.equal(later[25], 6, 'page two stays transparent RGBA');

            const pdf = accepted(...base, '--export-paper', 'pdf');
            assert.match(pdf.toString('latin1'), /\/Count 2\b/,
                'later manual ink extends the automatic PDF page budget');
            assert.deepEqual(pdf, accepted(...base,
                '--export-paper', 'pdf', '--pages', '2'),
                'automatic and explicit two-page PDF must be identical');

            const outputPng = join(dir, 'approved.png');
            const outputPdf = join(dir, 'approved.pdf');
            writeFileSync(outputPng, later);
            writeFileSync(outputPdf, pdf);
            refused([...base, ...png.slice(0, -1), '3',
                '--output', outputPng, '--replace'],
            /selected PNG page is empty/, outputPng);
            refused([...base, '--export-paper', 'pdf', '--pages', '1',
                '--output', outputPdf, '--replace'],
            /ink.*require 2 physical pages/, outputPdf);
            refused([...base, '--ink-line', '3', '1000', '1000',
                '2000', '2000', '250', '#123456', ...png,
                '--output', outputPng, '--replace'],
            /page 3 ink needs printable endpoints/, outputPng);
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

test('syntax-only Setext and fenced markers retain native PDF/PNG row pitch',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-syntax-pitch-'));
        try {
            const statistics = join(dir, 'tall-markers.json');
            const tall = ['25000', '25000', '25000', '25000', '0'];
            accepted('handwriting',
                ...['=', '-', '`', '~'].flatMap((glyph) => [
                    '--set-glyph', glyph, 'height', ...tall,
                ]), '--output', statistics);
            const glyph = ['--glyph-stroke', 'a', '600', '500',
                '1800', '3000', '250', '#123456',
                '--glyph-stroke', 'b', '900', '500',
                '2300', '3000', '250', '#654321'];
            // Fourteen mm leaves room for exactly one ordinary 9-mm row.
            // A 25-mm syntax-only glyph would make that page unusable.
            const base = ['notebook', '--printed-sheet', 'false',
                '--handwriting-profile', statistics, ...glyph,
                '--heading-style', 'none', '--flow-region',
                '30000', '40000', '40000', '14000'];
            const cases = [
                ['primary Setext', '# a\n\nb', 'a\n===\n\nb'],
                ['secondary Setext', '## a\n\nb', 'a\n---\n\nb'],
                ['backtick fence', 'a\n\nb', '```text\na\n```\n\nb'],
                ['tilde fence', 'a\n\nb', '~~~text\na\n~~~\n\nb'],
            ];
            for (const [name, ordinary, syntactic] of cases) {
                let firstPng = null;
                for (const format of ['pdf', 'png']) {
                    const pages = format === 'pdf' ? [1] : [1, 2];
                    for (const page of pages) {
                        const options = [...base, '--export-paper', format,
                            ...(format === 'pdf'
                                ? ['--pages', '2']
                                : ['--dpi', '72', '--png-page', `${page}`])];
                        const output = join(dir,
                            `${name.replaceAll(' ', '-')}-${format}-${page}`);
                        accepted(...options, '--flow-markdown', ordinary,
                            '--output', output);
                        const original = readFileSync(output);
                        assert.ok(original.length > 100,
                            `${name} must render a real ${format}`);
                        if (format === 'pdf') {
                            assert.match(original.toString('latin1'),
                                /\/Count 2/, 'selected two-page PDF');
                        } else {
                            assert.deepEqual(original.subarray(0, 8),
                                Buffer.from('89504e470d0a1a0a', 'hex'));
                            if (page === 1) {
                                firstPng = original;
                            } else {
                                assert.notDeepEqual(original, firstPng,
                                    'second page must contain distinct ink');
                            }
                        }
                        accepted(...options, '--flow-markdown', syntactic,
                            '--output', output, '--replace');
                        assert.deepEqual(readFileSync(output), original,
                            `${name} ${format}/${page}: syntax changed ink`);
                        const failed = profiles(...options,
                            '--flow-markdown', '\\===', '--output', output,
                            '--replace');
                        assert.notEqual(failed.status, 0,
                            'escaped equals must remain tall authored ink');
                        assert.match(failed.stderr.toString('utf8'),
                            /flow line cannot fit printable vertical region/);
                        assert.equal(failed.stdout.length, 0);
                        assert.deepEqual(readFileSync(output), original,
                            'invalid literal ink cannot overwrite export');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('Unicode source separators preserve canonical native PDF and PNG pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-unicode-breaks-'));
        try {
            const base = ['notebook', '--printed-sheet', 'false',
                '--glyph-stroke', 'a', '600', '500', '1800', '3000',
                '250', '#123456', '--glyph-stroke', 'b', '800', '500',
                '2500', '3000', '250', '#654321', '--flow-region',
                '30000', '40000', '40000', '14000'];
            const fixtures = [
                ['markdown', '# a\u2028a\u2029b', '# a  \na\n\nb', 3],
                ['text', 'a\u2028a\u2029b', 'a  \na\n\nb', 4],
            ];
            for (const [mode, unicode, canonical, pageCount] of fixtures) {
                const source = join(dir, `${mode}.txt`);
                writeFileSync(source, unicode);
                const inline = `--flow-${mode}`;
                const file = `--flow-${mode}-file`;
                const exports = [
                    ['pdf', ['--export-paper', 'pdf']],
                    ...Array.from({ length: pageCount }, (_, index) => [
                        `png-page-${index + 1}`, ['--export-paper', 'png',
                            '--dpi', '72', '--png-page', `${index + 1}`],
                    ]),
                ];
                let priorPng = null;
                for (const [description, options] of exports) {
                    const render = (...sourceArgs) => accepted(...base,
                        ...options, ...sourceArgs);
                    const expected = render(inline, canonical);
                    assert.deepEqual(render(inline, unicode), expected,
                        `${mode}/${description}: Unicode changed page ink`);
                    assert.deepEqual(render(file, source), expected,
                        `${mode}/${description}: file changed page ink`);
                    if (description === 'pdf') {
                        assert.match(expected.toString('latin1'),
                            new RegExp(`/Count ${pageCount}\\b`));
                    } else {
                        assert.deepEqual(expected.subarray(0, 8),
                            Buffer.from('89504e470d0a1a0a', 'hex'));
                        if (description === `png-page-${pageCount}`) {
                            assert.notDeepEqual(expected, priorPng,
                                'final source glyph must remain page-local');
                        }
                        priorPng = expected;
                    }
                }
                assert.equal(readFileSync(source, 'utf8'), unicode,
                    'read-only flow admission must not rewrite source');
                // Even when only page one is requested, later Unicode
                // boundaries cannot hide an unauthored final-page glyph.
                const previous = join(dir, `${mode}-previous.png`);
                const png = ['--export-paper', 'png', '--dpi', '72',
                    '--png-page', '1', '--output', previous];
                accepted(...base, inline, canonical, ...png);
                const before = readFileSync(previous);
                const failed = profiles(...base, inline,
                    unicode.replace(/b$/, '\u2603'), ...png, '--replace');
                assert.notEqual(failed.status, 0);
                assert.match(failed.stderr.toString('utf8'),
                    /glyph|U\+2603/i);
                assert.equal(failed.stdout.length, 0,
                    'missing later-page glyph must not emit partial PNG');
                assert.deepEqual(readFileSync(previous), before,
                    'missing later-page glyph cannot replace page-one PNG');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('native Markdown space breaks preserve the backslash glyph and output',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-space-slash-'));
        try {
            const letter = [
                '--glyph-stroke', 'a', '600', '500', '1800', '3000',
                '250', '#123456',
            ];
            const slash = [
                '--glyph-stroke', '\\', '600', '500', '1800', '3000',
                '250', '#654321',
            ];
            const common = ['notebook', '--printed-sheet', 'false'];
            const region = ['--flow-region', '30000', '40000',
                '40000', '14000'];
            const literal = 'a\\  \na';
            const unicode = 'a\\\u2028a';
            const escaped = 'a\\\na';
            const spaces = 'a  \na';
            for (const [name, formatArgs] of [
                ['pdf', ['--export-paper', 'pdf', '--pages', '2']],
                ['png-1', ['--export-paper', 'png', '--dpi', '72',
                    '--png-page', '1']],
                ['png-2', ['--export-paper', 'png', '--dpi', '72',
                    '--png-page', '2']],
            ]) {
                const flow = (...args) => accepted(...common, ...letter,
                    ...args, ...region, ...formatArgs);
                const actual = flow(...slash, '--flow-markdown', literal);
                assert.deepEqual(flow(...slash, '--flow-markdown', unicode),
                    actual, `${name}: Unicode line break lost literal ink`);
                const manual = accepted(...common, ...letter, ...slash,
                    '--place-text', '1', '32500', '42500', 'a\\',
                    '--place-text', '2', '32500', '42500', 'a',
                    ...formatArgs);
                assert.deepEqual(actual, manual,
                    `${name}: native PDF/PNG omitted authored backslash`);
                const syntaxOnly = flow('--flow-markdown', escaped);
                assert.deepEqual(syntaxOnly,
                    flow('--flow-markdown', spaces),
                    `${name}: lone trailing backslash should stay syntactic`);
                if (name !== 'png-2') {
                    assert.notDeepEqual(actual, syntaxOnly,
                        `${name}: literal slash made no visible ink`);
                }
                const previous = join(dir, `${name}.output`);
                writeFileSync(previous, actual);
                const failure = profiles(...common, ...letter, ...region,
                    ...formatArgs, '--flow-markdown', literal,
                    '--output', previous, '--replace');
                assert.notEqual(failure.status, 0,
                    'missing literal slash shape must refuse');
                assert.match(failure.stderr.toString('utf8'),
                    /missing authored glyph shapes.*U\+005C/);
                assert.equal(failure.stdout.length, 0,
                    'refused flow must emit no partial PDF/PNG');
                assert.deepEqual(readFileSync(previous), actual,
                    `${name}: refusal replaced accepted export`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
