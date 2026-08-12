export const goldenTags = [
  'raw', 'camera-color', 'tone', 'wb', 'curve', 'color', 'grading', 'detail',
  'optics', 'geometry', 'mask', 'portrait', 'skin', 'ai', 'night', 'high-iso',
  'neon', 'landscape', 'hdr',
]

export const targets = {
  color: {
    rust: [['test', '--locked', '-p', 'starroom-color', '-p', 'starroom-color-management', '-p', 'starroom-grading']],
    web: ['src/imagePipeline.test.ts', 'src/nativeRender.test.ts'],
    golden: ['color', 'camera-color'],
  },
  tone: {
    rust: [
      ['test', '--locked', '-p', 'starroom-color', 'tone'],
      ['test', '--locked', '-p', 'starroom-pipeline', 'shadow'],
      ['test', '--locked', '-p', 'starroom-pipeline', 'preview_and_export'],
    ],
    web: ['src/imagePipeline.test.ts', 'src/nativeRender.test.ts'],
    golden: ['tone', 'portrait', 'night', 'hdr'],
  },
  curve: {
    rust: [
      ['test', '--locked', '-p', 'starroom-color', 'curve'],
      ['test', '--locked', '-p', 'starroom-pipeline', 'curve'],
      ['test', '--locked', '-p', 'starroom-pipeline', 'portrait_and_gradient'],
      ['test', '--locked', '-p', 'starroom-project', 'adjustment_state'],
    ],
    web: ['src/imagePipeline.test.ts', 'src/nativeRender.test.ts', 'src/editorState.test.ts'],
    golden: ['curve', 'portrait'],
  },
  raw: {
    rust: [
      ['test', '--locked', '-p', 'starroom-raw'],
      ['test', '--locked', '-p', 'starroom-pipeline', '--test', 'raw_shared_graph'],
    ],
    web: [],
    golden: ['raw', 'camera-color'],
  },
  detail: {
    rust: [['test', '--locked', '-p', 'starroom-detail', '-p', 'starroom-heal', '-p', 'starroom-portrait']],
    web: ['src/imagePipeline.test.ts'],
    golden: ['detail', 'high-iso'],
  },
  optics: {
    rust: [['test', '--locked', '-p', 'starroom-optics']],
    web: [],
    golden: ['optics'],
  },
  geometry: {
    rust: [['test', '--locked', '-p', 'starroom-geometry']],
    web: [],
    golden: ['geometry', 'landscape'],
  },
  gpu: {
    rust: [
      ['test', '--locked', '-p', 'starroom-render', 'gpu'],
      ['test', '--locked', '-p', 'starroom-pipeline', 'm12_gpu'],
    ],
    web: ['src/nativeRender.test.ts'],
    golden: ['raw', 'tone', 'curve', 'color', 'grading', 'detail', 'portrait', 'skin', 'neon', 'landscape', 'hdr'],
  },
  masks: {
    rust: [['test', '--locked', '-p', 'starroom-project', 'mask'], ['test', '--locked', '-p', 'starroom-render']],
    web: ['src/editorState.test.ts'],
    golden: ['mask'],
  },
  portrait: {
    rust: [['test', '--locked', '-p', 'starroom-portrait', '-p', 'starroom-detail']],
    web: [],
    golden: ['portrait', 'skin'],
  },
  ai: {
    rust: [['test', '--locked', '-p', 'starroom-advisor']],
    web: [],
    golden: ['ai'],
  },
  web: {
    rust: [],
    web: ['src'],
    golden: ['tone', 'curve'],
  },
}

export const sharedGraphRust = [
  ['test', '--locked', '-p', 'starroom-pipeline'],
  ['test', '--locked', '-p', 'starroom-render'],
]
