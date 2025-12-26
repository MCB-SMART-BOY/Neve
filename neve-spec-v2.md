# Neve Language Specification v1.1

**Neve** — 纯函数式系统配置与包管理语言

> 纯 Rust 实现 · 零二义性 · 语法统一

---

## 1. 设计原则

- **零二义性**：每个构造唯一解析
- **语法统一**：相似结构用相似语法
- **不依赖缩进**：显式分隔符
- **纯函数式**：无副作用

---

## 2. 语法符号约定

| 符号 | 用途 | 示例 |
|------|------|------|
| `( )` | 分组、元组、函数参数 | `(1, 2)`, `f(x)` |
| `[ ]` | 列表 | `[1, 2, 3]` |
| `#{ }` | 记录 | `#{ x = 1 }` |
| `{ }` | 代码块、定义体 | `{ let x = 1; x }` |
| `< >` | 泛型参数 | `List<Int>` |
| `->` | 函数类型、match 分支 | `Int -> Int` |
| `,` | 并列项分隔 | `[1, 2, 3]` |
| `;` | 语句/定义终止 | `let x = 1;` |
| `:` | 类型声明 | `x: Int` |
| `=` | 值绑定 | `x = 1` |

---

## 3. 词法

### 3.1 注释

```neve
-- 单行或行内注释 --

--
   多行注释
   -- 可嵌套 --
--
```

### 3.2 标识符

```neve
foo
_bar
camelCase
snake_case
Type1
```

### 3.3 字面量

```neve
-- 整数 --
42
-17
0xFF
0o77
0b1010
1_000_000

-- 浮点 --
3.14
-2.5
1.0e-5

-- 特殊浮点 --
Float.nan
Float.inf
Float.neg_inf

-- 布尔 --
true
false

-- 字符 --
'a'
'\n'
'\x41'
'\u{1F600}'

-- 字符串（无插值） --
"hello\nworld"

-- 插值字符串（反引号） --
`hello {name}`
`value: {1 + 2}`
`literal brace: \{not interpolated\}`

-- 多行字符串（三双引号） --
"""
多行内容
自动去除公共缩进
"""

-- 多行插值字符串（三反引号） --
\`\`\`
多行插值
{expr} 会被求值
\`\`\`

-- 路径（以 ./ 或 ../ 或 / 开头） --
./relative/path
../parent
/absolute/path
```

### 3.4 转义字符

| 转义 | 含义 |
|------|------|
| `\\` | 反斜杠 |
| `\"` | 双引号 |
| `\'` | 单引号 |
| `\n` | 换行 |
| `\r` | 回车 |
| `\t` | 制表符 |
| `\0` | 空字符 |
| `\xNN` | 十六进制字节 |
| `\u{NNNN}` | Unicode 码点 |
| `\{` | 插值字符串中的字面 `{` |
| `\}` | 插值字符串中的字面 `}` |

---

## 4. 类型

### 4.1 原始类型

```neve
Int         -- 任意精度整数 --
Float       -- 64位浮点 --
Bool        -- 布尔 --
Char        -- Unicode 字符 --
String      -- UTF-8 字符串 --
Path        -- 文件路径 --
Unit        -- 空类型 () --
```

### 4.2 复合类型

```neve
List<Int>                       -- 列表 --
Option<Int>                     -- 可选 --
Result<Int, String>             -- 结果 --
(Int, String)                   -- 元组 --
(Int, Int) -> Int               -- 函数 --
#{ name: String, port: Int }    -- 记录类型 --
```

### 4.3 元组访问

```neve
let t = (1, "hello", true);
t.0     -- 1 --
t.1     -- "hello" --
t.2     -- true --
```

---

## 5. 定义

**所有顶层定义以 `;` 结尾**

### 5.1 类型别名

```neve
type Port = Int;
type Config = #{ name: String, port: Int };
```

### 5.2 结构体

```neve
struct Point { x: Float, y: Float };

struct Server {
    host: String = "localhost",
    port: Int = 8080,
};
```

### 5.3 枚举

```neve
enum Option<T> { Some(T), None };

enum Result<T, E> { Ok(T), Err(E) };

enum Source {
    Url #{ url: String, sha256: String },
    Git #{ url: String, rev: String },
    Local(Path),
};
```

### 5.4 Trait

```neve
trait Show {
    fn show(self) -> String;
};

trait Eq {
    fn eq(self, other: Self) -> Bool;
};

impl Show for Point {
    fn show(self) -> String = `({self.x}, {self.y})`;
};

impl<T: Show> Show for List<T> {
    fn show(self) -> String = `[{self.map(Show.show).join(", ")}]`;
};
```

