<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

<h1>Diagnostic Codes</h1>

<p><em>诊断错误码说明</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

---

> *Error codes are the stable identifiers for diagnostics.*
> *错误码是诊断信息的稳定标识。*

## Overview / 概览

- Codes are grouped by component: lexer (E0001+), parser (E0100+), type checker (E0200+), eval (E0300+).
- CLI output and LSP diagnostics show these codes so you can search them quickly.
- Some codes include a suggested fix; the compiler will surface it as `help` when available.

- 错误码按模块划分：词法（E0001+）、语法（E0100+）、类型（E0200+）、求值（E0300+）。
- CLI 输出和 LSP 诊断都会显示这些代码，方便快速定位。
- 部分错误码带有修复建议，编译器会以 `help` 形式展示。

## Lexer Errors (E0001 - E0005) / 词法错误 (E0001 - E0005)

<a name="E0001"></a>
### E0001 — Unexpected character / 非法字符
- Description: unexpected character in input
- 描述：输入中出现了意料之外的字符

<a name="E0002"></a>
### E0002 — Unterminated string / 字符串未闭合
- Description: string literal is not terminated
- Suggestion: add a closing quote `"` to terminate the string
- 描述：字符串字面量没有结束
- 建议：补上结束引号 `"`

<a name="E0003"></a>
### E0003 — Unterminated comment / 注释未闭合
- Description: comment is not terminated
- Suggestion: add `-- --` to close the block comment
- 描述：注释块没有结束
- 建议：补上结束标记 `-- --`

<a name="E0004"></a>
### E0004 — Invalid escape / 无效转义
- Description: invalid escape sequence in string
- 描述：字符串中出现了无效的转义序列

<a name="E0005"></a>
### E0005 — Invalid number / 无效数字
- Description: invalid number literal
- 描述：数字字面量格式不合法

## Parser Errors (E0100 - E0107) / 语法错误 (E0100 - E0107)

<a name="E0100"></a>
### E0100 — Unexpected token / 意外的 token
- Description: unexpected token
- 描述：遇到了不符合语法的 token

<a name="E0101"></a>
### E0101 — Expected expression / 需要表达式
- Description: expected an expression
- 描述：这里应该是一个表达式

<a name="E0102"></a>
### E0102 — Expected pattern / 需要模式
- Description: expected a pattern
- 描述：这里应该是一个模式

<a name="E0103"></a>
### E0103 — Expected type / 需要类型
- Description: expected a type
- 描述：这里应该是一个类型

<a name="E0104"></a>
### E0104 — Unclosed delimiter / 分隔符未闭合
- Description: unclosed delimiter
- Suggestion: add the matching closing delimiter
- 描述：有成对分隔符没有闭合
- 建议：补上对应的右括号/右花括号

<a name="E0105"></a>
### E0105 — Missing semicolon / 缺少分号
- Description: missing semicolon
- Suggestion: add `;` at the end of the statement
- 描述：语句缺少分号结束
- 建议：在语句末尾补上 `;`

<a name="E0106"></a>
### E0106 — Expected identifier / 需要标识符
- Description: expected an identifier
- 描述：这里应该是一个标识符（名称）

<a name="E0107"></a>
### E0107 — Invalid tuple index / 无效元组索引
- Description: invalid tuple index
- 描述：元组索引无效（可能过大）

## Type Errors (E0200 - E0226) / 类型错误 (E0200 - E0226)

<a name="E0200"></a>
### E0200 — Type mismatch / 类型不匹配
- Description: mismatched types
- 描述：类型不一致

<a name="E0201"></a>
### E0201 — Unbound variable / 未绑定变量
- Description: cannot find value in this scope
- Suggestion: check the spelling or ensure the variable is in scope
- 描述：当前作用域找不到这个变量
- 建议：检查拼写或确保变量在作用域内

<a name="E0202"></a>
### E0202 — Unbound type / 未绑定类型
- Description: cannot find type in this scope
- Suggestion: check the spelling or import the type
- 描述：当前作用域找不到这个类型
- 建议：检查拼写或导入该类型

<a name="E0203"></a>
### E0203 — Infinite type / 无限类型
- Description: cannot construct infinite type
- 描述：无法构造无限类型

<a name="E0204"></a>
### E0204 — Not a function / 不是函数
- Description: expected a function, found a different type
- 描述：期望函数类型，但实际不是

<a name="E0205"></a>
### E0205 — Wrong arity / 参数数量错误
- Description: wrong number of arguments
- Suggestion: check the function signature for the expected number of arguments
- 描述：实参与形参数量不匹配
- 建议：检查函数签名要求的参数数量

<a name="E0206"></a>
### E0206 — Missing field / 记录缺字段
- Description: missing field in record
- Suggestion: add the missing field to the record
- 描述：记录缺少必需字段
- 建议：为记录补上缺失字段

<a name="E0207"></a>
### E0207 — Unknown field / 记录字段未知
- Description: unknown field in record
- 描述：访问了不存在的字段

<a name="E0208"></a>
### E0208 — Trait not implemented / Trait 未实现
- Description: trait is not implemented for type
- 描述：该类型没有实现指定的 trait

<a name="E0209"></a>
### E0209 — Missing method / 缺少方法
- Description: missing required method in trait implementation
- Suggestion: implement all required methods for the trait
- 描述：trait 实现缺少必需方法
- 建议：补齐 trait 要求的方法

<a name="E0210"></a>
### E0210 — Missing associated type / 缺少关联类型
- Description: missing required associated type in trait implementation
- Suggestion: specify all required associated types in the impl block
- 描述：trait 实现缺少必需关联类型
- 建议：在 impl 中指定所有关联类型

<a name="E0211"></a>
### E0211 — If/else type mismatch / if/else 类型不一致
- Description: if and else branches have incompatible types
- 描述：if 与 else 分支类型不一致

