<div align="center">

<img src="../assets/logo.svg" width="120" alt="n3v3 logo">

<h1>Examples / 示例</h1>

<p><em>Representative runnable samples for n3v3 — also serves as teaching material.</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="../docs/">Docs</a></strong>
</p>

</div>

---

## Basics / 基础

| File | Content |
|------|---------|
| [`basics/arithmetic.n3v3`](basics/arithmetic.n3v3) | Integer arithmetic, operator precedence, negative numbers |
| [`basics/booleans.n3v3`](basics/booleans.n3v3) | Boolean logic, equality, short-circuit evaluation |
| [`basics/variables.n3v3`](basics/variables.n3v3) | Let bindings, shadowing, nested scopes |

## Functions / 函数

| File | Content |
|------|---------|
| [`functions/lambda.n3v3`](functions/lambda.n3v3) | Lambda expressions, higher-order functions, closures |
| [`functions/pipe.n3v3`](functions/pipe.n3v3) | Pipe operator `\|>`, function chaining |

## Control Flow / 控制流

| File | Content |
|------|---------|
| [`control-flow/match.n3v3`](control-flow/match.n3v3) | Pattern matching with `match`, wildcard and binding patterns |

## Data / 数据结构

| File | Content |
|------|---------|
| [`data/records.n3v3`](data/records.n3v3) | Record creation, field access, nested records |
| [`data/lists.n3v3`](data/lists.n3v3) | List literals, map/filter/length, concatenation |

## I/O / 输入输出

| File | Content |
|------|---------|
| [`io/files.n3v3`](io/files.n3v3) | File read/write/append, directory operations |
| [`io/process.n3v3`](io/process.n3v3) | Process execution, pipelines, stdin, exit codes |

## Running / 运行

```bash
n3v3 run examples/basics/arithmetic.n3v3
n3v3 run examples/functions/lambda.n3v3
n3v3 run examples/control-flow/match.n3v3
n3v3 run examples/data/records.n3v3
n3v3 run examples/io/files.n3v3
```

## Bootstrap

```bash
ls examples/bootstrap
n3v3 show examples/bootstrap/musl.n3v3
```
