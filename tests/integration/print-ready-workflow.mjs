// Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier: LicenseRef-Proprietary
// Runs real Rust CLIs; no printer or localhost server is contacted.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import test from 'node:test';

const root = fileURLToPath(new URL('../../', import.meta.url));

function command(pkg, binary, args) {
    const result = spawnSync('cargo', [
        'run', '--offline', '--quiet', '-p', pkg, '--bin', binary,
        '--', ...args,
    ], { cwd: root, encoding: 'utf8', timeout: 120000 });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function notebook(...args) {
    return command('atrament_profile_editor_cli',
        'atrament-profiles', ['notebook', ...args]);
}

function glyphs(...args) {
    return command('atrament_profile_editor_cli',
        'atrament-profiles', ['glyphs', ...args]);
}

function printer(...args) {
    return command('atrament_notebook_paper_cli',
        'atrament-print', args);
}

function pngPixels(bytes) {
    assert.deepEqual(bytes.subarray(0, 8),
        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
    assert.equal(bytes.readUInt32BE(16), 596);
    assert.equal(bytes.readUInt32BE(20), 842);
    assert.equal(bytes[25], 6, 'physical sheet uses RGBA');
    const chunks = [];
    for (let offset = 8; offset + 12 <= bytes.length;) {
        const length = bytes.readUInt32BE(offset);
        const name = bytes.toString('ascii', offset + 4, offset + 8);
        if (name === 'IDAT') {
            chunks.push(bytes.subarray(offset + 8, offset + 8 + length));
        }
        offset += length + 12;
    }
    assert.ok(chunks.length, 'PNG contains a bounded raster');
    return inflateSync(Buffer.concat(chunks));
}

test('authored Markdown exports physical PDF and PNG before print dry-run',
    () => {
        const dir = mkdtempSync(join(root, '.temp/print-ready-'));
        try {
            const source = join(dir, 'notes.md');
            const pdfPath = join(dir, 'notes.pdf');
            const pngPath = join(dir, 'second-page.png');
            const profile = join(dir, 'notebook.json');
            const library = join(dir, 'glyphs.tsv');
            const lines = Array.from({ length: 54 }, () => 'a  ');
            writeFileSync(source, `# a\n\n${lines.join('\n')}\n`);
            const saveProfile = notebook(
                '--printed-sheet', 'true',
                '--date-style', 'line', '--date-placement', 'top-left',
                '--date-text', '2026-11-04',
                '--page-numbers', 'true', '--border-radius-um', '5000',
                '--output', profile);
            assert.equal(saveProfile.status, 0, saveProfile.stderr);
            assert.match(readFileSync(profile, 'utf8'),
                /"printed_sheet"\s*:\s*true/);
            const saveGlyph = glyphs(
                '--glyph-stroke', 'a', '600', '500', '1800', '3000',
                '250', '#123456', '--output', library);
            assert.equal(saveGlyph.status, 0, saveGlyph.stderr);
            assert.match(readFileSync(library, 'utf8'), /#123456/i);
            const base = [
                '--profile', profile, '--glyph-file', library,
                '--heading-style', 'marker',
                '--heading-color', '#F8DE91',
                '--flow-markdown-file', source,
            ];
            const pdfRun = notebook(...base, '--export-paper', 'pdf',
                '--output', pdfPath);
            assert.equal(pdfRun.status, 0, pdfRun.stderr);
            const pdf = readFileSync(pdfPath);
            assert.ok(pdf.toString('latin1').startsWith('%PDF-1.4'));
            assert.match(pdf.toString('latin1'), /\/Count 2\b/,
                'Markdown must paginate onto exactly two physical pages');
            assert.match(pdf.toString('latin1'), /0\.071 0\.204 0\.337 RG/,
                'caller-authored ink survives vector PDF output');
            const pngRun = notebook(...base, '--export-paper', 'png',
                '--png-page', '2', '--dpi', '72', '--output', pngPath);
            assert.equal(pngRun.status, 0, pngRun.stderr);
            const raw = pngPixels(readFileSync(pngPath));
            assert.equal(raw[4], 0, 'transparent page has no backdrop');
            assert.ok(raw.includes(Buffer.from([18, 52, 86, 255])),
                'selected second page contains the same authored ink');
            const dryRun = printer('--pdf', pdfPath,
                '--printer', 'Unconnected_Test_Queue', '--dry-run');
            assert.equal(dryRun.status, 0, dryRun.stderr);
            assert.match(dryRun.stdout, /no job submitted/);
            assert.deepEqual(readFileSync(pdfPath), pdf,
                'print validation must not alter the selected PDF');
            const refusal = notebook(...base, '--export-paper', 'pdf',
                '--pages', '1', '--output', pdfPath, '--replace');
            assert.notEqual(refusal.status, 0,
                'refuse silently dropping page-two handwritten ink');
            assert.deepEqual(readFileSync(pdfPath), pdf,
                'failed pagination cannot overwrite accepted output');
            writeFileSync(source, '# a\n\nmissing glyph: z\n');
            const unknown = notebook(...base, '--export-paper', 'pdf',
                '--output', pdfPath, '--replace');
            assert.notEqual(unknown.status, 0,
                'unapproved Unicode glyph shapes cannot be synthesized');
            assert.deepEqual(readFileSync(pdfPath), pdf,
                'failed glyph admission must leave output untouched');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
