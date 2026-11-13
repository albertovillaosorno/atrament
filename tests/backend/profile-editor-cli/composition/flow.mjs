// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
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
const cargo = join(root, '.dependencies/rust/1.99.1/bin/cargo');
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
    flowFlag = '--flow-markdown', pngPage = 1, input = undefined) {
    const output = join(dir, `flow-${format}.output`);
    const heading = flowFlag.startsWith('--flow-markdown')
        && !extra.includes('--heading-style') ? headingOptions : [];
    const args = [...base, ...heading, ...extra, flowFlag, source,
        '--export-paper', format, '--output', output, '--replace'];
    if (format === 'png') {
        args.push('--png-page', String(pngPage), '--dpi', '72');
    }
    const result = spawnSync(cargo, [
        'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
        ...args,
    ], { cwd: root, encoding: 'utf8', timeout: 120000, input });
    assert.equal(result.error, undefined, result.error?.message);
    return { result, output };
}

function coloredHeadingWidths(bytes) {
    return [...bytes.toString('latin1').matchAll(
        /0\.973 0\.871 0\.569 RG\n([0-9.]+) w/g,
    )].map((match) => match[1]);
}

test('compressed numbered label refuses overlap without clobbering PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-number-clearance-'));
        try {
            const profile = join(dir, 'compressed-marker.json');
            const options = ['handwriting'];
            for (const glyph of ['1', '.']) {
                options.push('--set-glyph', glyph, 'width',
                    ...Array(4).fill('12000'), '0');
                options.push('--set-glyph', glyph, 'letter_spacing',
                    ...Array(4).fill('-10000'), '0');
            }
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
                ...options,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(profile, configured.stdout);
            const extra = [
                '--handwriting-profile', profile,
                '--flow-region', '30000', '40000', '35000', '20000',
                '--glyph-stroke', '1', '500', '500', '3200', '3000',
                '250', '#123456',
                '--glyph-stroke', '.', '500', '500', '3200', '3000',
                '250', '#123456',
            ];
            for (const format of ['pdf', 'png']) {
                const safe = render(dir, '1.    a', format, extra);
                assert.equal(safe.result.status, 0, safe.result.stderr);
                const accepted = readFileSync(safe.output);
                const collision = render(dir, '1. a', format, extra);
                assert.notEqual(collision.result.status, 0);
                assert.match(collision.result.stderr,
                    /ordered list marker ink overlaps first item glyph/);
                assert.deepEqual(readFileSync(collision.output), accepted,
                    `refused ${format} must preserve previously rendered ink`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('mixed-width numeral ink and collision refusal match native PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-mixed-numeral-'));
        try {
            const profile = join(dir, 'mixed-marker.json');
            const edits = ['handwriting'];
            for (const [glyph, width] of [
                ['2', 13000], ['1', 12000], ['.', 12000],
            ]) {
                edits.push('--set-glyph', glyph, 'width',
                    ...Array(4).fill(String(width)), '0');
                edits.push('--set-glyph', glyph, 'letter_spacing',
                    ...Array(4).fill('-10000'), '0');
            }
            const configured = spawnSync(cargo, [
                'run', '--offline', '--quiet',
                '-p', 'atrament_profile_editor_cli', '--', ...edits,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(profile, configured.stdout);
            const numeralInk = ['2', '1', '.'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '500', '500', '3200', '3000',
                '250', '#123456',
            ]);
            const shared = ['--handwriting-profile', profile, ...numeralInk];
            const region = ['--flow-region', '30000', '40000',
                '35000', '20000'];
            for (const format of ['pdf', 'png']) {
                const acceptedFlow = render(dir, '21.   a', format,
                    [...shared, ...region]);
                assert.equal(acceptedFlow.result.status, 0,
                    acceptedFlow.result.stderr);
                const expected = readFileSync(acceptedFlow.output);
                const manuallyPlaced = join(dir, `manual-${format}`);
                const manual = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--', ...base,
                    ...shared, '--place-text', '1', '38500', '42500',
                    '21.   a', '--export-paper', format,
                    ...(format === 'png' ? ['--dpi', '72'] : []),
                    '--output', manuallyPlaced,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(manual.status, 0, manual.stderr);
                assert.deepEqual(expected, readFileSync(manuallyPlaced),
                    `mixed numeral and spaces retain authored ${format} ink`);
                const rejected = render(dir, '21. a', format,
                    [...shared, ...region]);
                assert.notEqual(rejected.result.status, 0);
                assert.match(rejected.result.stderr,
                    /ordered list marker ink overlaps first item glyph/);
                assert.deepEqual(readFileSync(rejected.output), expected,
                    `${format} collision cannot replace accepted output`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compressed adjacent words paginate without PDF/PNG ink collision',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-word-boundary-'));
        try {
            const profile = join(dir, 'compressed-a.json');
            const config = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', 'handwriting', '--set-glyph', 'a', 'width',
                ...Array(4).fill('12000'), '0',
                '--set-glyph', 'a', 'letter_spacing',
                ...Array(4).fill('-10000'), '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(config.status, 0, config.stderr);
            writeFileSync(profile, config.stdout);
            const selected = [
                '--handwriting-profile', profile,
                '--flow-region', '30000', '40000', '50000', '14000',
            ];
            for (const flowFlag of ['--flow-text', '--flow-markdown']) {
                for (const format of ['pdf', 'png']) {
                    for (const page of format === 'pdf' ? [1] : [1, 2]) {
                        const rendered = render(dir, 'a a', format,
                            selected, flowFlag, page);
                        assert.equal(rendered.result.status, 0,
                            rendered.result.stderr);
                        const accepted = readFileSync(rendered.output);
                        const reference = join(dir,
                            `manual-${flowFlag}-${format}-${page}`);
                        const manuallyPlaced = spawnSync(cargo, [
                            'run', '--quiet', '-p',
                            'atrament_profile_editor_cli',
                            '--', ...base, '--handwriting-profile', profile,
                            '--place-text', '1', '32500', '42500', 'a',
                            '--place-text', '2', '32500', '42500', 'a',
                            ...(format === 'pdf'
                                ? ['--pages', '2']
                                : ['--png-page', String(page), '--dpi', '72']),
                            '--export-paper', format, '--output', reference,
                        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                        assert.equal(manuallyPlaced.status, 0,
                            manuallyPlaced.stderr);
                        assert.deepEqual(accepted, readFileSync(reference),
                            `${flowFlag} ${format} page ${page} word boundary`);

                        // A later missing glyph must not replace the
                        // previously accepted page (including page one).
                        const refused = render(dir, 'a a β', format,
                            selected, flowFlag, page);
                        assert.notEqual(refused.result.status, 0);
                        assert.deepEqual(
                            readFileSync(refused.output), accepted);
                    }
                    // Four authored spaces clear the 12-mm first contour
                    // and keep both words on the same physical sheet.
                    const spaced = render(dir, 'a    a', format,
                        selected, flowFlag);
                    assert.equal(spaced.result.status, 0,
                        spaced.result.stderr);
                    assert.notDeepEqual(readFileSync(spaced.output),
                        readFileSync(join(dir,
                            `manual-${flowFlag}-${format}-1`)));
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compressed word contours keep whole word on the next PDF/PNG page',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-word-contour-'));
        try {
            const profile = join(dir, 'compressed-accent.json');
            const config = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', 'handwriting', '--set-glyph', 'ó', 'width',
                ...Array(4).fill('12000'), '0',
                '--set-glyph', 'ó', 'letter_spacing',
                ...Array(4).fill('-10000'), '0',
                '--set-glyph', 'o\u0301', 'width',
                ...Array(4).fill('12000'), '0',
                '--set-glyph', 'o\u0301', 'letter_spacing',
                ...Array(4).fill('-10000'), '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(config.status, 0, config.stderr);
            writeFileSync(profile, config.stdout);
            const selected = [
                '--handwriting-profile', profile,
                '--flow-region', '30000', '40000', '24800', '14000',
            ];
            // Two accents have 4 mm of summed advance but 14.8 mm of
            // contour clearance. The entire word fits the next sheet,
            // not after the first word on the current sheet.
            for (const [label, vowel] of [
                ['NFC', 'ó'], ['NFD', 'o\u0301'],
            ]) {
                for (const format of ['pdf', 'png']) {
                    for (const page of format === 'pdf' ? [1] : [1, 2]) {
                        const exportOptions = format === 'pdf'
                            ? ['--pages', '2'] : [];
                        const source = `a ${vowel}${vowel}`;
                        const rendered = render(dir, source, format,
                            [...selected, ...exportOptions],
                            '--flow-markdown', page);
                        assert.equal(rendered.result.status, 0,
                            rendered.result.stderr);
                        const accepted = readFileSync(rendered.output);
                        const reference = join(dir,
                            `word-${label}-${page}.${format}`);
                        const manual = spawnSync(cargo, [
                            'run', '--quiet', '-p',
                            'atrament_profile_editor_cli',
                            '--', ...base, '--handwriting-profile', profile,
                            '--place-text', '1', '32500', '42500', 'a',
                            '--place-text', '2', '32500', '42500',
                            `${vowel}${vowel}`,
                            ...exportOptions, '--export-paper', format,
                            ...(format === 'png'
                                ? ['--png-page', String(page), '--dpi', '72']
                                : []),
                            '--output', reference,
                        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                        assert.equal(manual.status, 0, manual.stderr);
                        assert.deepEqual(accepted, readFileSync(reference),
                            `${label} page ${page} ${format} word wrap`);
                        const refused = render(dir, `${source} β`, format,
                            [...selected, ...exportOptions],
                            '--flow-markdown', page);
                        assert.notEqual(refused.result.status, 0);
                        assert.deepEqual(readFileSync(refused.output), accepted,
                            `${label} missing ink preserves ${format}`);
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('ordered continuation checks post-wrap glyph width in PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-ordered-width-'));
        try {
            const statistics = join(dir, 'wide-accent.json');
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', '--set-glyph', 'ó', 'width',
                '9000', '9000', '9000', '9000', '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(statistics, configured.stdout);
            const ink = [
                '--handwriting-profile', statistics,
                '--glyph-stroke', '1', '600', '500', '1800', '3000',
                '250', '#123456',
                '--glyph-stroke', '.', '600', '500', '1800', '3000',
                '250', '#123456',
            ];
            const region = (width) => [
                '--flow-region', '30000', '40000', String(width), '40000',
            ];
            for (const format of ['pdf', 'png']) {
                const admitted = render(dir, '1. aó', format,
                    [...ink, ...region(29000)]);
                assert.equal(admitted.result.status, 0,
                    admitted.result.stderr);
                const bytes = readFileSync(admitted.output);

                const reference = join(dir, `ordered-manual.${format}`);
                const args = [
                    ...base, ...ink,
                    '--place-text', '1', '38500', '42500', '1. a',
                    '--place-text', '1', '47600', '51500', 'ó',
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', '1', '--dpi', '72'] : []),
                    '--output', reference,
                ];
                const manual = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...args,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(manual.status, 0, manual.stderr);
                assert.deepEqual(bytes, readFileSync(reference),
                    `fitting ordered row matches physical ${format} ink`);

                for (const source of ['1. aó', '1. a ó']) {
                    const refused = render(dir, source, format,
                        [...ink, ...region(28000)]);
                    assert.notEqual(refused.result.status, 0,
                        'narrow continuation cannot place a wide glyph');
                    assert.match(refused.result.stderr,
                        /glyph exceeds horizontal printable region/);
                    assert.deepEqual(readFileSync(refused.output), bytes,
                        `refused ${format} must retain accepted bytes`);
                }
            }

            // The same narrow hanging indent must survive a physical page
            // break, not just a second row of the first sheet. Compare each
            // page with independently placed exact authored pen strokes.
            const oneRowRegion = (width) => [
                '--flow-region', '30000', '40000', String(width), '14000',
            ];
            const runs = [
                '--place-text', '1', '38500', '42500', '1. a',
                '--place-text', '2', '47600', '42500', 'ó',
            ];
            const manual = (format, page) => {
                const output = join(dir, `ordered-page-${format}-${page}`);
                const args = [...base, ...ink, ...runs,
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format, '--output', output];
                if (format === 'png') {
                    args.push('--png-page', String(page), '--dpi', '72');
                }
                const result = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...args,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(result.status, 0, result.stderr);
                return readFileSync(output);
            };
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const selected = render(dir, '1. aó', format,
                        [...ink, ...oneRowRegion(29000),
                            ...(format === 'pdf' ? ['--pages', '2'] : [])],
                        '--flow-markdown', page);
                    assert.equal(selected.result.status, 0,
                        selected.result.stderr);
                    const bytes = readFileSync(selected.output);
                    assert.deepEqual(bytes, manual(format, page),
                        `ordered page ${page} ${format} ink parity`);
                    if (format === 'pdf') {
                        assert.match(bytes.toString('latin1'), /\/Count 2/);
                        for (const options of [
                            [...oneRowRegion(29000), '--pages', '1'],
                            [...oneRowRegion(28000), '--pages', '2'],
                        ]) {
                            const refused = render(dir, '1. aó', format,
                                [...ink, ...options]);
                            assert.notEqual(refused.result.status, 0);
                            assert.deepEqual(readFileSync(refused.output),
                                bytes, 'refusal must preserve PDF');
                        }
                    } else if (page === 2) {
                        const refused = render(dir, '1. aó', format,
                            [...ink, ...oneRowRegion(29000)],
                            '--flow-markdown', 3);
                        assert.notEqual(refused.result.status, 0);
                        assert.deepEqual(readFileSync(refused.output),
                            bytes, 'missing continuation page retains PNG');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compressed ordered marker contours refuse before replacing PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-compressed-'));
        try {
            const ink = ['1', '.'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '600', '500', '1800', '3000',
                '250', '#123456',
            ]);
            const region = (width) => [
                '--flow-region', '30000', '40000', String(width), '20000',
            ];
            for (const glyph of ['1', '.']) {
                const profile = join(dir, `compressed-${glyph}.json`);
                const configure = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', 'handwriting',
                    '--set-glyph', glyph, 'width',
                    ...Array(4).fill('12000'), '0',
                    '--set-glyph', glyph, 'letter_spacing',
                    ...Array(4).fill('-10000'), '0',
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(configure.status, 0, configure.stderr);
                writeFileSync(profile, configure.stdout);
                const extras = ['--handwriting-profile', profile, ...ink];
                for (const format of ['pdf', 'png']) {
                    // Extra authored spaces keep marker ink clear of the item.
                    const accepted = render(dir, '1.   a', format,
                        [...extras, ...region(35000)]);
                    assert.equal(accepted.result.status, 0,
                        accepted.result.stderr);
                    const bytes = readFileSync(accepted.output);
                    const reference = join(dir, `manual-${glyph}.${format}`);
                    const args = [
                        ...base, ...extras,
                        '--place-text', '1', '38500', '42500', '1.   a',
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', '1', '--dpi', '72'] : []),
                        '--output', reference,
                    ];
                    const manual = spawnSync(cargo, [
                        'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                        '--', ...args,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(manual.status, 0, manual.stderr);
                    assert.deepEqual(bytes, readFileSync(reference),
                        `compressed ${glyph} ink is physically placed`);
                    const refused = render(dir, '1. a', format,
                        [...extras, ...region(22000)]);
                    assert.notEqual(refused.result.status, 0);
                    assert.match(refused.result.stderr,
                        /ordered list label and first glyph exceed/);
                    assert.deepEqual(readFileSync(refused.output), bytes,
                        `narrow ${glyph} refusal preserves ${format} output`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compressed heading ink keeps underline aligned in native PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-heading-contour-'));
        try {
            const profile = join(dir, 'compressed-heading.json');
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', 'handwriting',
                '--set-glyph', 'a', 'width',
                ...Array(4).fill('12000'), '0',
                '--set-glyph', 'a', 'letter_spacing',
                ...Array(4).fill('-10000'), '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(profile, configured.stdout);
            const selected = ['--handwriting-profile', profile,
                '--heading-style', 'underline'];
            const region = (width) => [
                '--flow-region', '30000', '40000', String(width), '20000',
            ];
            for (const format of ['pdf', 'png']) {
                const accepted = render(dir, '# aa', format,
                    [...selected, ...region(26000)]);
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const bytes = readFileSync(accepted.output);
                const manualOutput = join(dir, `heading-manual.${format}`);
                const reference = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...base, '--handwriting-profile', profile,
                    '--place-text', '1', '32500', '42500', 'aa',
                    '--ink-line', '1', '32500', '49200',
                    '46500', '49200', '550', '#1E304A',
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', '1', '--dpi', '72'] : []),
                    '--output', manualOutput,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(reference.status, 0, reference.stderr);
                assert.deepEqual(bytes, readFileSync(manualOutput),
                    `${format} underline reaches last glyph contour`);
                const refused = render(dir, '# aa', format,
                    [...selected, ...region(18000)]);
                assert.notEqual(refused.result.status, 0);
                assert.match(refused.result.stderr,
                    /heading exceeds one-line fixed region/);
                assert.deepEqual(readFileSync(refused.output), bytes,
                    `${format} tight-region refusal keeps accepted ink`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compressed multiword headings keep colored ornaments in PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-heading-styles-'));
        try {
            const profile = join(dir, 'compressed-heading.json');
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', 'handwriting',
                '--set-glyph', 'a', 'width',
                ...Array(4).fill('12000'), '0',
                '--set-glyph', 'a', 'letter_spacing',
                ...Array(4).fill('-10000'), '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(profile, configured.stdout);
            const color = '#D8E69A';
            const region = (width) => [
                '--flow-region', '30000', '40000', String(width), '30000',
            ];
            for (const style of ['marker', 'double-underline']) {
                const options = [
                    '--handwriting-profile', profile,
                    '--heading-style', style, '--heading-color', color,
                ];
                const ornaments = (style === 'marker'
                    ? [900, 1900, 2900, 3900].map((lift) => [
                        '--ink-line', '1', '32500', String(42500 + lift),
                        '46500', String(42500 + lift), '1400', color,
                    ])
                    : [48300, 49200].map((y) => [
                        '--ink-line', '1', '32500', String(y),
                        '46500', String(y), '550', color,
                    ])).flat();
                for (const format of ['pdf', 'png']) {
                    const manualOutput = join(dir,
                        `manual-${style}.${format}`);
                    const manual = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, '--handwriting-profile', profile,
                        ...ornaments,
                        '--place-text', '1', '32500', '42500', 'aa ó',
                        '--place-text', '1', '32500', '51500', 'a',
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', '1', '--dpi', '72'] : []),
                        '--output', manualOutput,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(manual.status, 0, manual.stderr);
                    const expected = readFileSync(manualOutput);
                    for (const source of ['# aa ó\na', 'aa ó\n===\na']) {
                        const rendered = render(dir, source, format,
                            [...options, ...region(26000)]);
                        assert.equal(rendered.result.status, 0,
                            rendered.result.stderr);
                        assert.deepEqual(readFileSync(rendered.output),
                            expected, `${style} ${format}: ${source}`);
                    }
                    const refused = render(dir, '# aa ó\na', format,
                        [...options, ...region(18000)]);
                    assert.notEqual(refused.result.status, 0);
                    assert.match(refused.result.stderr,
                        /heading exceeds one-line fixed region/);
                    assert.deepEqual(readFileSync(refused.output), expected,
                        `${style} ${format} refusal preserves accepted ink`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compressed heading ornaments stay on the page with their body',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-heading-pages-'));
        try {
            const profile = join(dir, 'compressed-heading.json');
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', 'handwriting',
                '--set-glyph', 'a', 'width',
                ...Array(4).fill('12000'), '0',
                '--set-glyph', 'a', 'letter_spacing',
                ...Array(4).fill('-10000'), '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(profile, configured.stdout);
            const color = '#D8E69A';
            const region = (width) => [
                '--flow-region', '30000', '40000', String(width), '27000',
            ];
            const sources = ['a  \n# aa ó\na', 'a  \naa ó\n===\na'];
            for (const style of ['underline', 'double-underline',
                'marker']) {
                const options = [
                    '--handwriting-profile', profile,
                    '--heading-style', style, '--heading-color', color,
                ];
                const strokes = (style === 'marker'
                    ? [900, 1900, 2900, 3900].map((lift) => [
                        '--ink-line', '2', '32500', String(42500 + lift),
                        '46500', String(42500 + lift), '1400', color,
                    ])
                    : (style === 'underline' ? [49200] : [48300, 49200])
                        .map((y) => [
                            '--ink-line', '2', '32500', String(y),
                            '46500', String(y), '550', color,
                        ])).flat();
                for (const format of ['pdf', 'png']) {
                    for (const page of format === 'pdf' ? [1] : [1, 2]) {
                        const reference = join(dir,
                            `manual-${style}-${format}-${page}`);
                        const manual = spawnSync(cargo, [
                            'run', '--quiet', '-p',
                            'atrament_profile_editor_cli', '--',
                            ...base, '--handwriting-profile', profile,
                            ...strokes,
                            '--place-text', '1', '32500', '42500', 'a',
                            '--place-text', '2', '32500', '42500', 'aa ó',
                            '--place-text', '2', '32500', '51500', 'a',
                            ...(format === 'pdf' ? ['--pages', '2']
                                : ['--png-page', String(page), '--dpi', '72']),
                            '--export-paper', format, '--output', reference,
                        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                        assert.equal(manual.status, 0, manual.stderr);
                        const expected = readFileSync(reference);
                        for (const source of sources) {
                            const output = render(dir, source, format,
                                [...options, ...region(26000)],
                                '--flow-markdown', page);
                            assert.equal(output.result.status, 0,
                                output.result.stderr);
                            assert.deepEqual(readFileSync(output.output),
                                expected, `${style} ${format} page ${page}`);
                        }
                        if (format === 'pdf') {
                            assert.match(expected.toString('latin1'),
                                /\/Count 2/, 'heading moves to page two');
                            for (const invalid of [
                                [...region(26000), '--pages', '1'],
                                region(18000),
                            ]) {
                                const refused = render(dir, sources[0],
                                    format, [...options, ...invalid]);
                                assert.notEqual(refused.result.status, 0);
                                assert.deepEqual(readFileSync(refused.output),
                                    expected, 'refused PDF keeps both pages');
                            }
                        } else if (page === 2) {
                            for (const [width, selectedPage] of [
                                [18000, 2], [26000, 3],
                            ]) {
                                const refused = render(dir, sources[0],
                                    format, [...options, ...region(width)],
                                    '--flow-markdown', selectedPage);
                                assert.notEqual(refused.result.status, 0);
                                assert.deepEqual(readFileSync(refused.output),
                                    expected, 'refused PNG retains page two');
                            }
                        }
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('tabbed ordered marker keeps its measured hang on physical pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-ordered-tabs-'));
        try {
            const numerals = ['1', '.'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '600', '500', '1800', '3000',
                '250', '#123456',
            ]);
            const region = [
                '--flow-region', '30000', '40000', '32000', '14000',
            ];
            const options = [...numerals, ...region];
            const file = join(dir, 'ordered.md');
            const original = Buffer.from('\uFEFF1.\taó\r\n', 'utf8');
            writeFileSync(file, original);
            const manual = (format, page) => {
                const output = join(dir, `tab-manual-${format}-${page}`);
                const args = [
                    ...base, ...numerals,
                    '--place-text', '1', '38500', '42500', '1.  a',
                    '--place-text', '2', '54500', '42500', 'ó',
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72'] : []),
                    '--output', output,
                ];
                const result = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...args,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(result.status, 0, result.stderr);
                return readFileSync(output);
            };
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const extra = [
                        ...options,
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                    ];
                    const expected = manual(format, page);
                    for (const [source, flag] of [
                        ['1.  aó', '--flow-markdown'],
                        ['1.\taó', '--flow-markdown'],
                        [file, '--flow-markdown-file'],
                    ]) {
                        const { result, output } = render(
                            dir, source, format, extra, flag, page);
                        assert.equal(result.status, 0, result.stderr);
                        assert.deepEqual(readFileSync(output), expected,
                            `tab-expanded numeral and item ink: ${flag}`);
                    }
                    if (format === 'pdf') {
                        assert.match(expected.toString('latin1'),
                            /\/Count 2/);
                        for (const [changed, message] of [
                            [[...options, '--pages', '1'],
                                /requires 2 physical pages/],
                            [[...numerals, '--flow-region',
                                '30000', '40000', '28000', '14000'],
                                /label and first glyph exceed/],
                        ]) {
                            const refused = render(dir, '1.\taó', format,
                                changed);
                            assert.notEqual(refused.result.status, 0);
                            assert.match(refused.result.stderr, message);
                            assert.deepEqual(readFileSync(refused.output),
                                expected, 'invalid layout preserves PDF');
                        }
                    } else if (page === 2) {
                        const refused = render(dir, file, 'png', options,
                            '--flow-markdown-file', 3);
                        assert.notEqual(refused.result.status, 0);
                        assert.deepEqual(readFileSync(refused.output),
                            expected, 'missing page preserves PNG');
                    }
                }
            }
            assert.deepEqual(readFileSync(file), original,
                'BOM and CRLF source remain unchanged');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('soft continued ordered Markdown keeps native page-local ink', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-ordered-soft-'));
    try {
        const profile = join(dir, 'measured-label.json');
        const edits = [
            ['1', 5000], ['2', 5000], [')', 4000], ['ó', 9000],
        ].flatMap(([glyph, width]) => [
            '--set-glyph', glyph, 'width',
            ...Array(4).fill(String(width)), '0',
        ]);
        const edited = spawnSync(cargo, [
            'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
            'handwriting', ...edits,
        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
        assert.equal(edited.status, 0, edited.stderr);
        writeFileSync(profile, edited.stdout);
        const digits = ['1', '2', ')'].flatMap((glyph) => [
            '--glyph-stroke', glyph, '600', '500', '1800', '3000',
            '250', '#123456',
        ]);
        const region = (width) => [
            '--flow-region', '30000', '40000', String(width), '14000',
        ];
        const options = [
            '--handwriting-profile', profile, ...digits, ...region(42000),
        ];
        const source = '12) a\n  ó';
        const manual = (format, page) => {
            const output = join(dir, `soft-manual-${format}-${page}`);
            const args = [
                ...base, '--handwriting-profile', profile, ...digits,
                '--place-text', '1', '38500', '42500', '12) a',
                '--place-text', '2', '57550', '42500', 'ó',
                ...(format === 'pdf' ? ['--pages', '2'] : []),
                '--export-paper', format,
                ...(format === 'png'
                    ? ['--png-page', String(page), '--dpi', '72'] : []),
                '--output', output,
            ];
            const result = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', ...args,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(result.status, 0, result.stderr);
            return readFileSync(output);
        };
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const flowed = render(dir, source, format,
                    options, '--flow-markdown', page);
                assert.equal(flowed.result.status, 0, flowed.result.stderr);
                const accepted = readFileSync(flowed.output);
                assert.deepEqual(accepted, manual(format, page),
                    `soft continuation preserves physical ${format} page`);
                if (format === 'pdf') {
                    assert.match(accepted.toString('latin1'), /\/Count 2/);
                    const same = render(dir, '12) a ó', 'pdf', options);
                    assert.equal(same.result.status, 0, same.result.stderr);
                    assert.deepEqual(readFileSync(same.output), accepted,
                        'inline and source-line soft continuation agree');
                    const refusals = [
                        [[...options, '--pages', '1'],
                            /requires 2 physical pages/],
                        [['--handwriting-profile', profile, ...digits,
                            ...region(36000)],
                            /glyph exceeds horizontal printable region/],
                    ];
                    for (const [extra, diagnostic] of refusals) {
                        const refused = render(dir, source, 'pdf', extra);
                        assert.notEqual(refused.result.status, 0);
                        assert.match(refused.result.stderr, diagnostic);
                        assert.deepEqual(readFileSync(refused.output), accepted,
                            'refused layout preserves accepted vector PDF');
                    }
                } else if (page === 2) {
                    const missing = render(dir, source, 'png', options,
                        '--flow-markdown', 3);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        'a nonexistent page cannot replace accepted PNG');
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('two-digit ordered labels use statistical spacing across PDF pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-ordered-spacing-'));
        try {
            const statistics = join(dir, 'digit-spacing.json');
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', '--set-glyph', '1', 'letter_spacing',
                '9000', '9000', '9000', '9000', '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(statistics, configured.stdout);
            const numeralInk = ['1', '0', ')'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '600', '500', '1800', '3000',
                '250', '#123456',
            ]);
            const source = '10)  aó';
            // The 16.3-mm profiled label and two 4-mm spaces give a
            // 24.3-mm hanging indent. The 29-mm text cap holds only
            // the first item glyph. Default advances fit both.
            const region = [
                '--flow-region', '30000', '40000', '39000', '14000',
            ];
            const extras = [...numeralInk, ...region];
            const baseline = render(dir, source, 'pdf',
                [...extras, '--pages', '1']);
            assert.equal(baseline.result.status, 0,
                baseline.result.stderr);
            assert.match(readFileSync(baseline.output)
                .toString('latin1'), /\/Count 1/,
            'unmodified digit advances fit a single physical row');

            const scaled = [
                '--handwriting-profile', statistics, ...extras,
            ];
            const manuallyPlaced = (format, page) => {
                const output = join(dir, `spacing-manual-${format}-${page}`);
                const args = [...base, ...numeralInk,
                    '--handwriting-profile', statistics,
                    '--place-text', '1', '38500', '42500', '10)  a',
                    '--place-text', '2', '62800', '42500', 'ó',
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format, '--output', output];
                if (format === 'png') {
                    args.push('--png-page', String(page), '--dpi', '72');
                }
                const result = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...args,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(result.status, 0, result.stderr);
                return readFileSync(output);
            };
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const actual = render(dir, source, format, [
                        ...scaled,
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                    ], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    assert.deepEqual(accepted, manuallyPlaced(format, page),
                        `page ${page} ${format} uses the full label advance`);
                    if (format === 'pdf') {
                        assert.match(accepted.toString('latin1'), /\/Count 2/);
                        const refusedCases = [
                            [[...scaled, '--pages', '1'],
                                /flow requires 2 physical pages/],
                            [['--handwriting-profile', statistics,
                                ...numeralInk, '--flow-region',
                                '30000', '40000', '35000', '14000'],
                                /ordered list label and first glyph exceed/],
                        ];
                        for (const [options, diagnostic] of refusedCases) {
                            const refused = render(dir, source, format,
                                options);
                            assert.notEqual(refused.result.status, 0,
                                'invalid page count or narrow label refuses');
                            assert.match(refused.result.stderr, diagnostic);
                            assert.deepEqual(readFileSync(refused.output),
                                accepted, 'refusal preserves prior PDF ink');
                        }
                    } else if (page === 2) {
                        const refused = render(dir, source, format,
                            scaled, '--flow-markdown', 3);
                        assert.notEqual(refused.result.status, 0);
                        assert.deepEqual(readFileSync(refused.output),
                            accepted, 'missing PNG page preserves last page');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('three-digit ordered labels retain heterogeneous ink across pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-three-digit-'));
        try {
            const statistics = join(dir, 'digit-widths.json');
            const edits = [
                ['9', '6500'], ['8', '5000'],
                ['7', '4000'], [')', '5500'],
            ].flatMap(([glyph, width]) => [
                '--set-glyph', glyph, 'width',
                width, width, width, width, '0',
            ]);
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', ...edits,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(statistics, configured.stdout);
            const markerInk = ['9', '8', '7', ')'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '600', '500', '1800', '3000',
                '250', '#123456',
            ]);
            const source = '987) aó';
            const region = [
                '--flow-region', '30000', '40000', '41000', '14000',
            ];
            const extras = [...markerInk, ...region];
            const baseline = render(dir, source, 'pdf',
                [...extras, '--pages', '1']);
            assert.equal(baseline.result.status, 0, baseline.result.stderr);
            assert.match(readFileSync(baseline.output)
                .toString('latin1'), /\/Count 1/,
            'nominal digits and punctuation fit in one row');

            const profile = ['--handwriting-profile', statistics];
            // The saved baseline adds 350 um of tracking to each of
            // the four profiled marker glyphs: 21,000 + 1,400 +
            // 4,000 = 26,400 um after the label separator space.
            const manual = (format, page) => {
                const output = join(dir, `three-manual-${format}-${page}`);
                const args = [...base, ...markerInk, ...profile,
                    '--place-text', '1', '38500', '42500', '987) a',
                    '--place-text', '2', '64900', '42500', 'ó',
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format, '--output', output];
                if (format === 'png') {
                    args.push('--png-page', String(page), '--dpi', '72');
                }
                const result = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...args,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(result.status, 0, result.stderr);
                return readFileSync(output);
            };
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const flowed = render(dir, source, format, [
                        ...extras, ...profile,
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                    ], '--flow-markdown', page);
                    assert.equal(flowed.result.status, 0,
                        flowed.result.stderr);
                    const accepted = readFileSync(flowed.output);
                    assert.deepEqual(accepted, manual(format, page),
                        `page ${page} ${format} retains all digit widths`);
                    if (format === 'pdf') {
                        assert.match(accepted.toString('latin1'), /\/Count 2/);
                        for (const [options, diagnostic] of [
                            [[...extras, ...profile, '--pages', '1'],
                                /flow requires 2 physical pages/],
                            [[...markerInk, ...profile, '--flow-region',
                                '30000', '40000', '38000', '14000'],
                                /ordered list label and first glyph exceed/],
                        ]) {
                            const refused = render(dir, source, format,
                                options);
                            assert.notEqual(refused.result.status, 0);
                            assert.match(refused.result.stderr, diagnostic);
                            assert.deepEqual(readFileSync(refused.output),
                                accepted, 'refusal preserves accepted PDF');
                        }
                    } else if (page === 2) {
                        const refused = render(dir, source, format,
                            [...extras, ...profile], '--flow-markdown', 3);
                        assert.notEqual(refused.result.status, 0);
                        assert.deepEqual(readFileSync(refused.output), accepted,
                            'missing page does not replace accepted PNG');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('nine-digit ordering stops at the exact Markdown marker boundary',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-nine-digit-'));
        try {
            const markerInk = [...'0123456789)'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '600', '500', '1800', '3000',
                '250', '#123456',
            ]);
            const region = [
                '--flow-region', '30000', '40000', '60000', '14000',
            ];
            const cases = [
                {
                    source: '123456789) aó',
                    rows: [
                        [1, 38500, 42500, '123456789) a'],
                        [2, 82500, 42500, 'ó'],
                    ],
                },
                {
                    // Ten digits are ordinary text, not a list whose
                    // continuation inherits a fictitious hanging indent.
                    source: '1234567890) aóóa',
                    rows: [
                        [1, 32500, 42500, '1234567890)'],
                        [2, 32500, 42500, 'aóóa'],
                    ],
                },
            ];
            const manual = (entry, format, page) => {
                const output = join(dir, `nine-manual-${format}-${page}`);
                const placements = entry.rows.flatMap(
                    ([number, x, y, content]) => [
                        '--place-text', String(number), String(x),
                        String(y), content,
                    ]);
                const args = [...base, ...markerInk, ...placements,
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format, '--output', output, '--replace'];
                if (format === 'png') {
                    args.push('--png-page', String(page), '--dpi', '72');
                }
                const result = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...args,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(result.status, 0, result.stderr);
                return readFileSync(output);
            };
            for (const entry of cases) {
                for (const format of ['pdf', 'png']) {
                    for (const page of format === 'pdf' ? [1] : [1, 2]) {
                        const options = [
                            ...markerInk, ...region,
                            ...(format === 'pdf' ? ['--pages', '2'] : []),
                        ];
                        const flowed = render(dir, entry.source, format,
                            options, '--flow-markdown', page);
                        assert.equal(flowed.result.status, 0,
                            flowed.result.stderr);
                        const accepted = readFileSync(flowed.output);
                        assert.deepEqual(accepted,
                            manual(entry, format, page),
                            `${entry.source} page ${page} ${format}`);
                        if (format === 'pdf') {
                            assert.match(accepted.toString('latin1'),
                                /\/Count 2/);
                            const refused = render(dir, entry.source,
                                format, [...markerInk, ...region,
                                    '--pages', '1']);
                            assert.notEqual(refused.result.status, 0);
                            assert.match(refused.result.stderr,
                                /flow requires 2 physical pages/);
                            assert.deepEqual(readFileSync(refused.output),
                                accepted, 'page refusal keeps prior PDF');
                        } else if (page === 2) {
                            const refused = render(dir, entry.source,
                                format, [...markerInk, ...region],
                                '--flow-markdown', 3);
                            assert.notEqual(refused.result.status, 0);
                            assert.deepEqual(readFileSync(refused.output),
                                accepted, 'missing PNG page keeps prior ink');
                        }
                    }
                }
            }
            const tooNarrow = render(dir, cases[0].source, 'pdf',
                [...markerInk, '--flow-region', '30000', '40000',
                    '57000', '14000', '--pages', '2']);
            assert.notEqual(tooNarrow.result.status, 0);
            assert.match(tooNarrow.result.stderr,
                /ordered list label and first glyph exceed/);
            assert.deepEqual(readFileSync(tooNarrow.output),
                manual(cases[1], 'pdf', 1),
                'oversized nine-digit label cannot overwrite accepted PDF');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('notebook CLI help advertises real Markdown task shapes', () => {
    const help = spawnSync(cargo, [
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

test('nonbreaking Unicode joins match independent PDF/PNG pen placement',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-nonbreaking-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '32000', '36000'];
            const glyphs = 'bcdefg';
            for (const separator of ['\u{a0}', '\u{202f}', '\u{2011}']) {
                const strokes = [...glyphs, separator].flatMap((glyph) => [
                    '--glyph-stroke', glyph, '600', '500', '1800',
                    '3000', '250', '#123456',
                ]);
                const source = `abcdef${separator}g`;
                for (const format of ['pdf', 'png']) {
                    const manual = join(dir, `manual-${format}.output`);
                    const expected = spawnSync(cargo, [
                        'run', '--offline', '--quiet',
                        '-p', 'atrament_profile_editor_cli', '--',
                        ...base, ...strokes,
                        '--place-text', '1', '32500', '42500', 'abcde',
                        '--place-text', '1', '32500', '51500',
                        `f${separator}g`,
                        '--export-paper', format,
                        ...(format === 'png' ? ['--dpi', '72'] : []),
                        '--output', manual, '--replace',
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    for (const flag of ['--flow-text', '--flow-markdown']) {
                        const flowed = render(dir, source, format,
                            [...strokes, ...region], flag);
                        assert.equal(flowed.result.status, 0,
                            flowed.result.stderr);
                        assert.deepEqual(readFileSync(flowed.output),
                            readFileSync(manual),
                            `${format} ${flag}: explicit Unicode pen ink`);
                        const refused = render(dir, source, format,
                            [...region], flag);
                        assert.notEqual(refused.result.status, 0,
                            'unauthored joiner must never become invisible');
                        assert.match(refused.result.stderr,
                            /missing authored glyph|no authored strokes/i);
                        assert.deepEqual(readFileSync(refused.output),
                            readFileSync(manual),
                            'failed coverage must preserve accepted ink');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('wrapped dashes and ellipsis match direct PDF/PNG ink', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-dash-'));
    try {
        const region = ['--flow-region', '30000', '40000',
            '32000', '36000'];
        for (const dash of ['-', '\u{2010}', '\u{2013}',
            '\u{2014}', '…']) {
            const strokes = [...'bcdefg', dash].flatMap((glyph) => [
                '--glyph-stroke', glyph, '600', '500', '1800',
                '3000', '250', '#123456',
            ]);
            for (const format of ['pdf', 'png']) {
                const manual = join(dir, `manual-dash-${format}.output`);
                const placed = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--',
                    ...base, ...strokes,
                    '--place-text', '1', '32500', '42500', 'abcde',
                    '--place-text', '1', '32500', '51500', `f${dash}g`,
                    '--export-paper', format,
                    ...(format === 'png' ? ['--dpi', '72'] : []),
                    '--output', manual, '--replace',
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(placed.status, 0, placed.stderr);
                const ink = readFileSync(manual);
                for (const flag of ['--flow-text', '--flow-markdown']) {
                    const flow = render(dir, `abcdef${dash}g`, format,
                        [...strokes, ...region], flag);
                    assert.equal(flow.result.status, 0,
                        flow.result.stderr);
                    assert.deepEqual(readFileSync(flow.output), ink,
                        `${format} ${flag} dash ${dash}: directed pen ink`);
                    const refused = render(dir, `abcdef${dash}g`,
                        format, [...region], flag);
                    assert.notEqual(refused.result.status, 0,
                        'missing dash ink is not synthesized');
                    assert.match(refused.result.stderr,
                        /missing authored glyph|no authored strokes/i);
                    assert.deepEqual(readFileSync(refused.output), ink,
                        'refused ink cannot replace an accepted export');
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

test('interior empty quote markers preserve real page-local PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-quote-gap-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '55000', '15000'];
            const alternatives = [
                '> a\n>\n> ó',
                '> a\n>\n\n> ó',
                '> a\r\n>\r\n\r\n>\r\n> ó',
                '> a\n>\n   \n>\n> ó',
                '> a\n>\n\n>\n> ó',
                '> a\n>\n\n\n>\n\n> ó',
                '> a\n> \n> ó',
                '  > a\n  >\n  > ó',
                '> a\r\n>\r\n> ó',
                '> a\n>\n> \n  >\n> ó',
                '  > a\r\n  >\r\n>\r\n> ó',
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const reference = render(dir, '> a\n\n> ó',
                        format, region, '--flow-markdown', page);
                    assert.equal(reference.result.status, 0,
                        reference.result.stderr);
                    const expected = readFileSync(reference.output);
                    if (format === 'pdf') {
                        assert.equal((expected.toString('latin1')
                            .match(/\/Type \/Page\b/g) ?? []).length, 2,
                            'both quote paragraphs need real physical pages');
                    }
                    for (const source of alternatives) {
                        const actual = render(dir, source, format,
                            region, '--flow-markdown', page);
                        assert.equal(actual.result.status, 0,
                            `${source}: ${actual.result.stderr}`);
                        assert.deepEqual(readFileSync(actual.output), expected,
                            `${source} must preserve page ${page} ${format}`);
                    }
                    for (const source of [
                        '>', '>\n> a', '> a\n>', '> a\n>\na',
                        '> a\n>\n>\na',
                        '> a\n>\n>\n> ',
                        '> a\n>\n\n',
                        '> a\n>\n\n---\n> ó',
                    ]) {
                        const rejected = render(dir, source, format,
                            region, '--flow-markdown', page);
                        assert.notEqual(rejected.result.status, 0, source);
                        assert.match(rejected.result.stderr,
                            /empty Markdown block refused/, source);
                        assert.deepEqual(readFileSync(rejected.output),
                            expected, 'refusal preserves selected export');
                    }
                    const plain = render(dir, '> a\n>\n> ó', format,
                        region, '--flow-text', page);
                    assert.notEqual(plain.result.status, 0);
                    assert.match(plain.result.stderr,
                        /missing authored glyph shapes/);
                    assert.deepEqual(readFileSync(plain.output), expected,
                        'plain text cannot consume quote syntax');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('repeated empty quote markers consume only one physical gap',
    () => {
        const dir = mkdtempSync(join(root, '.temp/markdown-quote-gap-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '55000', '40000'];
            const alternatives = [
                '> a\n>\n> ó',
                '> a\n>\n\n> ó',
                '> a\r\n>\r\n\r\n>\r\n> ó',
                '> a\n>\n   \n>\n> ó',
                '> a\n>\n\n>\n> ó',
                '> a\n>\n\n\n>\n\n> ó',
                '> a\n>\n>\n>\n> ó',
                `> a\n${'>\n'.repeat(100)}> ó`,
                `> a\n${'>\n\n'.repeat(100)}> ó`,
                '> a\n> \n  >\n> ó',
                '> a\n\n>\n> ó',
                '> a\r\n>\r\n>\r\n> ó',
            ];
            for (const format of ['pdf', 'png']) {
                const reference = render(dir, '> a\n\n> ó', format,
                    region);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                const manual = join(dir, `quote-gap-manual-${format}`);
                const marks = [
                    '--place-text', '1', '38500', '42500', 'a',
                    '--place-text', '1', '38500', '55500', 'ó',
                    '--ink-line', '1', '35500', '43000',
                    '35500', '50900', '400', '#1E304A',
                    '--ink-line', '1', '35500', '56000',
                    '35500', '63900', '400', '#1E304A',
                ];
                const composed = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...base, ...marks, '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', '1', '--dpi', '72'] : []),
                    '--output', manual,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(composed.status, 0, composed.stderr);
                assert.deepEqual(expected, readFileSync(manual),
                    `single gap matches explicit ${format} physical ink`);
                for (const source of alternatives) {
                    const actual = render(dir, source, format, region);
                    assert.equal(actual.result.status, 0,
                        `${source}: ${actual.result.stderr}`);
                    assert.deepEqual(readFileSync(actual.output), expected,
                        `${source} adds no extra physical gap to ${format}`);
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
                const refused = spawnSync(cargo, [
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
            const edited = spawnSync(cargo, [
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
                const literalSingle = render(dir,
                    `a${slash}  \na`, format, authoredSlash);
                assert.equal(literalSingle.result.status, 0,
                    literalSingle.result.stderr);
                const singleBytes = readFileSync(literalSingle.output);
                assert.notDeepEqual(singleBytes, hardBreak,
                    'a slash before two spaces remains visible ink');
                for (const alternate of [
                    `a${slash}\u2028a`, `a${slash}  \na`,
                ]) {
                    const replay = render(dir, alternate, format,
                        authoredSlash);
                    assert.equal(replay.result.status, 0,
                        replay.result.stderr);
                    assert.deepEqual(readFileSync(replay.output), singleBytes,
                        'line separators preserve literal single slash ink');
                }
                const missingSingle = render(dir,
                    `a${slash}\u2028a`, format);
                assert.notEqual(missingSingle.result.status, 0);
                assert.match(missingSingle.result.stderr,
                    /missing authored glyph shapes.*U\+005C/);
                assert.deepEqual(readFileSync(missingSingle.output),
                    singleBytes, 'refusal preserves last accepted ink');
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
            const configured = spawnSync(cargo, [
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
        const configured = spawnSync(cargo, [
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
        const configured = spawnSync(cargo, [
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
            const result = spawnSync(cargo, ['run', '--quiet', '-p',
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
        const configured = spawnSync(cargo, [
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
            const result = spawnSync(cargo, [
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

test('signed baseline statistics change native Markdown physical page count',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-signed-baseline-'));
        try {
            const profile = (glyph, baseline) => {
                const saved = join(dir, `baseline-${glyph === 'a'
                    ? 'ink' : 'syntax'}-${baseline}.json`);
                const configured = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', 'handwriting', '--set-glyph', glyph,
                    'baseline_offset', String(baseline), String(baseline),
                    String(baseline), String(baseline), '0',
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(configured.status, 0, configured.stderr);
                writeFileSync(saved, configured.stdout);
                return saved;
            };
            const ink = profile('a', 20000);
            const negativeInk = profile('a', -20000);
            const syntax = profile('#', -20000);
            const region = ['--flow-region', '30000', '40000',
                '100000', '34000'];
            const source = '# a\n- a\n> a';
            const options = [
                ...region, '--heading-style', 'none',
                '--handwriting-profile', ink,
            ];
            const pdf = render(dir, source, 'pdf',
                [...options, '--pages', '3']);
            assert.equal(pdf.result.status, 0, pdf.result.stderr);
            const acceptedPdf = readFileSync(pdf.output);
            assert.match(acceptedPdf.toString('latin1'), /\/Count 3/);

            const pages = [];
            for (const page of [1, 2, 3]) {
                const result = render(dir, source, 'png', options,
                    '--flow-markdown', page);
                assert.equal(result.result.status, 0, result.result.stderr);
                pages.push(readFileSync(result.output));
            }
            const expectedInk = [
                ['1', '32500', []],
                ['2', '38500', [
                    '--ink-line', '2', '34100', '44700',
                    '35800', '44700', '450', '#1E304A',
                ]],
                ['3', '38500', [
                    '--ink-line', '3', '35500', '43000',
                    '35500', '68900', '400', '#1E304A',
                ]],
            ];
            for (const [index, [page, x, ornament]]
                of expectedInk.entries()) {
                const output = join(dir, `manual-page-${page}.png`);
                const manual = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...base, '--handwriting-profile', ink,
                    '--place-text', page, x, '42500', 'a', ...ornament,
                    '--export-paper', 'png', '--png-page', page,
                    '--dpi', '72', '--output', output,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(manual.status, 0, manual.stderr);
                assert.deepEqual(pages[index], readFileSync(output),
                    `flow page ${page} must match independently placed ink`);
            }
            assert.notDeepEqual(pages[0], pages[1],
                'synthetic bullet mark belongs only to page two');
            assert.notDeepEqual(pages[1], pages[2],
                'quote rule belongs only to page three');
            const repeat = render(dir, source, 'pdf',
                [...options, '--pages', '3']);
            assert.equal(repeat.result.status, 0, repeat.result.stderr);
            assert.deepEqual(readFileSync(repeat.output), acceptedPdf,
                'signed baseline pagination is deterministic');

            const syntaxOnly = render(dir, source, 'pdf', [
                ...region, '--heading-style', 'none',
                '--handwriting-profile', syntax, '--pages', '1',
            ]);
            assert.equal(syntaxOnly.result.status, 0,
                syntaxOnly.result.stderr);
            assert.match(readFileSync(syntaxOnly.output)
                .toString('latin1'), /\/Count 1/,
            'the syntactic # cannot affect native row height');

            const refused = render(dir, source, 'pdf', [
                ...options, '--pages', '1',
            ]);
            assert.notEqual(refused.result.status, 0);
            assert.match(refused.result.stderr, /requires 3 physical pages/);
            assert.deepEqual(readFileSync(refused.output),
                readFileSync(syntaxOnly.output),
                'page-budget refusal cannot overwrite accepted one-page PDF');
            const clipped = render(dir, source, 'pdf', [
                ...region, '--heading-style', 'none',
                '--handwriting-profile', negativeInk, '--pages', '3',
            ]);
            assert.notEqual(clipped.result.status, 0);
            assert.match(clipped.result.stderr, /flow region ink exceeds/);
            assert.deepEqual(readFileSync(clipped.output),
                readFileSync(syntaxOnly.output),
                'negative-offset ink beyond the region must refuse atomically');
            const missingPage = render(dir, source, 'png', options,
                '--flow-markdown', 4);
            assert.notEqual(missingPage.result.status, 0);
            assert.match(missingPage.result.stderr,
                /selected PNG page is empty/);
            assert.deepEqual(readFileSync(missingPage.output), pages[2],
                'invalid PNG page cannot replace accepted page three');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('tall compressed heading page turns preserve exact ATX/Setext PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-tall-heading-'));
        try {
            const statistics = join(dir, 'tall-compressed.json');
            const configured = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', '--set-glyph', 'a', 'width',
                '12000', '12000', '12000', '12000', '0',
                '--set-glyph', 'a', 'letter_spacing',
                '-10000', '-10000', '-10000', '-10000', '0',
                '--set-glyph', '\u00f3', 'height',
                '20000', '20000', '20000', '20000', '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(statistics, configured.stdout);

            const source = 'a  \na  \n# aa\n\n\u00f3';
            const setext = 'a  \na  \naa\n===\n\n\u00f3';
            const region = [
                '--handwriting-profile', statistics,
                '--flow-region', '30000', '40000', '110000', '80000',
            ];
            for (const headingStyle of [
                'underline', 'double-underline', 'marker', 'none',
            ]) {
                const options = [...region, '--heading-style', headingStyle];
                const pdf = render(dir, source, 'pdf', [
                    ...options, '--pages', '2',
                ]);
                assert.equal(pdf.result.status, 0, pdf.result.stderr);
                const original = readFileSync(pdf.output);
                assert.match(original.toString('latin1'), /\/Count 2/);
                const alternate = render(dir, setext, 'pdf', [
                    ...options, '--pages', '2',
                ]);
                assert.equal(alternate.result.status, 0,
                    alternate.result.stderr);
                assert.deepEqual(readFileSync(alternate.output), original,
                    `${headingStyle} Setext and ATX draw identical PDF ink`);

                const pngPages = [];
                for (const page of [1, 2]) {
                    const selected = render(dir, source, 'png', options,
                        '--flow-markdown', page);
                    assert.equal(selected.result.status, 0,
                        selected.result.stderr);
                    const bytes = readFileSync(selected.output);
                    pngPages.push(bytes);
                    const alternatePng = render(dir, setext, 'png', options,
                        '--flow-markdown', page);
                    assert.equal(alternatePng.result.status, 0,
                        alternatePng.result.stderr);
                    assert.deepEqual(readFileSync(alternatePng.output), bytes,
                        `${headingStyle} page ${page} glyph and mark parity`);
                }
                assert.notDeepEqual(pngPages[0], pngPages[1],
                    'page two has the title and tall-accent body ink');

                const short = render(dir, source, 'pdf', [
                    ...options, '--pages', '1',
                ]);
                assert.notEqual(short.result.status, 0);
                assert.match(short.result.stderr, /requires 2 physical pages/);
                assert.deepEqual(readFileSync(short.output), original,
                    'a refused page budget preserves the valid native PDF');

                const missing = render(dir, source, 'png', options,
                    '--flow-markdown', 3);
                assert.notEqual(missing.result.status, 0);
                assert.match(missing.result.stderr,
                    /selected PNG page is empty/);
                assert.deepEqual(readFileSync(missing.output), pngPages[1],
                    'a missing selected page preserves page-two PNG bytes');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('compressed compound graphemes survive three native physical pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-compound-pages-'));
        try {
            const profile = join(dir, 'compressed.json');
            const glyph = '👩‍🔬';
            const nfd = 'o\u0301';
            const makeProfile = spawnSync(cargo, [
                'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                '--', 'handwriting',
                ...[glyph, nfd].flatMap((exact) => [
                    '--set-glyph', exact, 'width',
                    ...Array(4).fill('12000'), '0',
                    '--set-glyph', exact, 'letter_spacing',
                    ...Array(4).fill('-10000'), '0',
                ]),
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(makeProfile.status, 0, makeProfile.stderr);
            writeFileSync(profile, makeProfile.stdout);
            const emojiStroke = [
                '--glyph-stroke', glyph, '700', '700', '1600', '2800',
                '250', '#123456',
            ];
            const extra = [
                ...emojiStroke, '--handwriting-profile', profile,
                '--flow-region', '30000', '40000', '22000', '14000',
            ];
            const expected = [
                ['1', '32500', '42500', 'a'],
                ['2', '32500', '42500', glyph],
                ['3', '32500', '42500', nfd],
            ];
            const manual = expected.flatMap(([page, x, y, value]) => [
                '--place-text', page, x, y, value,
            ]);
            const input = `a ${glyph}\n${nfd}`;
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2, 3]) {
                    const output = render(dir, input, format, [
                        ...extra, ...(format === 'pdf'
                            ? ['--pages', '3'] : []),
                    ], '--flow-text', page);
                    assert.equal(output.result.status, 0,
                        output.result.stderr);
                    const reference = join(dir,
                        `compound-manual-${format}-${page}`);
                    const referenceArgs = [
                        ...base, ...emojiStroke,
                        '--handwriting-profile', profile, ...manual,
                        ...(format === 'pdf' ? ['--pages', '3'] : []),
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ];
                    const handwritten = spawnSync(cargo, [
                        'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                        '--', ...referenceArgs,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(handwritten.status, 0, handwritten.stderr);
                    assert.deepEqual(readFileSync(output.output),
                        readFileSync(reference),
                        `page ${page} ${format} preserves exact pen ink`);
                }
            }
            for (const [format, page] of [['pdf', 1], ['png', 3]]) {
                const accepted = render(dir, input, format, [
                    ...extra, ...(format === 'pdf'
                        ? ['--pages', '3'] : []),
                ], '--flow-text', page);
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const previous = readFileSync(accepted.output);
                const refused = render(dir, input, format, [
                    ...extra, ...(format === 'pdf'
                        ? ['--pages', '2'] : []),
                ], '--flow-text', format === 'pdf' ? 1 : 4);
                assert.notEqual(refused.result.status, 0,
                    'missing page must refuse the output replacement');
                assert.deepEqual(readFileSync(refused.output), previous);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('wrapped list markers move with native PDF and page-local PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-list-orphan-'));
        try {
            const region = [
                '--flow-region', '30000', '40000', '35000', '23000',
            ];
            const numeralInk = ['1', '2', ')'].flatMap((glyph) => [
                '--glyph-stroke', glyph, '600', '500', '1800', '3000',
                '250', '#123456',
            ]);
            const bullet = [
                '--ink-line', '2', '34100', '44700',
                '35800', '44700', '450', '#1E304A',
            ];
            const task = [
                ['33900', '43700', '36400', '43700', '350'],
                ['36400', '43700', '36400', '46200', '350'],
                ['36400', '46200', '33900', '46200', '350'],
                ['33900', '46200', '33900', '43700', '350'],
                ['34350', '44950', '35000', '45850', '450'],
                ['35000', '45850', '36150', '44100', '450'],
            ].flatMap((line) => [
                '--ink-line', '2', ...line, '#1E304A',
            ]);
            const examples = [
                {
                    source: 'a\n- aaaaaa aaaaaa',
                    rows: ['aaaaaa', 'aaaaaa'], x: '38500',
                    ink: bullet,
                },
                {
                    source: 'a\n12) aa aa',
                    rows: ['12) aa', 'aa'], x: '54500',
                    ink: [],
                },
                {
                    source: 'a\n- [x] aaaaaa aaaaaa',
                    rows: ['aaaaaa', 'aaaaaa'], x: '38500',
                    ink: task,
                },
            ];
            for (const [index, example] of examples.entries()) {
                const manualInk = [
                    '--place-text', '1', '32500', '42500', 'a',
                    '--place-text', '2', '38500', '42500', example.rows[0],
                    '--place-text', '2', example.x, '51500', example.rows[1],
                    ...example.ink,
                ];
                for (const format of ['pdf', 'png']) {
                    for (const page of format === 'pdf' ? [1] : [1, 2]) {
                        const options = [...region, ...numeralInk,
                            ...(format === 'pdf' ? ['--pages', '2'] : [])];
                        const actual = render(dir, example.source, format,
                            options, '--flow-markdown', page);
                        assert.equal(actual.result.status, 0,
                            actual.result.stderr);
                        const accepted = readFileSync(actual.output);
                        const reference = join(dir,
                            `list-${index}-${format}-${page}`);
                        const expected = spawnSync(cargo, [
                            'run', '--quiet', '-p',
                            'atrament_profile_editor_cli', '--',
                            ...base, ...numeralInk, ...manualInk,
                            ...(format === 'pdf' ? ['--pages', '2'] : []),
                            '--export-paper', format,
                            ...(format === 'png'
                                ? ['--png-page', String(page), '--dpi', '72']
                                : []),
                            '--output', reference,
                        ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                        assert.equal(expected.status, 0, expected.stderr);
                        assert.deepEqual(accepted, readFileSync(reference),
                            `list ${index} page ${page} ${format} ink`);
                        if (format === 'pdf') {
                            assert.match(accepted.toString('latin1'),
                                /\/Count 2/);
                        }
                        const refused = render(dir, example.source, format,
                            [...options, ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                            '--flow-markdown', format === 'pdf' ? 1 : 3);
                        assert.notEqual(refused.result.status, 0);
                        assert.deepEqual(readFileSync(refused.output), accepted,
                            `list ${index} failed export keeps ${format}`);
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('native heading and both wrapped list rows move as one group', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-heading-list-'));
    try {
        const source = 'a\n# a\n- aaaaaa aaaaaa';
        const region = [
            '--flow-region', '30000', '40000', '35000', '32000',
            '--heading-style', 'underline',
        ];
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--place-text', '2', '32500', '42500', 'a',
            '--place-text', '2', '38500', '51500', 'aaaaaa',
            '--place-text', '2', '38500', '60500', 'aaaaaa',
            '--ink-line', '2', '32500', '49200',
            '36500', '49200', '550', '#1E304A',
            '--ink-line', '2', '34100', '53700',
            '35800', '53700', '450', '#1E304A',
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const actual = render(dir, source, format, [
                    ...region, ...(format === 'pdf'
                        ? ['--pages', '2'] : []),
                ], '--flow-markdown', page);
                assert.equal(actual.result.status, 0, actual.result.stderr);
                const reference = join(dir, `heading-list-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...base, ...manual,
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72'] : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(readFileSync(actual.output),
                    readFileSync(reference),
                    `heading and bullet page ${page} ${format} ink`);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('wrapped quote rules follow native PDF and page-local PNG ink', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-quote-orphan-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '23000',
        ];
        for (const [index, source, last] of [
            [0, 'a\n> aaaaaa aaaaaa', 'aaaaaa'],
            [1, 'a\n> aaaaaaa', 'a'],
        ]) {
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '38500', '42500', 'aaaaaa',
                '--place-text', '2', '38500', '51500', last,
                '--ink-line', '2', '35500', '43000',
                '35500', '50900', '400', '#1E304A',
                '--ink-line', '2', '35500', '52000',
                '35500', '59900', '400', '#1E304A',
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const extra = [...region, ...(format === 'pdf'
                        ? ['--pages', '2'] : [])];
                    const actual = render(dir, source, format,
                        extra, '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `quote-${index}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual,
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `quote ${index} page ${page} ${format} ink`);
                    const refused = render(dir, source, format,
                        [...extra, ...(format === 'pdf'
                            ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `quote ${index} page selection keeps ${format}`);
                    const missing = render(dir, `${source} β`, format,
                        extra, '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0,
                        'missing glyph must refuse the export');
                    assert.deepEqual(readFileSync(missing.output), accepted);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('native heading and both wrapped quote rules move as one group', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-heading-quote-'));
    try {
        const source = 'a\n# a\n> aaaaaa aaaaaa';
        const region = [
            '--flow-region', '30000', '40000', '35000', '32000',
            '--heading-style', 'underline',
        ];
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--place-text', '2', '32500', '42500', 'a',
            '--place-text', '2', '38500', '51500', 'aaaaaa',
            '--place-text', '2', '38500', '60500', 'aaaaaa',
            '--ink-line', '2', '32500', '49200',
            '36500', '49200', '550', '#1E304A',
            '--ink-line', '2', '35500', '52000',
            '35500', '59900', '400', '#1E304A',
            '--ink-line', '2', '35500', '61000',
            '35500', '68900', '400', '#1E304A',
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const actual = render(dir, source, format, [
                    ...region, ...(format === 'pdf'
                        ? ['--pages', '2'] : []),
                ], '--flow-markdown', page);
                assert.equal(actual.result.status, 0, actual.result.stderr);
                const reference = join(dir,
                    `heading-quote-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...base, ...manual,
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72'] : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(readFileSync(actual.output),
                    readFileSync(reference),
                    `heading and quote page ${page} ${format} ink`);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('wrapped Markdown body starts keep native PDF and PNG rows together',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-body-orphan-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '23000',
        ];
        for (const [index, source, first, last] of [
            [0, 'a  \naaaaaa aaaaaa', 'aaaaaa', 'aaaaaa'],
            [1, 'a  \naaaaaaaa', 'aaaaaaa', 'a'],
        ]) {
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '32500', '42500', first,
                '--place-text', '2', '32500', '51500', last,
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const extra = [...region, ...(format === 'pdf'
                        ? ['--pages', '2'] : [])];
                    const actual = render(dir, source, format, extra,
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `body-${index}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual,
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `body ${index} page ${page} ${format} ink`);
                    const refused = render(dir, source, format,
                        [...extra, ...(format === 'pdf'
                            ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `body ${index} invalid ${format} page selection`);
                    const missing = render(dir, `${source} β`, format,
                        extra, '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `body ${index} missing glyph keeps ${format}`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('thematic rule shares its PDF/PNG sheet with following authored ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-rule-orphan-'));
    try {
        const source = 'a\n\n---\na';
        const region = [
            '--flow-region', '30000', '40000', '35000', '32000',
        ];
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--ink-line', '2', '32500', '47000',
            '63500', '47000', '400', '#1E304A',
            '--place-text', '2', '32500', '51500', 'a',
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const extra = [...region, ...(format === 'pdf'
                    ? ['--pages', '2'] : [])];
                const actual = render(dir, source, format, extra,
                    '--flow-markdown', page);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                const accepted = readFileSync(actual.output);
                const reference = join(dir, `rule-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p',
                    'atrament_profile_editor_cli', '--',
                    ...base, ...manual,
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(accepted, readFileSync(reference),
                    `rule page ${page} ${format} ink`);
                const refused = render(dir, source, format,
                    [...extra, ...(format === 'pdf'
                        ? ['--pages', '1'] : [])],
                    '--flow-markdown', format === 'pdf' ? 1 : 3);
                assert.notEqual(refused.result.status, 0);
                assert.deepEqual(readFileSync(refused.output), accepted,
                    `rule invalid ${format} page selection`);
                const missing = render(dir, `${source} β`, format,
                    extra, '--flow-markdown', page);
                assert.notEqual(missing.result.status, 0);
                assert.deepEqual(readFileSync(missing.output), accepted,
                    `rule missing glyph keeps ${format}`);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('rule keeps its first wrapped ink row in native PDF and PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-rule-priority-'));
    try {
        const source = 'a\n\n---\naaaaaa aaaaaa';
        const region = [
            '--flow-region', '30000', '40000', '35000', '23000',
        ];
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--ink-line', '2', '32500', '47000',
            '63500', '47000', '400', '#1E304A',
            '--place-text', '2', '32500', '51500', 'aaaaaa',
            '--place-text', '3', '32500', '42500', 'aaaaaa',
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2, 3]) {
                const extra = [...region, ...(format === 'pdf'
                    ? ['--pages', '3'] : [])];
                const actual = render(dir, source, format, extra,
                    '--flow-markdown', page);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                const accepted = readFileSync(actual.output);
                const reference = join(dir, `priority-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p',
                    'atrament_profile_editor_cli', '--',
                    ...base, ...manual,
                    ...(format === 'pdf' ? ['--pages', '3'] : []),
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(accepted, readFileSync(reference),
                    `first wrapped rule ${format} page ${page}`);
                const refused = render(dir, source, format,
                    [...region, ...(format === 'pdf'
                        ? ['--pages', '2'] : [])],
                    '--flow-markdown', format === 'pdf' ? 1 : 4);
                assert.notEqual(refused.result.status, 0);
                assert.deepEqual(readFileSync(refused.output), accepted,
                    `short ${format} export preserves accepted ink`);
                const missing = render(dir, `${source} β`, format,
                    extra, '--flow-markdown', page);
                assert.notEqual(missing.result.status, 0);
                assert.deepEqual(readFileSync(missing.output), accepted,
                    `missing glyph preserves ${format} page ${page}`);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('long Markdown body balances last PDF and PNG rows without lost ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-body-tail-'));
    try {
        const source = Array(4).fill('aaaaaa').join(' ');
        const region = [
            '--flow-region', '30000', '40000', '35000', '32000',
        ];
        const manual = [
            '--place-text', '1', '32500', '42500', 'aaaaaa',
            '--place-text', '1', '32500', '51500', 'aaaaaa',
            '--place-text', '2', '32500', '42500', 'aaaaaa',
            '--place-text', '2', '32500', '51500', 'aaaaaa',
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const extra = [...region, ...(format === 'pdf'
                    ? ['--pages', '2'] : [])];
                const actual = render(dir, source, format, extra,
                    '--flow-markdown', page);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                const accepted = readFileSync(actual.output);
                const reference = join(dir, `tail-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p',
                    'atrament_profile_editor_cli', '--',
                    ...base, ...manual,
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(accepted, readFileSync(reference),
                    `balanced body ${format} page ${page}`);
                const refused = render(dir, source, format,
                    [...extra, ...(format === 'pdf'
                        ? ['--pages', '1'] : [])],
                    '--flow-markdown', format === 'pdf' ? 1 : 3);
                assert.notEqual(refused.result.status, 0);
                assert.deepEqual(readFileSync(refused.output), accepted,
                    `invalid body-tail page keeps ${format}`);
                const missing = render(dir, `${source} β`, format,
                    extra, '--flow-markdown', page);
                assert.notEqual(missing.result.status, 0);
                assert.deepEqual(readFileSync(missing.output), accepted,
                    `missing glyph keeps ${format} page ${page}`);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('long wrapped quote and bullet tails preserve page-local PDF/PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-decorated-tail-'));
    try {
        const words = Array(4).fill('aaaaaa').join(' ');
        const region = [
            '--flow-region', '30000', '40000', '35000', '32000',
        ];
        const quoteBars = [1, 2].flatMap((page) => [
            '--ink-line', String(page), '35500', '43000',
            '35500', '50900', '400', '#1E304A',
            '--ink-line', String(page), '35500', '52000',
            '35500', '59900', '400', '#1E304A',
        ]);
        const bulletMark = [
            '--ink-line', '1', '34100', '44700',
            '35800', '44700', '450', '#1E304A',
        ];
        const rows = [1, 2].flatMap((page) => [
            '--place-text', String(page), '38500', '42500', 'aaaaaa',
            '--place-text', String(page), '38500', '51500', 'aaaaaa',
        ]);
        for (const [kind, source, marks] of [
            ['quote', `> ${words}`, quoteBars],
            ['bullet', `- ${words}`, bulletMark],
        ]) {
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const extra = [...region, ...(format === 'pdf'
                        ? ['--pages', '2'] : [])];
                    const actual = render(dir, source, format, extra,
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `tail-${kind}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...rows, ...marks,
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `balanced ${kind} ${format} page ${page} ink`);
                    const refused = render(dir, source, format,
                        [...extra, ...(format === 'pdf'
                            ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `invalid ${kind} page retains ${format}`);
                    const missing = render(dir, `${source} β`, format,
                        extra, '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `missing ${kind} glyph retains ${format}`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('short wrapped Markdown blocks move as complete native PDF/PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-short-block-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '32000',
        ];
        const manualPrefix = [
            '--place-text', '1', '32500', '42500', 'a',
        ];
        for (const [kind, prefix, x, ornament] of [
            ['body', '', '32500', []],
            ['bullet', '- ', '38500', [
                '--ink-line', '2', '34100', '44700',
                '35800', '44700', '450', '#1E304A',
            ]],
            ['quote', '> ', '38500', [1, 2, 3].flatMap((row) => {
                const start = 43000 + (row - 1) * 9000;
                return [
                    '--ink-line', '2', '35500', String(start),
                    '35500', String(start + 7900), '400', '#1E304A',
                ];
            })],
        ]) {
            const source = `\`\`\`\na\n\`\`\`\n${prefix}`
                + 'aaaaaa aaaaaa aaaaaa';
            const manual = [...manualPrefix, ...[42500, 51500, 60500]
                .flatMap((y) => [
                    '--place-text', '2', x, String(y), 'aaaaaa',
                ]), ...ornament];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const exportOptions = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `short-${kind}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual, ...exportOptions,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${kind} ${format} page ${page} ink parity`);
                    const refused = render(dir, source, format,
                        [...region, ...exportOptions,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${kind} ${format} rejected page is atomic`);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${kind} ${format} missing glyph is atomic`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('four-row Markdown blocks relocate complete native PDF/PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-four-row-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '41000',
        ];
        const earlierInk = [
            '--place-text', '1', '32500', '42500', 'a',
        ];
        const box = [
            ['33900', '43700', '36400', '43700'],
            ['36400', '43700', '36400', '46200'],
            ['36400', '46200', '33900', '46200'],
            ['33900', '46200', '33900', '43700'],
        ].flatMap((points) => [
            '--ink-line', '2', ...points, '350', '#1E304A',
        ]);
        const check = [
            '--ink-line', '2', '34350', '44950',
            '35000', '45850', '450', '#1E304A',
            '--ink-line', '2', '35000', '45850',
            '36150', '44100', '450', '#1E304A',
        ];
        for (const [kind, marker, x, ornaments] of [
            ['body', '', '32500', []],
            ['bullet', '- ', '38500', [
                '--ink-line', '2', '34100', '44700',
                '35800', '44700', '450', '#1E304A',
            ]],
            ['checkbox', '- [x] ', '38500', [...box, ...check]],
            ['quote', '> ', '38500', [1, 2, 3, 4].flatMap((row) => {
                const y = 43000 + (row - 1) * 9000;
                return [
                    '--ink-line', '2', '35500', String(y),
                    '35500', String(y + 7900), '400', '#1E304A',
                ];
            })],
        ]) {
            const source = `\`\`\`\na\n\`\`\`\n${marker}`
                + Array(4).fill('aaaaaa').join(' ');
            const manual = [...earlierInk, ...[42500, 51500, 60500, 69500]
                .flatMap((y) => [
                    '--place-text', '2', x, String(y), 'aaaaaa',
                ]), ...ornaments];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const exportOptions = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `four-${kind}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual, ...exportOptions,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${kind} ${format} page ${page} native ink parity`);
                    const refused = render(dir, source, format,
                        [...region, ...exportOptions,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${kind} ${format} rejected page is atomic`);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${kind} ${format} missing glyph is atomic`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('five-row Markdown blocks preserve native PDF/PNG ink and refusal',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-five-row-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '50000',
        ];
        const firstInk = ['--place-text', '1', '32500', '42500', 'a'];
        const checkbox = [
            ['33900', '43700', '36400', '43700'],
            ['36400', '43700', '36400', '46200'],
            ['36400', '46200', '33900', '46200'],
            ['33900', '46200', '33900', '43700'],
            ['34350', '44950', '35000', '45850'],
            ['35000', '45850', '36150', '44100'],
        ].flatMap((points, index) => [
            '--ink-line', '2', ...points,
            index < 4 ? '350' : '450', '#1E304A',
        ]);
        for (const [kind, prefix, x, ornaments] of [
            ['body', '', '32500', []],
            ['bullet', '- ', '38500', [
                '--ink-line', '2', '34100', '44700',
                '35800', '44700', '450', '#1E304A',
            ]],
            ['checkbox', '- [x] ', '38500', checkbox],
            ['quote', '> ', '38500', [0, 1, 2, 3, 4]
                .flatMap((index) => {
                    const y = 43000 + index * 9000;
                    return [
                        '--ink-line', '2', '35500', String(y),
                        '35500', String(y + 7900), '400', '#1E304A',
                    ];
                })],
        ]) {
            const source = `\`\`\`\na\n\`\`\`\n${prefix}`
                + Array(5).fill('aaaaaa').join(' ');
            const manual = [...firstInk, ...[42500, 51500, 60500,
                69500, 78500].flatMap((y) => [
                '--place-text', '2', x, String(y), 'aaaaaa',
            ]), ...ornaments];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const exportOptions = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `five-${kind}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual, ...exportOptions,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${kind} ${format} page ${page} measured ink`);
                    const refused = render(dir, source, format,
                        [...region, ...exportOptions,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${kind} ${format} invalid page is atomic`);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${kind} ${format} missing glyph is atomic`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('page-fitting eight-row Markdown ink matches native PDF/PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-fitting-block-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '77000',
        ];
        const prior = [42500, 51500, 60500].flatMap((y) => [
            '--place-text', '1', '32500', String(y), 'a',
        ]);
        const checkbox = [
            ['33900', '43700', '36400', '43700'],
            ['36400', '43700', '36400', '46200'],
            ['36400', '46200', '33900', '46200'],
            ['33900', '46200', '33900', '43700'],
            ['34350', '44950', '35000', '45850'],
            ['35000', '45850', '36150', '44100'],
        ].flatMap((points, index) => [
            '--ink-line', '2', ...points,
            index < 4 ? '350' : '450', '#1E304A',
        ]);
        for (const [kind, marker, x, ornaments] of [
            ['body', '', '32500', []],
            ['bullet', '- ', '38500', [
                '--ink-line', '2', '34100', '44700',
                '35800', '44700', '450', '#1E304A',
            ]],
            ['checkbox', '- [x] ', '38500', checkbox],
            ['quote', '> ', '38500', Array.from({ length: 8 },
                (_, index) => {
                    const y = 43000 + index * 9000;
                    return [
                        '--ink-line', '2', '35500', String(y),
                        '35500', String(y + 7900), '400', '#1E304A',
                    ];
                }).flat()],
        ]) {
            const source = `\`\`\`\na\na\na\n\`\`\`\n${marker}`
                + Array(8).fill('aaaaaa').join(' ');
            const manual = [...prior, ...Array.from({ length: 8 },
                (_, index) => [
                    '--place-text', '2', x,
                    String(42500 + index * 9000), 'aaaaaa',
                ]).flat(), ...ornaments];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const exportOptions = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `fitting-${kind}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual, ...exportOptions,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${kind} ${format} page ${page} physical ink`);
                    const refused = render(dir, source, format,
                        [...region, ...exportOptions,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${kind} ${format} wrong page is atomic`);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${kind} ${format} missing glyph is atomic`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('heading and two- or three-row quotes export together on PDF/PNG',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-heading-block-'));
    try {
        for (const count of [2, 3]) {
            const source = '```\na\n```\n# a\n> '
                + Array(count).fill('aaaaaa').join(' ');
            const region = [
                '--flow-region', '30000', '40000', '35000',
                String(5000 + (count + 1) * 9000),
            ];
            const headingMarker = [900, 1900, 2900, 3900].flatMap(
                (lift) => [
                    '--ink-line', '2', '32500', String(42500 + lift),
                    '36500', String(42500 + lift), '1400', '#F8DE91',
                ]);
            const quoteBars = Array.from({ length: count }, (_, i) => {
                const y = 52000 + i * 9000;
                return [
                    '--ink-line', '2', '35500', String(y),
                    '35500', String(y + 7900), '400', '#1E304A',
                ];
            }).flat();
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '32500', '42500', 'a',
                ...Array.from({ length: count }, (_, i) => [
                    '--place-text', '2', '38500',
                    String(51500 + i * 9000), 'aaaaaa',
                ]).flat(),
                ...headingMarker, ...quoteBars,
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const exportOptions = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `heading-block-${count}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual, ...exportOptions,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${count} ${format} page ${page} matches manual ink`);
                    const refused = render(dir, source, format,
                        [...region, ...exportOptions,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${count} ${format} page ${page} stays atomic`);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${count} ${format} missing glyph stays atomic`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('blank-separated headings and quotes retain 4-mm gap in PDF/PNG',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-heading-gap-'));
    try {
        for (const count of [2, 3]) {
            const region = [
                '--flow-region', '30000', '40000', '35000',
                String(9000 * (count + 1) + 9000),
            ];
            const headingMarker = [900, 1900, 2900, 3900].flatMap(
                (lift) => [
                    '--ink-line', '2', '32500', String(42500 + lift),
                    '36500', String(42500 + lift), '1400', '#F8DE91',
                ]);
            const quoteBars = Array.from({ length: count }, (_, i) => {
                const y = 56000 + i * 9000;
                return [
                    '--ink-line', '2', '35500', String(y),
                    '35500', String(y + 7900), '400', '#1E304A',
                ];
            }).flat();
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '32500', '42500', 'a',
                ...Array.from({ length: count }, (_, i) => [
                    '--place-text', '2', '38500',
                    String(55500 + i * 9000), 'aaaaaa',
                ]).flat(),
                ...headingMarker, ...quoteBars,
            ];
            const source = '```\na\n```\n# a\n\n> '
                + Array(count).fill('aaaaaa').join(' ');
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const exportOptions = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `heading-gap-${count}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual, ...exportOptions,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${count} ${format} page ${page} gap geometry`);
                    for (const variation of [
                        source.replace('\n\n> ', '\n\n\n> '),
                        source.replaceAll('\n', '\r\n'),
                    ]) {
                        const replay = render(dir, variation, format,
                            [...region, ...exportOptions],
                            '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `${count} ${format} page ${page} gap collapse`);
                    }
                    const refused = render(dir, source, format,
                        [...region, ...exportOptions,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${count} ${format} invalid page stays atomic`);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${count} ${format} missing glyph stays atomic`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('blank-separated numbered heading keeps hanging ink in PDF/PNG',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-heading-number-gap-'));
    try {
        const digits = [
            '--glyph-stroke', '1', '600', '500', '1800', '3000',
            '250', '#123456',
            '--glyph-stroke', '.', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        const region = [
            '--flow-region', '30000', '40000', '35000', '45000',
        ];
        const source = '```\na\n```\n# a\n\n1. a a a a a';
        const marker = [900, 1900, 2900, 3900].flatMap(
            (lift) => [
                '--ink-line', '2', '32500', String(42500 + lift),
                '36500', String(42500 + lift), '1400', '#F8DE91',
            ]);
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--place-text', '2', '32500', '42500', 'a',
            '--place-text', '2', '38500', '55500', '1. a a',
            '--place-text', '2', '50500', '64500', 'a a',
            '--place-text', '2', '50500', '73500', 'a',
            ...marker,
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const exportOptions = format === 'pdf'
                    ? ['--pages', '2'] : [];
                const actual = render(dir, source, format,
                    [...digits, ...region, ...exportOptions],
                    '--flow-markdown', page);
                assert.equal(actual.result.status, 0,
                    actual.result.stderr);
                const accepted = readFileSync(actual.output);
                const reference = join(dir,
                    `number-gap-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p',
                    'atrament_profile_editor_cli', '--',
                    ...base, ...digits, ...manual, ...exportOptions,
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(accepted, readFileSync(reference),
                    `${format} page ${page} exact numbered handwriting`);
                const replay = render(dir,
                    source.replace('\n\n1.', '\n\n\n1.'), format,
                    [...digits, ...region, ...exportOptions],
                    '--flow-markdown', page);
                assert.equal(replay.result.status, 0,
                    replay.result.stderr);
                assert.deepEqual(readFileSync(replay.output), accepted);
                const refused = render(dir, source, format,
                    [...digits, ...region, ...exportOptions,
                        ...(format === 'pdf'
                            ? ['--pages', '1'] : [])],
                    '--flow-markdown', format === 'pdf' ? 1 : 3);
                assert.notEqual(refused.result.status, 0);
                assert.deepEqual(readFileSync(refused.output), accepted);
                const missing = render(dir, `${source} β`, format,
                    [...digits, ...region, ...exportOptions],
                    '--flow-markdown', page);
                assert.notEqual(missing.result.status, 0);
                assert.deepEqual(readFileSync(missing.output), accepted);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('adjacent heading chain and fitting block match physical PDF/PNG',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-heading-chain-block-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '50000',
        ];
        const headingMarks = [
            ...[900, 1900, 2900, 3900].flatMap((lift) => [
                '--ink-line', '2', '32500', String(42500 + lift),
                '36500', String(42500 + lift), '1400', '#F8DE91',
            ]),
            ...[1000, 2000, 3000].flatMap((lift) => [
                '--ink-line', '2', '32500', String(51500 + lift),
                '36500', String(51500 + lift), '1150', '#F8DE91',
            ]),
        ];
        const digits = [
            '--glyph-stroke', '1', '600', '500', '1800', '3000',
            '250', '#123456',
            '--glyph-stroke', '.', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        for (const { label, body, extra, rows, ornaments } of [
            {
                label: 'quote',
                body: '> aaaaaa aaaaaa aaaaaa',
                extra: [],
                rows: [
                    ['38500', '60500', 'aaaaaa'],
                    ['38500', '69500', 'aaaaaa'],
                    ['38500', '78500', 'aaaaaa'],
                ],
                ornaments: [61000, 70000, 79000].flatMap((top) => [
                    '--ink-line', '2', '35500', String(top),
                    '35500', String(top + 7900), '400', '#1E304A',
                ]),
            },
            {
                label: 'number',
                body: '1. a a a a a',
                extra: digits,
                rows: [
                    ['38500', '60500', '1. a a'],
                    ['50500', '69500', 'a a'],
                    ['50500', '78500', 'a'],
                ],
                ornaments: [],
            },
        ]) {
            const source = `\`\`\`\na\n\`\`\`\n# a\n## a\n${body}`;
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '32500', '42500', 'a',
                '--place-text', '2', '32500', '51500', 'a',
                ...rows.flatMap(([x, y, value]) => [
                    '--place-text', '2', x, y, value,
                ]),
                ...headingMarks, ...ornaments,
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...extra, ...region, ...options],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `chain-${label}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...extra, ...manual, ...options,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${label} ${format} page ${page} physical ink`);
                    const setext = source.replace('## a', 'a\n---');
                    const replay = render(dir, setext, format,
                        [...extra, ...region, ...options],
                        '--flow-markdown', page);
                    assert.equal(replay.result.status, 0,
                        replay.result.stderr);
                    assert.deepEqual(readFileSync(replay.output), accepted,
                        `${label} Setext subtitle retains identical ink`);
                    const refused = render(dir, source, format,
                        [...extra, ...region, ...options,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${label} invalid page keeps prior ${format}`);
                    const missing = render(dir, `${source} β`, format,
                        [...extra, ...region, ...options],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${label} missing glyph keeps prior ${format}`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('adjacent heading chain keeps collapsed body gap in native PDF/PNG',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-chain-body-gap-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '54000',
        ];
        const source = '```\na\n```\n# a\n## a\n\n'
            + '> aaaaaa aaaaaa aaaaaa';
        const markers = [
            ...[900, 1900, 2900, 3900].flatMap((lift) => [
                '--ink-line', '2', '32500', String(42500 + lift),
                '36500', String(42500 + lift), '1400', '#F8DE91',
            ]),
            ...[1000, 2000, 3000].flatMap((lift) => [
                '--ink-line', '2', '32500', String(51500 + lift),
                '36500', String(51500 + lift), '1150', '#F8DE91',
            ]),
        ];
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--place-text', '2', '32500', '42500', 'a',
            '--place-text', '2', '32500', '51500', 'a',
            ...[64500, 73500, 82500].flatMap((y) => [
                '--place-text', '2', '38500', String(y), 'aaaaaa',
            ]),
            ...markers,
            ...[65000, 74000, 83000].flatMap((top) => [
                '--ink-line', '2', '35500', String(top),
                '35500', String(top + 7900), '400', '#1E304A',
            ]),
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const options = format === 'pdf' ? ['--pages', '2'] : [];
                const rendered = render(dir, source, format,
                    [...region, ...options], '--flow-markdown', page);
                assert.equal(rendered.result.status, 0,
                    rendered.result.stderr);
                const accepted = readFileSync(rendered.output);
                const reference = join(dir, `chain-gap-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p',
                    'atrament_profile_editor_cli', '--',
                    ...base, ...manual, ...options,
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(accepted, readFileSync(reference),
                    `${format} page ${page} exact chain gap ink`);
                for (const variant of [
                    source.replace('\n\n> ', '\n\n\n> '),
                    source.replaceAll('\n', '\r\n'),
                    source.replace('## a', 'a\n---'),
                ]) {
                    const replay = render(dir, variant, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(replay.result.status, 0,
                        replay.result.stderr);
                    assert.deepEqual(readFileSync(replay.output), accepted,
                        `collapsed gap replay for ${format} page ${page}`);
                }
                const rejected = render(dir, source, format,
                    [...region, ...options,
                        ...(format === 'pdf'
                            ? ['--pages', '1'] : [])],
                    '--flow-markdown', format === 'pdf' ? 1 : 3);
                assert.notEqual(rejected.result.status, 0);
                assert.deepEqual(readFileSync(rejected.output), accepted);
                const missing = render(dir, `${source} β`, format,
                    [...region, ...options], '--flow-markdown', page);
                assert.notEqual(missing.result.status, 0);
                assert.deepEqual(readFileSync(missing.output), accepted);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('blank-separated title chains retain exact native PDF/PNG gap geometry',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-title-chain-gaps-'));
    try {
        for (const { label, source, regionHeight, titles, bodyRows } of [
            {
                label: 'two-titles',
                source: '```\na\na\n```\n# a\n\n## a\n\n'
                    + '> aaaaaa aaaaaa aaaaaa',
                regionHeight: '59000',
                titles: [
                    [42500, [900, 1900, 2900, 3900], 1400],
                    [55500, [1000, 2000, 3000], 1150],
                ],
                bodyRows: [68500, 77500, 86500],
            },
            {
                label: 'three-titles',
                source: '```\na\na\n```\n# a\n\n## a\n\n### a\n\n'
                    + '> aaaaaa aaaaaa aaaaaa',
                regionHeight: '71000',
                titles: [
                    [42500, [900, 1900, 2900, 3900], 1400],
                    [55500, [1000, 2000, 3000], 1150],
                    [68500, [1400, 2400], 950],
                ],
                bodyRows: [81500, 90500, 99500],
            },
        ]) {
            const region = [
                '--flow-region', '30000', '40000', '35000', regionHeight,
            ];
            const titleInk = titles.flatMap(([y]) => [
                '--place-text', '2', '32500', String(y), 'a',
            ]);
            const titleMarks = titles.flatMap(([y, lifts, width]) =>
                lifts.flatMap((lift) => [
                    '--ink-line', '2', '32500', String(y + lift),
                    '36500', String(y + lift), String(width), '#F8DE91',
                ]));
            const quoteInk = bodyRows.flatMap((y) => [
                '--place-text', '2', '38500', String(y), 'aaaaaa',
            ]);
            const quoteMarks = bodyRows.flatMap((y) => [
                '--ink-line', '2', '35500', String(y + 500),
                '35500', String(y + 8400), '400', '#1E304A',
            ]);
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '1', '32500', '51500', 'a',
                ...titleInk, ...quoteInk, ...titleMarks, ...quoteMarks,
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const rendered = render(dir, source, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(rendered.result.status, 0,
                        rendered.result.stderr);
                    const accepted = readFileSync(rendered.output);
                    const reference = join(dir,
                        `${label}-manual-${format}-${page}`);
                    const independent = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--',
                        ...base, ...manual, ...options,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(independent.status, 0,
                        independent.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${label} ${format} page ${page} physical geometry`);
                    for (const variant of [
                        source.replaceAll('\n\n', '\n\n\n'),
                        source.replaceAll('\n', '\r\n'),
                        source.replace('## a', 'a\n---'),
                    ]) {
                        const replay = render(dir, variant, format,
                            [...region, ...options], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `${label} ${format} stable Setext/blank lines`);
                    }
                    const invalid = render(dir, source, format,
                        [...region, ...options,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(invalid.result.status, 0);
                    assert.deepEqual(readFileSync(invalid.output), accepted);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('numbered list below blank-separated titles keeps native hanging ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-title-number-gaps-'));
    try {
        const source = '```\na\na\n```\n# a\n\n## a\n\n1. a a a a a';
        const digits = [
            '--glyph-stroke', '1', '600', '500', '1800', '3000',
            '250', '#123456',
            '--glyph-stroke', '.', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        const region = [
            '--flow-region', '30000', '40000', '35000', '59000',
        ];
        const ornaments = [
            ...[900, 1900, 2900, 3900].flatMap((lift) => [
                '--ink-line', '2', '32500', String(42500 + lift),
                '36500', String(42500 + lift), '1400', '#F8DE91',
            ]),
            ...[1000, 2000, 3000].flatMap((lift) => [
                '--ink-line', '2', '32500', String(55500 + lift),
                '36500', String(55500 + lift), '1150', '#F8DE91',
            ]),
        ];
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--place-text', '1', '32500', '51500', 'a',
            '--place-text', '2', '32500', '42500', 'a',
            '--place-text', '2', '32500', '55500', 'a',
            '--place-text', '2', '38500', '68500', '1. a a',
            '--place-text', '2', '50500', '77500', 'a a',
            '--place-text', '2', '50500', '86500', 'a',
            ...ornaments,
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const options = format === 'pdf' ? ['--pages', '2'] : [];
                const rendered = render(dir, source, format,
                    [...digits, ...region, ...options],
                    '--flow-markdown', page);
                assert.equal(rendered.result.status, 0,
                    rendered.result.stderr);
                const accepted = readFileSync(rendered.output);
                const reference = join(dir, `number-${format}-${page}`);
                const independent = spawnSync(cargo, [
                    'run', '--quiet', '-p',
                    'atrament_profile_editor_cli', '--',
                    ...base, ...digits, ...manual, ...options,
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(independent.status, 0, independent.stderr);
                assert.deepEqual(accepted, readFileSync(reference),
                    `${format} page ${page} authored hanging indent`);
                for (const variant of [
                    source.replaceAll('\n\n', '\n\n\n'),
                    source.replace('## a', 'a\n---'),
                ]) {
                    const replay = render(dir, variant, format,
                        [...digits, ...region, ...options],
                        '--flow-markdown', page);
                    assert.equal(replay.result.status, 0,
                        replay.result.stderr);
                    assert.deepEqual(readFileSync(replay.output), accepted);
                }
                const rejected = render(dir, source, format,
                    [...digits, ...region, ...options,
                        ...(format === 'pdf' ? ['--pages', '1'] : [])],
                    '--flow-markdown', format === 'pdf' ? 1 : 3);
                assert.notEqual(rejected.result.status, 0);
                assert.deepEqual(readFileSync(rejected.output), accepted);
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('thematic rule and fitting wrapped block match native PDF/PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-rule-full-block-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '41000',
        ];
        const numberGlyphs = [
            '--glyph-stroke', '1', '600', '500', '1800', '3000',
            '250', '#123456',
            '--glyph-stroke', '.', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        const rule = [
            '--ink-line', '2', '32500', '47000', '63500', '47000',
            '400', '#1E304A',
        ];
        for (const { label, body, glyphs, rows, bars } of [
            {
                label: 'quote',
                body: '> aaaaaa aaaaaa aaaaaa',
                glyphs: [],
                rows: [51500, 60500, 69500].flatMap((y) => [
                    '--place-text', '2', '38500', String(y), 'aaaaaa',
                ]),
                bars: [52000, 61000, 70000].flatMap((y) => [
                    '--ink-line', '2', '35500', String(y),
                    '35500', String(y + 7900), '400', '#1E304A',
                ]),
            },
            {
                label: 'number',
                body: '1. a a a a a',
                glyphs: numberGlyphs,
                rows: [
                    '--place-text', '2', '38500', '51500', '1. a a',
                    '--place-text', '2', '50500', '60500', 'a a',
                    '--place-text', '2', '50500', '69500', 'a',
                ],
                bars: [],
            },
        ]) {
            const source = `\`\`\`\na\n\`\`\`\n---\n${body}`;
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                ...rows, ...rule, ...bars,
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...glyphs, ...options],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        `${label}: ${actual.result.stderr}`);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `rule-${label}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...glyphs, ...manual, ...options,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${label} ${format} page ${page} precise ink`);
                    for (const variant of [
                        source.replace('---', '***'),
                        source.replace('---', '___'),
                        source.replaceAll('\n', '\r\n'),
                    ]) {
                        const replay = render(dir, variant, format,
                            [...region, ...glyphs, ...options],
                            '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `${label} ${format} syntax parity`);
                    }
                    const refused = render(dir, source, format,
                        [...region, ...glyphs, ...options,
                            ...(format === 'pdf'
                                ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        `${label} invalid page must preserve output`);
                    const missing = render(dir, `${source} β`, format,
                        [...region, ...glyphs, ...options],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `${label} missing glyph must preserve output`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('collapsed gap after fitting rule preserves exact PDF/PNG geometry',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-rule-gap-block-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '36000',
        ];
        const source = '```\na\n```\n---\n\n> aaaaaa aaaaaa';
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            '--place-text', '2', '38500', '55500', 'aaaaaa',
            '--place-text', '2', '38500', '64500', 'aaaaaa',
            '--ink-line', '2', '32500', '47000', '63500', '47000',
            '400', '#1E304A',
            '--ink-line', '2', '35500', '56000', '35500', '63900',
            '400', '#1E304A',
            '--ink-line', '2', '35500', '65000', '35500', '72900',
            '400', '#1E304A',
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const options = format === 'pdf' ? ['--pages', '2'] : [];
                const actual = render(dir, source, format,
                    [...region, ...options], '--flow-markdown', page);
                assert.equal(actual.result.status, 0, actual.result.stderr);
                const accepted = readFileSync(actual.output);
                const reference = join(dir,
                    `rule-gap-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p',
                    'atrament_profile_editor_cli', '--', ...base,
                    ...manual, ...options,
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(accepted, readFileSync(reference));
                for (const variant of [
                    source.replace('\n\n> ', '\n\n\n> '),
                    source.replaceAll('\n', '\r\n'),
                ]) {
                    const replay = render(dir, variant, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(replay.result.status, 0, replay.result.stderr);
                    assert.deepEqual(readFileSync(replay.output), accepted);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('thematic rule and reserved heading chain move with native PDF/PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-rule-heading-chain-'));
    try {
        const digitGlyphs = [
            '--glyph-stroke', '1', '600', '500', '1800', '3000',
            '250', '#123456',
            '--glyph-stroke', '.', '600', '500', '1800', '3000',
            '250', '#123456',
        ];
        const scenarios = [
            {
                label: 'single-quote',
                source: '```\na\n```\n---\n# a\n> aaaaaa aaaaaa aaaaaa',
                height: '50000',
                glyphs: [],
                pageOne: [42500],
                titles: [{ y: 51500, lifts: [900, 1900, 2900, 3900],
                    width: 1400 }],
                text: [60500, 69500, 78500].flatMap((y) => [
                    '--place-text', '2', '38500', String(y), 'aaaaaa',
                ]),
                ornaments: [61000, 70000, 79000].flatMap((y) => [
                    '--ink-line', '2', '35500', String(y), '35500',
                    String(y + 7900), '400', '#1E304A',
                ]),
                variations: (source) => [
                    source.replace('---', '***'),
                    source.replaceAll('\n', '\r\n'),
                ],
            },
            {
                label: 'gapped-number',
                source: '```\na\na\n```\n---\n\n# a\n\n## a\n\n1. a a a a a',
                height: '71000',
                glyphs: digitGlyphs,
                pageOne: [42500, 51500],
                titles: [
                    { y: 55500, lifts: [900, 1900, 2900, 3900],
                        width: 1400 },
                    { y: 68500, lifts: [1000, 2000, 3000],
                        width: 1150 },
                ],
                text: [
                    '--place-text', '2', '38500', '81500', '1. a a',
                    '--place-text', '2', '50500', '90500', 'a a',
                    '--place-text', '2', '50500', '99500', 'a',
                ],
                ornaments: [],
                variations: (source) => [
                    source.replace('---', '___'),
                    source.replace('## a', 'a\n---'),
                    source.replaceAll('\n\n', '\n\n\n'),
                    source.replaceAll('\n', '\r\n'),
                ],
            },
        ];
        for (const scenario of scenarios) {
            const region = [
                '--flow-region', '30000', '40000', '35000', scenario.height,
            ];
            const manual = [
                ...scenario.pageOne.flatMap((y) => [
                    '--place-text', '1', '32500', String(y), 'a',
                ]),
                ...scenario.titles.flatMap(({ y }) => [
                    '--place-text', '2', '32500', String(y), 'a',
                ]),
                ...scenario.text,
                '--ink-line', '2', '32500', '47000',
                '63500', '47000', '400', '#1E304A',
                ...scenario.titles.flatMap(({ y, lifts, width }) =>
                    lifts.flatMap((lift) => [
                        '--ink-line', '2', '32500', String(y + lift),
                        '36500', String(y + lift), String(width), '#F8DE91',
                    ])),
                ...scenario.ornaments,
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, scenario.source, format,
                        [...scenario.glyphs, ...region, ...options],
                        '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        `${scenario.label}: ${actual.result.stderr}`);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `${scenario.label}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...scenario.glyphs, ...manual, ...options,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${scenario.label} ${format} page ${page} geometry`);
                    const variations = scenario.variations(scenario.source);
                    for (const variant of variations) {
                        const replay = render(dir, variant, format,
                            [...scenario.glyphs, ...region, ...options],
                            '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `${scenario.label} equivalent source replay`);
                    }
                    const rejected = render(dir, scenario.source, format,
                        [...scenario.glyphs, ...region, ...options,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(rejected.result.status, 0);
                    assert.deepEqual(readFileSync(rejected.output), accepted,
                        'invalid selected page preserves accepted bytes');
                    const missing = render(dir, `${scenario.source} β`, format,
                        [...scenario.glyphs, ...region, ...options],
                        '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        'missing authored glyph preserves accepted bytes');
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('consecutive thematic rules retain fitting native PDF/PNG groups',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-rule-chain-'));
    try {
        const scenarios = [
            {
                label: 'quoted-rule-chain',
                source: '```\na\n```\n---\n***\n> aaaaaa aaaaaa aaaaaa',
                height: '50000',
                titles: [],
                rules: [47000, 56000],
                quoteRows: [60500, 69500, 78500],
                variations: (source) => [
                    source.replace('---', '___'),
                    source.replaceAll('\n', '\r\n'),
                ],
            },
            {
                label: 'gapped-rule-heading-quote',
                source: '```\na\n```\n---\n\n***\n# a\n> aaaaaa aaaaaa aaaaaa',
                height: '63000',
                titles: [64500],
                rules: [47000, 60000],
                quoteRows: [73500, 82500, 91500],
                variations: (source) => [
                    source.replace('---', '___'),
                    source.replace('\n\n', '\n\n\n'),
                    source.replaceAll('\n', '\r\n'),
                ],
            },
        ];
        for (const scenario of scenarios) {
            const region = [
                '--flow-region', '30000', '40000', '35000', scenario.height,
            ];
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                ...scenario.titles.flatMap((y) => [
                    '--place-text', '2', '32500', String(y), 'a',
                ]),
                ...scenario.quoteRows.flatMap((y) => [
                    '--place-text', '2', '38500', String(y), 'aaaaaa',
                ]),
                ...scenario.rules.flatMap((y) => [
                    '--ink-line', '2', '32500', String(y),
                    '63500', String(y), '400', '#1E304A',
                ]),
                ...scenario.titles.flatMap((y) =>
                    [900, 1900, 2900, 3900].flatMap((lift) => [
                        '--ink-line', '2', '32500', String(y + lift),
                        '36500', String(y + lift), '1400', '#F8DE91',
                    ])),
                ...scenario.quoteRows.flatMap((y) => [
                    '--ink-line', '2', '35500', String(y + 500),
                    '35500', String(y + 8400), '400', '#1E304A',
                ]),
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, scenario.source, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        `${scenario.label}: ${actual.result.stderr}`);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `${scenario.label}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...options, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `${scenario.label} native ${format} ${page}`);
                    for (const variant of
                        scenario.variations(scenario.source)) {
                        const replay = render(dir, variant, format,
                            [...region, ...options], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `${scenario.label} equivalent rule syntax`);
                    }
                    const badPage = render(dir, scenario.source, format,
                        [...region, ...options,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(badPage.result.status, 0);
                    assert.deepEqual(readFileSync(badPage.output), accepted);
                    const missing = render(dir, `${scenario.source} β`, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('consecutive rules reserve their first following native PDF/PNG row',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-rule-first-ink-'));
    try {
        for (const scenario of [
            {
                source: '```\na\n```\n---\n***\n> a', height: '32000',
                rules: [47000, 56000], row: 60500,
                variants: ['```\na\n```\n___\n* * *\n> a'],
            },
            {
                source: '```\na\n```\n---\n\n***\n\n> a',
                height: '40000', rules: [47000, 60000], row: 68500,
                variants: ['```\na\n```\n---\n\n\n***\n\n\n> a'],
            },
        ]) {
            const region = [
                '--flow-region', '30000', '40000', '35000',
                scenario.height,
            ];
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '38500', String(scenario.row), 'a',
                ...scenario.rules.flatMap((y) => [
                    '--ink-line', '2', '32500', String(y),
                    '63500', String(y), '400', '#1E304A',
                ]),
                '--ink-line', '2', '35500', String(scenario.row + 500),
                '35500', String(scenario.row + 8400), '400', '#1E304A',
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf' ? ['--pages', '2'] : [];
                    const actual = render(dir, scenario.source, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0, actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `rule-first-${scenario.height}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...options, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `rule-chain native ${format} page ${page}`);
                    for (const variant of [
                        ...scenario.variants,
                        scenario.source.replaceAll('\n', '\r\n'),
                    ]) {
                        const replay = render(dir, variant, format,
                            [...region, ...options], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `equivalent rule chain ${format} page ${page}`);
                    }
                    const missing = render(dir, `${scenario.source} β`, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        `refusal preserves ${format} page ${page}`);
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('terminal thematic-rule chains keep exact native PDF and PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-terminal-rules-'));
    try {
        for (const scenario of [
            {
                source: '```\na\n```\n---\n***\n___',
                height: '32000', centers: [47000, 56000, 65000],
            },
            {
                source: '```\na\n```\n---\n\n***\n\n___',
                height: '40000', centers: [47000, 60000, 73000],
            },
        ]) {
            const region = [
                '--flow-region', '30000', '40000', '35000',
                scenario.height,
            ];
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                ...scenario.centers.flatMap((y) => [
                    '--ink-line', '2', '32500', String(y),
                    '63500', String(y), '400', '#1E304A',
                ]),
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, scenario.source, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `terminal-${scenario.height}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...options, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `terminal chain ${format} page ${page}`);
                    for (const variant of [
                        scenario.source.replace('---', '- - -')
                            .replace('***', '* * *'),
                        scenario.source.replace('\n\n', '\n\n\n'),
                        scenario.source.replaceAll('\n', '\r\n'),
                    ]) {
                        const replay = render(dir, variant, format,
                            [...region, ...options], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `equivalent terminal chain ${format}`);
                    }
                    const invalid = render(dir, scenario.source, format,
                        [...region, ...options,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(invalid.result.status, 0);
                    assert.deepEqual(readFileSync(invalid.output), accepted,
                        'invalid page selection preserves accepted bytes');
                    const missing = render(dir,
                        `${scenario.source}\nβ`, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        'missing glyph preserves accepted bytes');
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('terminal heading and thematic rules share exact native PDF and PNG pages',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-heading-terminal-'));
    try {
        for (const scenario of [
            {
                source: '```\na\n```\n# a\n---\n***\n___',
                height: '41000', centers: [56000, 65000, 74000],
            },
            {
                source: '```\na\n```\n# a\n\n---\n\n***\n\n___',
                height: '53000', centers: [60000, 73000, 86000],
            },
        ]) {
            const region = [
                '--flow-region', '30000', '40000', '35000',
                scenario.height, '--heading-style', 'underline',
            ];
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '32500', '42500', 'a',
                '--ink-line', '2', '32500', '49200',
                '36500', '49200', '550', '#1E304A',
                ...scenario.centers.flatMap((y) => [
                    '--ink-line', '2', '32500', String(y),
                    '63500', String(y), '400', '#1E304A',
                ]),
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, scenario.source, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `heading-rules-${scenario.height}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...options, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `terminal heading+rules ${format} page ${page}`);
                    for (const variant of [
                        scenario.source.replace('# a', 'a\n==='),
                        scenario.source.replace('---', '- - -')
                            .replace('***', '* * *'),
                        scenario.source.replace('\n\n', '\n\n\n'),
                        scenario.source.replaceAll('\n', '\r\n'),
                    ]) {
                        const replay = render(dir, variant, format,
                            [...region, ...options], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `equivalent heading+rules ${format}`);
                    }
                    const invalid = render(dir, scenario.source, format,
                        [...region, ...options,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(invalid.result.status, 0);
                    assert.deepEqual(readFileSync(invalid.output), accepted,
                        'invalid page selection preserves accepted bytes');
                    const missing = render(dir,
                        `${scenario.source}\nβ`, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        'missing glyph preserves accepted bytes');
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});


test('adjacent terminal headings and rules match independent native pages',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-terminal-title-run-'));
    try {
        for (const scenario of [
            {
                source: '```\na\n```\n# a\n## a\n---\n***',
                height: '41000', second: 51500,
                headingWidth: '420', centers: [65000, 74000],
            },
            {
                source: '```\na\n```\n# a\n\n## a\n\n---\n\n***',
                height: '53000', second: 55500,
                headingWidth: '420', centers: [73000, 86000],
            },
            {
                source: '```\na\n```\n# a\na\n===\n---\n***',
                height: '41000', second: 51500,
                headingWidth: '550', centers: [65000, 74000],
            },
            {
                source: '```\na\n```\n# a\n## a\n---',
                height: '32000', second: 51500,
                headingWidth: '420', centers: [65000],
            },
        ]) {
            const region = [
                '--flow-region', '30000', '40000', '35000',
                scenario.height, '--heading-style', 'underline',
            ];
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '32500', '42500', 'a',
                '--place-text', '2', '32500', String(scenario.second), 'a',
                '--ink-line', '2', '32500', '49200',
                '36500', '49200', '550', '#1E304A',
                '--ink-line', '2', '32500',
                String(scenario.second + 6700), '36500',
                String(scenario.second + 6700),
                scenario.headingWidth, '#1E304A',
                ...scenario.centers.flatMap((y) => [
                    '--ink-line', '2', '32500', String(y),
                    '63500', String(y), '400', '#1E304A',
                ]),
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, scenario.source, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `terminal-title-${scenario.height}-${scenario.second}`
                        + `-${scenario.headingWidth}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...options, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `adjacent title and rules ${format} page ${page}`);
                    for (const variant of [
                        scenario.source.replace('# a', 'a\n==='),
                        scenario.source.replace('---', '- - -')
                            .replace('***', '* * *'),
                        scenario.source.replace('\n\n', '\n\n\n'),
                        scenario.source.replaceAll('\n', '\r\n'),
                    ]) {
                        const replay = render(dir, variant, format,
                            [...region, ...options], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `equivalent terminal title chain ${format}`);
                    }
                    const refused = render(dir,
                        `${scenario.source}\nβ`, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.notEqual(refused.result.status, 0);
                    assert.deepEqual(readFileSync(refused.output), accepted,
                        'missing glyph cannot replace admitted page');
                    const invalid = render(dir, scenario.source, format,
                        [...region, ...options,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(invalid.result.status, 0);
                    assert.deepEqual(readFileSync(invalid.output), accepted,
                        'invalid selected sheet preserves accepted bytes');
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('interleaved terminal headings and separators keep exact PDF/PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-interleaved-tail-'));
    try {
        for (const scenario of [
            {
                source: '```\na\n```\n# a\n---\n## a',
                height: '32000', rule: 56000, second: 60500,
            },
            {
                source: '```\na\n```\n# a\n\n---\n\n## a',
                height: '40000', rule: 60000, second: 68500,
            },
        ]) {
            const region = [
                '--flow-region', '30000', '40000', '35000',
                scenario.height, '--heading-style', 'underline',
            ];
            const secondUnderline = scenario.second + 6700;
            const manual = [
                '--place-text', '1', '32500', '42500', 'a',
                '--place-text', '2', '32500', '42500', 'a',
                '--place-text', '2', '32500', String(scenario.second), 'a',
                '--ink-line', '2', '32500', '49200',
                '36500', '49200', '550', '#1E304A',
                '--ink-line', '2', '32500', String(scenario.rule),
                '63500', String(scenario.rule), '400', '#1E304A',
                '--ink-line', '2', '32500', String(secondUnderline),
                '36500', String(secondUnderline), '420', '#1E304A',
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, scenario.source, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `interleaved-${scenario.height}-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...options, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `interleaved terminal ${format} sheet ${page}`);
                    for (const variant of [
                        scenario.source.replace('# a', 'a\n==='),
                        scenario.source.replace('## a', 'a\n---'),
                        scenario.source.replace('\n---\n', '\n* * *\n'),
                        scenario.source.replaceAll('\n\n', '\n\n\n'),
                        scenario.source.replaceAll('\n', '\r\n'),
                    ]) {
                        const replay = render(dir, variant, format,
                            [...region, ...options], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `equivalent interleaved ${format} page ${page}`);
                    }
                    const invalid = render(dir, scenario.source, format,
                        [...region, ...options,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(invalid.result.status, 0);
                    assert.deepEqual(readFileSync(invalid.output), accepted,
                        'invalid selected page cannot replace PDF/PNG');
                    const missing = render(dir,
                        `${scenario.source}\nβ`, format,
                        [...region, ...options], '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        'missing authored glyph cannot replace PDF/PNG');
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('all six interleaved heading levels keep exact terminal PDF/PNG ink',
    () => {
    const dir = mkdtempSync(join(root, '.temp/flow-six-title-levels-'));
    try {
        const source = [
            '```', 'a', '```', '# a', '---', '## a', '***',
            '### a', '___', '#### a', '* * *', '##### a',
            '- - -', '###### a',
        ].join('\n');
        const region = [
            '--flow-region', '30000', '40000', '35000', '104000',
            '--heading-style', 'underline',
        ];
        const headingY = [42500, 60500, 78500, 96500, 114500, 132500];
        const widths = [550, 420, 300, 240, 180, 120];
        const ruleY = [56000, 74000, 92000, 110000, 128000];
        const manual = [
            '--place-text', '1', '32500', '42500', 'a',
            ...headingY.flatMap((y) => [
                '--place-text', '2', '32500', String(y), 'a',
            ]),
            ...headingY.flatMap((y, i) => [
                '--ink-line', '2', '32500', String(y + 6700),
                '36500', String(y + 6700), String(widths[i]), '#1E304A',
                ...(i < ruleY.length ? [
                    '--ink-line', '2', '32500', String(ruleY[i]),
                    '63500', String(ruleY[i]), '400', '#1E304A',
                ] : []),
            ]),
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const pages = format === 'pdf' ? ['--pages', '2'] : [];
                const extra = [...region, ...pages];
                const actual = render(dir, source, format,
                    extra, '--flow-markdown', page);
                assert.equal(actual.result.status, 0, actual.result.stderr);
                const accepted = readFileSync(actual.output);
                const reference = join(dir, `six-level-${format}-${page}`);
                const expected = spawnSync(cargo, [
                    'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                    '--', ...base, ...manual, ...pages,
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72'] : []),
                    '--output', reference,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(expected.status, 0, expected.stderr);
                assert.deepEqual(accepted, readFileSync(reference),
                    `six heading levels page ${page} ${format}`);
                for (const variant of [
                    source.replace('***', '* * *').replace('___', '_ _ _'),
                    source.replaceAll('\n', '\r\n'),
                ]) {
                    const replay = render(dir, variant, format,
                        extra, '--flow-markdown', page);
                    assert.equal(replay.result.status, 0,
                        replay.result.stderr);
                    assert.deepEqual(readFileSync(replay.output), accepted,
                        `equivalent six-title ${format} page ${page}`);
                }
                const invalid = render(dir, source, format,
                    [...extra, ...(format === 'pdf'
                        ? ['--pages', '1'] : [])],
                    '--flow-markdown', format === 'pdf' ? 1 : 3);
                assert.notEqual(invalid.result.status, 0);
                assert.deepEqual(readFileSync(invalid.output), accepted,
                    'invalid selected sheet preserves native output');
                const missing = render(dir, `${source}\nβ`, format,
                    extra, '--flow-markdown', page);
                assert.notEqual(missing.result.status, 0);
                assert.deepEqual(readFileSync(missing.output), accepted,
                    'unauthored glyph cannot replace native output');
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('equivalent mixed Markdown syntax preserves native pages and ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-markdown-equivalence-'));
        try {
            const first = [
                '```', 'a', '```', '', '# a', '---', '## a', '',
                '- [x] a a', '', '> a a', '', '- a a',
            ].join('\n');
            const second = [
                '~~~', 'a', '~~~', '', 'a', '===', '* * *',
                'a', '---', '', '+ [X] a a', '', '  > a a',
                '', '+ a a',
            ].join('\n');
            const region = [
                '--flow-region', '30000', '40000', '35000', '55000',
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? [...region, '--pages', '2'] : region;
                    const expected = render(dir, first, format,
                        options, '--flow-markdown', page);
                    assert.equal(expected.result.status, 0,
                        expected.result.stderr);
                    const reference = readFileSync(expected.output);
                    if (format === 'pdf') {
                        assert.match(reference.toString('latin1'),
                            /\/Count 2/);
                    }
                    const alternate = render(dir, second, format,
                        options, '--flow-markdown', page);
                    assert.equal(alternate.result.status, 0,
                        alternate.result.stderr);
                    assert.deepEqual(readFileSync(alternate.output), reference,
                        `equivalent mixed syntax page ${page} ${format}`);
                    const invalid = second.replace('> a a', '> a z');
                    const missing = render(dir, invalid, format,
                        options, '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.match(missing.result.stderr,
                        /missing authored glyph shapes.*U\+007A/);
                    assert.deepEqual(readFileSync(missing.output), reference,
                        'invalid glyph may not replace accepted output');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('chained blank quote paragraphs match native PDF and both PNG pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-quote-chain-'));
        try {
            const source = '> a\n\n> ó\n\n> a';
            const region = [
                '--flow-region', '30000', '40000', '35000', '27000',
            ];
            const manual = [
                '--place-text', '1', '38500', '42500', 'a',
                '--place-text', '1', '38500', '55500', 'ó',
                '--place-text', '2', '38500', '42500', 'a',
                '--ink-line', '1', '35500', '43000',
                '35500', '50900', '400', '#1E304A',
                '--ink-line', '1', '35500', '56000',
                '35500', '63900', '400', '#1E304A',
                '--ink-line', '2', '35500', '43000',
                '35500', '50900', '400', '#1E304A',
            ];
            const alternatives = [
                '> a\n>\n> ó\n>\n> a',
                '> a\n>\n\n> ó\n\n>\n> a',
                '> a\n\n>\n> ó\n>\n\n> a',
                '> a\n>\n\n>\n> ó\n>\n\n>\n> a',
                '  > a\n  >\n\n> ó\n\n>\n  > a',
                source.replaceAll('\n', '\r\n'),
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const pages = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const actual = render(dir, source, format,
                        [...region, ...pages], '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const manualPath = join(dir,
                        `chained-quote-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...pages, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', manualPath,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(manualPath),
                        `three quote rows physically match page ${page}`);
                    for (const variant of alternatives) {
                        const replay = render(dir, variant, format,
                            [...region, ...pages], '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `chained gaps preserve ${format} page ${page}`);
                    }
                    const missing = render(dir, `${source}\n> β`, format,
                        [...region, ...pages], '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        'missing glyph must not replace selected export');
                    const invalid = render(dir, source, format,
                        [...region, ...pages,
                            ...(format === 'pdf' ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(invalid.result.status, 0);
                    assert.deepEqual(readFileSync(invalid.output), accepted,
                        'invalid page selection preserves selected export');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('quoted hard-break variants preserve independent two-page PDF/PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-quote-break-pages-'));
        try {
            const source = '> a\\\n> ó';
            const region = [
                '--flow-region', '30000', '40000', '35000', '14000',
            ];
            const manual = [
                '--place-text', '1', '38500', '42500', 'a',
                '--place-text', '2', '38500', '42500', 'ó',
                '--ink-line', '1', '35500', '43000',
                '35500', '50900', '400', '#1E304A',
                '--ink-line', '2', '35500', '43000',
                '35500', '50900', '400', '#1E304A',
            ];
            const variants = [
                '> a  \n> ó', '> a\u2028> ó',
                source.replaceAll('\n', '\r\n'),
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const pages = format === 'pdf' ? ['--pages', '2'] : [];
                    const options = [...region, ...pages];
                    const actual = render(dir, source, format,
                        options, '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `quote-break-manual-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...pages, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `physical quote ink, ${format} page ${page}`);
                    for (const variant of variants) {
                        const replay = render(dir, variant, format,
                            options, '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            `same quoted ink for ${variant} in ${format}`);
                    }
                    const missing = render(dir, `${source}\n> β`, format,
                        options, '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.deepEqual(readFileSync(missing.output), accepted,
                        'missing glyph must preserve selected export');
                    const invalid = render(dir, source, format,
                        [...options, ...(format === 'pdf'
                            ? ['--pages', '1'] : [])],
                        '--flow-markdown', format === 'pdf' ? 1 : 3);
                    assert.notEqual(invalid.result.status, 0);
                    assert.deepEqual(readFileSync(invalid.output), accepted,
                        'invalid page selection preserves accepted output');
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('intervening Markdown block cannot authorize empty quote gap export',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-quote-boundary-'));
        try {
            const valid = '> a\n>\n> ó';
            const region = [
                '--flow-region', '30000', '40000', '35000', '14000',
            ];
            const boundaries = [
                ['rule', '> a\n---\n>\n> ó'],
                ['heading', '> a\n# a\n>\n> ó'],
                ['indented text', '> a\n    a\n>\n> ó'],
                ['fenced code', '> a\n```\na\n```\n>\n> ó'],
                ['empty fence', '> a\n```\n```\n>\n> ó'],
                ['heading and blank', '> a\n# a\n\n>\n> ó'],
                ['ordinary paragraph', '> a\na\n>\n> ó'],
                ['Setext heading', '> a\na\n===\n>\n> ó'],
                ['numbered item', '> a\n1. a\n>\n> ó'],
                ['numbered continuation',
                    '> a\n1. a\n  a\n>\n> ó'],
                ['checked task', '> a\n- [x] a\n>\n> ó'],
                ['escaped quote', '> a\n\\> a\n>\n> ó'],
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = [
                        ...region,
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                    ];
                    const accepted = render(dir, valid, format,
                        options, '--flow-markdown', page);
                    assert.equal(accepted.result.status, 0,
                        accepted.result.stderr);
                    const original = readFileSync(accepted.output);
                    for (const [boundary, sourceText] of boundaries) {
                        for (const newline of ['\n', '\r\n']) {
                            const refused = render(dir,
                                sourceText.replaceAll('\n', newline),
                                format, options, '--flow-markdown', page);
                            assert.notEqual(refused.result.status, 0,
                                `${boundary} ${format} ${page} ${newline}`);
                            assert.match(refused.result.stderr,
                                /empty Markdown block refused/);
                            assert.deepEqual(readFileSync(refused.output),
                                original,
                                `${boundary}: ${format} page ${page}`);
                        }
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('hard-break-only quote cannot become a silent separator in PDF/PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-quote-empty-break-'));
        try {
            const source = '> a\n>\n> ó';
            const region = [
                '--flow-region', '30000', '40000', '35000', '14000',
            ];
            const manual = [
                '--place-text', '1', '38500', '42500', 'a',
                '--place-text', '2', '38500', '42500', 'ó',
                '--ink-line', '1', '35500', '43000',
                '35500', '50900', '400', '#1E304A',
                '--ink-line', '2', '35500', '43000',
                '35500', '50900', '400', '#1E304A',
            ];
            const valid = [
                '> a\n>  \n> ó',
                '> a\n>\n\n> ó',
                source.replaceAll('\n', '\r\n'),
            ];
            const invalid = [
                ['> a\n>\\\n> ó', /empty Markdown block refused/],
                ['> a\n> \\\n> ó', /empty Markdown block refused/],
                ['> a\n>\\  \n> ó',
                    /missing authored glyph shapes.*U\+005C/],
                ['> a\n  >\\\n> ó', /empty Markdown block refused/],
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const pages = format === 'pdf' ? ['--pages', '2'] : [];
                    const options = [...region, ...pages];
                    const actual = render(dir, source, format,
                        options, '--flow-markdown', page);
                    assert.equal(actual.result.status, 0,
                        actual.result.stderr);
                    const accepted = readFileSync(actual.output);
                    const reference = join(dir,
                        `quote-empty-break-reference-${format}-${page}`);
                    const expected = spawnSync(cargo, [
                        'run', '--quiet', '-p',
                        'atrament_profile_editor_cli', '--', ...base,
                        ...manual, ...pages, '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []), '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(expected.status, 0, expected.stderr);
                    assert.deepEqual(accepted, readFileSync(reference),
                        `manual physical quote ink, ${format} page ${page}`);
                    for (const sourceText of valid) {
                        const replay = render(dir, sourceText, format,
                            options, '--flow-markdown', page);
                        assert.equal(replay.result.status, 0,
                            replay.result.stderr);
                        assert.deepEqual(readFileSync(replay.output), accepted,
                            'valid empty quote separators retain exact ink');
                    }
                    for (const [sourceText, expectedError] of invalid) {
                        const refused = render(dir, sourceText, format,
                            options, '--flow-markdown', page);
                        assert.notEqual(refused.result.status, 0);
                        assert.match(refused.result.stderr, expectedError);
                        assert.deepEqual(readFileSync(refused.output), accepted,
                            'backslash-only quote must never replace output');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('Setext and ATX quoted gaps retain identical physical PDF/PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-quote-heading-'));
    try {
        const region = [
            '--flow-region', '30000', '40000', '35000', '32000',
        ];
        const pairs = [
            ['# a\n> a\n>\n> ó', 'a\n===\n> a\n>\n> ó'],
            ['## a\n> a\n>\n> ó', 'a\n---\n> a\n>\n> ó'],
            ['---\n# a\n> a\n>\n> ó',
                '---\na\n===\n> a\n>\n> ó'],
        ];
        for (const format of ['pdf', 'png']) {
            for (const page of format === 'pdf' ? [1] : [1, 2]) {
                const extra = [...region,
                    ...(format === 'pdf' ? ['--pages', '2'] : [])];
                for (const [atx, setext] of pairs) {
                    for (const newline of ['\n', '\r\n']) {
                        const first = render(dir,
                            atx.replaceAll('\n', newline),
                            format, extra, '--flow-markdown', page);
                        assert.equal(first.result.status, 0,
                            first.result.stderr);
                        const accepted = readFileSync(first.output);
                        const second = render(dir,
                            setext.replaceAll('\n', newline),
                            format, extra, '--flow-markdown', page);
                        assert.equal(second.result.status, 0,
                            second.result.stderr);
                        assert.deepEqual(readFileSync(second.output),
                            accepted, `${format} page ${page}: ${atx}`);
                    }
                }
            }
        }
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('wrapped quotes follow ATX and Setext titles on physical PDF/PNG pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-setext-wrap-'));
        try {
            const region = [
                '--flow-region', '30000', '40000', '35000', '32000',
                '--heading-style', 'underline',
            ];
            const sources = [
                'a\n\n# a\n> a a a a a a',
                'a\n\na\n===\n> a a a a a a',
                'a\r\n\r\na\r\n===\r\n  > a a a a a a',
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const exportOptions = format === 'pdf'
                        ? ['--pages', '2'] : [];
                    const first = render(dir, sources[0], format,
                        [...region, ...exportOptions],
                        '--flow-markdown', page);
                    assert.equal(first.result.status, 0, first.result.stderr);
                    const accepted = readFileSync(first.output);
                    for (const source of sources.slice(1)) {
                        const alternate = render(dir, source, format,
                            [...region, ...exportOptions],
                            '--flow-markdown', page);
                        assert.equal(alternate.result.status, 0,
                            alternate.result.stderr);
                        assert.deepEqual(readFileSync(alternate.output),
                            accepted, `title syntax / page ${page}`);
                    }
                    const target = join(dir,
                        `manually-placed-quote-${page}.${format}`);
                    const manual = spawnSync(cargo, [
                        'run', '--quiet', '-p', 'atrament_profile_editor_cli',
                        '--', ...base,
                        '--place-text', '1', '32500', '42500', 'a',
                        '--place-text', '2', '32500', '42500', 'a',
                        '--place-text', '2', '38500', '51500', 'a a a',
                        '--place-text', '2', '38500', '60500', 'a a a',
                        '--ink-line', '2', '32500', '49200',
                        '36500', '49200', '550', '#1E304A',
                        '--ink-line', '2', '35500', '52000',
                        '35500', '59900', '400', '#1E304A',
                        '--ink-line', '2', '35500', '61000',
                        '35500', '68900', '400', '#1E304A',
                        ...exportOptions,
                        '--export-paper', format,
                        ...(format === 'png'
                            ? ['--png-page', String(page), '--dpi', '72']
                            : []),
                        '--output', target,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(manual.status, 0, manual.stderr);
                    assert.deepEqual(accepted, readFileSync(target),
                        `independently placed page ${page} ${format}`);
                    for (const invalid of [
                        'a\n\n# a\n> a a a a a a\n> \\',
                        'a\n\n# a\n> a a a a a a\n>\n---',
                    ]) {
                        const refused = render(dir, invalid, format,
                            [...region, ...exportOptions],
                            '--flow-markdown', page);
                        assert.notEqual(refused.result.status, 0);
                        assert.match(refused.result.stderr,
                            /empty Markdown block refused/);
                        assert.deepEqual(readFileSync(refused.output),
                            accepted, 'invalid quote cannot replace ink');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('invalid fixed rectangles preserve accepted PDF and selected PNG pages',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-fixed-region-'));
        try {
            const cases = [
                [['30000', '40000', '0', '14000'], /must be positive/],
                [['30000', '40000', '14000', '0'], /must be positive/],
                [['30000', '40000', '4294967295', '14000'],
                    /right edge overflows/],
                [['30000', '40000', '14000', '4294967295'],
                    /bottom edge overflows/],
                [['0', '40000', '14000', '14000'],
                    /fit within printable paper margins/],
            ];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const extra = format === 'pdf' ? ['--pages', '2'] : [];
                    const accepted = render(dir, 'a\n# a', format,
                        ['--flow-region', '30000', '40000',
                            '14000', '14000', ...extra],
                        '--flow-markdown', page);
                    assert.equal(accepted.result.status, 0,
                        accepted.result.stderr);
                    const original = readFileSync(accepted.output);
                    for (const [dimensions, diagnostic] of cases) {
                        const refused = render(dir, 'a\n# a', format,
                            ['--flow-region', ...dimensions, ...extra],
                            '--flow-markdown', page);
                        assert.notEqual(refused.result.status, 0,
                            `${format} page ${page}: ${dimensions}`);
                        assert.match(refused.result.stderr, diagnostic);
                        assert.deepEqual(readFileSync(refused.output), original,
                            `region ${dimensions} changed ${format}/${page}`);
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('unprinted Markdown markers cannot inflate native PDF/PNG row pitch',
    () => {
        const dir = mkdtempSync(join(root, '.temp/syntax-only-pitch-'));
        try {
            const statistics = join(dir, 'tall-syntax.json');
            const height = ['25000', '25000', '25000', '25000', '0'];
            const configured = spawnSync(cargo, [
                'run', '--offline', '--quiet',
                '-p', 'atrament_profile_editor_cli', '--', 'handwriting',
                '--set-glyph', '#', 'height', ...height,
                '--set-glyph', '-', 'height', ...height,
                '--set-glyph', '+', 'height', ...height,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(statistics, configured.stdout);

            // 9 mm of physical ink fits exactly between 2.5 mm insets.
            // If a syntax-only '#' or bullet marker falsely applies its
            // 25 mm height, neither of these two one-row pages will fit.
            const region = [
                '--handwriting-profile', statistics,
                '--flow-region', '30000', '40000', '40000', '14000',
                '--heading-style', 'none',
            ];
            const firstSource = '# a\n\n- a';
            const alternateSource = '## a\n\n+ a';
            const literalHash = '\\# a\n\n- a';
            const pngPages = [];
            for (const format of ['pdf', 'png']) {
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const options = format === 'pdf'
                        ? [...region, '--pages', '2'] : region;
                    const first = render(dir, firstSource, format,
                        options, '--flow-markdown', page);
                    assert.equal(first.result.status, 0, first.result.stderr);
                    const expected = readFileSync(first.output);
                    if (format === 'pdf') {
                        assert.match(expected.toString('latin1'), /\/Count 2/);
                    } else {
                        assert.deepEqual(expected.subarray(0, 8),
                            Buffer.from('89504e470d0a1a0a', 'hex'));
                        pngPages.push(expected);
                    }
                    const alternate = render(dir, alternateSource, format,
                        options, '--flow-markdown', page);
                    assert.equal(alternate.result.status, 0,
                        alternate.result.stderr);
                    assert.deepEqual(readFileSync(alternate.output), expected,
                        `${format} page ${page}: syntax does not shift ink`);

                    // An escaped hash really is tall authored ink. It
                    // cannot fit a 14-mm row, even before glyph approval.
                    const refused = render(dir, literalHash, format,
                        options, '--flow-markdown', page);
                    assert.notEqual(refused.result.status, 0);
                    assert.match(refused.result.stderr,
                        /flow line cannot fit printable vertical region/);
                    assert.deepEqual(readFileSync(refused.output), expected,
                        'unfit literal ink must not replace output');

                    // With sufficient height, the missing authored hash
                    // still refuses before changing the selected file.
                    const roomy = [
                        '--handwriting-profile', statistics,
                        '--flow-region', '30000', '40000', '40000',
                        '35000', '--heading-style', 'none',
                        ...(format === 'pdf' ? ['--pages', '2'] : []),
                    ];
                    const missing = render(dir, literalHash, format,
                        roomy, '--flow-markdown', page);
                    assert.notEqual(missing.result.status, 0);
                    assert.match(missing.result.stderr,
                        /missing authored glyph shapes.*U\+0023/);
                    assert.deepEqual(readFileSync(missing.output), expected,
                        'missing literal glyph must not replace output');
                }
            }
            assert.notDeepEqual(pngPages[0], pngPages[1],
                'first and second physical sheets contain distinct ink');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('explicit stdin Markdown matches inline and saved native PDF/PNG', () => {
    const dir = mkdtempSync(join(root, '.temp/flow-stdin-markdown-'));
    try {
        const source = '# a\n\na a\n- ó\n';
        const raw = Buffer.from('\uFEFF' + source.replaceAll('\n', '\r\n'));
        const file = join(dir, 'saved.md');
        writeFileSync(file, raw);
        for (const format of ['pdf', 'png']) {
            const inline = render(dir, source, format);
            assert.equal(inline.result.status, 0, inline.result.stderr);
            const expected = readFileSync(inline.output);
            for (const [selected, input] of [
                [file, undefined],
                ['-', raw],
            ]) {
                const piped = render(dir, selected, format, [],
                    '--flow-markdown-file', 1, input);
                assert.equal(piped.result.status, 0, piped.result.stderr);
                assert.deepEqual(readFileSync(piped.output), expected,
                    `${format} identical saved and piped authored ink`);
            }
        }
        assert.deepEqual(readFileSync(file), raw);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('plain stdin keeps exact native line breaks and selected PNG page',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-stdin-plain-'));
        try {
            const source = 'a\nó';
            const region = [
                '--flow-region', '30000', '40000', '30000', '14000',
            ];
            for (const format of ['pdf', 'png']) {
                const extra = [...region, ...(format === 'pdf'
                    ? ['--pages', '2'] : [])];
                for (const page of format === 'pdf' ? [1] : [1, 2]) {
                    const inline = render(dir, source, format, extra,
                        '--flow-text', page);
                    assert.equal(inline.result.status, 0,
                        inline.result.stderr);
                    const expected = readFileSync(inline.output);
                    const piped = render(dir, '-', format, extra,
                        '--flow-text-file', page, Buffer.from('a\r\nó'));
                    assert.equal(piped.result.status, 0,
                        piped.result.stderr);
                    assert.deepEqual(readFileSync(piped.output), expected,
                        `${format} page ${page} unchanged line breaks`);
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('invalid stdin flow refuses before replacing accepted PDF and PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-stdin-refusal-'));
        try {
            for (const format of ['pdf', 'png']) {
                const accepted = render(dir, 'a', format);
                assert.equal(accepted.result.status, 0,
                    accepted.result.stderr);
                const expected = readFileSync(accepted.output);
                for (const [raw, diagnostic] of [
                    [Buffer.alloc(4097, 0x61), /flow stdin exceeds 4096/],
                    [Buffer.from([0xff]), /flow stdin must be UTF-8/],
                    [Buffer.from('a\u0000'), /control characters/],
                    [Buffer.from('  \n\t'), /visible text/],
                ]) {
                    const refused = render(dir, '-', format, [],
                        '--flow-markdown-file', 1, raw);
                    assert.notEqual(refused.result.status, 0);
                    assert.match(refused.result.stderr, diagnostic);
                    assert.deepEqual(readFileSync(refused.output), expected,
                        'invalid stdin cannot overwrite accepted output');
                }
                const invalidFile = join(dir, 'invalid-utf8.md');
                writeFileSync(invalidFile, Buffer.from([0xff]));
                const refusedFile = render(dir, invalidFile, format, [],
                    '--flow-markdown-file');
                assert.notEqual(refusedFile.result.status, 0);
                assert.match(refusedFile.result.stderr,
                    /flow file must be UTF-8/);
                assert.deepEqual(readFileSync(refusedFile.output), expected,
                    'invalid selected file cannot overwrite accepted output');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('bounded stdin retains exact UTF-8 byte limit and expansion refusals',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-stdin-byte-cap-'));
        try {
            const visible = 'ó';
            const exact = Buffer.from('\uFEFF' + visible
                + ' '.repeat(4096 - 3 - Buffer.byteLength(visible)));
            assert.equal(exact.length, 4096,
                'UTF-8 signature and accented glyph count as source bytes');
            const overlong = Buffer.concat([exact, Buffer.from(' ')]);
            const exactExpansion = Buffer.from('a' + '\t'.repeat(1024));
            const expanding = Buffer.from('a' + '\t'.repeat(1025));
            assert.ok(expanding.length < 4096,
                'expanded tab refusal is distinct from source byte cap');
            for (const format of ['pdf', 'png']) {
                for (const [fileFlag, inlineFlag] of [
                    ['--flow-markdown-file', '--flow-markdown'],
                    ['--flow-text-file', '--flow-text'],
                ]) {
                    const baseline = render(dir, visible, format, [],
                        inlineFlag);
                    assert.equal(baseline.result.status, 0,
                        baseline.result.stderr);
                    const expected = readFileSync(baseline.output);
                    const accepted = render(dir, '-', format, [],
                        fileFlag, 1, exact);
                    assert.equal(accepted.result.status, 0,
                        accepted.result.stderr);
                    assert.deepEqual(readFileSync(accepted.output), expected,
                        `${format} ${fileFlag} exact source byte cap`);
                    const tabReference = render(dir, 'a', format, [],
                        inlineFlag);
                    assert.equal(tabReference.result.status, 0,
                        tabReference.result.stderr);
                    const tabExpected = readFileSync(tabReference.output);
                    const tabsAccepted = render(dir, '-', format, [],
                        fileFlag, 1, exactExpansion);
                    assert.equal(tabsAccepted.result.status, 0,
                        tabsAccepted.result.stderr);
                    assert.deepEqual(readFileSync(tabsAccepted.output),
                        tabExpected,
                        'exact expanded 4096-byte tab cap is admitted');
                    for (const [input, diagnostic] of [
                        [overlong, /flow stdin exceeds 4096 UTF-8 bytes/],
                        [expanding, /expanded flow text exceeds 4096/],
                    ]) {
                        const refused = render(dir, '-', format, [],
                            fileFlag, 1, input);
                        assert.notEqual(refused.result.status, 0);
                        assert.match(refused.result.stderr, diagnostic);
                        assert.deepEqual(readFileSync(refused.output),
                            tabExpected,
                            'source or expansion refusal preserves export');
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('piped Markdown stdout matches native PDF and second-page PNG files',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-pipe-stdout-'));
        try {
            const markdown = '# a\n\n- a';
            const transported = Buffer.from('\uFEFF# a\r\n\r\n- a');
            const source = join(dir, 'source.md');
            writeFileSync(source, transported);
            const region = [
                '--flow-region', '30000', '40000', '30000', '14000',
            ];
            for (const [format, page] of [
                ['pdf', 1], ['png', 1], ['png', 2],
            ]) {
                const flags = [...region, ...(format === 'pdf'
                    ? ['--pages', '2'] : [])];
                const reference = render(dir, markdown, format, flags,
                    '--flow-markdown', page);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const expected = readFileSync(reference.output);
                const stdoutArgs = [
                    ...base, ...headingOptions, ...flags,
                    '--flow-markdown-file', '-', '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72'] : []),
                ];
                const piped = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--', ...stdoutArgs,
                ], { cwd: root, input: transported, timeout: 120000,
                    maxBuffer: 24 * 1024 * 1024 });
                assert.equal(piped.error, undefined, piped.error?.message);
                assert.equal(piped.status, 0, piped.stderr.toString());
                assert.deepEqual(piped.stdout, expected,
                    `${format} page ${page} is binary-clean on stdout`);

                const savedArgs = [...stdoutArgs];
                savedArgs[savedArgs.indexOf('--flow-markdown-file') + 1] =
                    source;
                const saved = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--',
                    ...savedArgs,
                ], { cwd: root, timeout: 120000,
                    maxBuffer: 24 * 1024 * 1024 });
                assert.equal(saved.error, undefined, saved.error?.message);
                assert.equal(saved.status, 0, saved.stderr.toString());
                assert.deepEqual(saved.stdout, expected,
                    `${format} page ${page} saved-source stdout parity`);
            }
            assert.deepEqual(readFileSync(source), transported,
                'selected source is never rewritten');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('invalid piped flow emits no partial native PDF or PNG on stdout',
    () => {
        const baseArgs = [...base, '--flow-region', '30000', '40000',
            '30000', '14000', '--flow-markdown-file', '-'];
        for (const [format, extra] of [
            ['pdf', ['--pages', '2']],
            ['png', ['--png-page', '2', '--dpi', '72']],
        ]) {
            for (const [input, diagnostic] of [
                [Buffer.from([0xff]), /flow stdin must be UTF-8/],
                [Buffer.alloc(4097, 0x61), /flow stdin exceeds 4096/],
                [Buffer.from('# a\n\n- b'), /missing authored glyph shapes/],
                [Buffer.from('# \n\n- a'), /empty Markdown block refused/],
            ]) {
                const result = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--',
                    ...baseArgs, '--export-paper', format, ...extra,
                ], { cwd: root, input, timeout: 120000,
                    maxBuffer: 24 * 1024 * 1024 });
                assert.equal(result.error, undefined, result.error?.message);
                assert.notEqual(result.status, 0,
                    `${format} must reject source without stdout bytes`);
                assert.match(result.stderr.toString(), diagnostic);
                assert.equal(result.stdout.length, 0,
                    `${format} cannot stream output before admission`);
            }
        }
    });

test('plain piped stdout retains physical breaks in PDF and second PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-plain-stdout-'));
        try {
            const source = 'a\nó';
            const region = [
                '--flow-region', '30000', '40000', '30000', '14000',
            ];
            for (const [format, page] of [
                ['pdf', 1], ['png', 1], ['png', 2],
            ]) {
                const args = [...base, ...region,
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--flow-text-file', '-', '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72'] : []),
                ];
                const reference = render(dir, source, format,
                    [...region, ...(format === 'pdf'
                        ? ['--pages', '2'] : [])], '--flow-text', page);
                assert.equal(reference.result.status, 0,
                    reference.result.stderr);
                const piped = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--', ...args,
                ], { cwd: root, input: Buffer.from('a\r\nó'),
                    timeout: 120000, maxBuffer: 24 * 1024 * 1024 });
                assert.equal(piped.error, undefined, piped.error?.message);
                assert.equal(piped.status, 0, piped.stderr.toString());
                assert.deepEqual(piped.stdout, readFileSync(reference.output),
                    `${format} page ${page}: plain stdin stdout parity`);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('32-page native flow ceiling preserves final PNG ink and output',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-page-ceiling-'));
        try {
            const oneRow = ['--flow-region', '30000', '40000',
                '30000', '14000'];
            const thirtyTwo = [...Array(31).fill('a'), 'ó'].join('\n');
            const pdf = render(dir, thirtyTwo, 'pdf',
                [...oneRow, '--pages', '32'], '--flow-text');
            assert.equal(pdf.result.status, 0, pdf.result.stderr);
            const acceptedPdf = readFileSync(pdf.output);
            const pages = [...acceptedPdf.toString('latin1')
                .matchAll(/\/Type \/Page\b/g)].length;
            assert.equal(pages, 32, 'one measured row per PDF page');
            const replay = render(dir, thirtyTwo, 'pdf',
                [...oneRow, '--pages', '32'], '--flow-text');
            assert.equal(replay.result.status, 0, replay.result.stderr);
            assert.deepEqual(readFileSync(pdf.output), acceptedPdf);

            for (const [source, extra, error] of [
                [thirtyTwo, [...oneRow, '--pages', '31'],
                    /requires 32 physical pages/],
                [[...Array(33).fill('a')].join('\n'), oneRow,
                    /exceeds 32 physical pages/],
            ]) {
                const refused = render(dir, source, 'pdf', extra,
                    '--flow-text');
                assert.notEqual(refused.result.status, 0);
                assert.match(refused.result.stderr, error);
                assert.deepEqual(readFileSync(refused.output), acceptedPdf,
                    'page-budget refusal cannot replace the accepted PDF');
            }

            const last = render(dir, thirtyTwo, 'png', oneRow,
                '--flow-text', 32);
            assert.equal(last.result.status, 0, last.result.stderr);
            const acceptedPng = readFileSync(last.output);
            const reference = join(dir, 'manual-last-page.png');
            const manual = spawnSync(cargo, [
                'run', '--offline', '--quiet',
                '-p', 'atrament_profile_editor_cli', '--', ...base,
                '--place-text', '32', '32500', '42500', 'ó',
                '--export-paper', 'png', '--png-page', '32',
                '--dpi', '72', '--output', reference,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(manual.status, 0, manual.stderr);
            assert.deepEqual(acceptedPng, readFileSync(reference),
                'last-page PNG is exactly the selected authored glyph ink');

            const lateMissing = render(dir,
                [...Array(31).fill('a'), 'β'].join('\n'), 'png', oneRow,
                '--flow-text', 1);
            assert.notEqual(lateMissing.result.status, 0);
            assert.match(lateMissing.result.stderr,
                /missing authored glyph shapes/);
            assert.deepEqual(readFileSync(lateMissing.output), acceptedPng,
                'page-one selection cannot conceal invalid page-32 ink');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('128-row physical flow ceiling yields an exact last-page PNG',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-row-ceiling-'));
        try {
            const region = ['--flow-region', '30000', '40000',
                '30000', '41000'];
            const full = [...Array(127).fill('a'), 'ó'].join('\n');
            const accepted = render(dir, full, 'png', region,
                '--flow-text', 32);
            assert.equal(accepted.result.status, 0, accepted.result.stderr);
            const expected = readFileSync(accepted.output);
            const reference = join(dir, 'manual-128th-row.png');
            const placements = ['a', 'a', 'a', 'ó'].flatMap(
                (letter, index) => ['--place-text', '32', '32500',
                    String(42500 + 9000 * index), letter]);
            const manual = spawnSync(cargo, [
                'run', '--offline', '--quiet',
                '-p', 'atrament_profile_editor_cli', '--', ...base,
                ...placements, '--export-paper', 'png',
                '--png-page', '32', '--dpi', '72', '--output', reference,
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(manual.status, 0, manual.stderr);
            assert.deepEqual(expected, readFileSync(reference),
                'rows 125..128 on page 32 match independent manual ink');
            const excessive = render(dir, `${full}\na`, 'png', region,
                '--flow-text', 32);
            assert.notEqual(excessive.result.status, 0);
            assert.match(excessive.result.stderr,
                /exceeds 128 measured lines/);
            assert.deepEqual(readFileSync(excessive.output), expected,
                'row-budget refusal cannot replace a selected PNG');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

test('ordered marked-space glyph preserves exact PDF/PNG and atomic refusal',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-ordered-marked-'));
        try {
            const source = '1.  \u0301a';
            const ink = [
                '--glyph-stroke', '1', '200', '500', '1500',
                '3000', '250', '#123456',
                '--glyph-stroke', '.', '200', '500', '1500',
                '3000', '250', '#123456',
                '--glyph-stroke', ' \u0301', '200', '500', '1500',
                '3000', '250', '#D02040',
            ];
            const region = ['--flow-region', '30000', '40000',
                '44000', '20000'];
            for (const format of ['pdf', 'png']) {
                const rendered = render(dir, source, format,
                    [...ink, ...region]);
                assert.equal(rendered.result.status, 0,
                    rendered.result.stderr);
                const expected = readFileSync(rendered.output);
                const directOutput = join(dir, 'manual-' + format);
                const direct = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--',
                    ...base, ...ink, '--place-text',
                    '1', '38500', '42500', source,
                    '--export-paper', format,
                    ...(format === 'png' ? ['--dpi', '72'] : []),
                    '--output', directOutput,
                ], {
                    cwd: root, encoding: 'utf8', timeout: 120000,
                });
                assert.equal(direct.status, 0, direct.stderr);
                assert.deepEqual(expected, readFileSync(directOutput),
                    'ordered marked-space native ' + format
                        + ' must preserve all authored glyphs');
                const narrow = render(dir, source, format, [
                    ...ink, '--flow-region', '30000', '40000',
                    '22000', '20000',
                ]);
                assert.notEqual(narrow.result.status, 0);
                assert.match(narrow.result.stderr,
                    /ordered list label and first glyph exceed/);
                assert.deepEqual(readFileSync(narrow.output), expected,
                    'narrow refusal cannot replace saved ' + format);
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// A soft-joined source-line combining mark becomes a space-mark glyph.
// Its selected handwriting height must drive native page pitch, matching
// equivalent manual positions in both vector PDF and transparent PNG.
test('soft-joined combining ink sets native PDF/PNG page pitch and refusal',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-joined-pitch-'));
        try {
            const profile = join(dir, 'tall-joined-space.json');
            const combined = ' \u0301';
            const configured = spawnSync(cargo, [
                'run', '--offline', '--quiet',
                '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', '--set-glyph', combined, 'height',
                '25000', '25000', '25000', '25000', '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(profile, configured.stdout);
            const options = [
                '--handwriting-profile', profile,
                '--glyph-stroke', combined, '200', '500',
                '1500', '3000', '250', '#D02040',
            ];
            const region = [
                '--flow-region', '30000', '40000', '40000', '35000',
            ];
            const source = 'a\n\u0301a\n\na';
            const placed = 'a \u0301a';
            for (const [format, page] of [
                ['pdf', 1], ['png', 1], ['png', 2],
            ]) {
                const flags = [...options, ...region,
                    ...(format === 'pdf' ? ['--pages', '2'] : [])];
                const rendered = render(dir, source, format, flags,
                    '--flow-markdown', page);
                assert.equal(rendered.result.status, 0,
                    rendered.result.stderr);
                const expected = readFileSync(rendered.output);
                const manualOutput = join(dir, 'manual-' + format + '-' + page);
                const direct = spawnSync(cargo, [
                    'run', '--offline', '--quiet',
                    '-p', 'atrament_profile_editor_cli', '--', ...base,
                    ...options, '--place-text', '1', '32500', '42500',
                    placed, '--place-text', '2', '32500', '42500', 'a',
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                    '--export-paper', format,
                    ...(format === 'png'
                        ? ['--png-page', String(page), '--dpi', '72']
                        : []),
                    '--output', manualOutput,
                ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                assert.equal(direct.status, 0, direct.stderr);
                assert.deepEqual(expected, readFileSync(manualOutput),
                    format + ' page ' + page + ' follows 29-mm pitch');

                const short = render(dir, source, format, [
                    ...options, '--flow-region', '30000', '40000',
                    '40000', '14000',
                    ...(format === 'pdf' ? ['--pages', '2'] : []),
                ], '--flow-markdown', page);
                assert.notEqual(short.result.status, 0);
                assert.match(short.result.stderr,
                    /flow line cannot fit printable vertical region/);
                assert.deepEqual(readFileSync(short.output), expected,
                    format + ' page ' + page + ' keeps ink on refusal');
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });

// Tab expansion creates an authored space-mark grapheme, but a Unicode
// hard line break must not create that glyph across the physical row.
// Both paths are compared against explicitly placed native pen strokes.
test('tab-attached and hard-broken marks render exact native PDF/PNG ink',
    () => {
        const dir = mkdtempSync(join(root, '.temp/flow-tab-mark-'));
        try {
            const profile = join(dir, 'marked-space-profile.json');
            const marked = ' \u0301';
            const standalone = '\u0301';
            const configured = spawnSync(cargo, [
                'run', '--offline', '--quiet',
                '-p', 'atrament_profile_editor_cli', '--',
                'handwriting', '--set-glyph', marked, 'height',
                '25000', '25000', '25000', '25000', '0',
            ], { cwd: root, encoding: 'utf8', timeout: 120000 });
            assert.equal(configured.status, 0, configured.stderr);
            writeFileSync(profile, configured.stdout);
            const ink = [
                '--handwriting-profile', profile,
                '--glyph-stroke', marked, '200', '500',
                '1500', '3000', '250', '#D02040',
                '--glyph-stroke', standalone, '200', '500',
                '1500', '3000', '250', '#2060D0',
            ];
            const region = [
                '--flow-region', '30000', '40000', '40000', '35000',
            ];
            const cases = [
                {
                    name: 'tab', source: 'a\t\u0301a',
                    positions: ['--place-text', '1', '32500', '42500',
                        'a   \u0301a'],
                },
                {
                    name: 'unicode hard break',
                    source: 'a\u2028\u0301a',
                    positions: [
                        '--place-text', '1', '32500', '42500', 'a',
                        '--place-text', '1', '32500', '51500', '\u0301a',
                    ],
                },
            ];
            for (const format of ['pdf', 'png']) {
                for (const sample of cases) {
                    const rendered = render(dir, sample.source, format,
                        [...ink, ...region]);
                    assert.equal(rendered.result.status, 0,
                        sample.name + ': ' + rendered.result.stderr);
                    const expected = readFileSync(rendered.output);
                    const reference = join(dir,
                        `direct-${sample.name.replaceAll(' ', '-')}.${format}`);
                    const manual = spawnSync(cargo, [
                        'run', '--offline', '--quiet',
                        '-p', 'atrament_profile_editor_cli', '--',
                        ...base, ...ink, ...sample.positions,
                        '--export-paper', format,
                        ...(format === 'png' ? ['--dpi', '72'] : []),
                        '--output', reference,
                    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
                    assert.equal(manual.status, 0,
                        sample.name + ': ' + manual.stderr);
                    assert.deepEqual(expected, readFileSync(reference),
                        `${sample.name} ${format} matches exact authored ink`);
                    if (sample.name === 'tab') {
                        const narrow = render(dir, sample.source, format, [
                            ...ink, '--flow-region', '30000', '40000',
                            '40000', '27000',
                        ]);
                        assert.notEqual(narrow.result.status, 0);
                        assert.match(narrow.result.stderr,
                            /flow line cannot fit printable vertical region/);
                        assert.deepEqual(readFileSync(narrow.output), expected,
                            `${format} keeps selected ink after refusal`);
                    }
                }
            }
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
