// Tauri invokes this on every platform; the native NSIS plugin is Windows-only.
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
if (process.platform === 'win32') {
  const script = fileURLToPath(new URL('./build-installer-ui.ps1', import.meta.url));
  const result = spawnSync('pwsh', ['-NoProfile', '-File', script], { stdio: 'inherit' });
  if (result.error) throw result.error;
  process.exit(result.status ?? 1);
}
