<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Examples / 示例</h1>

<p><em>Representative runnable samples for Neve — also serves as teaching material.</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="../docs/">Docs</a></strong>
</p>

</div>

---

## Basics / 基础

| File | Content |
|------|---------|
| [`basics/arithmetic.neve`](basics/arithmetic.neve) | Integer arithmetic, operator precedence, negative numbers |
| [`basics/booleans.neve`](basics/booleans.neve) | Boolean logic, equality, short-circuit evaluation |
| [`basics/variables.neve`](basics/variables.neve) | Let bindings, shadowing, nested scopes |

## Functions / 函数

| File | Content |
|------|---------|
| [`functions/lambda.neve`](functions/lambda.neve) | Lambda expressions, higher-order functions, closures |
| [`functions/pipe.neve`](functions/pipe.neve) | Pipe operator `\|>`, function chaining |

## Control Flow / 控制流

| File | Content |
|------|---------|
| [`control-flow/match.neve`](control-flow/match.neve) | Pattern matching with `match`, wildcard and binding patterns |

## Data / 数据结构

| File | Content |
|------|---------|
| [`data/records.neve`](data/records.neve) | Record creation, field access, nested records |
| [`data/lists.neve`](data/lists.neve) | List literals, map/filter/length, concatenation |

## I/O / 输入输出

| File | Content |
|------|---------|
| [`io/files.neve`](io/files.neve) | File read/write/append, directory operations |
| [`io/process.neve`](io/process.neve) | Process execution, pipelines, stdin, exit codes |

## Running / 运行

```bash
neve run examples/basics/arithmetic.neve
neve run examples/functions/lambda.neve
neve run examples/control-flow/match.neve
neve run examples/data/records.neve
neve run examples/io/files.neve
```

## Bootstrap

```bash
ls examples/bootstrap
neve show examples/bootstrap/musl.neve
```
