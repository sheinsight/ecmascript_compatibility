const assert = require('node:assert/strict')
const { mkdirSync, mkdtempSync, writeFileSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join, relative, sep } = require('node:path')
const test = require('node:test')

const { analyzeCwd } = require('..')

test('analyzeCwd recursively scans JavaScript files under cwd', () => {
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

  const report = analyzeCwd(cwd, ['chrome 60'])
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.fileCount, 3)
  assert.equal(report.errors.length, 0)
  assert.deepEqual(paths, ['b.cjs', 'src/a.js', 'src/view.jsx'])
  assert.ok(report.diagnosticCount > 0)
})

test('analyzeCwd accepts custom extensions', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'ecmascript-compat-'))
  writeFileSync(cwd + '/component.jsx', 'export const value = item?.name;\n')
  writeFileSync(cwd + '/ignored.js', 'const value = item?.name;\n')

  const report = analyzeCwd(cwd, ['chrome 60'], { extensions: ['.jsx'] })
  const paths = report.reports.map((item) =>
    relative(report.cwd, item.path).split(sep).join('/'),
  )

  assert.equal(report.fileCount, 1)
  assert.deepEqual(paths, ['component.jsx'])
})
