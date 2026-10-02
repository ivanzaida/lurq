import { command } from '../../../scripts/dev/desktop.mjs';
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, copyFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';

const folder = import.meta.dirname;
const root = resolve(folder, '../../..');
const source = 'H:/projects/pencil-web/.codex/worktrees/lurq-canvas-text-performance';
const konturHead = '61d73d08b7c047d6c62ad55d6ed127a495881772';
const lurqHead = 'cf0c5d81ddab2c960f1aff34a4b5117a73ade9e5';
const registryHash = 'f7229f6464dab118725c5d4436f97dcd450c388cdf25407e3813396caa75df50';
const envPath = resolve(root, '.env.desktop');
const envHash = '055b967168508d226e60a53c5fb1cef62966fcd88709209b31c33b7decce7722';
const lockPath = resolve(root, 'Cargo.lock');
const patchedPath = resolve(folder, 'Cargo.lock.local');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function git(cwd, ...args) {
  const result = spawnSync('git', args, { cwd, encoding: 'utf8', windowsHide: true });
  if (result.status !== 0) throw new Error(`Git failed: ${args[0]}`);
  return result.stdout.trim();
}
function verifySources() {
  if (sha(readFileSync(envPath)) !== envHash) throw new Error('Configured env changed');
  if (git(root, 'rev-parse', 'HEAD') !== konturHead
      || git(root, 'status', '--porcelain', '--untracked-files=no')) throw new Error('Kontur source changed');
  if (git(source, 'rev-parse', 'HEAD') !== lurqHead
      || git(source, 'status', '--porcelain', '--untracked-files=no')) throw new Error('Lurq source changed');
}
function blocks(bytes) {
  return bytes.toString('utf8').split('[[package]]').slice(1).map(text => ({
    name: text.match(/\nname = "([^"]+)"/)?.[1], text,
  }));
}
function verifyLockDelta(original, local) {
  const before = blocks(original), after = blocks(local);
  if (before.length !== after.length) throw new Error('Package graph changed');
  const changed = [];
  for (let i = 0; i < before.length; i++) {
    if (before[i].name !== after[i].name) throw new Error('Package order changed');
    if (before[i].text === after[i].text) continue;
    const name = before[i].name;
    if (!['lurq', 'lurq_macros'].includes(name)) throw new Error(`Unexpected changed package: ${name}`);
    const stripped = before[i].text.replace(/^source = .*\r?\n/gm, '').replace(/^checksum = .*\r?\n/gm, '');
    if (stripped !== after[i].text) throw new Error(`Non-source lock change: ${name}`);
    changed.push(name);
  }
  if (changed.join(',') !== 'lurq,lurq_macros') throw new Error('Expected both local source removals');
  return changed;
}
process.loadEnvFile(envPath);
// Explicit process-only source override; the user configuration file is untouched.
process.env.KONTUR_LURQ_PATH = source;
const stage = process.argv[2];
if (!['prepare', 'preview', 'build'].includes(stage)) throw new Error('Expected prepare, preview, or build');
verifySources();
for (const name of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_PROFILE_RELEASE_OPT_LEVEL',
  'CARGO_PROFILE_RELEASE_DEBUG', 'CARGO_PROFILE_RELEASE_LTO', 'CARGO_PROFILE_RELEASE_CODEGEN_UNITS',
  'CARGO_PROFILE_RELEASE_INCREMENTAL']) {
  if (process.env[name]) throw new Error(`Unexpected profile override: ${name}`);
}
const plan = command(root, process.env, ['--locked', '--offline']);
if (plan.local_crate !== `${source}/crates/lurq`) throw new Error('Unexpected configured local source');
if (plan.target !== resolve(root, '.tmp/target') || plan.temp !== resolve(root, '.tmp/rust-temp')) {
  throw new Error('Unexpected target or temp');
}
if (process.env.CARGO_HOME?.replaceAll('\\', '/') !== `${root.replaceAll('\\', '/')}/.tmp/lurq-adoption-cargo-home`) {
  throw new Error('Unexpected Cargo home');
}
const shared = ['--config', '--features'].flatMap(flag => {
  const i = plan.args.indexOf(flag);
  if (i < 0) throw new Error(`Missing ${flag}`);
  return plan.args.slice(i, i + 2);
});
const args = ['build', '--release', ...plan.args.slice(1)];
const identity = { kontur_head: konturHead, lurq_head: lurqHead, launcher: plan,
  executable: 'cargo', args, registry_lock_sha256: registryHash,
  configured_env_sha256: envHash, local_source_override_scope: 'process only; configured .env.desktop unchanged' };
