const assert = require('node:assert/strict')
const { mkdirSync, mkdtempSync, realpathSync, writeFileSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join, relative, sep } = require('node:path')
const test = require('node:test')

const { checkDirectory } = require('..')

test('checkDirectory recursively scans JavaScript files under cwd', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  mkdirSync(join(cwd, 'src'))

  writeFileSync(join(cwd, 'src/a.js'), 'const value = object?.field ?? 1;\n')
  writeFileSync(
    join(cwd, 'b.cjs'),
    'module.exports = async () => await import("./x.js");\n',
  )
  writeFileSync(
    join(cwd, 'src/view.jsx'),
    'export const View = () => <div>{items?.length}</div>;\n',
  )
  writeFileSync(join(cwd, 'src/ignored.ts'), 'const ignored: number = 1;\n')

  const report = checkDirectory(cwd, ['chrome 60'])
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.fileCount, 3)
  assert.equal(report.errors.length, 0)
  assert.deepEqual(paths, ['b.cjs', 'src/a.js', 'src/view.jsx'])
  assert.ok(report.diagnosticCount > 0)
  assert.ok(report.reports.every((item) => !('timing' in item)))
})

test('checkDirectory accepts custom extensions', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  writeFileSync(cwd + '/component.jsx', 'export const value = item?.name;\n')
  writeFileSync(cwd + '/ignored.js', 'const value = item?.name;\n')

  const report = checkDirectory(cwd, ['chrome 60'], { extensions: ['.jsx'] })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.fileCount, 1)
  assert.deepEqual(paths, ['component.jsx'])
})

test('checkDirectory excludes reports without diagnostics by default', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  writeFileSync(join(cwd, 'modern.js'), 'const value = object?.field;\n')
  writeFileSync(join(cwd, 'legacy.js'), 'const value = object.field;\n')

  const report = checkDirectory(cwd, ['chrome 60'])
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.fileCount, 1)
  assert.deepEqual(paths, ['modern.js'])
  assert.ok(report.reports.every((item) => item.diagnostics.length > 0))
})

test('checkDirectory can include reports without diagnostics', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  writeFileSync(join(cwd, 'modern.js'), 'const value = object?.field;\n')
  writeFileSync(join(cwd, 'legacy.js'), 'const value = object.field;\n')

  const report = checkDirectory(cwd, ['chrome 60'], {
    excludeEmptyReports: false,
  })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.fileCount, 2)
  assert.deepEqual(paths, ['legacy.js', 'modern.js'])
  assert.ok(report.reports.some((item) => item.diagnostics.length === 0))
})

test('checkDirectory returns source map references as plain strings', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  const sourcePath = join(cwd, 'bundle.js')
  const sourceMapPath = `${sourcePath}.map`

  writeFileSync(
    sourcePath,
    'const value = object?.field;\n//# sourceMappingURL=bundle.js.map\n',
  )
  writeFileSync(
    sourceMapPath,
    JSON.stringify({
      version: 3,
      sources: ['src/input.js'],
      names: [],
      mappings: 'AAAA',
    }),
  )

  const report = checkDirectory(cwd, ['chrome 60'])

  assert.equal(report.fileCount, 1)
  assert.equal(report.reports[0].sourceMapStatus.kind, 'resolved')
  assert.equal(
    report.reports[0].sourceMapStatus.reference,
    realpathSync(sourceMapPath),
  )
})
