import { execFileSync, spawnSync } from 'node:child_process'
import process from 'node:process'

const cwd = process.cwd()
const needsAsciiAlias = process.platform === 'win32' && /[^\x00-\x7F]/.test(cwd)

function runBuild(buildCwd) {
  const command = process.platform === 'win32' ? 'cmd.exe' : 'npm'
  const args = process.platform === 'win32'
    ? ['/d', '/s', '/c', 'npm.cmd run build:web']
    : ['run', 'build:web']
  const result = spawnSync(command, args, {
    cwd: buildCwd,
    stdio: 'inherit',
    shell: false,
  })
  if (result.error) throw result.error
  return result.status ?? 1
}

if (!needsAsciiAlias) {
  process.exit(runBuild(cwd))
}

let drive
for (const letter of ['R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z']) {
  try {
    execFileSync('cmd.exe', ['/d', '/c', `if not exist ${letter}:\\ exit /b 0 else exit /b 1`])
    drive = `${letter}:`
    break
  } catch {
    // Try the next drive letter.
  }
}

if (!drive) {
  console.error('Starroom build: no free drive letter is available for the Windows ASCII path alias.')
  process.exit(1)
}

try {
  console.log(`Starroom build: mapping ${drive} to the Unicode workspace for toolchain compatibility.`)
  execFileSync('subst.exe', [drive, cwd], { stdio: 'inherit' })
  process.exitCode = runBuild(`${drive}\\`)
} finally {
  execFileSync('subst.exe', [drive, '/D'], { stdio: 'inherit' })
}