if (stage === 'preview') {
  console.log(JSON.stringify(identity, null, 2));
  process.exit(0);
}
const original = readFileSync(lockPath);
if (sha(original) !== registryHash) throw new Error('Committed registry lock changed');
const env = { ...process.env, CARGO_TARGET_DIR: plan.target, TEMP: plan.temp, TMP: plan.temp,
  CARGO_BUILD_JOBS: '1', CARGO_INCREMENTAL: '0', CARGO_PROFILE_DEV_DEBUG: '0', CARGO_PROFILE_TEST_DEBUG: '0' };
let expectedLock = original;
let status = null;
const started = new Date().toISOString();
try {
  if (stage === 'prepare') {
    const metadataArgs = ['metadata', '--format-version', '1', '--offline', ...shared];
    const result = spawnSync('cargo', metadataArgs, { cwd: root, env, encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 });
    expectedLock = readFileSync(lockPath);
    writeFileSync(resolve(folder, 'metadata-error.log'), result.stderr ?? '');
    status = result.status;
    if (status !== 0) throw new Error('Offline metadata failed');
    const changed = verifyLockDelta(original, expectedLock);
    const metadata = JSON.parse(result.stdout);
    for (const name of changed) {
      const pkg = metadata.packages.filter(pkg => pkg.name === name);
      const dir = name === 'lurq' ? 'lurq' : 'lurq_macros';
      if (pkg.length !== 1 || pkg[0].source !== null
          || pkg[0].manifest_path.replaceAll('\\', '/') !== `${source}/crates/${dir}/Cargo.toml`) {
        throw new Error(`Wrong ${name} metadata source`);
      }
    }
    const lurq = metadata.packages.find(pkg => pkg.name === 'lurq');
    const features = metadata.resolve.nodes.find(node => node.id === lurq.id).features;
    if (!features.includes('perf_profile') || !features.includes('canvas')) throw new Error('Missing profiler or Canvas');
    writeFileSync(patchedPath, expectedLock);
    writeFileSync(resolve(folder, 'metadata.json'), result.stdout);
    writeFileSync(resolve(folder, 'prepared.json'), JSON.stringify({ ...identity,
      local_lock_sha256: sha(expectedLock), changed, features, metadata_args: metadataArgs,
      started_utc: started, finished_utc: new Date().toISOString(), exit_code: status }, null, 2));
  } else {
    if (existsSync(resolve(folder, 'identity.json'))) throw new Error('Completed build exists');
    const prepared = JSON.parse(readFileSync(resolve(folder, 'prepared.json'), 'utf8'));
    expectedLock = readFileSync(patchedPath);
    if (sha(expectedLock) !== prepared.local_lock_sha256) throw new Error('Prepared lock changed');
    verifyLockDelta(original, expectedLock);
    writeFileSync(lockPath, expectedLock);
    writeFileSync(resolve(folder, 'build-command.json'), JSON.stringify(identity, null, 2));
    const result = spawnSync('cargo', args, { cwd: root, env, stdio: 'inherit', windowsHide: true });
    status = result.status;
    if (status !== 0) throw new Error(`Build failed: ${status}`);
    if (git(root, 'rev-parse', 'HEAD') !== konturHead || git(source, 'rev-parse', 'HEAD') !== lurqHead
        || git(source, 'status', '--porcelain', '--untracked-files=no')) throw new Error('Source changed during build');
    const binary = resolve(folder, 'kontur-desktop.exe');
    copyFileSync(resolve(plan.target, 'release/kontur-desktop.exe'), binary);
    writeFileSync(resolve(folder, 'identity.json'), JSON.stringify({ ...identity,
      local_lock_sha256: sha(expectedLock), binary_sha256: sha(readFileSync(binary)),
      started_utc: started, finished_utc: new Date().toISOString(), exit_code: status }, null, 2));
  }
} finally {
  if (sha(readFileSync(lockPath)) !== sha(expectedLock) || git(root, 'rev-parse', 'HEAD') !== konturHead || sha(readFileSync(envPath)) !== envHash) {
    throw new Error('Lock or HEAD changed; refusing blind restore');
  }
  writeFileSync(lockPath, original);
  writeFileSync(resolve(folder, `${stage}-cleanup.json`), JSON.stringify({
    registry_lock_restored: sha(readFileSync(lockPath)) === registryHash, exit_code: status,
    finished_utc: new Date().toISOString() }, null, 2));
}
