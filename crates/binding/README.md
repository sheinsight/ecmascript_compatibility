# @shined/ecma-compat

ECMAScript syntax compatibility analyzer for JavaScript files.

This package detects syntax features that still exist in your source or build output, then reports whether those features are supported by the target runtimes you provide. It focuses on syntax compatibility, not runtime API compatibility.

## Usage

```js
const { checkFileList } = require("@shined/ecma-compat");

const report = await checkFileList(
  ["dist/app.js", "dist/chunk.js"],
  ["chrome 60", "safari 13"],
  {
    cwd: process.cwd(),
  },
);

console.log(report.counts.reportedFiles);
console.log(report.counts.diagnostics);
```

## API

```ts
checkFileList(
  files: string[],
  targets: string[],
  options?: CheckFileListOptions | null,
): Promise<CompatFilesReport>
```

`files` is the explicit file list to analyze. File discovery, globbing, and ignore rules belong to the caller.

```ts
interface CheckFileListOptions {
  cwd?: string;
  parallelism?: number;
  includeEmptyReports?: boolean;
  sourceMap?: "auto" | "always" | "off";
  targetStatus?: "problems" | "all";
}
```

- `cwd` is optional report metadata used for relative path display.
- `files` may be absolute paths or paths relative to the current process working directory.
- `parallelism` limits the Rayon worker count used by the analysis stage.
- `includeEmptyReports` defaults to `false`.
- `sourceMap` defaults to `"auto"` for `checkFileList` and `"always"` for `checkFile`.
- `targetStatus` defaults to `"problems"`.

The package also exports `checkFile(path, targets, options)` for analyzing a single file. Both APIs return promises.

## Scope

This analyzer reports ECMAScript syntax features such as optional chaining, nullish coalescing, class fields, and ESM import/export. It does not check runtime APIs such as `Promise.any()`, `Array.prototype.at()`, or `Object.hasOwn()`.
