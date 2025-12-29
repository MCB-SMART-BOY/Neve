# Neve 语言规范 v2.0

## 1. 设计原则

- **零二义性**: 每个构造唯一解析
- **语法统一**: 相似结构用相似语法
- **不依赖缩进**: 显式分隔符
- **纯函数式**: 无副作用

## 2. 符号约定

| 符号 | 用途 | 示例 |
|------|------|------|
| `( )` | 分组、元组、函数参数 | `(1, 2)`, `f(x)` |
| `[ ]` | 列表 | `[1, 2, 3]` |
| `#{ }` | 记录 | `#{ x = 1 }` |
| `{ }` | 代码块 | `{ let x = 1; x }` |
| `< >` | 泛型参数 | `List<Int>` |
| `->` | 函数类型、match 分支 | `Int -> Int` |
| `,` | 并列项分隔 | `[1, 2, 3]` |
| `;` | 语句终止 | `let x = 1;` |
| `:` | 类型声明 | `x: Int` |
| `=` | 值绑定 | `x = 1` |

## 3. 词法

### 注释

```neve
-- 单行注释 --

--
   多行注释
   -- 可嵌套 --
--
```

### 字面量

```neve
-- 整数
42  -17  0xFF  0o77  0b1010  1_000_000

-- 浮点
3.14  -2.5  1.0e-5

-- 布尔
true  false

-- 字符
'a'  '\n'  '\u{1F600}'

-- 字符串
"hello\nworld"

-- 插值字符串
`hello {name}`

-- 多行字符串
"""
多行内容
"""

-- 路径
./relative  ../parent  /absolute
```

## 4. 类型

### 原始类型

```neve
Int     -- 任意精度整数
Float   -- 64位浮点
Bool    -- 布尔
Char    -- Unicode 字符
String  -- UTF-8 字符串
Path    -- 文件路径
Unit    -- 空类型 ()
```

### 复合类型

```neve
List<Int>                       -- 列表
Option<Int>                     -- 可选
Result<Int, String>             -- 结果
(Int, String)                   -- 元组
(Int, Int) -> Int               -- 函数
#{ name: String, port: Int }    -- 记录类型
```

## 5. 定义

所有顶层定义以 `;` 结尾。

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
[x * 2 | x <- xs, x > 0]    -- 推导
```

### 闭包

```neve
fn(x) x + 1
fn(x, y) x + y
fn(x: Int) -> Int { x + 1 }
```

### 条件

```neve
if x > 0 then "positive" else "non-positive"
```

### 模式匹配

```neve
match x {
    0 -> "zero",
    n if n > 0 -> "positive",
    _ -> "negative",
}
```

### 错误处理

```neve
let data = fetch(url)?;     -- 传播错误
let x = maybe ?? default;   -- 默认值
user?.profile?.name         -- 安全访问
```

## 7. 操作符

| 操作符 | 含义 |
|--------|------|
| `+ - * / %` | 算术 |
| `^` | 幂 |
| `== !=` | 等于/不等于 |
| `< <= > >=` | 比较 |
| `&& \|\|` | 逻辑与/或 |
| `!` | 逻辑非 |
| `++` | 拼接 |
| `//` | 记录合并 |
| `??` | 默认值 |
| `?.` | 安全访问 |
| `\|>` | 管道 |
| `?` | 错误传播 |

### 优先级（高到低）

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

**共 17 个关键字**

## 附录 B: 与 Nix 对照

| Nix | Neve |
|-----|------|
| `{ a = 1; }` | `#{ a = 1 }` |
| `[ 1 2 3 ]` | `[1, 2, 3]` |
| `x: x + 1` | `fn(x) x + 1` |
| `a // b` | `a // b` |
| `"${x}"` | `` `{x}` `` |
| `inherit x;` | `#{ x }` |
| `rec { }` | 自动递归 |
