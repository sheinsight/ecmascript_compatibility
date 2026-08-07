const { readdirSync, statSync } = require('node:fs')
const { join, relative, sep } = require('node:path')
const { performance } = require('node:perf_hooks')

const { analyzeCwd, analyzePath } = require('..')

const cwd = '/Users/10015448/Git/modb-front/dist/statics'
const targets = ['chrome 60']
const sourceMaps = process.env.SOURCE_MAPS === '1'
const parallelism = process.env.PARALLELISM
  ? Number(process.env.PARALLELISM)
  : undefined
const extensions = new Set(['.js', '.mjs', '.cjs', '.jsx'])

const files = collectJavaScriptFiles(cwd)
const cwdOptions = {
  sourceMaps,
  ...(parallelism ? { parallelism } : {}),
}

const cwdStartedAt = performance.now()
const cwdReport = analyzeCwd(cwd, targets, cwdOptions)
const cwdElapsedMs = performance.now() - cwdStartedAt

const fileRows = []
const filesStartedAt = performance.now()

for (const file of files) {
  const startedAt = performance.now()
  const report = analyzePath(file, targets, { sourceMaps })
  const elapsedMs = performance.now() - startedAt

  fileRows.push({
    path: relativePath(file),
    size: statSync(file).size,
    elapsedMs: Math.round(elapsedMs),
    detectedUsageCount: report.detectedUsageCount,
    diagnosticCount: report.diagnostics.length,
  })
}

const filesElapsedMs = performance.now() - filesStartedAt
fileRows.sort((left, right) => right.elapsedMs - left.elapsedMs)

console.log(
  JSON.stringify(
    {
      cwd,
      targets,
      sourceMaps,
      parallelism: parallelism || 'rayon-default',
      fileCount: files.length,
      analyzeCwd: {
        elapsedMs: Math.round(cwdElapsedMs),
        analyzedFileCount: cwdReport.fileCount,
        skippedFileCount: cwdReport.skippedFileCount,
        errorCount: cwdReport.errors.length,
        diagnosticCount: cwdReport.diagnosticCount,
      },
      analyzePathSerialTotal: {
        elapsedMs: Math.round(filesElapsedMs),
        top15ElapsedMs: fileRows
          .slice(0, 15)
          .reduce((sum, row) => sum + row.elapsedMs, 0),
      },
      slowestFiles: fileRows.slice(0, 15),
      firstErrors: cwdReport.errors.slice(0, 10).map((error) => ({
        path: relativePath(error.path),
        message: error.message,
      })),
    },
    null,
    2,
  ),
)

function collectJavaScriptFiles(dir) {
  const files = []

  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name)

    if (entry.isDirectory()) {
      files.push(...collectJavaScriptFiles(path))
    } else if (entry.isFile() && extensions.has(extensionOf(entry.name))) {
      files.push(path)
    }
  }

  return files.sort()
}

function extensionOf(fileName) {
  const index = fileName.lastIndexOf('.')
  return index === -1 ? '' : fileName.slice(index).toLowerCase()
}

function relativePath(path) {
  return relative(cwd, path).split(sep).join('/')
}
