// Static accessibility and localization checks for the Angular app.
// Fails (exit 1) on:
//  * catalogue key mismatch between languages
//  * a translation key used in code/templates that is missing from the catalogue
//  * a form control (input/select/textarea) without an associated <label>
//  * a missing aria-live status region or skip link in the shell
//  * a register domain missing from, or listed twice in, core/taxonomy.ts
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
  return new Set([...body.matchAll(/^ *'([\w.]+)':/gm)].map((m) => m[1]));
};
const en = block('EN');
const hi = block('HI');
for (const k of en) if (!hi.has(k)) errors.push(`hi catalogue missing ${k}`);
for (const k of hi) if (!en.has(k)) errors.push(`hi catalogue has unknown key ${k}`);
if (en.size < 50) errors.push(`suspiciously small catalogue (${en.size} keys)`);

for (const f of files) {
  const src = readFileSync(f, 'utf8');
  const used = [
    ...[...src.matchAll(/'([a-z]+\.[A-Za-z0-9]+)'\s*\|\s*t\b/g)].map((m) => m[1]),
    ...[...src.matchAll(/\bt\('([a-z]+\.[A-Za-z0-9]+)'/g)].map((m) => m[1]),
    ...[...src.matchAll(/\?\s*'([a-z]+\.[A-Za-z0-9]+)'\s*:\s*'([a-z]+\.[A-Za-z0-9]+)'\s*\)\s*\|\s*t/g)].flatMap((m) => [m[1], m[2]]),
    // `label: 'group.field'` entries in taxonomy, navigation and lab tables
    ...[...src.matchAll(/\blabel:\s*'([a-z]+\.[A-Za-z0-9]+)'/g)].map((m) => m[1]),
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

// Every register domain belongs to exactly one agricultural area in core/taxonomy.ts.
const catalog = JSON.parse(readFileSync(new URL('../../register/catalog.json', import.meta.url), 'utf8'));
const taxonomy = readFileSync(join(root, 'core', 'taxonomy.ts'), 'utf8');
const groupsStart = taxonomy.indexOf('export const GROUPS');
const groupsSrc = taxonomy.slice(groupsStart, taxonomy.indexOf('\n];', groupsStart));
const grouped = [...groupsSrc.matchAll(/domains: \[([^\]]*)\]/g)].flatMap((m) => [...m[1].matchAll(/'([a-z0-9-]+)'/g)].map((x) => x[1]));
const slugs = new Set(catalog.domains.map((d) => d.slug));
for (const d of slugs) {
  const n = grouped.filter((g) => g === d).length;
  if (n !== 1) errors.push(`taxonomy.ts lists domain ${d} ${n} times (expected 1)`);
}
for (const g of grouped) if (!slugs.has(g)) errors.push(`taxonomy.ts lists unknown domain ${g}`);

// Every icon motion used in shared/icons.ts has keyframes in styles/_icons.scss.
const iconsSrc = readFileSync(join(root, 'shared', 'icons.ts'), 'utf8');
const iconStyles = readFileSync(new URL('../src/styles/_icons.scss', import.meta.url), 'utf8');
for (const m of new Set([...iconsSrc.matchAll(/motion: '([a-z]+)'/g)].map((x) => x[1]))) {
  if (!iconStyles.includes(`@keyframes icon-${m} `)) errors.push(`icon motion ${m} has no @keyframes icon-${m}`);
  if (!iconStyles.includes(`\n  ${m}: (`)) errors.push(`icon motion ${m} missing from $motions in _icons.scss`);
}

if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.log(`check-ui: ${files.length} files, ${en.size} keys x 2 locales, all controls labelled, ${slugs.size} domains grouped`);
