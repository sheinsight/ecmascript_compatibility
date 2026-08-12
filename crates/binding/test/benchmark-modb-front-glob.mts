import { globby } from "globby"
import { checkFileList } from "../index.js"

const cwd = '/Users/10015448/Git/modb-front/dist/statics'
const targets = ['chrome 70']


const res = await globby(['**/*.js'], {
  cwd,
  dot: true,
  gitignore: false,
  onlyFiles: true,
  absolute: true,
})

const filesReport = await checkFileList(res, targets, {
  targetStatus: 'problems',
  cwd,
})

console.log(JSON.stringify(filesReport, null, 2))
