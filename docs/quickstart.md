```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                         5-MINUTE QUICK START                                  ║
║                            5 分钟快速上手                                      ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  Installation / REPL / Files / Types / Patterns │
│  [中文]     #chinese   ──→  安装 / 交互环境 / 写文件 / 类型 / 模式匹配     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

<a name="english"></a>

# English

> *Life's too short for long tutorials. Let's get you hacking in 5 minutes.*

## Step 1: Install (30 sec)

```bash
# Pre-built binary
curl -fsSL https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv neve /usr/local/bin/

# Or Arch Linux
yay -S neve-git

# Or from source
git clone https://github.com/MCB-SMART-BOY/neve.git && cd neve
cargo build --release
```

## Step 2: Play with REPL (1 min)

```bash
$ neve repl
neve> 1 + 2 * 3
7
neve> let double = fn(x) x * 2
neve> double(21)
42
neve> #{ name = "hacker", power = 9001 }
#{power = 9001, name = "hacker"}
neve> { let a = 10; let b = 20; a + b }
30
neve> :quit
```

**REPL Commands:** `:help` `:env` `:clear` `:load file.neve` `:quit`

## Step 3: Write a File (1 min)

Create `hello.neve`:

```neve
fn greet(name) = `Hello, {name}!`;

fn factorial(n) = {
    if n <= 1 then 1
    else n * factorial(n - 1)
};

#{
    greeting = greet("World"),
    magic = factorial(5),
}
```

Run it:

```bash
$ neve run hello.neve
#{magic = 120, greeting = "Hello, World!"}

$ neve check hello.neve   # Type check (no output = OK)
```

## Step 4: Types (1 min)

```neve
-- Inferred
let x = 42;                -- x: Int
let f = fn(n) n * 2;       -- f: Int -> Int

-- Explicit
fn add(a: Int, b: Int) -> Int = a + b;

-- Generics
fn identity<T>(x: T) -> T = x;
```

## Step 5: Pattern Matching (1 min)

```neve
fn describe(opt) = match opt {
    Some(x) -> `Got: {x}`,
    None    -> "Nothing",
};

fn sum(list) = match list {
    []       -> 0,
    [h, ..t] -> h + sum(t),
};
```

## Cheat Sheet

| What | Neve |
|------|------|
| Record | `#{ x = 1 }` |
| Lambda | `fn(x) x + 1` |
| Function | `fn add(a, b) = a + b;` |
| Block | `{ let x = 1; x }` |
| List | `[1, 2, 3]` |
| Pipe | `x \|> f \|> g` |
| Interpolation | `` `Hello {name}` `` |
| Match | `match x { p -> e }` |
| Comment | `-- text --` |

## Next

- [Tutorial](tutorial.md) — deeper dive
- [Spec](spec.md) — language reference
- [API](api.md) — standard library

---

<a name="chinese"></a>

# 中文

> 人生苦短，教程太长。5 分钟让你上手。

## 第一步：安装（30 秒）

```bash
# 下载预编译包
curl -fsSL https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv neve /usr/local/bin/

# Arch 用户
yay -S neve-git

# 从源码编译
git clone https://github.com/MCB-SMART-BOY/neve.git && cd neve
cargo build --release
```

## 第二步：玩玩 REPL（1 分钟）

```bash
$ neve repl
neve> 1 + 2 * 3
7
neve> let double = fn(x) x * 2
neve> double(21)
42
neve> #{ name = "极客", power = 9001 }
#{power = 9001, name = "极客"}
neve> { let a = 10; let b = 20; a + b }
30
neve> :quit
```

**常用命令：** `:help` `:env` `:clear` `:load 文件.neve` `:quit`

## 第三步：写个文件（1 分钟）

创建 `hello.neve`：

```neve
fn greet(name) = `你好，{name}！`;

fn factorial(n) = {
    if n <= 1 then 1
    else n * factorial(n - 1)
};

#{
    greeting = greet("世界"),
    magic = factorial(5),
}
```

运行：

```bash
$ neve run hello.neve
#{magic = 120, greeting = "你好，世界！"}

$ neve check hello.neve   # 类型检查，没输出就是没问题
```

## 第四步：类型系统（1 分钟）

```neve
-- 自动推导
let x = 42;                -- x: Int
let f = fn(n) n * 2;       -- f: Int -> Int

-- 显式标注
fn add(a: Int, b: Int) -> Int = a + b;

-- 泛型
fn identity<T>(x: T) -> T = x;
```

## 第五步：模式匹配（1 分钟）

```neve
fn describe(opt) = match opt {
    Some(x) -> `拿到了：{x}`,
    None    -> "啥也没有",
};

fn sum(list) = match list {
    []       -> 0,
    [h, ..t] -> h + sum(t),
};
```

## 速查表

| 语法 | 写法 |
|------|------|
| 记录 | `#{ x = 1 }` |
| Lambda | `fn(x) x + 1` |
| 函数 | `fn add(a, b) = a + b;` |
| 代码块 | `{ let x = 1; x }` |
| 列表 | `[1, 2, 3]` |
| 管道 | `x \|> f \|> g` |
| 插值 | `` `你好 {name}` `` |
| 匹配 | `match x { p -> e }` |
| 注释 | `-- 文字 --` |

## 接下来

- [完整教程](tutorial.md)
- [语言规范](spec.md)
- [标准库](api.md)

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                         Now go build something cool! 🚀
═══════════════════════════════════════════════════════════════════════════════
```

</div>
