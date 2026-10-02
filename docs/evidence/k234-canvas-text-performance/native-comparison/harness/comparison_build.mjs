import { command } from '../../scripts/dev/desktop.mjs';
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '../..');
process.loadEnvFile(resolve(root, '.env.desktop'));
const [stage, variant] = process.argv.slice(2);
if (!['metadata', 'build'].includes(stage)) throw new Error('Unknown stage');
const sources = {
  baseline: 'lurq-k232-mcp-performance',
  candidate: 'lurq-canvas-text-performance',
};
if (!Object.hasOwn(sources, variant)) throw new Error('Unknown variant');
const plan = command(root, process.env, ['--locked', '--offline']);
if (!plan.local_crate?.endsWith(`/${sources[variant]}/crates/lurq`)) throw new Error('Unexpected local source');
if (!plan.target.startsWith(root) || !plan.temp.startsWith(root)
    || !process.env.CARGO_HOME?.startsWith(root.replaceAll('\\', '/'))) throw new Error('Unexpected cache path');
const shared = ['--config', '--features'].flatMap(flag => {
  const index = plan.args.indexOf(flag);
  if (index < 0) throw new Error('Missing local feature/config');
  return plan.args.slice(index, index + 2);
});
const args = stage === 'metadata'
  ? ['metadata', '--format-version', '1', '--locked', '--offline', ...shared]
  : ['build', '--release', ...plan.args.slice(1)];
const output = resolve(import.meta.dirname, 'comparison', variant);
mkdirSync(output, { recursive: true });
writeFileSync(resolve(output, `${stage}-command.json`), JSON.stringify({
  launcher: plan, executable: plan.executable, args,
  reason: 'Optimized build-only adaptation of configured local-Lurq launcher; owned MCP harness launches later',
}, null, 2));
const result = spawnSync(plan.executable, args, {
  cwd: root, stdio: 'inherit', windowsHide: true,
  env: { ...process.env, CARGO_TARGET_DIR: plan.target, TEMP: plan.temp, TMP: plan.temp },
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
