import { existsSync, readFileSync, statSync } from 'node:fs'
import { createHash } from 'node:crypto'

const manifest = JSON.parse(readFileSync(new URL('../fixtures/golden/manifest.json', import.meta.url), 'utf8'))
const required = new Set([
  'portrait-daylight', 'portrait-low-key', 'backlit-portrait', 'high-dynamic-range',
  'night-city', 'neon', 'high-iso', 'white-black-clothing', 'colorchecker',
  'fine-texture', 'mixed-temperature',
])
const assertions = ['identity', 'extremeControl', 'finite', 'toneRegression', 'colorRegression']

if (manifest.schemaVersion !== 1 || manifest.workingSpace !== 'linear Rec.2020 D65') {
  throw new Error('Golden manifest schema or working space is invalid')
}
for (const assertion of assertions) {
  if (!manifest.commonAssertions?.includes(assertion)) throw new Error(`Missing common assertion: ${assertion}`)
}
if (!manifest.futureAssertions?.includes('cpuGpuParity')) throw new Error('Missing future CPU/GPU parity contract')
for (const entry of manifest.cases ?? []) {
  if (!entry.id || !entry.scene || !['planned', 'active'].includes(entry.status)) throw new Error(`Invalid case: ${JSON.stringify(entry)}`)
  if (!entry.extremeControls?.length || !entry.rois?.length) throw new Error(`Case lacks controls or ROIs: ${entry.id}`)
  required.delete(entry.id)
}
if (required.size) throw new Error(`Missing required Golden cases: ${[...required].join(', ')}`)
console.log(`OK fixtures/golden/manifest.json (${manifest.cases.length} required cases)`)

const rawManifestUrl = new URL('../fixtures/raw/manifest.json', import.meta.url)
const rawManifest = JSON.parse(readFileSync(rawManifestUrl, 'utf8'))
const requiredFormats = new Set(['NEF', 'ARW', 'CR2', 'CR3', 'DNG', 'RAF'])
if (rawManifest.schemaVersion !== 1 || rawManifest.provider?.license !== 'CC0-1.0') {
  throw new Error('RAW fixture manifest schema or provider license is invalid')
}
for (const fixture of rawManifest.fixtures ?? []) {
  for (const field of ['id', 'path', 'upstreamPath', 'sourceUrl', 'cameraMake', 'cameraModel', 'format', 'sha256', 'license']) {
    if (!fixture[field]) throw new Error(`RAW fixture ${fixture.id ?? '<unknown>'} lacks ${field}`)
  }
  if (fixture.license !== 'CC0-1.0') throw new Error(`RAW fixture ${fixture.id} is not CC0`)
  const sourceUrl = new URL(fixture.path, rawManifestUrl)
  if (!existsSync(sourceUrl)) throw new Error(`RAW fixture is missing: ${fixture.path}`)
  const bytes = readFileSync(sourceUrl)
  if (statSync(sourceUrl).size !== fixture.byteLength) throw new Error(`RAW fixture length mismatch: ${fixture.id}`)
  const hash = createHash('sha256').update(bytes).digest('hex')
  if (hash !== fixture.sha256) throw new Error(`RAW fixture hash mismatch: ${fixture.id}`)
  requiredFormats.delete(fixture.format)
}
if (requiredFormats.size) throw new Error(`Missing RAW formats: ${[...requiredFormats].join(', ')}`)
console.log(`OK fixtures/raw/manifest.json (${rawManifest.fixtures.length} CC0 sensor fixtures)`)
