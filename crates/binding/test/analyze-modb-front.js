const { relative, sep } = require('node:path')

const { analyzeCwd } = require('..')

const cwd = '/Users/10015448/Git/modb-front/dist/statics'
const targets = ['chrome 60']
const startedAt = Date.now()
const report = analyzeCwd(cwd, targets, {
  // sourceMaps: false,
})

console.log(
  JSON.stringify(
    {
      cwd,
      targets,
      elapsedMs: Date.now() - startedAt,
      analyzedFileCount: report.fileCount,
      skippedFileCount: report.skippedFileCount,
      errorCount: report.errors.length,
      diagnosticCount: report.diagnosticCount,
      topReports: report.reports
        .filter((item) => item.diagnostics.length > 0)
        .slice(0, 10)
        .map((item) => ({
          path: relativePath(item.path),
          detectedUsageCount: item.detectedUsageCount,
          diagnosticCount: item.diagnostics.length,
          firstDiagnostic: item.diagnostics[0],
        })),
      firstSkipped: report.skipped.slice(0, 10).map((item) => ({
        path: relativePath(item.path),
        size: item.size,
      })),
      firstErrors: report.errors.slice(0, 10).map((item) => ({
        path: relativePath(item.path),
        message: item.message,
      })),
    },
    null,
    2,
  ),
)

function relativePath(path) {
  return relative(cwd, path).split(sep).join('/')
}
