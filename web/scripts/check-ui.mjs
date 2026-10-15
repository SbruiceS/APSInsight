// Static accessibility and localization checks for the Angular app.
// Fails (exit 1) on:
//  * catalogue key mismatch between languages
//  * a translation key used in code/templates that is missing from the catalogue
//  * a form control (input/select/textarea) without an associated <label>
//  * a missing aria-live status region or skip link in the shell
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../src/app/', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1');
const files = [];
const walk = (dir) => {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p);
    else if (p.endsWith('.ts') && !p.endsWith('.spec.ts')) files.push(p);
  }
};
walk(root);

const errors = [];
const i18n = readFileSync(join(root, 'core', 'i18n.ts'), 'utf8');
const block = (name) => {
  const start = i18n.indexOf(`export const ${name}`);
  const body = i18n.slice(start, i18n.indexOf('};', start));
  return new Set([...body.matchAll(/^\s*'([\w.]+)':/gm)].map((m) => m[1]));
};
const en = block('EN');
const hi = block('HI');
for (const k of en) if (!hi.has(k)) errors.push(`hi catalogue missing ${k}`);
for (const k of hi) if (!en.has(k)) errors.push(`hi catalogue has unknown key ${k}`);
if (en.size < 50) errors.push(`suspiciously small catalogue (${en.size} keys)`);

for (const f of files) {
  const src = readFileSync(f, 'utf8');
  const used = [
    ...[...src.matchAll(/'([a-z]+\.[A-Za-z]+)'\s*\|\s*t\b/g)].map((m) => m[1]),
    ...[...src.matchAll(/\bt\('([a-z]+\.[A-Za-z]+)'/g)].map((m) => m[1]),
    ...[...src.matchAll(/\?\s*'([a-z]+\.[A-Za-z]+)'\s*:\s*'([a-z]+\.[A-Za-z]+)'\s*\)\s*\|\s*t/g)].flatMap((m) => [m[1], m[2]]),
  ];
  for (const k of used) if (!en.has(k)) errors.push(`${f}: unknown translation key ${k}`);

  for (const m of src.matchAll(/<(input|select|textarea)\b([^>]*)>/g)) {
    const attrs = m[2];
    if (/type="(hidden|submit)"/.test(attrs)) continue;
    const id = attrs.match(/\sid="([^"]+)"/)?.[1];
    const before = src.slice(Math.max(0, m.index - 200), m.index);
    const wrapped = before.lastIndexOf('<label') > before.lastIndexOf('</label>');
    const labelled = (id && src.includes(`for="${id}"`)) || wrapped || /aria-label/.test(attrs);
    if (!labelled) errors.push(`${f}: unlabelled <${m[1]}${attrs.slice(0, 60)}>`);
  }
}

const shell = readFileSync(join(root, 'app.ts'), 'utf8');
if (!shell.includes('aria-live="polite"')) errors.push('app shell has no aria-live status region');
if (!shell.includes('class="skip"')) errors.push('app shell has no skip link');

if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.log(`check-ui: ${files.length} files, ${en.size} keys x 2 locales, all controls labelled`);
