// Runs every check the project requires and stops at the first failure.
// Usage: node tools/verify.mjs [--skip-web]
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const skipWeb = process.argv.includes('--skip-web');

const steps = [
  ['register analysis', 'node', ['tools/analyse-register.mjs'], root],
  ['canonical structure (72 directories)', 'node', ['tools/structure.mjs', '--check'], root],
  ['rustfmt', 'cargo', ['fmt', '--all', '--check'], root],
  ['clippy (warnings are errors)', 'cargo', ['clippy', '--workspace', '--all-targets'], root],
  ['rust tests', 'cargo', ['test', '--workspace'], root],
  ['sbom licence-expression self-test', 'node', ['tools/sbom.mjs', '--self-test'], root],
  ['sbom + licence gate', 'node', ['tools/sbom.mjs'], root],
  ...(skipWeb
    ? []
    : [
        ['ui static checks', 'npm', ['run', 'check'], join(root, 'web')],
        ['ui unit tests', 'npm', ['test'], join(root, 'web')],
        ['ui production build', 'npm', ['run', 'build'], join(root, 'web')],
      ]),
];

for (const [name, cmd, args, cwd] of steps) {
  process.stdout.write(`\n=== ${name}: ${cmd} ${args.join(' ')}\n`);
  // Windows needs a shell to run npm.cmd. The arguments above are fixed literals,
  // so they are passed as one command line (an args array with shell: true is
  // deprecated because it is not escaped).
  const r =
    process.platform === 'win32'
      ? spawnSync([cmd, ...args].join(' '), { cwd, stdio: 'inherit', shell: true })
      : spawnSync(cmd, args, { cwd, stdio: 'inherit' });
  if (r.status !== 0) {
    console.error(`\nFAILED: ${name} (exit ${r.status})`);
    process.exit(r.status ?? 1);
  }
}
console.log('\nAll checks passed.');
