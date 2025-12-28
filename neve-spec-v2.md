# Neve Language Specification v2.0

**Neve** — A pure functional language for system configuration and package management

**Neve** — 纯函数式系统配置与包管理语言

> Pure Rust implementation · Zero ambiguity · Unified syntax
>
> 纯 Rust 实现 · 零二义性 · 语法统一

---

## 1. Design Principles / 设计原则

- **Zero Ambiguity / 零二义性**: Every construct parses uniquely / 每个构造唯一解析
- **Syntax Unification / 语法统一**: Similar structures use similar syntax / 相似结构用相似语法
- **Indentation Independent / 不依赖缩进**: Explicit delimiters / 显式分隔符
- **Pure Functional / 纯函数式**: No side effects / 无副作用

---

## 2. Syntax Symbol Conventions / 语法符号约定

| Symbol / 符号 | Purpose / 用途 | Example / 示例 |
|---------------|----------------|----------------|
| `( )` | Grouping, tuples, function params / 分组、元组、函数参数 | `(1, 2)`, `f(x)` |
| `[ ]` | Lists / 列表 | `[1, 2, 3]` |
| `#{ }` | Records / 记录 | `#{ x = 1 }` |
| `{ }` | Code blocks, definition bodies / 代码块、定义体 | `{ let x = 1; x }` |
| `< >` | Generic parameters / 泛型参数 | `List<Int>` |
| `->` | Function types, match branches / 函数类型、match 分支 | `Int -> Int` |
| `,` | Item separator / 并列项分隔 | `[1, 2, 3]` |
| `;` | Statement/definition terminator / 语句/定义终止 | `let x = 1;` |
| `:` | Type declaration / 类型声明 | `x: Int` |
| `=` | Value binding / 值绑定 | `x = 1` |

---

## 3. Lexical Elements / 词法

### 3.1 Comments / 注释

```neve
-- Single line or inline comment / 单行或行内注释 --

--
   Multiline comment / 多行注释
   -- Can be nested / 可嵌套 --
--
```

### 3.2 Identifiers / 标识符

```neve
foo
_bar
camelCase
snake_case
Type1
```

### 3.3 Literals / 字面量

```neve
-- Integers / 整数 --
42
-17
0xFF
0o77
0b1010
1_000_000

-- Floats / 浮点 --
3.14
-2.5
1.0e-5

-- Special floats / 特殊浮点 --
Float.nan
Float.inf
Float.neg_inf

-- Booleans / 布尔 --
true
false

-- Characters / 字符 --
'a'
'\n'
'\x41'
'\u{1F600}'

-- Strings (no interpolation) / 字符串（无插值） --
"hello\nworld"

-- Interpolated strings (backticks) / 插值字符串（反引号） --
`hello {name}`
`value: {1 + 2}`
`literal brace: \{not interpolated\}`

-- Multiline strings (triple quotes) / 多行字符串（三双引号） --
"""
Multiline content
Auto-dedents common indentation
"""

-- Multiline interpolated strings (triple backticks) / 多行插值字符串（三反引号） --
\`\`\`
Multiline interpolation
{expr} gets evaluated
\`\`\`

-- Paths (start with ./ or ../ or /) / 路径（以 ./ 或 ../ 或 / 开头） --
./relative/path
../parent
/absolute/path
```

### 3.4 Escape Characters / 转义字符

| Escape / 转义 | Meaning / 含义 |
|---------------|----------------|
| `\\` | Backslash / 反斜杠 |
| `\"` | Double quote / 双引号 |
| `\'` | Single quote / 单引号 |
| `\n` | Newline / 换行 |
| `\r` | Carriage return / 回车 |
| `\t` | Tab / 制表符 |
| `\0` | Null character / 空字符 |
| `\xNN` | Hex byte / 十六进制字节 |
| `\u{NNNN}` | Unicode codepoint / Unicode 码点 |
| `\{` | Literal `{` in interpolated string / 插值字符串中的字面 `{` |
| `\}` | Literal `}` in interpolated string / 插值字符串中的字面 `}` |

---

## 4. Types / 类型

### 4.1 Primitive Types / 原始类型

