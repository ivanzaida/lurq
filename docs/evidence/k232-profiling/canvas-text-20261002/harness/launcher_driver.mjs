import { command } from '../../scripts/dev/desktop.mjs';
import { spawnSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '../..');
process.loadEnvFile(resolve(root, '.env.desktop'));
const plan = command(root, process.env, ['--locked', '--offline']);
const stage = process.argv[2];
const prefix = process.argv[3] ?? 'refined';
if (!['refined', 'canvas-text'].includes(prefix)) throw new Error('unknown receipt prefix');
if (!['metadata', 'build'].includes(stage)) throw new Error('unknown stage');
if (!plan.local_crate?.endsWith('/lurq-k232-mcp-performance/crates/lurq')) throw new Error('unexpected local crate');
if (!plan.target.startsWith(root) || !plan.temp.startsWith(root)
    || !process.env.CARGO_HOME?.startsWith(root.replaceAll('\\', '/'))) throw new Error('unexpected cache path');
const shared = ['--config', '--features'].flatMap(flag => {
  const index = plan.args.indexOf(flag);
  if (index < 0) throw new Error('missing local feature/config argument');
  return plan.args.slice(index, index + 2);
});
const args = stage === 'metadata'
  ? ['metadata', '--format-version', '1', '--locked', '--offline', ...shared]
  : ['build', ...plan.args.slice(1)];
writeFileSync(resolve(import.meta.dirname, `${prefix}-${stage}-command.json`), JSON.stringify({
  launcher: plan, executed_executable: plan.executable, executed_args: args,
  reason: 'Build-only adaptation of configured dev:desktop command; canonical harness owns later native launch',
}, null, 2));
const result = spawnSync(plan.executable, args, {
  cwd: root, stdio: 'inherit', windowsHide: true,
  env: { ...process.env, CARGO_TARGET_DIR: plan.target, TEMP: plan.temp, TMP: plan.temp },
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
