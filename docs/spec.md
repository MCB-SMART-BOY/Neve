```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                       NEVE LANGUAGE SPECIFICATION                             ║
║                             语言规范 v2.0                                      ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  Principles / Syntax / Types / Expressions       │
│  [中文]     #chinese   ──→  设计原则 / 语法 / 类型 / 表达式                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

<a name="english"></a>

# English

> *The formal spec. For when you need the precise truth.*

## 1. Design Principles

- **Zero Ambiguity**: Every construct parses exactly one way
- **Syntactic Consistency**: Similar things look similar
- **Indentation Independent**: Explicit delimiters, no significant whitespace
- **Purely Functional**: No side effects, referential transparency

## 2. Symbol Reference

| Symbol | Purpose | Example |
|--------|---------|---------|
| `( )` | Grouping, tuples, function args | `(1, 2)`, `f(x)` |
| `[ ]` | Lists | `[1, 2, 3]` |
| `#{ }` | Records | `#{ x = 1 }` |
| `{ }` | Blocks | `{ let x = 1; x }` |
| `< >` | Generic parameters | `List<Int>` |
| `->` | Function types, match arms | `Int -> Int` |
| `,` | Item separator | `[1, 2, 3]` |
| `;` | Statement terminator | `let x = 1;` |
| `:` | Type annotation | `x: Int` |
| `=` | Value binding | `x = 1` |

## 3. Lexical Elements

### Comments

```neve
-- single line comment --

--
   multi-line comment
   -- can be nested --
--
```

### Literals

```neve
-- Integers
42  -17  0xFF  0o77  0b1010  1_000_000

-- Floats
3.14  -2.5  1.0e-5

-- Booleans
true  false

-- Characters
'a'  '\n'  '\u{1F600}'

-- Strings
"hello\nworld"

-- Interpolated strings
`hello {name}`

-- Multi-line strings
"""
multi-line
content
"""

-- Paths
./relative  ../parent  /absolute
```

## 4. Types

### Primitive Types

```neve
Int     -- arbitrary precision integer
Float   -- 64-bit floating point
Bool    -- boolean
Char    -- Unicode character
String  -- UTF-8 string
Path    -- filesystem path
Unit    -- empty type ()
```

### Compound Types

```neve
List<Int>                       -- list
Option<Int>                     -- optional value
Result<Int, String>             -- result with error
(Int, String)                   -- tuple
(Int, Int) -> Int               -- function
#{ name: String, port: Int }    -- record type
```

## 5. Definitions

All top-level definitions end with `;`.

```neve
-- Type alias
type Port = Int;

-- Struct
struct Point { x: Float, y: Float };

-- Enum
enum Option<T> { Some(T), None };

-- Trait
trait Show {
    fn show(self) -> String;
};

-- Implementation
impl Show for Point {
    fn show(self) -> String = `({self.x}, {self.y})`;
};

-- Function
fn add(x: Int, y: Int) -> Int = x + y;
```

## 6. Expressions

### Bindings

```neve
let x = 42;
let (a, b) = (1, 2);
let #{ x, y } = point;
```

### Records

```neve
#{ x = 0, y = 0 }
#{ name }              -- shorthand
#{ point | x = 10 }    -- update
config // override     -- merge
```

### Lists

```neve
[1, 2, 3]
[1, 2] ++ [3, 4]
[x * 2 | x <- xs, x > 0]    -- comprehension
```

### Closures

```neve
fn(x) x + 1
fn(x, y) x + y
fn(x: Int) -> Int { x + 1 }
```

### Conditionals

```neve
if x > 0 then "positive" else "non-positive"
```

### Pattern Matching

```neve
match x {
    0 -> "zero",
    n if n > 0 -> "positive",
    _ -> "negative",
}
```

### Error Handling

```neve
let data = fetch(url)?;     -- propagate error
let x = maybe ?? default;   -- default value
user?.profile?.name         -- safe access
```

## 7. Operators

| Operator | Meaning |
|----------|---------|
| `+ - * / %` | Arithmetic |
| `^` | Power |
| `== !=` | Equality |
| `< <= > >=` | Comparison |
| `&& \|\|` | Logical and/or |
| `!` | Logical not |
| `++` | Concatenation |
| `//` | Record merge |
| `??` | Default value |
| `?.` | Safe access |
| `\|>` | Pipe |
| `?` | Error propagation |

### Precedence (high to low)

1. `.` `?.` `()` `[]`
2. `?` (postfix)
3. `!` `-` (prefix)
4. `^`
5. `* / %`
6. `+ -`
7. `++`
8. `< <= > >= == !=`
9. `&&`
10. `||`
11. `??`
12. `|>`
13. `//`

## 8. Modules

```neve
pub fn add(x: Int, y: Int) -> Int = x + y;

import std.list;
import std.list (map, filter);
import std.list as L;
import self.utils;
import super.common;
```

## 9. Lazy Evaluation

```neve
lazy let expensive = compute();
let result = force(lazy_expr);
```

## Appendix A: Keywords

```
let fn type struct enum trait impl
pub import as self super
if then else match
lazy true false
```

**17 keywords total**

## Appendix B: Nix Comparison

| Nix | Neve |
|-----|------|
| `{ a = 1; }` | `#{ a = 1 }` |
| `[ 1 2 3 ]` | `[1, 2, 3]` |
| `x: x + 1` | `fn(x) x + 1` |
| `a // b` | `a // b` |
| `"${x}"` | `` `{x}` `` |
| `inherit x;` | `#{ x }` |
| `rec { }` | Automatic recursion |