```neve
Int         -- Arbitrary precision integer / 任意精度整数 --
Float       -- 64-bit float / 64位浮点 --
Bool        -- Boolean / 布尔 --
Char        -- Unicode character / Unicode 字符 --
String      -- UTF-8 string / UTF-8 字符串 --
Path        -- File path / 文件路径 --
Unit        -- Empty type () / 空类型 () --
```

### 4.2 Compound Types / 复合类型

```neve
List<Int>                       -- List / 列表 --
Option<Int>                     -- Optional / 可选 --
Result<Int, String>             -- Result / 结果 --
(Int, String)                   -- Tuple / 元组 --
(Int, Int) -> Int               -- Function / 函数 --
#{ name: String, port: Int }    -- Record type / 记录类型 --
```

### 4.3 Tuple Access / 元组访问

```neve
let t = (1, "hello", true);
t.0     -- 1 --
t.1     -- "hello" --
t.2     -- true --
```

---

## 5. Definitions / 定义

**All top-level definitions end with `;` / 所有顶层定义以 `;` 结尾**

### 5.1 Type Aliases / 类型别名

```neve
type Port = Int;
type Config = #{ name: String, port: Int };
```

### 5.2 Structs / 结构体

```neve
struct Point { x: Float, y: Float };

struct Server {
    host: String = "localhost",
    port: Int = 8080,
};
```

### 5.3 Enums / 枚举

```neve
enum Option<T> { Some(T), None };

enum Result<T, E> { Ok(T), Err(E) };

enum Source {
    Url #{ url: String, sha256: String },
    Git #{ url: String, rev: String },
    Local(Path),
};
```

### 5.4 Traits

```neve
trait Show {
    fn show(self) -> String;
};

trait Eq {
    fn eq(self, other: Self) -> Bool;
};

-- Associated types allow traits to define type placeholders
trait Iterator {
    type Item;
    fn next(self) -> Option<Self.Item>;
};

-- Associated types can have bounds and defaults
trait Container {
    type Item: Show;
    type Error = String;
    fn get(self, index: Int) -> Result<Self.Item, Self.Error>;
};

impl Show for Point {
    fn show(self) -> String = `({self.x}, {self.y})`;
};

impl<T: Show> Show for List<T> {
    fn show(self) -> String = `[{self.map(Show.show).join(", ")}]`;
};

-- Implement traits with associated types
impl<T> Iterator for List<T> {
    type Item = T;
    fn next(self) -> Option<T> = -- implementation
};
```

### 5.5 Functions / 函数

```neve
fn add(x: Int, y: Int) -> Int = x + y;

fn factorial(n: Int) -> Int = {
    if n <= 1 then 1
    else n * factorial(n - 1)
};

fn identity<T>(x: T) -> T = x;

fn map<A, B>(f: A -> B, xs: List<A>) -> List<B> = {
    match xs {
        [] -> [],
        [h, ..t] -> [f(h)] ++ map(f, t),
    }
};
```

---

## 6. Expressions / 表达式

### 6.1 Bindings / 绑定

```neve
let x = 42;
let x: Int = 42;
let (a, b) = (1, 2);
let #{ x, y } = point;
let [head, ..tail] = list;
```

### 6.2 Records / 记录

**Records use `#{ }`, `=` for assignment, `,` for separation**

**记录用 `#{ }` 包裹，`=` 赋值，`,` 分隔**

```neve
#{ x = 0, y = 0 }
#{ name = "server", port = 8080, debug = false }

-- Field shorthand / 字段简写 --
let name = "app";
#{ name, version = "1.0" }

-- Record update (with |) / 记录更新（用 |） --
#{ point | x = 10 }

-- Record merge (right overrides left) / 记录合并（右覆盖左） --
config // #{ port = 9090 }

-- Field access / 字段访问 --
point.x
config.server.port
```

### 6.3 Code Blocks / 代码块

**Code blocks use `{ }`, `;` separates statements, last is return expression**

**代码块用 `{ }` 包裹，`;` 分隔语句，最后是返回表达式**

```neve
{
    let a = 1;
    let b = 2;
    a + b
}

-- Empty block returns Unit / 空代码块返回 Unit --
{ () }
```

### 6.4 Lists / 列表

