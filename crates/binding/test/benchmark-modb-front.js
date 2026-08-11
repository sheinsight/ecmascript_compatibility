const { checkFiles } = require('..')
const fs = require('fs')

const cwd = '/Users/10015448/Git/modb-front/dist/statics'
const targets = ['chrome 70']
const parallelism = process.env.PARALLELISM
  ? Number(process.env.PARALLELISM)
  : undefined

const filesOptions = {
  ...(parallelism ? { parallelism } : {}),
  includeSupportedTargets: false,
  // excludeEmptyReports:true
}

const filesReport = checkFiles(['**/*.js'], targets, {
  ...filesOptions,
  cwd,
})

console.log(JSON.stringify(filesReport, null, 2))

fs.writeFileSync(
  'benchmark-modb-front.json',
  JSON.stringify(filesReport, null, 2),
  'utf-8'
)