---

<a name="chinese"></a>

# 中文

> 正式规范。当你需要精确答案的时候,来这里找。

## 1. 设计原则

- **零二义性**: 每个语法结构只有一种解析方式
- **语法一致**: 相似的东西长得相似
- **不靠缩进**: 用显式分隔符,不玩空格游戏
- **纯函数式**: 没有副作用,引用透明

## 2. 符号速查

| 符号 | 干啥的 | 例子 |
|------|--------|------|
| `( )` | 分组、元组、函数参数 | `(1, 2)`, `f(x)` |
| `[ ]` | 列表 | `[1, 2, 3]` |
| `#{ }` | 记录 | `#{ x = 1 }` |
| `{ }` | 代码块 | `{ let x = 1; x }` |
| `< >` | 泛型参数 | `List<Int>` |
| `->` | 函数类型、匹配分支 | `Int -> Int` |
| `,` | 分隔并列项 | `[1, 2, 3]` |
| `;` | 语句结尾 | `let x = 1;` |
| `:` | 类型声明 | `x: Int` |
| `=` | 绑定值 | `x = 1` |

## 3. 词法元素

### 注释

```neve
-- 单行注释 --

--
   多行注释
   -- 可以嵌套 --
--
```

### 字面量

```neve
-- 整数
42  -17  0xFF  0o77  0b1010  1_000_000

-- 浮点数
3.14  -2.5  1.0e-5

-- 布尔值
true  false

-- 字符
'a'  '\n'  '\u{1F600}'

-- 字符串
"hello\nworld"

-- 插值字符串
`你好 {name}`

-- 多行字符串
"""
多行
内容
"""

-- 路径
./relative  ../parent  /absolute
```

## 4. 类型

### 原始类型

```neve
Int     -- 任意精度整数
Float   -- 64 位浮点
Bool    -- 布尔
Char    -- Unicode 字符
String  -- UTF-8 字符串
Path    -- 文件路径
Unit    -- 空类型 ()
```

### 复合类型

```neve
List<Int>                       -- 列表
Option<Int>                     -- 可选值
Result<Int, String>             -- 带错误的结果
(Int, String)                   -- 元组
(Int, Int) -> Int               -- 函数
#{ name: String, port: Int }    -- 记录类型
```

## 5. 定义

所有顶层定义都以 `;` 结尾。

```neve
-- 类型别名
type Port = Int;

-- 结构体
struct Point { x: Float, y: Float };

-- 枚举
enum Option<T> { Some(T), None };

-- Trait
trait Show {
    fn show(self) -> String;
};

-- 实现
impl Show for Point {
    fn show(self) -> String = `({self.x}, {self.y})`;
};

-- 函数
fn add(x: Int, y: Int) -> Int = x + y;
```

## 6. 表达式

### 绑定

```neve
let x = 42;
let (a, b) = (1, 2);
let #{ x, y } = point;
```

### 记录

```neve
#{ x = 0, y = 0 }
#{ name }              -- 简写
#{ point | x = 10 }    -- 更新
config // override     -- 合并
```

### 列表

```neve
[1, 2, 3]
[1, 2] ++ [3, 4]
[x * 2 | x <- xs, x > 0]    -- 推导式
```

### 闭包

```neve
fn(x) x + 1
fn(x, y) x + y
fn(x: Int) -> Int { x + 1 }
```

### 条件

```neve
if x > 0 then "正数" else "非正数"
```

### 模式匹配

```neve
match x {
    0 -> "零",
    n if n > 0 -> "正数",
    _ -> "负数",
}
```

### 错误处理

```neve
let data = fetch(url)?;     -- 传播错误
let x = maybe ?? default;   -- 默认值
user?.profile?.name         -- 安全访问
```

## 7. 操作符

| 操作符 | 意思 |
|--------|------|
| `+ - * / %` | 算术运算 |
| `^` | 幂运算 |
| `== !=` | 相等判断 |
| `< <= > >=` | 比较大小 |
| `&& \|\|` | 逻辑与/或 |
| `!` | 逻辑非 |
| `++` | 拼接 |
| `//` | 记录合并 |
| `??` | 默认值 |
| `?.` | 安全访问 |
| `\|>` | 管道 |
| `?` | 错误传播 |

### 优先级（从高到低）

1. `.` `?.` `()` `[]`
2. `?` (后缀)
3. `!` `-` (前缀)
4. `^`
5. `* / %`
6. `+ -`
7. `++`
8. `< <= > >= == !=`
9. `&&`
10. `||`
11. `??`
12. `|>`
13. `//`

## 8. 模块

```neve
pub fn add(x: Int, y: Int) -> Int = x + y;

import std.list;
import std.list (map, filter);
import std.list as L;
import self.utils;
import super.common;
```

## 9. 惰性求值

```neve
lazy let expensive = compute();
let result = force(lazy_expr);
```

## 附录 A: 关键字

```
let fn type struct enum trait impl
pub import as self super
if then else match
lazy true false
```

**一共 17 个关键字**

## 附录 B: 跟 Nix 对照

| Nix | Neve |
|-----|------|
| `{ a = 1; }` | `#{ a = 1 }` |
| `[ 1 2 3 ]` | `[1, 2, 3]` |
| `x: x + 1` | `fn(x) x + 1` |
| `a // b` | `a // b` |
| `"${x}"` | `` `{x}` `` |
| `inherit x;` | `#{ x }` |
| `rec { }` | 自动递归 |

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                    The spec is the contract. Read it well.
═══════════════════════════════════════════════════════════════════════════════
```

</div>