```neve
[1, 2, 3]
[]
[1, 2] ++ [3, 4]

-- List comprehension (| separates expression and generators) --
-- 列表推导（用 | 分隔表达式和生成器） --
[x * 2 | x <- xs]
[x | x <- xs, x > 0]
[(x, y) | x <- xs, y <- ys]
```

### 6.5 Closures / 闭包

**Closures use `fn(params)` syntax, consistent with named functions**

**闭包用 `fn(参数)` 语法，与命名函数一致**

```neve
-- Basic closure / 基本闭包 --
fn(x) x + 1
fn(x, y) x + y

-- With type annotations / 带类型注解 --
fn(x: Int) x + 1
fn(x: Int, y: Int) x + y

-- Multiline closure / 多行闭包 --
fn(x) {
    let y = x + 1;
    y * 2
}

-- No-parameter closure / 无参数闭包 --
fn() 42
```

### 6.6 Function Calls / 函数调用

```neve
add(1, 2)
list.map(fn(x) x * 2)
list.filter(fn(x) x > 0)

-- Pipeline / 管道 --
data |> parse |> validate |> transform
```

### 6.7 Conditionals / 条件

```neve
if x > 0 then "positive" else "non-positive"

if x > 0 then "positive"
else if x < 0 then "negative"
else "zero"
```

### 6.8 Pattern Matching / 模式匹配

```neve
match x {
    0 -> "zero",
    1 -> "one",
    n -> `other: {n}`,
}

-- Guards / 守卫 --
match n {
    x if x < 0 -> "negative",
    x if x > 0 -> "positive",
    _ -> "zero",
}

-- Record destructuring / 解构记录 --
match point {
    #{ x = 0, y } -> `on y-axis at {y}`,
    #{ x, y = 0 } -> `on x-axis at {x}`,
    #{ x, y } -> `at ({x}, {y})`,
}

-- List / 列表 --
match list {
    [] -> "empty",
    [x] -> `single: {x}`,
    [h, ..t] -> `head: {h}`,
}

-- Binding entire value (with @) / 绑定整体（用 @） --
match opt {
    v @ Some(x) -> `got {x}`,
    None -> "nothing",
}

-- Or patterns (with |) / Or 模式（用 |） --
match x {
    1 | 2 | 3 -> "small",
    _ -> "other",
}
```

### 6.9 Error Handling / 错误处理

```neve
-- ? propagates errors (postfix) / ? 传播错误（后缀） --
fn process(s: String) -> Result<Data, Error> = {
    let data = fetch(s)?;
    let parsed = parse(data)?;
    Ok(transform(parsed))
};

-- ?? default value / ?? 默认值 --
let x = maybe_none ?? default;

-- ?. safe access / ?. 安全访问 --
let name = user?.profile?.name ?? "anon";

-- Assert / 断言 --
assert(port > 0, "port must be positive");
```

---

## 7. Operators / 操作符

### 7.1 Operator Table / 操作符表

| Operator / 操作符 | Meaning / 含义 | Example / 示例 |
|-------------------|----------------|----------------|
| `+ - * / %` | Arithmetic / 算术 | `1 + 2` |
| `^` | Power / 幂 | `2 ^ 10` |
| `== !=` | Equality / 等于/不等于 | `a == b` |
| `< <= > >=` | Comparison / 比较 | `a < b` |
| `&& \|\|` | Logical and/or / 逻辑与/或 | `a && b` |
| `!` | Logical not / 逻辑非 | `!a` |
| `++` | Concatenation / 拼接 | `[1] ++ [2]` |
| `//` | Record merge / 记录合并 | `a // b` |
| `??` | Default value / 默认值 | `x ?? 0` |
| `?.` | Safe access / 安全访问 | `x?.y` |
| `\|>` | Pipeline / 管道 | `x \|> f` |
| `?` | Error propagation (postfix) / 错误传播（后缀） | `f()?` |

### 7.2 Precedence (high to low) / 优先级（高到低）

