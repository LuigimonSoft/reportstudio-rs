import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, '../..');
for (const [command, args, cwd] of [
  ['cargo', ['test', '--locked'], root],
  [process.execPath, [path.join(directory, 'node_modules/@playwright/test/cli.js'), 'test'], directory],
]) {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit', shell: process.platform === 'win32' && command === 'cargo' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
