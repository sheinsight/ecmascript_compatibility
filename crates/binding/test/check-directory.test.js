const assert = require('node:assert/strict')
const { mkdirSync, mkdtempSync, realpathSync, writeFileSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join, relative, sep } = require('node:path')
const test = require('node:test')

const { checkFiles } = require('..')

test('checkFiles scans files matching patterns under cwd', () => {
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

  const report = checkFiles(['*.cjs', 'src/**/*.{js,jsx}'], ['chrome 60'], {
    cwd,
  })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.deepEqual(report.counts, {
    matchedFiles: 3,
    analyzedFiles: 3,
    reportedFiles: 3,
    diagnostics: report.reports.reduce(
      (count, item) => count + item.diagnostics.length,
      0,
    ),
    errors: 0,
  })
  assert.equal(report.errors.length, 0)
  assert.deepEqual(paths, ['b.cjs', 'src/a.js', 'src/view.jsx'])
  assert.ok(report.counts.diagnostics > 0)
  assert.ok(report.reports.every((item) => !('timing' in item)))
})

test('checkFiles accepts custom extensions', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  writeFileSync(cwd + '/component.jsx', 'export const value = item?.name;\n')
  writeFileSync(cwd + '/ignored.js', 'const value = item?.name;\n')

  const report = checkFiles(['*'], ['chrome 60'], {
    cwd,
    extensions: ['.jsx'],
  })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.counts.matchedFiles, 1)
  assert.equal(report.counts.analyzedFiles, 1)
  assert.equal(report.counts.reportedFiles, 1)
  assert.deepEqual(paths, ['component.jsx'])
})

test('checkFiles filters files with include patterns', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  mkdirSync(join(cwd, 'src'))
  mkdirSync(join(cwd, 'dist'))

  writeFileSync(join(cwd, 'src/app.js'), 'const value = item?.name;\n')
  writeFileSync(join(cwd, 'dist/app.js'), 'const value = item?.name;\n')

  const report = checkFiles(['src/**/*.js'], ['chrome 60'], { cwd })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.counts.matchedFiles, 1)
  assert.equal(report.counts.analyzedFiles, 1)
  assert.equal(report.counts.reportedFiles, 1)
  assert.deepEqual(paths, ['src/app.js'])
})

test('checkFiles accepts multiple include patterns', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  mkdirSync(join(cwd, 'src'))
  mkdirSync(join(cwd, 'dist'))

  writeFileSync(join(cwd, 'src/app.js'), 'const value = item?.name;\n')
  writeFileSync(join(cwd, 'dist/app.mjs'), 'const value = item?.name;\n')
  writeFileSync(join(cwd, 'ignored.js'), 'const value = item?.name;\n')

  const report = checkFiles(['src/**/*.js', 'dist/**/*.mjs'], ['chrome 60'], {
    cwd,
  })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.counts.matchedFiles, 2)
  assert.equal(report.counts.analyzedFiles, 2)
  assert.equal(report.counts.reportedFiles, 2)
  assert.deepEqual(paths, ['dist/app.mjs', 'src/app.js'])
})

test('checkFiles rejects empty patterns', () => {
  assert.throws(
    () => checkFiles([], ['chrome 60']),
    /at least one file pattern is required/,
  )
})

test('checkFiles excludes reports without diagnostics by default', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  writeFileSync(join(cwd, 'modern.js'), 'const value = object?.field;\n')
  writeFileSync(join(cwd, 'legacy.js'), 'const value = object.field;\n')

  const report = checkFiles(['*.js'], ['chrome 60'], { cwd })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.counts.matchedFiles, 2)
  assert.equal(report.counts.analyzedFiles, 2)
  assert.equal(report.counts.reportedFiles, 1)
  assert.deepEqual(paths, ['modern.js'])
  assert.ok(report.reports.every((item) => item.diagnostics.length > 0))
})

test('checkFiles can include reports without diagnostics', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  writeFileSync(join(cwd, 'modern.js'), 'const value = object?.field;\n')
  writeFileSync(join(cwd, 'legacy.js'), 'const value = object.field;\n')

  const report = checkFiles(['*.js'], ['chrome 60'], {
    cwd,
    excludeEmptyReports: false,
  })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.counts.matchedFiles, 2)
  assert.equal(report.counts.analyzedFiles, 2)
  assert.equal(report.counts.reportedFiles, 2)
  assert.deepEqual(paths, ['legacy.js', 'modern.js'])
  assert.ok(report.reports.some((item) => item.diagnostics.length === 0))
})

test('checkFiles returns source map references as plain strings', () => {
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

  const report = checkFiles(['*.js'], ['chrome 60'], { cwd })

  assert.equal(report.counts.matchedFiles, 1)
  assert.equal(report.counts.analyzedFiles, 1)
  assert.equal(report.counts.reportedFiles, 1)
  assert.equal(report.reports[0].sourceMapStatus.kind, 'resolved')
  assert.equal(
    report.reports[0].sourceMapStatus.reference,
    realpathSync(sourceMapPath),
  )
})