| Precedence / 优先级 | Operators / 操作符 | Associativity / 结合性 |
|---------------------|--------------------|-----------------------|
| 12 | `.` `?.` `()` `[]` `.N`(tuple) | Left / 左 |
| 11 | `?` (postfix) | Left / 左 |
| 10 | `!` `-`(prefix) | Right / 右 |
| 9 | `^` | Right / 右 |
| 8 | `* / %` | Left / 左 |
| 7 | `+ -` | Left / 左 |
| 6 | `++` | Right / 右 |
| 5 | `< <= > >=` `== !=` | Left / 左 |
| 4 | `&&` | Left / 左 |
| 3 | `\|\|` | Left / 左 |
| 2 | `??` | Right / 右 |
| 1 | `\|>` | Left / 左 |
| 0 | `//` | Right / 右 |

---

## 8. Modules / 模块

```neve
-- Public / 公开 --
pub fn add(x: Int, y: Int) -> Int = x + y;

-- Private (default) / 私有（默认） --
fn internal(x: Int) -> Int = x * 2;

-- Import module / 导入模块 --
import std.list;

-- Import specific items / 导入特定项 --
import std.list (map, filter, fold);

-- Rename module / 重命名模块 --
import std.list as L;

-- Relative imports / 相对导入 --
import self.utils;      -- Current module's submodule / 当前模块的子模块 --
import super.common;    -- Parent module's submodule / 父模块的子模块 --
```

Module paths use `.` separator, same syntax as field access, but only module paths allowed after `import`.

模块路径用 `.` 分隔，与字段访问在语法上相同，但 `import` 后只能是模块路径。

---

## 9. Collection Operations / 集合操作

```neve
let xs = [1, 2, 3, 4, 5];

-- Transform / 变换 --
xs.map(fn(x) x * 2)
xs.filter(fn(x) x > 2)
xs.flat_map(fn(x) [x, x])

-- Aggregate / 聚合 --
xs.fold(0, fn(acc, x) acc + x)
xs.sum()
xs.all(fn(x) x > 0)
xs.any(fn(x) x > 4)

-- Access / 访问 --
xs.len()
xs.first()      -- Option<Int> --
xs.get(2)       -- Option<Int> --

-- Combine / 组合 --
[1, 2] ++ [3, 4]
[1, 2].zip([3, 4])

-- Comprehension / 推导 --
[x * 2 | x <- xs]
[x | x <- xs, x > 2]
```

---

## 10. Package Management / 包管理

```neve
import std.pkg (Package, fetch);

pub let hello = Package #{
    name = "hello",
    version = "2.12.1",
    src = fetch.url #{
        url = "https://ftp.gnu.org/gnu/hello/hello-2.12.1.tar.gz",
        sha256 = "abc123...",
    },
    deps = fn(pkgs) #{
        build = [pkgs.gcc, pkgs.make],
        runtime = [pkgs.glibc],
    },
    build = fn(ctx) {
        ctx.run(`./configure --prefix={ctx.out}`);
        ctx.run(`make -j{ctx.cores}`);
        ctx.run("make install");
    },
};
```

---

## 11. System Configuration / 系统配置

```neve
import std.system (Config, Service);

pub let workstation = Config #{
    hostname = "workstation",
    timezone = "Asia/Shanghai",
    boot = #{
        loader = "systemd-boot",
        efi = "/boot/efi",
    },
    network = #{
        dhcp = true,
        firewall = #{ allow_tcp = [22, 80, 443] },
    },
    users = #{
        alice = #{
            home = "/home/alice",
            shell = pkgs.zsh,
            groups = ["wheel", "docker"],
        },
    },
    services = [
        Service.sshd #{ enable = true },
        Service.docker #{ enable = true },
    ],
    packages = [pkgs.vim, pkgs.git],
};
```

---

## 12. Lazy Evaluation / 惰性求值

```neve
lazy let expensive = compute();

fn if_lazy<T>(cond: Bool, lazy t: T, lazy e: T) -> T = {
    if cond then t else e
};

let result = force(lazy_expr);
```

---

## Appendix A: Keywords / 附录 A：关键字

```
let fn type struct enum trait impl
pub import as self super
if then else match
lazy true false
```

**17 keywords total / 共 17 个关键字**

## Appendix A.1: Built-in Functions / 附录 A.1：内置函数

