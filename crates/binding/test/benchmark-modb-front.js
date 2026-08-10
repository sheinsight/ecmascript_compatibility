const { analyzeCwd } = require('..')
const fs = require('fs')

const cwd = '/Users/10015448/Git/modb-front/dist/statics'
const targets = ['chrome 70']
const parallelism = process.env.PARALLELISM
  ? Number(process.env.PARALLELISM)
  : undefined

const cwdOptions = {
  ...(parallelism ? { parallelism } : {}),
  includeSupportedTargets: false,
  // excludeEmptyReports:true
}

const cwdReport = analyzeCwd(cwd, targets, cwdOptions)

console.log(JSON.stringify(cwdReport, null, 2))

fs.writeFileSync(
  'benchmark-modb-front.json',
  JSON.stringify(cwdReport, null, 2),
  'utf-8'
)