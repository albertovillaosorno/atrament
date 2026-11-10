// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
// Validate test discovery and refusal without running expensive PDF/PNG tests.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { completeShardTap } from './flow-shards.mjs';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const runner = fileURLToPath(new URL('./flow-shards.mjs', import.meta.url));
const flow = fileURLToPath(new URL('./flow.mjs', import.meta.url));

function run(...args) {
    const result = spawnSync(process.execPath, [runner, ...args], {
        cwd: root, encoding: 'utf8', timeout: 20_000,
    });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

test('flow shard listing covers each literal source test once in order', () => {
    const source = readFileSync(flow, 'utf8');
    const declared = [...source.matchAll(/^test\('([^'\n]+)'/gm)]
        .map((match) => match[1]);
    assert.ok(declared.length > 0);
    assert.equal([...source.matchAll(/^test\(/gm)].length, declared.length);
    assert.equal(new Set(declared).size, declared.length);

    const listed = run('--list');
    assert.equal(listed.status, 0, listed.stderr);
    const [heading, ...lines] = listed.stdout.trimEnd().split('\n');
    const shardSize = 3;
    const shardCount = Math.ceil(declared.length / shardSize);
    assert.equal(heading, `${declared.length} tests in ${shardCount} shards`);
    assert.equal(lines.length, shardCount);
    const selected = [];
    for (let index = 0; index < lines.length; index += 1) {
        const names = declared.slice(index * shardSize,
            (index + 1) * shardSize);
        assert.equal(lines[index], `${index + 1}: ${names.join(' | ')}`);
        selected.push(...names);
    }
    assert.deepEqual(selected, declared);
});

test('flow shard runner rejects invalid selectors before native export', () => {
    for (const args of [
        ['--shard', '0'], ['--shard', '-1'], ['--shard', '1.0'],
        ['--shard', '999999999999999999999'], ['--shard'],
        ['--shard', '1', '--list'], ['--unknown'], ['--all', '--list'],
    ]) {
        const result = run(...args);
        assert.equal(result.status, 2, `${args}: ${result.stderr}`);
        assert.match(result.stderr, /Usage: node flow-shards\.mjs/);
        assert.doesNotMatch(result.stdout, /Flow shard \d+\//);
    }
});

test('flow shard receipts refuse zero matches and partial TAP results', () => {
    const accepted = [
        '# tests 3', '# pass 3', '# fail 0', '# cancelled 0',
        '# skipped 0', '# todo 0',
    ].join('\n') + '\n';
    assert.equal(completeShardTap(accepted, 3), true);
    for (const [from, to] of [
        ['# tests 3', '# tests 0'],
        ['# pass 3', '# pass 0'],
        ['# pass 3', '# pass 2'],
        ['# fail 0', '# fail 1'],
        ['# cancelled 0', '# cancelled 1'],
        ['# skipped 0', '# skipped 1'],
        ['# todo 0', '# todo 1'],
    ]) {
        assert.equal(completeShardTap(accepted.replace(from, to), 3),
            false, to);
        assert.equal(completeShardTap(accepted.replace(from, ''), 3),
            false, `${from} missing`);
        assert.equal(completeShardTap(accepted + `${from}\n`, 3),
            false, `${from} duplicate`);
    }
    assert.equal(completeShardTap('', 3), false);
    assert.equal(completeShardTap(accepted, 2), false);
});