```
assert(condition, message)    -- Assert, terminates on failure / 断言，失败时终止 --
force(lazy_expr)              -- Force evaluate lazy expression / 强制求值惰性表达式 --
```

## Appendix B: Symbol Purposes / 附录 B：符号用途

| Symbol / 符号 | Unique Purpose / 唯一用途 |
|---------------|--------------------------|
| `#{ }` | Records / 记录 |
| `{ }` | Code blocks, definition bodies / 代码块、定义体 |
| `[ ]` | Lists / 列表 |
| `< >` | Generics / 泛型 |
| `( )` | Grouping, tuples, parameters / 分组、元组、参数 |
| `->` | Function types, match branches / 函数类型、match 分支 |
| `:` | Type declaration / 类型声明 |
| `=` | Value binding / 值绑定 |
| `,` | Item separator / 并列分隔 |
| `;` | Statement terminator / 语句终止 |
| `\|` | Or pattern, comprehension separator, record update / Or 模式、推导分隔、记录更新 |
| `@` | Pattern binding / 模式绑定 |
| `..` | List spread / 列表展开 |

## Appendix C: Three Uses of `|` / 附录 C：`|` 的三种用途

| Context / 上下文 | Purpose / 用途 | Example / 示例 |
|------------------|----------------|----------------|
| Inside match branch / match 分支内 | Or pattern / Or 模式 | `1 \| 2 \| 3 -> "small"` |
| Inside `[ ]` / `[ ]` 内 | Comprehension separator / 推导分隔 | `[x \| x <- xs]` |
| Inside `#{ }` / `#{ }` 内 | Record update / 记录更新 | `#{ r \| x = 1 }` |

Three uses in different brackets, unambiguous parsing.

三种用途在不同括号内，解析无歧义。

## Appendix D: Grammar Summary / 附录 D：语法摘要

```ebnf
program     = (definition ";")*

definition  = let_def | fn_def | type_def | struct_def 
            | enum_def | trait_def | impl_def | import_def

let_def     = "let" pattern (":" type)? "=" expr
fn_def      = "pub"? "fn" IDENT generics? "(" params ")" "->" type "=" expr

record      = "#" "{" (field ",")* field? "}"
field       = IDENT ("=" expr)?

block       = "{" (stmt ";")* expr "}"

list        = "[" (expr ",")* expr? "]"
list_comp   = "[" expr "|" generators "]"

lambda      = "fn" "(" params ")" expr

match_expr  = "match" expr "{" (branch ",")* branch? "}"
branch      = pattern ("->" expr)?
```

## Appendix E: Comparison with Nix / 附录 E：与 Nix 对照

| Nix | Neve |
|-----|------|
| `{ a = 1; }` | `#{ a = 1 }` |
| `[ 1 2 3 ]` | `[1, 2, 3]` |
| `x: x + 1` | `fn(x) x + 1` |
| `a // b` | `a // b` |
| `./path` | `./path` |
| `"${x}"` | `` `{x}` `` |
| `inherit x;` | `#{ x }` |
| `rec { }` | Automatic recursion / 自动递归 |

---

## Design Summary / 设计总结

### Ambiguity-Free Guarantee / 无歧义保证

| Construct / 构造 | Syntax / 语法 | Recognition / 识别方式 |
|------------------|---------------|----------------------|
| Record / 记录 | `#{ x = 1 }` | `#` prefix / `#` 前缀 |
| Code block / 代码块 | `{ stmt; expr }` | No `#` prefix / 无 `#` 前缀 |
| List / 列表 | `[1, 2]` | `[ ]` |
| Closure / 闭包 | `fn(x) e` | Starts with `fn(` / `fn(` 开头 |
| Named function / 命名函数 | `fn name(x)` | `fn` + identifier / `fn` + 标识符 |
| Generic / 泛型 | `T<A>` | Type position / 类型位置 |
| Comment / 注释 | `-- --` | Starts with `--` / `--` 开头 |

### Symbol Responsibilities / 符号职责

- `->` only for types and match branches / 只用于类型和 match 分支
- `=` only for value binding / 只用于值绑定
- `:` only for type declaration / 只用于类型声明
- `|` distinguished by enclosing bracket type / 根据所在括号类型区分用途