### 5.5 函数

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

## 6. 表达式

### 6.1 绑定

```neve
let x = 42;
let x: Int = 42;
let (a, b) = (1, 2);
let #{ x, y } = point;
let [head, ..tail] = list;
```

### 6.2 记录

**记录用 `#{ }` 包裹，`=` 赋值，`,` 分隔**

```neve
#{ x = 0, y = 0 }
#{ name = "server", port = 8080, debug = false }

-- 字段简写 --
let name = "app";
#{ name, version = "1.0" }

-- 记录更新（用 | ） --
#{ point | x = 10 }

-- 记录合并（右覆盖左） --
config // #{ port = 9090 }

-- 字段访问 --
point.x
config.server.port
```

### 6.3 代码块

**代码块用 `{ }` 包裹，`;` 分隔语句，最后是返回表达式**

```neve
{
    let a = 1;
    let b = 2;
    a + b
}

-- 空代码块返回 Unit --
{ () }
```

### 6.4 列表

```neve
[1, 2, 3]
[]
[1, 2] ++ [3, 4]

-- 列表推导（用 | 分隔表达式和生成器） --
[x * 2 | x <- xs]
[x | x <- xs, x > 0]
[(x, y) | x <- xs, y <- ys]
```

### 6.5 闭包

**闭包用 `fn(参数)` 语法，与命名函数一致**

```neve
-- 基本闭包 --
fn(x) x + 1
fn(x, y) x + y

-- 带类型注解 --
fn(x: Int) x + 1
fn(x: Int, y: Int) x + y

-- 多行闭包 --
fn(x) {
    let y = x + 1;
    y * 2
}

-- 无参数闭包 --
fn() 42
```

### 6.6 函数调用

```neve
add(1, 2)
list.map(fn(x) x * 2)
list.filter(fn(x) x > 0)

-- 管道 --
data |> parse |> validate |> transform
```

### 6.7 条件

```neve
if x > 0 then "positive" else "non-positive"

if x > 0 then "positive"
else if x < 0 then "negative"
else "zero"
```

### 6.8 模式匹配

```neve
match x {
    0 -> "zero",
    1 -> "one",
    n -> `other: {n}`,
}

-- 守卫 --
match n {
    x if x < 0 -> "negative",
    x if x > 0 -> "positive",
    _ -> "zero",
}

-- 解构记录 --
match point {
    #{ x = 0, y } -> `on y-axis at {y}`,
    #{ x, y = 0 } -> `on x-axis at {x}`,
    #{ x, y } -> `at ({x}, {y})`,
}

-- 列表 --
match list {
    [] -> "empty",
    [x] -> `single: {x}`,
    [h, ..t] -> `head: {h}`,
}

-- 绑定整体（用 @） --
match opt {
    v @ Some(x) -> `got {x}`,
    None -> "nothing",
}

-- Or 模式（用 |） --
match x {
    1 | 2 | 3 -> "small",
    _ -> "other",
}
```

### 6.9 错误处理

```neve
-- ? 传播错误（后缀） --
fn process(s: String) -> Result<Data, Error> = {
    let data = fetch(s)?;
    let parsed = parse(data)?;
    Ok(transform(parsed))
};

-- ?? 默认值 --
let x = maybe_none ?? default;

-- ?. 安全访问 --
let name = user?.profile?.name ?? "anon";

-- 断言 --
assert(port > 0, "port must be positive");
```

---

## 7. 操作符

### 7.1 操作符表

| 操作符 | 含义 | 示例 |
|--------|------|------|
| `+ - * / %` | 算术 | `1 + 2` |
| `^` | 幂 | `2 ^ 10` |
| `== !=` | 等于/不等于 | `a == b` |
| `< <= > >=` | 比较 | `a < b` |
| `&& \|\|` | 逻辑与/或 | `a && b` |
| `!` | 逻辑非 | `!a` |
| `++` | 拼接 | `[1] ++ [2]` |
| `//` | 记录合并 | `a // b` |
| `??` | 默认值 | `x ?? 0` |
| `?.` | 安全访问 | `x?.y` |
| `\|>` | 管道 | `x \|> f` |
| `?` | 错误传播（后缀） | `f()?` |

### 7.2 优先级（高到低）

