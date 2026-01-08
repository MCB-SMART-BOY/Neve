```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                         NEVE DIAGNOSTIC CODES                                ║
║                             诊断错误码说明                                   ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  Overview / Lexer / Parser / Type / Eval         │
│  [中文]     #chinese   ──→  概览 / 词法 / 语法 / 类型 / 求值                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

<a name="english"></a>

# English

> *Error codes are the stable identifiers for diagnostics.*

## Overview

- Codes are grouped by component: lexer (E0001+), parser (E0100+), type checker (E0200+), eval (E0300+).
- CLI output and LSP diagnostics show these codes so you can search them quickly.
- Some codes include a suggested fix; the compiler will surface it as `help` when available.

## Lexer Errors (E0001 - E0005)

<a name="E0001"></a>
### E0001 — Unexpected character
- Description: unexpected character in input

<a name="E0002"></a>
### E0002 — Unterminated string
- Description: string literal is not terminated
- Suggestion: add a closing quote `"` to terminate the string

<a name="E0003"></a>
### E0003 — Unterminated comment
- Description: comment is not terminated
- Suggestion: add `-- --` to close the block comment

<a name="E0004"></a>
### E0004 — Invalid escape
- Description: invalid escape sequence in string

<a name="E0005"></a>
### E0005 — Invalid number
- Description: invalid number literal

## Parser Errors (E0100 - E0105)

<a name="E0100"></a>
### E0100 — Unexpected token
- Description: unexpected token

<a name="E0101"></a>
### E0101 — Expected expression
- Description: expected an expression

<a name="E0102"></a>
### E0102 — Expected pattern
- Description: expected a pattern

<a name="E0103"></a>
### E0103 — Expected type
- Description: expected a type

<a name="E0104"></a>
### E0104 — Unclosed delimiter
- Description: unclosed delimiter
- Suggestion: add the matching closing delimiter

<a name="E0105"></a>
### E0105 — Missing semicolon
- Description: missing semicolon
- Suggestion: add `;` at the end of the statement

## Type Errors (E0200 - E0223)

<a name="E0200"></a>
### E0200 — Type mismatch
- Description: mismatched types

<a name="E0201"></a>
### E0201 — Unbound variable
- Description: cannot find value in this scope
- Suggestion: check the spelling or ensure the variable is in scope

<a name="E0202"></a>
### E0202 — Unbound type
- Description: cannot find type in this scope
- Suggestion: check the spelling or import the type

<a name="E0203"></a>
### E0203 — Infinite type
- Description: cannot construct infinite type

<a name="E0204"></a>
### E0204 — Not a function
- Description: expected a function, found a different type

<a name="E0205"></a>
### E0205 — Wrong arity
- Description: wrong number of arguments
- Suggestion: check the function signature for the expected number of arguments

<a name="E0206"></a>
### E0206 — Missing field
- Description: missing field in record
- Suggestion: add the missing field to the record

<a name="E0207"></a>
### E0207 — Unknown field
- Description: unknown field in record

<a name="E0208"></a>
### E0208 — Trait not implemented
- Description: trait is not implemented for type

<a name="E0209"></a>
### E0209 — Missing method
- Description: missing required method in trait implementation
- Suggestion: implement all required methods for the trait

<a name="E0210"></a>
### E0210 — Missing associated type
- Description: missing required associated type in trait implementation
- Suggestion: specify all required associated types in the impl block

<a name="E0211"></a>
### E0211 — If/else type mismatch
- Description: if and else branches have incompatible types

<a name="E0212"></a>
### E0212 — Match arm mismatch
- Description: match arms have incompatible types

<a name="E0213"></a>
### E0213 — Return type mismatch
- Description: return type does not match function signature

<a name="E0214"></a>
### E0214 — Argument type mismatch
- Description: argument type does not match parameter type

<a name="E0215"></a>
### E0215 — Binary operator mismatch
- Description: binary operator cannot be applied to these types

<a name="E0216"></a>
### E0216 — Unary operator mismatch
- Description: unary operator cannot be applied to this type

<a name="E0217"></a>
### E0217 — Cannot infer type
- Description: cannot infer type

<a name="E0218"></a>
### E0218 — Recursive type
- Description: recursive type detected

<a name="E0219"></a>
### E0219 — Ambiguous type
- Description: type is ambiguous

<a name="E0220"></a>
### E0220 — Non-exhaustive match
- Description: match expression is not exhaustive
- Suggestion: add patterns for all possible cases or use a wildcard `_` pattern

<a name="E0221"></a>
### E0221 — Unreachable pattern
- Description: unreachable pattern in match
- Suggestion: remove the unreachable pattern or reorder the match arms

<a name="E0222"></a>
### E0222 — Private access
- Description: cannot access private binding
- Suggestion: make the binding public with `pub` or access it from within the same module

<a name="E0223"></a>
### E0223 — Cyclic dependency
- Description: cyclic dependency detected
- Suggestion: break the cycle by restructuring the dependencies

## Eval Errors (E0300 - E0302)

<a name="E0300"></a>
### E0300 — Division by zero
- Description: division by zero

<a name="E0301"></a>
### E0301 — Assertion failed
- Description: assertion failed

<a name="E0302"></a>
### E0302 — Pattern match failed
- Description: pattern matching failed

---

<a name="chinese"></a>

