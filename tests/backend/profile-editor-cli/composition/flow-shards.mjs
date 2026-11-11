// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - true
// License-File:
//   - LICENSE
// Run long native flow acceptance in independently bounded Node test batches.
// Usage: node tests/backend/profile-editor-cli/composition/flow-shards.mjs
//        add --list to inspect shard membership
//        add --shard 1 to run only the first batch
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const file = join(root,
    'tests/backend/profile-editor-cli/composition/flow.mjs');
const source = readFileSync(file, 'utf8');
const declarations = [...source.matchAll(/^test\(/gm)];
const names = [...source.matchAll(/^test\('([^'\n]+)'/gm)]
    .map((match) => match[1]);
if (names.length === 0 || declarations.length !== names.length
    || new Set(names).size !== names.length) {
    throw new Error('flow tests require unique, literal top-level names');
}

// Each batch is small enough to have its own bounded runner lifetime.
// All names are anchored and regex-escaped: adding a test never changes
// the meaning of an existing name or silently skips a duplicate.
const shardSize = 3;
const shardCount = Math.ceil(names.length / shardSize);
const escapeRegex = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
// TAP's exit status alone is not enough: an unmatched name filter can exit
// successfully with zero selected tests. Refuse missing, duplicate or partial
// test receipts without trusting log text as evidence of a passed shard.
export function completeShardTap(stdout, expected) {
    const count = (label) => {
        const lines = [...stdout.matchAll(
            new RegExp(`^# ${label} (\\d+)\\r?$`, 'gm'),
        )];
        return lines.length === 1 ? Number(lines[0][1]) : null;
    };
    return count('tests') === expected && count('pass') === expected
        && count('fail') === 0 && count('cancelled') === 0
        && count('skipped') === 0 && count('todo') === 0;
}

function run() {
    const args = process.argv.slice(2);

    if (args.length === 1 && args[0] === '--list') {
        console.log(`${names.length} tests in ${shardCount} shards`);
        for (let shard = 0; shard < shardCount; shard += 1) {
            const selected = names.slice(shard * shardSize,
                (shard + 1) * shardSize);
            console.log(`${shard + 1}: ${selected.join(' | ')}`);
        }
    } else {
        const selectedShard = args.length === 0 ? null
            : args.length === 1 && args[0] === '--all' ? null
                : args.length === 2 && args[0] === '--shard'
                    && /^[1-9]\d*$/.test(args[1])
                    ? Number(args[1]) : NaN;
        if (Number.isNaN(selectedShard) || (selectedShard !== null
            && (!Number.isSafeInteger(selectedShard)
                || selectedShard > shardCount))) {
            console.error('Usage: node flow-shards.mjs'
                + ` [--all | --list | --shard N] (1 <= N <= ${shardCount})`);
            process.exitCode = 2;
        } else {
            // Full acceptance also verifies the test runner's own discovery,
            // selector refusal and fail-closed receipt behavior before exports.
            if (selectedShard === null) {
                const regression = spawnSync(process.execPath, [
                    '--test', fileURLToPath(new URL(
                        './flow-shards.test.mjs', import.meta.url)),
                ], { cwd: root, stdio: 'inherit', timeout: 20_000 });
                if (regression.status !== 0 || regression.error) {
                    console.error('Flow runner regression checks failed:',
                        regression.error?.message ?? regression.signal
                            ?? regression.status);
                    process.exitCode = 1;
                    return;
                }
            }
            const shards = selectedShard === null
                ? Array.from({ length: shardCount }, (_, index) => index)
                : [selectedShard - 1];
            let completed = 0;
            for (const shard of shards) {
                const selected = names.slice(shard * shardSize,
                    (shard + 1) * shardSize);
                const pattern = `^(?:${selected.map(escapeRegex).join('|')})$`;
                console.log(`Flow shard ${shard + 1}/${shardCount}`
                    + ` (${selected.length} cases)`);
                const result = spawnSync(process.execPath, [
                    '--test', '--test-reporter=tap',
                    '--test-name-pattern', pattern, file,
                ], {
                    cwd: root, encoding: 'utf8', timeout: 240_000,
                    maxBuffer: 8 * 1024 * 1024,
                });
                process.stdout.write(result.stdout ?? '');
                process.stderr.write(result.stderr ?? '');
                const complete = completeShardTap(result.stdout ?? '',
                    selected.length);
                if (result.status !== 0 || result.error || !complete) {
                    console.error(`Flow shard ${shard + 1} failed:`,
                        result.error?.message ?? result.signal ?? result.status,
                        complete ? '' : 'incomplete TAP test counts');
                    process.exitCode = 1;
                    break;
                }
                completed += selected.length;
            }
            if (!process.exitCode) {
                console.log(`Passed ${completed} selected native flow cases.`);
            }
        }
    }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
    run();
}
