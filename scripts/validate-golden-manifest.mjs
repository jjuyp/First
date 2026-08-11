import { readFileSync } from 'node:fs'

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