| 优先级 | 操作符 | 结合性 |
|--------|--------|--------|
| 12 | `.` `?.` `()` `[]` `.N`(元组) | 左 |
| 11 | `?` (后缀) | 左 |
| 10 | `!` `-`(前缀) | 右 |
| 9 | `^` | 右 |
| 8 | `* / %` | 左 |
| 7 | `+ -` | 左 |
| 6 | `++` | 右 |
| 5 | `< <= > >=` `== !=` | 左 |
| 4 | `&&` | 左 |
| 3 | `\|\|` | 左 |
| 2 | `??` | 右 |
| 1 | `\|>` | 左 |
| 0 | `//` | 右 |

---

## 8. 模块

```neve
-- 公开 --
pub fn add(x: Int, y: Int) -> Int = x + y;

-- 私有（默认） --
fn internal(x: Int) -> Int = x * 2;

-- 导入模块 --
import std.list;

-- 导入特定项 --
import std.list (map, filter, fold);

-- 重命名模块 --
import std.list as L;

-- 相对导入 --
import self.utils;      -- 当前模块的子模块 --
import super.common;    -- 父模块的子模块 --
```

模块路径用 `.` 分隔，与字段访问在语法上相同，但 `import` 后只能是模块路径。

---

## 9. 集合操作

```neve
let xs = [1, 2, 3, 4, 5];

-- 变换 --
xs.map(fn(x) x * 2)
xs.filter(fn(x) x > 2)
xs.flat_map(fn(x) [x, x])

-- 聚合 --
xs.fold(0, fn(acc, x) acc + x)
xs.sum()
xs.all(fn(x) x > 0)
xs.any(fn(x) x > 4)

-- 访问 --
xs.len()
xs.first()      -- Option<Int> --
xs.get(2)       -- Option<Int> --

-- 组合 --
[1, 2] ++ [3, 4]
[1, 2].zip([3, 4])

-- 推导 --
[x * 2 | x <- xs]
[x | x <- xs, x > 2]
```

---

## 10. 包管理

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

## 11. 系统配置

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

## 12. 惰性求值

```neve
lazy let expensive = compute();

fn if_lazy<T>(cond: Bool, lazy t: T, lazy e: T) -> T = {
    if cond then t else e
};

let result = force(lazy_expr);
```

---

## 附录 A：关键字

```
let fn type struct enum trait impl
pub import as self super
if then else match
lazy true false
```

**共 17 个关键字**

## 附录 A.1：内置函数

```
assert(condition, message)    -- 断言，失败时终止 --
force(lazy_expr)              -- 强制求值惰性表达式 --
```

## 附录 B：符号用途

| 符号 | 唯一用途 |
|------|----------|
| `#{ }` | 记录 |
| `{ }` | 代码块、定义体 |
| `[ ]` | 列表 |
| `< >` | 泛型 |
| `( )` | 分组、元组、参数 |
| `->` | 函数类型、match 分支 |
| `:` | 类型声明 |
| `=` | 值绑定 |
| `,` | 并列分隔 |
| `;` | 语句终止 |
| `\|` | Or 模式、推导分隔、记录更新 |
| `@` | 模式绑定 |
| `..` | 列表展开 |

## 附录 C：`|` 的三种用途

| 上下文 | 用途 | 示例 |
|--------|------|------|
| match 分支内 | Or 模式 | `1 \| 2 \| 3 -> "small"` |
| `[ ]` 内 | 推导分隔 | `[x \| x <- xs]` |
| `#{ }` 内 | 记录更新 | `#{ r \| x = 1 }` |

三种用途在不同括号内，解析无歧义。

## 附录 D：语法摘要

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

## 附录 E：与 Nix 对照

| Nix | Neve |
|-----|------|
| `{ a = 1; }` | `#{ a = 1 }` |
| `[ 1 2 3 ]` | `[1, 2, 3]` |
| `x: x + 1` | `fn(x) x + 1` |
| `a // b` | `a // b` |
| `./path` | `./path` |
| `"${x}"` | `` `{x}` `` |
| `inherit x;` | `#{ x }` |
| `rec { }` | 自动递归 |

---

## 设计总结

### 无歧义保证

| 构造 | 语法 | 识别方式 |
|------|------|----------|
| 记录 | `#{ x = 1 }` | `#` 前缀 |
| 代码块 | `{ stmt; expr }` | 无 `#` 前缀 |
| 列表 | `[1, 2]` | `[ ]` |
| 闭包 | `fn(x) e` | `fn(` 开头 |
| 命名函数 | `fn name(x)` | `fn` + 标识符 |
| 泛型 | `T<A>` | 类型位置 |
| 注释 | `-- --` | `--` 开头 |

### 符号职责

- `->` 只用于类型和 match 分支
- `=` 只用于值绑定
- `:` 只用于类型声明
- `|` 根据所在括号类型区分用途
