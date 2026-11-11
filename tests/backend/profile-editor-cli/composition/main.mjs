// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: LicenseRef-Proprietary
// Verify real native CLI selected-source resolution and no-clobber refusal.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
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
    return profilesInput(undefined, ...args);
}

function profilesInput(input, ...args) {
    const result = spawnSync(cargo, [
        'run', '--offline', '--quiet', '-p', 'atrament_profile_editor_cli',
        '--bin', 'atrament-profiles', '--', ...args,
    ], { cwd: root, input, timeout: 45_000,
        maxBuffer: 24 * 1024 * 1024 });
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
            refused(['notebook', '--paper-profile', paper,
                '--paper-profile', join(dir, 'missing-paper.json'),
                '--output', destination, '--replace'],
            /--paper-profile may appear only once/, destination);
            refused(['notebook', '--profile', notebook,
                '--paper-profile', paper, '--paper-profile', paper,
                '--output', destination, '--replace'],
            /--paper-profile may appear only once/, destination);
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
                ['notebook', '--handwriting-profile', sourcePaths[0],
                    '--glyph-stroke', 'a', '500', '500', '1800', '3000',
                    '250', '#123456', '--place-text', '1', '30000',
                    '40000', 'a', '--export-paper', 'pdf'],
                ['notebook', '--flow-markdown-file', sourcePaths[4],
                    '--glyph-file', glyph, '--export-paper', 'pdf'],
                ['notebook', '--flow-text-file', sourcePaths[4],
                    '--glyph-file', glyph, '--export-paper', 'pdf'],
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

test('handwriting rejects impossible flags before selected profile access',
    { skip: process.platform !== 'linux' }, () => {
        const dir = mkdtempSync(join(root, '.temp/handwriting-preflight-'));
        try {
            const source = join(dir, 'writer.json');
            const linked = join(dir, 'selected-link.json');
            const destination = join(dir, 'reviewed.json');
            accepted('handwriting', '--seed', '37', '--output', source);
            symlinkSync(source, linked);
            writeFileSync(destination, 'previously approved profile');
            const original = readFileSync(destination);
            const saved = readFileSync(source);
            const cases = [
                [['--profile', linked, '--profile', source,
                    '--output', destination, '--replace'],
                /--profile must appear once, before edits/],
                [['--profile', linked, '--seed', 'not-a-seed',
                    '--output', destination, '--replace'], /invalid seed/],
                [['--profile', linked, '--set-global', 'unknown_dimension',
                    '1', '2', '1', '1', '0', '--output', destination,
                    '--replace'], /unknown statistics dimension/],
                [['--profile', linked, '--set-global', 'width',
                    '2000', '1000', '1500', '1500', '0'],
                /statistics range refused: InvertedEnvelope/],
                [['--profile', linked, '--set-glyph', 'ñ', 'width',
                    '2000', '3000', '5000', '2500', '0'],
                /statistics range refused: MeanOutsideEnvelope/],
                [['--profile', linked, '--remove-glyph-dimension', 'a',
                    'not_a_dimension', '--output', destination, '--replace'],
                /unknown dimension/],
                [['--profile', linked, '--preview-glyph', 'a', '-1',
                    '--output', destination, '--replace'],
                /preview occurrence requires an unsigned integer/],
                [['--profile', linked, '--preview-glyph', 'ab', '0'],
                    /numeric preview requires one exact/],
                [['--profile', linked, '--set-glyph', 'ab',
                    'width', '1', '2', '1', '1', '0'],
                /glyph edit requires one exact/],
                [['--profile', linked, '--remove-glyph', 'ab'],
                /glyph removal requires one exact/],
                [['--profile', linked, '--inspect', '--output',
                    destination, '--replace'], /--inspect is stdout only/],
                [['--profile', linked, '--inventory-text', 'ñ',
                    '--output', destination, '--replace'],
                /--inventory-text is stdout only/],
                [['--profile', linked, '--replace', '--replace',
                    '--output', destination],
                /--replace may appear only once/],
                [['--profile', linked, '--output', '', '--replace'],
                /--output requires one explicit filename/],
                [['--profile', linked, '--unknown-handwriting-option',
                    '--output', destination, '--replace'],
                /unknown handwriting option/],
            ];
            for (const [flags, diagnostic] of cases) {
                const result = profiles('handwriting', ...flags);
                assert.notEqual(result.status, 0,
                    'impossible option set must be rejected');
                assert.match(result.stderr.toString('utf8'), diagnostic,
                    'source must not be opened before flag diagnostics');
                assert.equal(result.stdout.length, 0,
                    'invalid commands cannot emit partial JSON');
                assert.deepEqual(readFileSync(destination), original);
                assert.deepEqual(readFileSync(source), saved,
                    'rejected command cannot change source profile');
            }
            assert.deepEqual(accepted('handwriting', '--profile', source),
                saved, 'valid selected profiles remain readable');
            assert.match(accepted('handwriting', '--profile', source,
                '--inventory-text', '--output').toString('utf8'),
            /Exact text-inventory audit/,
            'literal flag-like inventory text must not be reinterpreted');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('standalone manual ink infers later PDF pages without flow', () => {
    const dir = mkdtempSync(join(root, '.temp/manual-pdf-pages-'));
    try {
        const paper = ['notebook', '--printed-sheet', 'true'];
        const shape = ['--glyph-stroke', 'a', '500', '500',
            '1800', '3000', '250', '#234567'];
        const placed = ['--place-text', '2', '80000', '50000', 'aa'];
        const line = ['--ink-line', '3', '70000', '40000',
            '71000', '41000', '250', '#123456'];
        const png = (page) => ['--export-paper', 'png', '--dpi', '72',
            '--png-page', String(page)];
        const selected = [...paper, ...shape, ...placed, ...line];

        for (const [manual, expectedPages] of [
            [[...paper, ...shape, ...placed], 2],
            [[...paper, ...line], 3],
            [selected, 3],
        ]) {
            const automatic = accepted(...manual, '--export-paper', 'pdf');
            assert.match(automatic.toString('latin1'),
                new RegExp(`/Count ${expectedPages}\\b`),
                'native PDF counts the last independently authored page');
            assert.deepEqual(automatic, accepted(...manual,
                '--export-paper', 'pdf', '--pages', String(expectedPages)),
            'automatic and explicitly sized PDF bytes must match');
        }

        assert.deepEqual(accepted(...selected, ...png(2)),
            accepted(...paper, ...shape, ...placed, ...png(2)),
            'page two draws only its manually placed glyphs');
        assert.deepEqual(accepted(...selected, ...png(3)),
            accepted(...paper, ...line, ...png(3)),
            'page three draws only its manual pen segment');
        assert.deepEqual(accepted(...selected, ...png(1)),
            accepted(...paper, ...png(1)),
            'unwritten first page stays transparent');

        const outputPdf = join(dir, 'reviewed.pdf');
        const outputPng = join(dir, 'reviewed.png');
        writeFileSync(outputPdf,
            accepted(...selected, '--export-paper', 'pdf'));
        writeFileSync(outputPng, accepted(...selected, ...png(2)));
        refused([...selected, '--export-paper', 'pdf', '--pages', '2',
            '--output', outputPdf, '--replace'],
        /manual ink requires 3 physical pages/, outputPdf);
        refused([...selected, '--place-text', '3', '80000', '50000',
            'b', ...png(2), '--output', outputPng, '--replace'],
        /missing authored glyph shapes/, outputPng);
        refused([...selected, '--ink-line', '3', '1000', '1000',
            '2000', '2000', '250', '#123456', '--export-paper', 'pdf',
            '--output', outputPdf, '--replace'],
        /page 3 ink needs printable endpoints/, outputPdf);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('sparse page-32 annotations preserve native PDF and PNG boundaries',
    () => {
        const dir = mkdtempSync(join(root, '.temp/sparse-page-32-'));
        try {
            const paper = ['notebook', '--printed-sheet', 'true'];
            const glyph = ['--glyph-stroke', 'a', '500', '500',
                '1800', '3000', '250', '#234567'];
            const placed = ['--place-text', '32', '80000', '50000', 'aa'];
            const line = ['--ink-line', '31', '70000', '40000',
                '71000', '41000', '250', '#123456'];
            const png = (page) => ['--export-paper', 'png', '--dpi', '72',
                '--png-page', String(page)];
            const reference31 = accepted(...paper, ...line, ...png(31));
            const reference32 = accepted(...paper, ...glyph, ...placed,
                ...png(32));
            assert.notDeepEqual(reference31, reference32,
                'distinct last-page ink must have distinct PNG geometry');

            for (const [name, flow] of [
                ['manual', []],
                ['plain', ['--flow-text', 'a', '--flow-region',
                    '30000', '40000', '30000', '14000']],
                ['markdown', ['--flow-markdown', '# a', '--flow-region',
                    '30000', '40000', '30000', '14000']],
            ]) {
                const base = [...paper, ...glyph, ...flow,
                    ...placed, ...line];
                const pdf = accepted(...base, '--export-paper', 'pdf');
                assert.match(pdf.toString('latin1'), /\/Count 32\b/,
                    `${name}: final annotation must include page 32`);
                assert.deepEqual(pdf, accepted(...base,
                    '--export-paper', 'pdf', '--pages', '32'),
                    `${name}: inferred and explicit 32-page PDFs must match`);
                assert.deepEqual(accepted(...base, ...png(31)), reference31,
                    `${name}: page 31 must contain only its manual line`);
                assert.deepEqual(accepted(...base, ...png(32)), reference32,
                    `${name}: page 32 must contain only its placed text`);
                if (flow.length !== 0) {
                    assert.deepEqual(accepted(...base, ...png(1)),
                        accepted(...paper, ...glyph, ...flow, ...png(1)),
                        `${name}: late ink must not alter first-page flow`);
                }

                const outputPdf = join(dir, `${name}.pdf`);
                const outputPng = join(dir, `${name}.png`);
                writeFileSync(outputPdf, pdf);
                writeFileSync(outputPng, reference32);
                refused([...base, '--export-paper', 'pdf', '--pages', '31',
                    '--output', outputPdf, '--replace'],
                    /require.*32 physical pages/, outputPdf);
                refused([...base, '--place-text', '31', '80000', '50000',
                    'b', ...png(32), '--output', outputPng, '--replace'],
                    /missing authored glyph shapes/, outputPng);
                refused([...base, '--ink-line', '31', '1000', '1000',
                    '2000', '2000', '250', '#123456', '--export-paper',
                    'pdf', '--output', outputPdf, '--replace'],
                    /page 31 ink needs printable endpoints/, outputPdf);
                refused([...base, '--place-text', '33', '80000', '50000',
                    'a', ...png(32), '--output', outputPng, '--replace'],
                    /text page/, outputPng);
                if (flow.length !== 0) {
                    refused([...base, ...png(30), '--output', outputPng,
                        '--replace'], /selected PNG page is empty/,
                        outputPng);
                }
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

test('native flow keeps manual text overlays outside its fixed rectangle',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-text-overlay-'));
        try {
            const shape = ['--glyph-stroke', 'a', '500', '500',
                '1800', '3000', '250', '#123456'];
            const overlay = ['--place-text', '2', '80000', '50000', 'aa'];
            const png = (page) => ['--export-paper', 'png', '--dpi', '72',
                '--png-page', String(page)];
            const paper = ['notebook', '--printed-sheet', 'true', ...shape];
            const manualPage = accepted(...paper, ...overlay, ...png(2));
            const destinationPng = join(dir, 'accepted.png');
            const destinationPdf = join(dir, 'accepted.pdf');

            for (const [flag, source] of [
                ['--flow-text', 'a'],
                ['--flow-markdown', 'a'],
            ]) {
                const flow = [...paper, flag, source, '--flow-region',
                    '30000', '40000', '30000', '14000'];
                const both = [...flow, ...overlay];
                const flowPage = accepted(...flow, ...png(1));
                assert.deepEqual(accepted(...both, ...png(1)), flowPage,
                    'later annotation cannot change page-one flowed ink');
                const later = accepted(...both, ...png(2));
                assert.deepEqual(later, manualPage,
                    'page-two manual glyph ink remains independent of flow');
                const pdf = accepted(...both, '--export-paper', 'pdf');
                assert.match(pdf.toString('latin1'), /\/Count 2\b/);
                assert.deepEqual(pdf, accepted(...both,
                    '--export-paper', 'pdf', '--pages', '2'));

                // Same-page annotations may sit outside flow-region but
                // still inside printable paper. They must not be clipped.
                const adjacent = accepted(...flow, '--place-text', '1',
                    '80000', '50000', 'a', ...png(1));
                assert.notDeepEqual(adjacent, flowPage);
                writeFileSync(destinationPng, later);
                writeFileSync(destinationPdf, pdf);
                refused([...both, ...png(3), '--output', destinationPng,
                    '--replace'], /selected PNG page is empty/,
                destinationPng);
                refused([...both, '--export-paper', 'pdf', '--pages', '1',
                    '--output', destinationPdf, '--replace'],
                /require 2 physical pages/, destinationPdf);
                refused([...both, '--place-text', '3', '80000', '50000',
                    'b', ...png(2), '--output', destinationPng, '--replace'],
                /missing authored glyph shapes/, destinationPng);
                refused([...both, '--place-text', '3', '1000', '1000',
                    'a', ...png(2), '--output', destinationPng, '--replace'],
                /page 3 ink needs printable endpoints/, destinationPng);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('seeded flow ink stays stable when later positioned runs are added',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-seeded-overlay-'));
        try {
            const shape = ['--glyph-stroke', 'a', '750', '600',
                '2100', '3200', '300', '#123456'];
            const variation = ['--glyph-variation', '19', '8', '7',
                '9', '4', '60', '65'];
            const paper = ['notebook', '--printed-sheet', 'true',
                ...shape, ...variation];
            const png = (page) => ['--export-paper', 'png', '--dpi',
                '72', '--png-page', String(page)];
            const overlay = ['--place-text', '2', '80000', '50000', 'aaa'];
            const outputPng = join(dir, 'approved.png');
            const outputPdf = join(dir, 'approved.pdf');

            for (const [flag, content] of [
                ['--flow-text', 'aa'],
                ['--flow-markdown', '# aa'],
            ]) {
                const flow = [...paper, flag, content, '--flow-region',
                    '30000', '40000', '40000', '30000'];
                const base = accepted(...flow, ...png(1));
                const combined = [...flow, ...overlay];
                assert.deepEqual(accepted(...combined, ...png(1)), base,
                    'manual ink cannot consume earlier flow variation');
                const later = accepted(...combined, ...png(2));
                assert.deepEqual(accepted(...combined, ...png(2)), later,
                    'authored later-page variation stays deterministic');
                assert.notDeepEqual(later, base,
                    'page-two overlay must produce independent ink');
                const pdf = accepted(...combined, '--export-paper', 'pdf');
                assert.match(pdf.toString('latin1'), /\/Count 2\b/);
                assert.deepEqual(pdf, accepted(...combined,
                    '--export-paper', 'pdf', '--pages', '2'));
                const longer = accepted(...flow, '--place-text', '2',
                    '80000', '50000', 'aaaa', ...png(2));
                assert.notDeepEqual(later, longer,
                    'later-page glyph content must affect selected PNG');

                writeFileSync(outputPng, later);
                writeFileSync(outputPdf, pdf);
                refused([...combined, '--place-text', '3', '80000',
                    '50000', 'b', ...png(1), '--output', outputPng,
                    '--replace'], /missing authored glyph shapes/, outputPng);
                refused([...combined, '--place-text', '3', '1000',
                    '1000', 'a', '--export-paper', 'pdf', '--output',
                    outputPdf, '--replace'],
                /page 3 ink needs printable endpoints/, outputPdf);
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

test('landscape Letter sheet preserves 32-page mixed-ink geometry and marks',
    () => {
        const dir = mkdtempSync(join(root, '.temp/letter-mixed-ink-'));
        try {
            const paperPath = join(dir, 'landscape-paper.json');
            accepted('paper', '--paper-size', 'letter', '--landscape',
                '--style', 'blank', '--emit-profile', '--output', paperPath);
            const paper = readFileSync(paperPath);
            const base = ['notebook', '--paper-profile', paperPath,
                '--printed-sheet', 'true', '--page-numbers', 'true',
                '--date-style', 'line', '--date-placement', 'top-left',
                '--date-text', '2026-11-10'];
            const glyph = ['--glyph-stroke', 'a', '500', '500',
                '1800', '3000', '250', '#234567'];
            const lateGlyph = ['--place-text', '32', '180000',
                '50000', 'aa'];
            const lateLine = ['--ink-line', '31', '140000', '50000',
                '160000', '51000', '350', '#123456'];
            const png = (page) => ['--export-paper', 'png', '--dpi', '72',
                '--png-page', String(page)];
            const manual31 = accepted(...base, ...lateLine, ...png(31));
            const manual32 = accepted(...base, ...glyph, ...lateGlyph,
                ...png(32));
            assert.notDeepEqual(accepted(...base, ...png(31)),
                accepted(...base, ...png(32)),
                'page numbers distinguish otherwise empty physical sheets');
            assert.notDeepEqual(manual31, manual32,
                'different numbered pages contain different physical ink');
            const pdfPath = join(dir, 'approved.pdf');
            const pngPath = join(dir, 'approved.png');
            for (const [name, flow] of [
                ['plain', ['--flow-text', 'a']],
                ['markdown', ['--flow-markdown', '# a']],
            ]) {
                const region = ['--flow-region', '30000', '40000',
                    '30000', '14000'];
                const source = [...base, ...glyph, ...flow, ...region];
                const mixed = [...source, ...lateGlyph, ...lateLine];
                const pdf = accepted(...mixed, '--export-paper', 'pdf');
                const content = pdf.toString('latin1');
                assert.match(content, /\/Count 32\b/,
                    `${name}: manual final ink extends the physical PDF`);
                assert.equal((content.match(
                    /\/MediaBox \[0 0 792\.000 612\.000\]/g) ?? []).length,
                32, `${name}: every page uses the landscape Letter box`);
                assert.deepEqual(pdf, accepted(...mixed,
                    '--export-paper', 'pdf', '--pages', '32'));
                const firstPage = accepted(...source, ...png(1));
                assert.deepEqual(accepted(...mixed, ...png(1)), firstPage,
                    `${name}: late ink cannot alter flow or printed marks`);
                for (const [page, reference] of [
                    [31, manual31], [32, manual32],
                ]) {
                    const selected = accepted(...mixed, ...png(page));
                    assert.deepEqual(selected, reference,
                        `${name}: only page ${page} ink can be selected`);
                    assert.deepEqual(selected.subarray(0, 8),
                        Buffer.from('89504e470d0a1a0a', 'hex'));
                    assert.equal(selected.readUInt32BE(16), 792,
                        'landscape Letter PNG width at 72 DPI');
                    assert.equal(selected.readUInt32BE(20), 612,
                        'landscape Letter PNG height at 72 DPI');
                    assert.equal(selected[25], 6, 'transparent RGBA output');
                }
                writeFileSync(pdfPath, pdf);
                writeFileSync(pngPath, manual32);
                refused([...mixed, '--export-paper', 'pdf',
                    '--pages', '31', '--output', pdfPath, '--replace'],
                /require 32 physical pages/, pdfPath);
                refused([...mixed, '--place-text', '32', '270000',
                    '50000', 'a', ...png(1), '--output', pngPath,
                    '--replace'], /page 32 ink needs printable endpoints/,
                pngPath);
                refused([...mixed, '--place-text', '33', '180000',
                    '50000', 'a', ...png(1), '--output', pngPath,
                    '--replace'], /text page/, pngPath);
                assert.deepEqual(readFileSync(paperPath), paper,
                    'native exports cannot mutate the selected paper file');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('saved physical ink TSV matches inline native PDF/PNG on late pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/page-ink-file-'));
        try {
            const file = join(dir, 'authored-ink.tsv');
            const rows = [
                '31\t140000\t50000\t160000\t51000\t350\t#123456',
                '32\t170000\t51000\t190000\t52000\t400\t#234567',
            ];
            const header = 'atrament.page-ink.v1\n';
            writeFileSync(file, `${header}${rows.join('\n')}\n`);
            const saved = readFileSync(file);
            const inline = rows.flatMap((row) => [
                '--ink-line', ...row.split('\t'),
            ]);
            const selected = ['--ink-file', file];
            const notebook = ['notebook', '--printed-sheet', 'true',
                '--page-numbers', 'true', '--date-style', 'line',
                '--date-placement', 'top-left', '--date-text', '2026-11-10'];
            const shape = ['--glyph-stroke', 'a', '500', '500', '1800',
                '3000', '250', '#142536'];
            const scenarios = [
                ['manual', []],
                ['plain flow', [...shape, '--flow-text', 'a']],
                ['Markdown flow', [...shape, '--flow-markdown', '# a']],
                ['flow with positioned text', [...shape, '--flow-text', 'a',
                    '--place-text', '32', '100000', '50000', 'aa']],
            ];
            for (const [kind, flow] of scenarios) {
                const base = [...notebook, ...flow];
                const actual = accepted(...base, ...selected,
                    '--export-paper', 'pdf');
                assert.match(actual.toString('latin1'), /\/Count 32\b/,
                    `${kind}: late saved ink extends native PDF page count`);
                assert.deepEqual(actual, accepted(...base, ...inline,
                    '--export-paper', 'pdf'),
                `${kind}: persisted and inline segments share PDF geometry`);
                assert.deepEqual(actual, accepted(...base, ...selected,
                    '--export-paper', 'pdf', '--pages', '32'));
                for (const page of [1, 31, 32]) {
                    const png = ['--export-paper', 'png', '--dpi', '72',
                        '--png-page', String(page)];
                    assert.deepEqual(accepted(...base, ...selected, ...png),
                        accepted(...base, ...inline, ...png),
                        `${kind}: page ${page} matches inline RGBA ink`);
                }
            }
            const first = join(dir, 'first.tsv');
            const last = join(dir, 'last.tsv');
            writeFileSync(first, `${header}${rows[0]}\n`);
            writeFileSync(last, `${header}${rows[1]}\n`);
            assert.deepEqual(accepted(...notebook, '--ink-file', first,
                '--ink-file', last, '--export-paper', 'pdf'),
                accepted(...notebook, ...selected, '--export-paper', 'pdf'),
                'multiple selected files combine in authored stroke order');
            assert.deepEqual(readFileSync(file), saved,
                'input ink is never rewritten by export');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('malformed and untrusted physical ink files cannot replace exports',
    { skip: process.platform !== 'linux' }, () => {
        const dir = mkdtempSync(join(root, '.temp/page-ink-refusal-'));
        try {
            const selected = join(dir, 'candidate.tsv');
            const approved = join(dir, 'approved.png');
            const approvedPdf = join(dir, 'approved.pdf');
            const line = '31\t140000\t50000\t160000\t51000\t350\t#123456';
            const source = `atrament.page-ink.v1\n${line}\n`;
            writeFileSync(selected, source);
            const args = ['notebook', '--ink-file', selected];
            const png = ['--export-paper', 'png', '--dpi', '72',
                '--png-page', '31'];
            writeFileSync(approved, accepted(...args, ...png));
            writeFileSync(approvedPdf, accepted(...args,
                '--export-paper', 'pdf'));
            const check = (bytes, expected) => {
                writeFileSync(selected, bytes);
                for (const [format, output] of [
                    [png, approved], [['--export-paper', 'pdf'], approvedPdf],
                ]) {
                    refused([...args, ...format, '--output', output,
                        '--replace'], expected, output);
                }
            };
            check('wrong.v1\n' + line + '\n', /ink file needs.*header/);
            check('atrament.page-ink.v1\n', /at least one physical segment/);
            check('atrament.page-ink.v1\n' + line + '\n\n' + line,
                /seven tab-separated fields/);
            check('atrament.page-ink.v1\n31\t140000\t50000\n',
                /seven tab-separated fields/);
            check('atrament.page-ink.v1\n' + line + '\textra\n',
                /seven tab-separated fields/);
            check('atrament.page-ink.v1\n' + line.replace('31', '33') + '\n',
                /ink page requires 1\.\.32/);
            check('atrament.page-ink.v1\n' + line.replace('#123456',
                '#broken') + '\n', /ink color must be #RRGGBB/);
            check('atrament.page-ink.v1\n' + line.replace('140000',
                '0') + '\n', /page 31 ink needs printable endpoints/);
            check(Buffer.from([0xff, 0xfe, 0xfd]), /UTF-8/);
            check(Buffer.alloc(262_145, 0x61), /input refused/);
            check(`atrament.page-ink.v1\n${Array(4097).fill(line)
                .join('\n')}\n`, /4096 explicit line segments/);
            writeFileSync(selected, source);
            const link = join(dir, 'ink-link.tsv');
            symlinkSync(selected, link);
            refused(['notebook', '--ink-file', link, ...png,
                '--output', approved, '--replace'],
            /SymlinkForbidden/, approved);
            refused(['notebook', '--ink-file',
                `${dir}/real/../candidate.tsv`, ...png,
                '--output', approved, '--replace'],
            /ParentTraversal/, approved);
            refused(['notebook', '--ink-file', selected,
                '--ink-line', '33', '100000', '50000', '120000',
                '51000', '300', '#123456', ...png,
                '--output', approved, '--replace'],
            /ink page requires 1\.\.32/, approved);
            refused(['notebook', '--ink-file', selected,
                '--export-paper', 'pdf', '--pages', '30',
                '--output', approvedPdf, '--replace'],
            /requires 31 physical pages/, approvedPdf);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compiled page ink TSV replays mixed flow and manual PDF/PNG bytes',
    () => {
        const dir = mkdtempSync(join(root, '.temp/page-ink-output-'));
        try {
            const file = join(dir, 'compiled.tsv');
            const notebook = ['notebook', '--printed-sheet', 'true',
                '--page-numbers', 'true', '--date-style', 'line',
                '--date-placement', 'top-left', '--date-text', '2026-11-10'];
            const authored = [
                '--glyph-stroke', 'a', '500', '500', '1800', '3000',
                '250', '#142536',
                '--glyph-stroke', 'a', '1800', '3000', '2500', '3000',
                '250', '#142536',
            ];
            const content = [
                ...authored, '--flow-markdown', '# a\n\n- aa',
                '--heading-style', 'marker', '--heading-color', '#F8DE91',
                '--heading-ink-color', '#BD376A', '--heading-ink-scale', '180',
                '--place-text', '2', '100000', '50000', 'aa',
                '--ink-line', '2', '70000', '40000', '71000', '41000',
                '250', '#abcdef',
            ];
            const bytes = accepted(...notebook, ...content, '--emit-ink-file');
            const rows = bytes.toString('utf8').trimEnd().split('\n');
            assert.equal(rows[0], 'atrament.page-ink.v1');
            assert.ok(rows.length > 6, 'flowed glyphs and ornaments compiled');
            for (const row of rows.slice(1)) {
                assert.match(
                    row, /^\d+\t\d+\t\d+\t\d+\t\d+\t\d+\t#[0-9A-F]{6}$/,
                );
            }
            assert.ok(rows.includes(
                '2\t70000\t40000\t71000\t41000\t250\t#ABCDEF',
            ));
            assert.deepEqual(
                accepted(...notebook, ...content, '--emit-ink-file'),
                bytes, 'repeated compilation is byte stable',
            );
            assert.deepEqual(accepted(...notebook, ...content,
                '--emit-ink-file', '--output', file), Buffer.alloc(0));
            assert.deepEqual(readFileSync(file), bytes,
                'explicit file output equals stdout without a second codec');
            const physical = [...notebook, '--ink-file', file];
            assert.deepEqual(accepted(...physical, '--emit-ink-file'), bytes,
                'read-only reimport preserves exact physical segment order');
            assert.deepEqual(accepted(...physical, '--export-paper', 'pdf'),
                accepted(...notebook, ...content, '--export-paper', 'pdf'),
                'compiled ink and original mixed source have identical PDF');
            for (const page of [1, 2]) {
                const png = ['--export-paper', 'png', '--dpi', '72',
                    '--png-page', String(page)];
                assert.deepEqual(accepted(...physical, ...png),
                    accepted(...notebook, ...content, ...png),
                    `compiled physical page ${page} matches original PNG`);
            }
            refused([...notebook, ...content, '--emit-ink-file',
                '--output', file], /not saved|Exists/, file);
            assert.deepEqual(readFileSync(file), bytes);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('glyph library stdin preserves exact Unicode ink and rejects unsafe pipes',
    () => {
        const dir = mkdtempSync(join(root, '.temp/glyph-library-pipe-'));
        try {
            const saved = join(dir, 'authored.tsv');
            const destination = join(dir, 'accepted.pdf');
            const glyph = accepted('glyphs',
                '--glyph-stroke', 'ñ', '500', '500', '1800', '3000',
                '250', '#123456',
                '--glyph-stroke', 'n\u0303', '600', '600', '1800',
                '3100', '250', '#AABBCC');
            writeFileSync(saved, glyph);
            const authored = ['--place-text', '1', '80000', '50000',
                'ñn\u0303'];
            const viaFile = (format) => accepted('notebook',
                '--printed-sheet', 'true', '--glyph-file', saved,
                ...authored, ...format);
            const viaPipe = (input, ...args) =>
                profilesInput(input, ...args);
            const canonical = viaPipe(glyph, 'glyphs', '--profile', '-');
            assert.equal(canonical.status, 0, canonical.stderr.toString());
            assert.deepEqual(canonical.stdout, glyph,
                'stdin preserves exact NFC/NFD glyph identities and order');
            const inspected = viaPipe(glyph, 'glyphs', '--profile', '-',
                '--inspect-glyph', 'n\u0303');
            assert.equal(inspected.status, 0, inspected.stderr.toString());
            assert.match(inspected.stdout.toString(), /U\+006E U\+0303/);

            for (const format of [
                ['--export-paper', 'pdf'],
                ['--export-paper', 'png', '--dpi', '72'],
                ['--emit-ink-file'],
            ]) {
                const piped = viaPipe(glyph, 'notebook',
                    '--printed-sheet', 'true', '--glyph-file', '-',
                    ...authored, ...format);
                assert.equal(piped.status, 0, piped.stderr.toString());
                assert.deepEqual(piped.stdout, viaFile(format),
                    `${format.join(' ')} keeps native ink byte-identical`);
            }
            const pdf = viaFile(['--export-paper', 'pdf']);
            writeFileSync(destination, pdf);
            const refusedPipe = (input, args, expected) => {
                const result = viaPipe(input, ...args,
                    '--output', destination, '--replace');
                assert.notEqual(result.status, 0, args.join(' '));
                assert.match(result.stderr.toString(), expected);
                assert.equal(result.stdout.length, 0,
                    'invalid glyph stream cannot emit partial output');
                assert.deepEqual(readFileSync(destination), pdf,
                    'invalid glyph stream cannot replace accepted PDF');
            };
            const notebook = ['notebook', '--glyph-file', '-',
                ...authored, '--export-paper', 'pdf'];
            refusedPipe(Buffer.alloc(524_289, 0x61), notebook,
                /glyph stdin exceeds 524288/);
            refusedPipe(Buffer.from([0xff, 0xfe, 0xfd]), notebook,
                /glyph library must be UTF-8/);
            refusedPipe(Buffer.from('atrament.glyph-strokes.v1\n'),
                notebook, /glyph library needs at least one authored stroke/);
            refusedPipe(Buffer.concat([glyph,
                Buffer.from('broken glyph record\n')]), notebook,
                /glyph library line 4 needs seven tab-separated fields/);
            refusedPipe(glyph, ['notebook', '--glyph-file', '-',
                '--ink-file', '-', ...authored, '--export-paper', 'pdf'],
            /standard input may be selected only once/);
            refusedPipe(glyph, ['notebook', '--glyph-file', '-',
                '--flow-markdown-file', '-', '--export-paper', 'pdf'],
            /standard input may be selected only once/);
            refusedPipe(Buffer.from('ñ'), [
                'notebook', '--flow-text-file', '-', '--glyph-file', '-',
                '--export-paper', 'pdf'],
            /standard input may be selected only once/);
            refusedPipe(Buffer.from(
                'atrament.page-ink.v1\n1\t70000\t40000\t71000'
                    + '\t41000\t250\t#123456\n'), [
                'notebook', '--ink-file', '-', '--glyph-file', '-',
                '--export-paper', 'pdf'],
            /standard input may be selected only once/);
            const double = viaPipe(glyph, 'glyphs', '--profile', '-',
                '--profile', '-');
            assert.notEqual(double.status, 0);
            assert.match(double.stderr.toString(),
                /must precede authored strokes and may appear only once/);
            // Refuse ordering before reading even a missing source or a
            // second pipe. Invalid commands cannot replace accepted output.
            const shape = ['--glyph-stroke', 'a', '500', '500',
                '1800', '3000', '250', '#123456'];
            const missing = join(dir, 'does-not-exist.tsv');
            refused(['glyphs', ...shape, '--profile', missing,
                '--output', destination, '--replace'],
            /must precede authored strokes and may appear only once/,
            destination);
            refused(['notebook', ...shape, '--glyph-file', missing,
                '--export-paper', 'pdf', '--output', destination, '--replace'],
            /must precede authored strokes and may appear only once/,
            destination);
            assert.deepEqual(readFileSync(saved), glyph,
                'saved source remains untouched by piped replay');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('invalid selected output names refuse before stdin is read',
    { skip: process.platform !== 'linux' }, async () => {
        const dir = mkdtempSync(join(root, '.temp/output-preflight-'));
        try {
            accepted('notebook');
            const executable = join(root,
                '.cache/cargo-target/debug/atrament-profiles');
            const destination = join(dir, 'reviewed.pdf');
            writeFileSync(destination, 'previous reviewed bytes');
            const original = readFileSync(destination);
            for (const [args, expected] of [
                [['notebook', '--ink-file', '-', '--output', ''],
                    /--output requires one explicit filename/],
                [['notebook', '--glyph-file', '-', '--output', '-'],
                    /--output requires one explicit filename/],
                [['notebook', '--flow-text-file', '-', '--output',
                    '--replace'], /--output requires one explicit filename/],
                [['notebook', '--ink-file', '-', '--output'],
                    /--output requires a filename/],
                [['glyphs', '--profile', '-', '--output', ''],
                    /--output requires one explicit filename/],
                [['glyphs', '--profile', '-', '--output'],
                    /--output requires a filename/],
            ]) {
                const result = await new Promise((resolve, reject) => {
                    const child = spawn(executable, args, {
                        cwd: root, stdio: ['pipe', 'pipe', 'pipe'],
                    });
                    let stderr = '';
                    let stdout = '';
                    child.stderr.setEncoding('utf8');
                    child.stdout.setEncoding('utf8');
                    child.stderr.on('data', (part) => { stderr += part; });
                    child.stdout.on('data', (part) => { stdout += part; });
                    const timer = setTimeout(() => {
                        child.kill();
                        reject(new Error('invalid output blocked on stdin'));
                    }, 5_000);
                    child.on('error', (error) => {
                        clearTimeout(timer);
                        reject(error);
                    });
                    child.on('close', (status) => {
                        clearTimeout(timer);
                        resolve({ status, stderr, stdout });
                    });
                });
                assert.notEqual(result.status, 0);
                assert.match(result.stderr, expected);
                assert.equal(result.stdout, '');
                assert.deepEqual(readFileSync(destination), original);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('over-budget inline ink refuses before an earlier open stdin read',
    { skip: process.platform !== 'linux' }, async () => {
        const dir = mkdtempSync(join(root, '.temp/inline-ink-budget-'));
        try {
            accepted('notebook');
            const executable = join(root,
                '.cache/cargo-target/debug/atrament-profiles');
            const destination = join(dir, 'approved.pdf');
            writeFileSync(destination, 'previous approved document');
            const previous = readFileSync(destination);
            const line = ['--ink-line', '1', '70000', '40000',
                '71000', '41000', '250', '#123456'];
            const inline = Array.from({ length: 4097 }, () => line).flat();
            for (const source of [
                ['--ink-file', '-'],
                ['--flow-markdown-file', '-'],
            ]) {
                const flags = ['notebook', ...source, ...inline,
                    '--export-paper', 'pdf', '--output', destination,
                    '--replace'];
                const result = await new Promise((resolve, reject) => {
                    const child = spawn(executable, flags, {
                        cwd: root, stdio: ['pipe', 'pipe', 'pipe'],
                    });
                    let stdout = '';
                    let stderr = '';
                    child.stdout.setEncoding('utf8');
                    child.stderr.setEncoding('utf8');
                    child.stdout.on('data', (chunk) => { stdout += chunk; });
                    child.stderr.on('data', (chunk) => { stderr += chunk; });
                    const timeout = setTimeout(() => {
                        child.kill();
                        reject(new Error('inline ink budget blocked on stdin'));
                    }, 5_000);
                    child.on('error', (error) => {
                        clearTimeout(timeout);
                        reject(error);
                    });
                    child.on('close', (status) => {
                        clearTimeout(timeout);
                        resolve({ status, stdout, stderr });
                    });
                    // Do not close stdin: no invalid request should read it.
                });
                assert.notEqual(result.status, 0);
                assert.match(result.stderr,
                    /ink exceeds 4096 explicit line segments/);
                assert.equal(result.stdout, '');
                assert.deepEqual(readFileSync(destination), previous,
                    'the previously approved document cannot be replaced');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Keep the pipe open and unwritten: an impossible request must exit before
// attempting even a bounded stdin read. The native binary was built by the
// ordinary CLI invocation, so this check does not wait on Cargo's file lock.
test('unavailable glyph and ink inputs refuse without waiting for stdin',
    { skip: process.platform !== 'linux' }, async () => {
        const dir = mkdtempSync(join(root, '.temp/glyph-order-pipe-'));
        try {
            const shape = ['--glyph-stroke', 'a', '500', '500',
                '1800', '3000', '250', '#123456'];
            accepted('glyphs', ...shape);
            const destination = join(dir, 'accepted.pdf');
            writeFileSync(destination, 'previous accepted output');
            const previous = readFileSync(destination);
            const executable = join(root,
                '.cache/cargo-target/debug/atrament-profiles');
            const saturated = join(dir, 'full-ink.tsv');
            const line = '1\t70000\t40000\t71000\t41000\t250\t#123456\n';
            writeFileSync(saturated,
                'atrament.page-ink.v1\n' + line.repeat(4096));
            for (const [flags, expected] of [
                [['glyphs', ...shape, '--profile', '-'],
                    /must precede authored strokes and may appear only once/],
                [['glyphs', '--profile', '-', '--profile', '-'],
                    /must precede authored strokes and may appear only once/],
                [['glyphs', '--profile', '-', '--profile',
                    join(dir, 'missing-glyph.tsv')],
                    /must precede authored strokes and may appear only once/],
                [['glyphs', '--profile', '-', ...shape, '--profile',
                    join(dir, 'missing-glyph.tsv')],
                    /must precede authored strokes and may appear only once/],
                [['glyphs', '--profile', '-', '--audit-text', 'a',
                    '--audit-text', 'b'],
                    /--audit-text may appear only once/],
                [['glyphs', '--profile', '-', '--inspect-paths', 'a',
                    '--inspect-glyph', 'b'],
                    /glyph inspection may appear only once/],
                [['glyphs', '--profile', '-', '--replace'],
                    /--replace may appear only once/],
                [['glyphs', '--profile', '-', '--output',
                    join(dir, 'another.tsv')],
                    /--output requires one explicit filename/],
                [['glyphs', '--profile', '-', '--audit-text', 'a',
                    '--inspect-glyph', 'b'],
                    /glyph inspection cannot accompany a glyph audit/],
                [['notebook', ...shape, '--glyph-file', '-',
                    '--export-paper', 'pdf'],
                    /must precede authored strokes and may appear only once/],
                // Source-independent missing glyphs/placement must refuse
                // even while the selected stdin pipe remains open.
                [['notebook', '--flow-text-file', '-', '--export-paper', 'pdf'],
                    /flow requires authored glyph shapes/],
                [['notebook', '--flow-markdown-file', '-',
                    '--export-paper', 'png'],
                    /flow requires authored glyph shapes/],
                [['notebook', '--ink-file', '-', '--handwriting-profile',
                    join(dir, 'unopened-statistics.json'),
                    '--export-paper', 'pdf'],
                    /--handwriting-profile requires both/],
                [['notebook', '--ink-file', '-', '--glyph-variation',
                    '1', '1', '1', '1', '1', '1', '1',
                    '--export-paper', 'pdf'],
                    /--glyph-variation requires placed text/],
                [['notebook', '--ink-file', '-', '--glyph-stroke', 'a',
                    '500', '500', '1800', '3000', '250', '#123456',
                    '--export-paper', 'pdf'],
                    /--glyph-stroke and --place-text require one another/],
                [['notebook', '--ink-file', '-', '--place-text', '1',
                    '30000', '40000', 'a', '--export-paper', 'pdf'],
                    /--glyph-stroke and --place-text require one another/],
                [['notebook', '--ink-file', '-', '--join-text-pair', 'a',
                    'b', '--export-paper', 'pdf'],
                    /--join-text-pair requires placed text/],
                [['notebook', '--glyph-file', '-', '--join-text-pair', 'a',
                    'b', '--export-paper', 'pdf'],
                    /--join-text-pair requires placed text/],
                [['notebook', '--ink-file', '-', '--fallback-glyph', 'ñ',
                    'a', '--export-paper', 'pdf'],
                    /--fallback-glyph requires authored glyphs and placed text/],
                [['notebook', '--glyph-file', '-', '--fallback-glyph', 'ñ',
                    'a', '--export-paper', 'pdf'],
                    /--fallback-glyph requires authored glyphs and placed text/],
                [['notebook', '--ink-file', saturated, '--ink-file', '-',
                    '--emit-ink-file'], /ink exceeds 4096 explicit/],
                [['notebook', '--ink-file', '-', '--ink-file', '-',
                    '--emit-ink-file'],
                    /standard input may be selected only once/],
                [['notebook', '--ink-file', '-', '--ink-line', '1',
                    '70000', '40000', '71000', '41000', '250', '#123456'],
                    /ink and glyph overlays need --export-paper/],
                [['notebook', '--glyph-file', '-', '--place-text',
                    '1', '30000', '40000', 'a'],
                    /ink and glyph overlays need --export-paper/],
                [['notebook', '--ink-file', '-', '--glyph-variation',
                    '1', '1', '1', '1', '1', '1', '1'],
                    /ink and glyph overlays need --export-paper/],
                [['notebook', '--glyph-file', '-', '--ink-file', '-',
                    '--export-paper', 'pdf'],
                    /standard input may be selected only once/],
                [['notebook', '--flow-markdown-file', '-', '--glyph-file', '-',
                    '--export-paper', 'pdf'],
                    /standard input may be selected only once/],
                [['notebook', '--ink-file', '-', '--flow-text-file', '-',
                    '--export-paper', 'pdf'],
                    /standard input may be selected only once/],
                [['notebook', '--glyph-file', '-', '--glyph-file',
                    join(dir, 'absent-glyph.tsv'), '--export-paper', 'pdf'],
                    /must precede authored strokes and may appear only once/],
                [['notebook', '--glyph-file', '-', ...shape,
                    '--glyph-file', join(dir, 'absent-glyph.tsv'),
                    '--export-paper', 'pdf'],
                    /must precede authored strokes and may appear only once/],
                [['notebook', '--flow-text-file', '-', '--flow-markdown', 'a',
                    '--export-paper', 'pdf'],
                    /only one flow source may be supplied/],
                [['notebook', '--flow-markdown-file', '-',
                    '--flow-text-file', join(dir, 'absent-flow.md'),
                    '--export-paper', 'pdf'],
                    /only one flow source may be supplied/],
                [['notebook', '--ink-file', '-',
                    '--paper-profile', join(dir, 'missing-paper.json'),
                    '--paper-profile', join(dir, 'missing-paper-2.json'),
                    '--export-paper', 'pdf'],
                    /--paper-profile may appear only once/],
                [['notebook', '--glyph-file', '-', '--date-style', 'line',
                    '--profile', join(dir, 'missing-notebook.json'),
                    '--export-paper', 'pdf'],
                    /--profile must appear once, before edits/],
                [['notebook', '--ink-file', '-',
                    '--handwriting-profile', join(dir, 'first.json'),
                    '--handwriting-profile', join(dir, 'second.json'),
                    '--export-paper', 'pdf'],
                    /--handwriting-profile may appear only once/],
                [['notebook', '--ink-file', '-', '--handwriting-profile',
                    '', '--export-paper', 'pdf'],
                    /invalid handwriting profile path/],
                [['notebook', '--glyph-file', '-', '--handwriting-profile',
                    '-', '--export-paper', 'pdf'],
                    /invalid handwriting profile path/],
                [['notebook', '--flow-text-file', '-',
                    '--handwriting-profile', '--replace',
                    '--export-paper', 'pdf'],
                    /invalid handwriting profile path/],
                [['notebook', '--flow-text-file', '-',
                    '--flow-region', '20000', '30000', '40000', '50000',
                    '--flow-region', '10000', '20000', '30000', '40000',
                    '--export-paper', 'pdf'],
                    /--flow-region may appear only once/],
                [['notebook', '--glyph-file', '-',
                    '--output', join(dir, 'other.pdf'),
                    '--export-paper', 'pdf'],
                    /--output requires one explicit filename/],
                [['notebook', '--ink-file', '-', '--inspect', '--inspect'],
                    /--inspect may appear only once/],
                [['notebook', '--ink-file', '-', '--replace'],
                    /--replace may appear only once/],
                [['notebook', '--glyph-file', '-', '--export-paper', 'pdf',
                    '--export-paper', 'png'],
                    /--export-paper may appear only once/],
                [['notebook', '--ink-file', '-', '--emit-ink-file',
                    '--export-paper', 'pdf'],
                    /--export-paper may appear only once/],
                [['notebook', '--flow-text-file', '-', '--export-paper',
                    'pdf', '--emit-ink-file'],
                    /--emit-ink-file may appear only once/],
                [['notebook', '--ink-file', '-', '--pages', '1',
                    '--pages', '2', '--export-paper', 'pdf'],
                    /--pages may appear only once/],
                [['notebook', '--ink-file', '-', '--dpi', '150',
                    '--dpi', '72', '--export-paper', 'png'],
                    /--dpi may appear only once/],
                [['notebook', '--ink-file', '-', '--quality', 'preview',
                    '--quality', 'final', '--export-paper', 'png'],
                    /--quality may appear only once/],
                [['notebook', '--ink-file', '-', '--png-page', '1',
                    '--png-page', '2', '--export-paper', 'png'],
                    /--png-page may appear only once/],
                [['notebook', '--ink-file', '-', '--inspect'],
                    /--inspect is stdout only/],
                [['notebook', '--flow-text-file', '-', '--pages', '2',
                    '--emit-ink-file'],
                    /--pages requires --export-paper pdf/],
                [['notebook', '--ink-file', '-', '--dpi', '150',
                    '--export-paper', 'pdf'],
                    /require --export-paper png/],
                [['notebook', '--ink-file', '-', '--quality', 'preview',
                    '--dpi', '150', '--export-paper', 'png'],
                    /--quality and --dpi are mutually exclusive/],
                [['notebook', '--ink-file', '-', '--pages', '0',
                    '--export-paper', 'pdf'], /--pages requires 1\.\.32/],
                [['notebook', '--glyph-file', '-', '--pages', '33',
                    '--export-paper', 'pdf'], /--pages requires 1\.\.32/],
                [['notebook', '--ink-file', '-', '--dpi', '71',
                    '--export-paper', 'png'], /--dpi requires 72\.\.300/],
                [['notebook', '--flow-text-file', '-', '--dpi', '301',
                    '--export-paper', 'png'], /--dpi requires 72\.\.300/],
                [['notebook', '--ink-file', '-', '--png-page', '0',
                    '--export-paper', 'png'], /--png-page requires 1\.\.32/],
                [['notebook', '--glyph-file', '-', '--png-page', '33',
                    '--export-paper', 'png'], /--png-page requires 1\.\.32/],
                [['notebook', '--ink-file', '-', '--quality', 'draft',
                    '--export-paper', 'png'],
                    /--quality requires preview or final/],
                [['notebook', '--ink-file', '-', '--quality'],
                    /--quality requires preview or final/],
                [['notebook', '--ink-file', '-', '--export-paper', 'bad'],
                    /--export-paper requires pdf or png/],
                [['notebook', '--flow-text-file', '-', '--flow-region',
                    '-1', '30000', '40000', '50000', '--export-paper', 'pdf'],
                    /flow region left must be unsigned um/],
                [['notebook', '--flow-markdown-file', '-', '--flow-region',
                    '20000', '30000', '0', '50000', '--export-paper', 'pdf'],
                    /flow region width and height must be positive/],
                [['notebook', '--flow-markdown-file', '-', '--flow-region',
                    '20000', '30000', '40000', '0', '--export-paper', 'pdf'],
                    /flow region width and height must be positive/],
                [['notebook', '--flow-text-file', '-', '--flow-region',
                    '20000', '4294967296', '40000', '50000',
                    '--export-paper', 'pdf'],
                    /flow region top exceeds u32 micrometres/],
                [['notebook', '--flow-markdown-file', '-', '--flow-region',
                    '20000', '30000', '40000'],
                    /flow region height must be unsigned um/],
                [['notebook', '--flow-markdown-file', '-', '--flow-region',
                    '--flow-region', '20000', '30000', '40000'],
                    /flow region left must be unsigned um/],
                [['notebook', '--ink-file', '-', '--ink-line', '0',
                    '30000', '40000', '32000', '42000', '250', '#123456',
                    '--export-paper', 'pdf'], /ink page requires 1\.\.32/],
                [['notebook', '--glyph-file', '-', '--ink-line', '1',
                    '700001', '40000', '32000', '42000', '250', '#123456',
                    '--export-paper', 'pdf'], /ink x1 requires 0\.\.700000/],
                [['notebook', '--flow-text-file', '-', '--ink-line', '1',
                    '30000', '40000', '32000', '42000', '79', '#123456',
                    '--export-paper', 'pdf'], /ink width requires 80\.\.1500/],
                [['notebook', '--flow-markdown-file', '-', '--ink-line', '1',
                    '30000', '40000', '32000', '42000', '250', '#GGGGGG',
                    '--export-paper', 'pdf'], /ink color must be #RRGGBB/],
                [['notebook', '--flow-text-file', '-', '--ink-line', '1',
                    '30000', '40000', '32000', '42000', '250', '--ink-file',
                    '--export-paper', 'pdf'], /ink color must be #RRGGBB/],
                // Typed glyph options must refuse before opening an
                // earlier selected but unwritten standard-input source.
                [['notebook', '--glyph-file', '-', '--glyph-stroke', 'a',
                    '3501', '500', '1800', '3000', '250', '#123456'],
                    /glyph x1 requires 0\.\.3500/],
                [['notebook', '--ink-file', '-', '--glyph-stroke', 'a',
                    '500', '500', '500', '500', '250', '#123456'],
                    /glyph stroke must have two distinct points/],
                [['notebook', '--flow-text-file', '-', '--glyph-stroke', 'a',
                    '500', '500', '1800', '3000', '250', '#GGGGGG'],
                    /glyph color requires #RRGGBB/],
                [['notebook', '--glyph-file', '-', '--glyph-variation',
                    '42', '21', '0', '0', '0', '0', '0'],
                    /variation width requires 0\.\.20/],
                [['notebook', '--flow-markdown-file', '-',
                    '--glyph-variation', '42', '0', '0', '0', '0', '0', '151'],
                    /variation regularity requires 0\.\.150/],
                [['notebook', '--glyph-file', '-', '--place-text', '0',
                    '30000', '40000', 'a'], /text page requires 1\.\.32/],
                [['notebook', '--ink-file', '-', '--place-text', '1',
                    '30000', '40000', '   '], /placed text requires/],
                [['notebook', '--flow-text-file', '-', '--fallback-glyph',
                    'ab', 'a'], /fallback requires two distinct exact/],
                [['notebook', '--glyph-file', '-', '--fallback-glyph',
                    'a', 'a'], /fallback requires two distinct exact/],
                [['notebook', '--flow-text-file', '-', '--join-text-pair',
                    'ab', 'a'], /text join pair needs two visible exact/],
                [['notebook', '--ink-file', '-', '--join-text-anchors',
                    'a', 'b', '0', '1'], /join exit stroke requires 1\.\.4096/],
                [['notebook', '--glyph-file', '-', '--join-text-anchors',
                    'a', 'b', '1', '4097'],
                    /join entry stroke requires 1\.\.4096/],
                [['notebook', '--glyph-file', '-', '--join-text-pair',
                    'a', 'b', '--join-text-pair', 'a', 'b'],
                    /duplicate text join pair refused/],
                [['notebook', '--glyph-file', '-', '--glyph-variation',
                    '42', '0', '0', '0', '0', '0', '0',
                    '--glyph-variation', '42', '0', '0', '0', '0', '0', '0'],
                    /--glyph-variation may appear only once/],
                // Inline flow is validated before opening earlier inputs.
                [['notebook', '--glyph-file', '-', '--flow-text', ''],
                    /flow text needs 1\.\.4096 UTF-8 bytes/],
                [['notebook', '--ink-file', '-', '--flow-markdown', '  \n'],
                    /flow text needs 1\.\.4096 UTF-8 bytes/],
                [['notebook', '--ink-file', '-', '--flow-markdown',
                    'a\v'], /flow text needs 1\.\.4096 UTF-8 bytes/],
                [['notebook', '--ink-file', '-', '--flow-text',
                    'a'.repeat(4097)],
                    /flow text needs 1\.\.4096 UTF-8 bytes/],
                [['notebook', '--glyph-file', '-', '--flow-text',
                    'a\t'.repeat(1025)],
                    /expanded flow text exceeds 4096 UTF-8 bytes/],
                [['notebook', '--ink-file', '-', '--date-style', 'other',
                    '--export-paper', 'pdf'],
                    /date style must be none, line or box/],
                [['notebook', '--glyph-file', '-', '--date-placement',
                    'bottom', '--export-paper', 'pdf'],
                    /date placement must be none, top-left or top-right/],
                [['notebook', '--flow-text-file', '-', '--border-radius-um',
                    '-1', '--export-paper', 'pdf'],
                    /border radius requires unsigned integer micrometres/],
                [['notebook', '--ink-file', '-', '--border-radius-um',
                    '4294967296', '--export-paper', 'pdf'],
                    /invalid border radius/],
                [['notebook', '--flow-markdown-file', '-', '--page-numbers',
                    'yes', '--export-paper', 'pdf'],
                    /--page-numbers requires true or false/],
                [['notebook', '--ink-file', '-', '--printed-sheet', '1',
                    '--export-paper', 'pdf'],
                    /--printed-sheet requires true or false/],
                [['notebook', '--flow-markdown-file', '-', '--heading-style',
                    'marker', '--heading-style', 'underline',
                    '--export-paper', 'pdf'],
                    /--heading-style may appear only once/],
                [['notebook', '--flow-markdown-file', '-', '--heading-color',
                    '#123456', '--heading-color', '#654321',
                    '--export-paper', 'pdf'],
                    /--heading-color may appear only once/],
                [['notebook', '--glyph-file', '-', '--heading-ink-color',
                    '#123456', '--heading-ink-color', '#654321',
                    '--export-paper', 'pdf'],
                    /--heading-ink-color may appear only once/],
                [['notebook', '--ink-file', '-', '--heading-marker-scale',
                    '75', '--heading-marker-scale', '95',
                    '--export-paper', 'pdf'],
                    /--heading-marker-scale repeated/],
                [['notebook', '--ink-file', '-', '--heading-ink-scale',
                    '150', '--heading-ink-scale', '175',
                    '--export-paper', 'pdf'],
                    /--heading-ink-scale repeated/],
                [['notebook', '--flow-markdown-file', '-'],
                    /flow source needs --export-paper pdf or png/],
                [['notebook', '--ink-file', '-', '--flow-region',
                    '20000', '30000', '40000', '50000',
                    '--export-paper', 'pdf'],
                    /--flow-region requires a text or Markdown flow/],
                [['notebook', '--flow-text-file', '-', '--heading-style',
                    'marker', '--export-paper', 'pdf'],
                    /heading customization requires --flow-markdown/],
                [['notebook', '--flow-markdown-file', '-',
                    '--heading-marker-scale', '90', '--export-paper', 'pdf'],
                    /--heading-marker-scale requires --heading-style marker/],
                [['notebook', '--flow-markdown-file', '-',
                    '--heading-style', 'none', '--heading-color', '#112233',
                    '--export-paper', 'pdf'],
                    /--heading-color cannot accompany --heading-style none/],
                [['notebook', '--glyph-file', '-', '--heading-ink-scale',
                    '99', '--export-paper', 'pdf'],
                    /heading ink scale must be 100..250/],
                [['notebook', '--flow-markdown-file', '-', '--heading-color',
                    '#GGGGGG', '--export-paper', 'pdf'],
                    /--heading-color requires #RRGGBB/],
            ]) {
                const args = [...flags, '--output', destination,
                    '--replace'];
                const result = await new Promise((resolve, reject) => {
                    const child = spawn(executable, args, {
                        cwd: root, stdio: ['pipe', 'pipe', 'pipe'],
                    });
                    let stderr = '';
                    let stdout = '';
                    child.stderr.setEncoding('utf8');
                    child.stdout.setEncoding('utf8');
                    child.stderr.on('data', (chunk) => { stderr += chunk; });
                    child.stdout.on('data', (chunk) => { stdout += chunk; });
                    const timeout = setTimeout(() => {
                        child.kill();
                        reject(new Error(
                            'invalid CLI source blocked on open stdin'));
                    }, 5_000);
                    child.on('error', (error) => {
                        clearTimeout(timeout);
                        reject(error);
                    });
                    child.on('close', (status) => {
                        clearTimeout(timeout);
                        resolve({ status, stderr, stdout });
                    });
                    // Do not close or write to child.stdin before it exits.
                });
                assert.notEqual(result.status, 0);
                assert.match(result.stderr, expected);
                assert.equal(result.stdout, '');
                assert.deepEqual(readFileSync(destination), previous,
                    'refused source leaves accepted output unchanged');
            }
            refused(['notebook', '--ink-file', saturated, '--ink-file',
                join(dir, 'missing.tsv'), '--emit-ink-file',
                '--output', destination, '--replace'],
            /ink exceeds 4096 explicit/, destination);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('missing CLI operands refuse before an open stdin source',
    { skip: process.platform !== 'linux' }, async () => {
        const dir = mkdtempSync(join(root, '.temp/file-arity-preflight-'));
        try {
            accepted('notebook');
            const executable = join(root,
                '.cache/cargo-target/debug/atrament-profiles');
            const destination = join(dir, 'reviewed.pdf');
            writeFileSync(destination, 'previous reviewed notebook PDF');
            const original = readFileSync(destination);
            const notebook = ['notebook', '--output', destination,
                '--replace', '--ink-file', '-'];
            const glyphs = ['glyphs', '--output', destination,
                '--replace', '--profile', '-'];
            const cases = [
                ...[
                    ['--glyph-file', /--glyph-file requires a library path/],
                    ['--ink-file', /--ink-file requires a file/],
                    ['--profile', /--profile requires a file/],
                    ['--paper-profile', /--paper-profile requires a file/],
                    ['--flow-text-file', /flow needs a UTF-8 text value/],
                    ['--flow-markdown-file', /flow needs a UTF-8 text value/],
                ].flatMap(([option, expected]) => [
                    [[...notebook, option], expected],
                    [[...notebook, option, ''], expected],
                ]),
                ...[
                    [['--audit-text'], /--audit-text requires UTF-8 text/],
                    [['--inspect-glyph'], /inspection requires an exact/],
                    [['--inspect-paths'], /inspection requires an exact/],
                    [['--remove-glyph'], /--remove-glyph requires an exact/],
                    [['--copy-glyph', 'a'],
                        /--copy-glyph requires 2 operands/],
                    [['--rotate-glyph', 'a', '10', '200'],
                        /--rotate-glyph requires 4 operands/],
                    [['--move-stroke', 'a', '1'],
                        /--move-stroke requires 3 operands/],
                    [['--translate-path', 'a', '1', '0'],
                        /--translate-path requires 4 operands/],
                    [['--set-stroke', 'a', '1', '500', '500',
                        '1800', '3000', '250'],
                        /--set-stroke requires 8 operands/],
                    [['--glyph-stroke', 'a', '500', '500',
                        '1800', '3000', '250'],
                        /--glyph-stroke requires 7 operands/],
                    [['--scale-glyph-ink', 'a'],
                        /--scale-glyph-ink requires 2 operands/],
                    [['--glyph-stroke', 'a', '500', '500',
                        '500', '500', '250', '#123456'],
                        /glyph stroke must have two distinct points/],
                    [['--glyph-stroke', 'a', '500', '500',
                        '1800', '3000', '79', '#123456'],
                        /glyph ink width requires 80\.\.1500/],
                    [['--glyph-stroke', 'a', '500', '500',
                        '1800', '3000', '250', '#GGGGGG'],
                        /glyph color requires #RRGGBB/],
                    [['--set-stroke', 'a', '1', '500', '500',
                        '500', '500', '250', '#123456'],
                        /glyph stroke must have two distinct points/],
                    [['--move-stroke', 'a', '1', '5000'],
                        /stroke index requires 1\.\.4096/],
                    [['--reverse-path', 'a', '0'],
                        /path index requires 1\.\.4096/],
                    [['--translate-path', 'a', '1', '5000', '0'],
                        /path dx outside native bounds/],
                    [['--translate-path', 'a', '1', '0', '0'],
                        /path shift needs nonzero bounded whole um/],
                    [['--rotate-glyph', 'a', '21', '1750', '2250'],
                        /rotation angle outside native bounds/],
                    [['--translate-glyph', 'a', '-3501', '0'],
                        /glyph dx outside native bounds/],
                    [['--shear-glyph', 'a', '21', '2250'],
                        /italic angle requires 0\.\.20/],
                    [['--scale-glyph-height', 'a', '151', '2250'],
                        /glyph height percent requires 50\.\.150/],
                    [['--scale-glyph-width', 'a', '100', '3501'],
                        /glyph width anchor requires 0\.\.3500/],
                    [['--scale-glyph-ink', 'a', '49'],
                        /glyph ink percent requires 50\.\.150/],
                    [['--copy-glyph', 'a', 'a'],
                        /glyph copy needs two distinct exact/],
                    [['--copy-glyph', 'a', 'ab'],
                        /glyph copy needs two distinct exact/],
                    [['--inspect-glyph', 'ab'],
                        /inspection requires one exact visible grapheme/],
                    [['--inspect-paths', ''],
                        /inspection requires one exact visible grapheme/],
                    [['--remove-glyph', 'ab'],
                        /glyph removal requires one exact visible grapheme/],
                    [['--remove-glyph', ' '],
                        /glyph removal requires one exact visible grapheme/],
                    [['--audit-text', ''],
                        /glyph audit needs 1\.\.4096 UTF-8 bytes/],
                    [['--audit-text', '  \n'],
                        /glyph audit needs 1\.\.4096 UTF-8 bytes/],
                    [['--audit-text', 'a\v'],
                        /glyph audit needs 1\.\.4096 UTF-8 bytes/],
                    [['--audit-text', 'ñ'.repeat(2049)],
                        /glyph audit needs 1\.\.4096 UTF-8 bytes/],
                ].map(([tail, expected]) => [[...glyphs, ...tail], expected]),
            ];
            for (const [args, expected] of cases) {
                const result = await new Promise((resolve, reject) => {
                    const child = spawn(executable, args, {
                        cwd: root, stdio: ['pipe', 'pipe', 'pipe'],
                    });
                    let stderr = '';
                    let stdout = '';
                    child.stderr.setEncoding('utf8');
                    child.stdout.setEncoding('utf8');
                    child.stderr.on('data', (chunk) => {
                        stderr += chunk;
                    });
                    child.stdout.on('data', (chunk) => {
                        stdout += chunk;
                    });
                    const timeout = setTimeout(() => {
                        child.kill();
                        reject(new Error(
                            'missing source blocked on stdin'));
                    }, 5_000);
                    child.on('error', (error) => {
                        clearTimeout(timeout);
                        reject(error);
                    });
                    child.on('close', (status) => {
                        clearTimeout(timeout);
                        resolve({ status, stderr, stdout });
                    });
                    // Keep stdin open: an actual source read would hang.
                });
                assert.notEqual(result.status, 0, args.join(' '));
                assert.match(result.stderr, expected, args.join(' '));
                assert.equal(result.stdout, '', args.join(' '));
                assert.deepEqual(readFileSync(destination), original,
                    'refused source cannot replace the reviewed PDF');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compiled ink stdin replays sparse pages and rejects unsafe pipes',
    () => {
        const dir = mkdtempSync(join(root, '.temp/page-ink-pipe-'));
        try {
            const saved = join(dir, 'compiled.tsv');
            const destination = join(dir, 'accepted.pdf');
            const notebook = ['notebook', '--printed-sheet', 'true',
                '--page-numbers', 'true', '--date-style', 'line',
                '--date-placement', 'top-left', '--date-text', '2026-11-10'];
            const ink = ['--glyph-stroke', 'a', '500', '500', '1800',
                '3000', '250', '#142536', '--glyph-variation', '19',
                '8', '7', '9', '4', '60', '65'];
            const content = [...ink, '--flow-markdown', '# aa\n\n- aa',
                '--heading-style', 'marker', '--heading-color', '#F8DE91',
                '--heading-ink-scale', '180', '--place-text', '32',
                '100000', '50000', 'aa', '--ink-line', '31',
                '70000', '40000', '71000', '41000', '250', '#abcdef'];
            const physical = accepted(...notebook, ...content,
                '--emit-ink-file');
            assert.ok(physical.length > 100);
            writeFileSync(saved, physical);
            const pipe = (input, ...flags) => {
                const result = profilesInput(input, 'notebook', ...notebook
                    .slice(1), '--ink-file', '-', ...flags);
                assert.equal(result.status, 0, result.stderr.toString());
                return result.stdout;
            };
            const emittedAgain = pipe(physical, '--emit-ink-file');
            assert.deepEqual(emittedAgain, physical,
                'piped canonical TSV retains exact segment order and bytes');
            const pdf = accepted(...notebook, ...content,
                '--export-paper', 'pdf');
            assert.match(pdf.toString('latin1'), /\/Count 32\b/);
            assert.deepEqual(pipe(physical, '--export-paper', 'pdf'), pdf);
            assert.deepEqual(accepted(...notebook, '--ink-file', saved,
                '--export-paper', 'pdf'), pdf);
            for (const page of [1, 31, 32]) {
                const png = ['--export-paper', 'png', '--dpi', '72',
                    '--png-page', String(page)];
                assert.deepEqual(pipe(physical, ...png),
                    accepted(...notebook, ...content, ...png),
                    `pipe page ${page} is actual original native PNG`);
            }
            writeFileSync(destination, pdf);
            const refusedPipe = (input, flags, expected) => {
                const before = readFileSync(destination);
                const result = profilesInput(input, 'notebook',
                    '--ink-file', '-', ...flags, '--output', destination,
                    '--replace');
                assert.notEqual(result.status, 0, flags.join(' '));
                assert.match(result.stderr.toString('utf8'), expected);
                assert.equal(result.stdout.length, 0,
                    'invalid pipe cannot emit partial output');
                assert.deepEqual(readFileSync(destination), before,
                    'invalid pipe cannot replace native PDF');
            };
            refusedPipe(Buffer.alloc(262_145, 0x61),
                ['--export-paper', 'pdf'], /ink stdin exceeds 262144/);
            refusedPipe(Buffer.from([0xff, 0xfe, 0xfd]),
                ['--export-paper', 'pdf'], /ink stdin must be UTF-8/);
            refusedPipe(Buffer.from('atrament.page-ink.v1\n'),
                ['--export-paper', 'pdf'], /at least one physical segment/);
            refusedPipe(Buffer.from('invalid.v1\n'),
                ['--export-paper', 'pdf'], /ink file needs.*header/);
            refusedPipe(physical,
                ['--ink-file', '-', '--export-paper', 'pdf'],
                /standard input may be selected only once/);
            refusedPipe(physical,
                ['--flow-markdown-file', '-', ...ink,
                    '--export-paper', 'pdf'],
                /standard input may be selected only once/);
            // Reverse argument order: flow claims the same pipe first.
            const reverse = profilesInput(Buffer.from('a'), 'notebook',
                ...ink, '--flow-text-file', '-', '--ink-file', '-',
                '--export-paper', 'pdf', '--output', destination,
                '--replace');
            assert.notEqual(reverse.status, 0);
            assert.match(reverse.stderr.toString(),
                /standard input may be selected only once/);
            assert.deepEqual(readFileSync(destination), pdf);
            assert.deepEqual(readFileSync(saved), physical,
                'saved physical source remains untouched by pipe replay');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('page ink output rejects incompatible modes and unprintable segments',
    { skip: process.platform !== 'linux' }, () => {
        const dir = mkdtempSync(join(root, '.temp/page-ink-output-refusal-'));
        try {
            const target = join(dir, 'accepted.tsv');
            const ink = ['--ink-line', '1', '70000', '40000',
                '71000', '41000', '250', '#123456'];
            accepted('notebook', ...ink, '--emit-ink-file',
                '--output', target);
            const check = (arguments_, expected) => refused([
                'notebook', ...arguments_, '--output', target, '--replace',
            ], expected, target);
            check(['--emit-ink-file'], /at least one physical segment/);
            check([...ink, '--emit-ink-file', '--emit-ink-file'],
                /may appear only once/);
            check([...ink, '--emit-ink-file', '--export-paper', 'pdf'],
                /cannot accompany --emit-ink-file/);
            check([...ink, '--emit-ink-file', '--pages', '2'],
                /--pages requires --export-paper pdf/);
            check([...ink, '--emit-ink-file', '--png-page', '1'],
                /require --export-paper png/);
            check([...ink, '--emit-ink-file', '--quality', 'preview'],
                /require --export-paper png/);
            check([...ink, '--emit-ink-file', '--inspect'],
                /--inspect is stdout only/);
            check(['--ink-line', '1', '0', '40000', '71000', '41000',
                '250', '#123456', '--emit-ink-file'],
            /page 1 ink needs printable endpoints/);
            check(['--glyph-stroke', 'a', '500', '500', '1800',
                '3000', '250', '#123456', '--flow-text', 'ab',
                '--emit-ink-file'], /missing authored glyph shapes/);
            check(['--ink-line', '2', '70000', '40000', '71000',
                '41000', '250', '#123456', '--emit-ink-file',
                '--export-paper', 'png'], /cannot accompany --emit-ink-file/);
            const link = join(dir, 'linked.tsv');
            symlinkSync(target, link);
            refused(
                ['notebook', ...ink, '--emit-ink-file', '--output', link,
                    '--replace'],
                /InvalidExistingTarget/, target,
            );
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