# 中文

> *错误码是诊断信息的稳定标识。*

## 概览

- 错误码按模块划分：词法（E0001+）、语法（E0100+）、类型（E0200+）、求值（E0300+）。
- CLI 输出和 LSP 诊断都会显示这些代码，方便快速定位。
- 部分错误码带有修复建议，编译器会以 `help` 形式展示。

## 词法错误 (E0001 - E0005)

<a name="E0001"></a>
### E0001 — 非法字符
- 描述：输入中出现了意料之外的字符

<a name="E0002"></a>
### E0002 — 字符串未闭合
- 描述：字符串字面量没有结束
- 建议：补上结束引号 `"`

<a name="E0003"></a>
### E0003 — 注释未闭合
- 描述：注释块没有结束
- 建议：补上结束标记 `-- --`

<a name="E0004"></a>
### E0004 — 无效转义
- 描述：字符串中出现了无效的转义序列

<a name="E0005"></a>
### E0005 — 无效数字
- 描述：数字字面量格式不合法

## 语法错误 (E0100 - E0105)

<a name="E0100"></a>
### E0100 — 意外的 token
- 描述：遇到了不符合语法的 token

<a name="E0101"></a>
### E0101 — 需要表达式
- 描述：这里应该是一个表达式

<a name="E0102"></a>
### E0102 — 需要模式
- 描述：这里应该是一个模式

<a name="E0103"></a>
### E0103 — 需要类型
- 描述：这里应该是一个类型

<a name="E0104"></a>
### E0104 — 分隔符未闭合
- 描述：有成对分隔符没有闭合
- 建议：补上对应的右括号/右花括号

<a name="E0105"></a>
### E0105 — 缺少分号
- 描述：语句缺少分号结束
- 建议：在语句末尾补上 `;`

## 类型错误 (E0200 - E0223)

<a name="E0200"></a>
### E0200 — 类型不匹配
- 描述：类型不一致

<a name="E0201"></a>
### E0201 — 未绑定变量
- 描述：当前作用域找不到这个变量
- 建议：检查拼写或确保变量在作用域内

<a name="E0202"></a>
### E0202 — 未绑定类型
- 描述：当前作用域找不到这个类型
- 建议：检查拼写或导入该类型

<a name="E0203"></a>
### E0203 — 无限类型
- 描述：无法构造无限类型

<a name="E0204"></a>
### E0204 — 不是函数
- 描述：期望函数类型，但实际不是

<a name="E0205"></a>
### E0205 — 参数数量错误
- 描述：实参与形参数量不匹配
- 建议：检查函数签名要求的参数数量

<a name="E0206"></a>
### E0206 — 记录缺字段
- 描述：记录缺少必需字段
- 建议：为记录补上缺失字段

<a name="E0207"></a>
### E0207 — 记录字段未知
- 描述：访问了不存在的字段

<a name="E0208"></a>
### E0208 — Trait 未实现
- 描述：该类型没有实现指定的 trait

<a name="E0209"></a>
### E0209 — 缺少方法
- 描述：trait 实现缺少必需方法
- 建议：补齐 trait 要求的方法

<a name="E0210"></a>
### E0210 — 缺少关联类型
- 描述：trait 实现缺少必需关联类型
- 建议：在 impl 中指定所有关联类型

<a name="E0211"></a>
### E0211 — if/else 类型不一致
- 描述：if 与 else 分支类型不一致

<a name="E0212"></a>
### E0212 — match 分支类型不一致
- 描述：match 的分支类型不一致

<a name="E0213"></a>
### E0213 — 返回类型不匹配
- 描述：返回值类型与函数签名不一致

<a name="E0214"></a>
### E0214 — 实参类型不匹配
- 描述：实参与形参类型不一致

<a name="E0215"></a>
### E0215 — 二元运算类型不匹配
- 描述：该二元运算符无法应用于这些类型

<a name="E0216"></a>
### E0216 — 一元运算类型不匹配
- 描述：该一元运算符无法应用于该类型

<a name="E0217"></a>
### E0217 — 无法推断类型
- 描述：类型无法自动推断

<a name="E0218"></a>
### E0218 — 递归类型
- 描述：检测到递归类型

<a name="E0219"></a>
### E0219 — 类型歧义
- 描述：类型存在歧义

<a name="E0220"></a>
### E0220 — 匹配不穷尽
- 描述：match 没有覆盖所有情况
- 建议：补齐所有分支或使用 `_` 通配符

<a name="E0221"></a>
### E0221 — 不可达模式
- 描述：match 中存在不可达的模式
- 建议：删除不可达分支或调整顺序

<a name="E0222"></a>
### E0222 — 私有访问
- 描述：无法访问私有绑定
- 建议：使用 `pub` 导出或在同一模块内访问

<a name="E0223"></a>
### E0223 — 循环依赖
- 描述：检测到循环依赖
- 建议：调整结构打破依赖环

## 求值错误 (E0300 - E0302)

<a name="E0300"></a>
### E0300 — 除以零
- 描述：发生了除以零

<a name="E0301"></a>
### E0301 — 断言失败
- 描述：断言失败

<a name="E0302"></a>
### E0302 — 模式匹配失败
- 描述：模式匹配失败

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                     Codes are contracts. Learn them.
═══════════════════════════════════════════════════════════════════════════════
```

</div>
