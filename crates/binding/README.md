# @shined/ecma-compat

ECMAScript syntax compatibility analyzer for JavaScript files.

This package detects syntax features that still exist in your source or build output, then reports whether those features are supported by the target runtimes you provide. It focuses on syntax compatibility, not runtime API compatibility.

## Usage

```js
const { checkFiles } = require("@shined/ecma-compat");

const report = checkFiles(
  ["src/**/*.{js,jsx}", "dist/**/*.mjs"],
  ["chrome 60", "safari 13"],
  {
    cwd: process.cwd(),
  },
);

console.log(report.fileCount);
console.log(report.diagnosticCount);
```

## API

```ts
checkFiles(
  patterns: string[],
  targets: string[],
  options?: CheckFilesOptions | null,
): CompatFilesReport
```

`patterns` is a required include glob list resolved relative to `options.cwd`.

```ts
interface CheckFilesOptions {
  cwd?: string;
  extensions?: string[];
  respectGitignore?: boolean;
  ignoreHidden?: boolean;
  parallelism?: number;
  excludeEmptyReports?: boolean;
  includeSupportedTargets?: boolean;
}
```

- `cwd` defaults to the current process working directory.
- `extensions` defaults to `["js", "mjs", "cjs", "jsx"]`.
- `respectGitignore` defaults to `false`.
- `ignoreHidden` defaults to `false`.
- `parallelism` limits the Rayon worker count used by the analysis stage.
- `excludeEmptyReports` defaults to `true`.

The package also exports `checkFile(path, targets, options)` for analyzing a single file.

## Scope

This analyzer reports ECMAScript syntax features such as optional chaining, nullish coalescing, class fields, and ESM import/export. It does not check runtime APIs such as `Promise.any()`, `Array.prototype.at()`, or `Object.hasOwn()`.
