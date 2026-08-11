const { checkDirectory } = require('..')
const fs = require('fs')

const cwd = '/Users/10015448/Git/modb-front/dist/statics'
const targets = ['chrome 70']
const parallelism = process.env.PARALLELISM
  ? Number(process.env.PARALLELISM)
  : undefined

const directoryOptions = {
  ...(parallelism ? { parallelism } : {}),
  includeSupportedTargets: false,
  // excludeEmptyReports:true
}

const directoryReport = checkDirectory(cwd, targets, directoryOptions)

console.log(JSON.stringify(directoryReport, null, 2))

fs.writeFileSync(
  'benchmark-modb-front.json',
  JSON.stringify(directoryReport, null, 2),
  'utf-8'
)