<a name="E0212"></a>
### E0212 — Match arm mismatch / match 分支类型不一致
- Description: match arms have incompatible types
- 描述：match 的分支类型不一致

<a name="E0213"></a>
### E0213 — Return type mismatch / 返回类型不匹配
- Description: return type does not match function signature
- 描述：返回值类型与函数签名不一致

<a name="E0214"></a>
### E0214 — Argument type mismatch / 实参类型不匹配
- Description: argument type does not match parameter type
- 描述：实参与形参类型不一致

<a name="E0215"></a>
### E0215 — Binary operator mismatch / 二元运算类型不匹配
- Description: binary operator cannot be applied to these types
- 描述：该二元运算符无法应用于这些类型

<a name="E0216"></a>
### E0216 — Unary operator mismatch / 一元运算类型不匹配
- Description: unary operator cannot be applied to this type
- 描述：该一元运算符无法应用于该类型

<a name="E0217"></a>
### E0217 — Cannot infer type / 无法推断类型
- Description: cannot infer type
- 描述：类型无法自动推断

<a name="E0218"></a>
### E0218 — Recursive type / 递归类型
- Description: recursive type detected
- 描述：检测到递归类型

<a name="E0219"></a>
### E0219 — Ambiguous type / 类型歧义
- Description: type is ambiguous
- 描述：类型存在歧义

<a name="E0220"></a>
### E0220 — Non-exhaustive match / 匹配不穷尽
- Description: match expression is not exhaustive
- Suggestion: add patterns for all possible cases or use a wildcard `_` pattern
- 描述：match 没有覆盖所有情况
- 建议：补齐所有分支或使用 `_` 通配符

<a name="E0221"></a>
### E0221 — Unreachable pattern / 不可达模式
- Description: unreachable pattern in match
- Suggestion: remove the unreachable pattern or reorder the match arms
- 描述：match 中存在不可达的模式
- 建议：删除不可达分支或调整顺序

<a name="E0222"></a>
### E0222 — Private access / 私有访问
- Description: cannot access private binding
- Suggestion: ensure the binding is accessible from the calling module
- 描述：无法访问私有绑定
- 建议：确保绑定可从调用模块访问

<a name="E0223"></a>
### E0223 — Cyclic dependency / 循环依赖
- Description: cyclic dependency detected
- Suggestion: break the cycle by restructuring the dependencies
- 描述：检测到循环依赖
- 建议：调整结构打破依赖环

<a name="E0224"></a>
### E0224 — Unknown method / 未知方法
- Description: cannot resolve method call on receiver type
- Suggestion: implement the method for the receiver type or define a matching callable fallback
- 描述：当前接收者类型上无法解析该方法调用
- 建议：为接收者类型实现该方法，或定义匹配的 callable fallback

<a name="E0225"></a>
### E0225 — Unused variable / 未使用变量
- Description: unused variable (warning)
- Suggestion: prefix the variable name with an underscore to suppress this warning
- 描述：存在未使用的变量（警告）
- 建议：在变量名前加下划线以抑制此警告

<a name="E0226"></a>
### E0226 — Redundant annotation / 冗余标注
- Description: redundant type annotation (warning)
- Suggestion: remove the unnecessary type annotation
- 描述：类型标注可以推断，无需显式标注（警告）
- 建议：移除不必要的类型标注

## Eval Errors (E0300 - E0306) / 求值错误 (E0300 - E0306)

<a name="E0300"></a>
### E0300 — Division by zero / 除以零
- Description: division by zero
- 描述：发生了除以零

<a name="E0301"></a>
### E0301 — Assertion failed / 断言失败
- Description: assertion failed
- 描述：断言失败

<a name="E0302"></a>
### E0302 — Pattern match failed / 模式匹配失败
- Description: pattern matching failed
- 描述：模式匹配失败

<a name="E0303"></a>
### E0303 — Runtime type error / 运行时类型错误
- Description: runtime type error
- Suggestion: add a type annotation or guard to narrow the type
- 描述：运行时发生了类型错误
- 建议：添加类型标注或运行时守卫来缩小类型

<a name="E0304"></a>
### E0304 — Not a function (eval) / 非函数（求值）
- Description: expected a function at runtime
- Suggestion: check that the call target is a function
- 描述：运行时期望函数，但调用目标不是函数
- 建议：检查调用目标是否为函数

<a name="E0305"></a>
### E0305 — Wrong arity (eval) / 参数数量错误（求值）
- Description: wrong number of arguments at runtime
- Suggestion: check the function definition for the expected number of arguments
- 描述：运行时实参与形参数量不匹配
- 建议：检查函数定义的参数数量

<a name="E0306"></a>
### E0306 — Unbound variable (eval) / 未绑定变量（求值）
- Description: unbound variable at runtime
- Suggestion: define the variable before using it
- 描述：运行时访问了未绑定的变量
- 建议：在使用变量之前定义它

## Module Errors (E0400 - E0402) / 模块错误 (E0400 - E0402)

<a name="E0400"></a>
### E0400 — Module not found / 模块未找到
- Description: module not found
- Suggestion: check the module name or add it as a dependency
- 描述：找不到指定的模块
- 建议：检查模块名称或将其添加为依赖

<a name="E0401"></a>
### E0401 — Module parse error / 模块解析错误
- Description: parse error in imported module
- Suggestion: fix the parse errors in the imported module first
- 描述：导入的模块中存在解析错误
- 建议：先修复导入模块中的解析错误

<a name="E0402"></a>
### E0402 — Circular import / 循环导入
- Description: circular import detected
- Suggestion: break the import cycle by restructuring the modules
- 描述：检测到循环导入
- 建议：通过重构模块结构打破循环导入
