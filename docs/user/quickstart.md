<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

<h1>5-Minute Quick Start</h1>

<p><em>5 分钟快速上手</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

---

> *Life's too short for long tutorials. Let's get you hacking in 5 minutes.*  
> 人生苦短，教程太长。5 分钟让你上手。

## Step 1: Install (30 sec) / 第一步：安装（30 秒）

```bash
# Pre-built binary
curl -fsSL https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv neve /usr/local/bin/

# Or Arch Linux
yay -S neve-git

# Or from source
git clone https://github.com/MCB-SMART-BOY/Neve.git && cd Neve
cargo build --release
```

## Step 2: Play with REPL (1 min) / 第二步：玩玩 REPL（1 分钟）

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

**REPL Commands / 常用命令:** `:help` `:env` `:clear` `:load file.neve` `:quit`

## Step 3: Write a File (1 min) / 第三步：写个文件（1 分钟）

Create `hello.neve`:
创建 `hello.neve`：

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
运行：

```bash
$ neve run hello.neve
#{magic = 120, greeting = "Hello, World!"}

$ neve check hello.neve   # Type check (no output = OK)
```

## Step 4: Types (1 min) / 第四步：类型系统（1 分钟）

```neve
-- Inferred
let x = 42;                -- x: Int
let f = fn(n) n * 2;       -- f: Int -> Int

-- Explicit
fn add(a: Int, b: Int) -> Int = a + b;

-- Generics
fn identity<T>(x: T) -> T = x;
```

## Step 5: Pattern Matching (1 min) / 第五步：模式匹配（1 分钟）

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

## Cheat Sheet / 语法速查

| What / 项目 | Neve |
|------------|------|
| Record | `#{ x = 1 }` |
| Lambda | `fn(x) x + 1` |
| Function | `fn add(a, b) = a + b;` |
| Block | `{ let x = 1; x }` |
| List | `[1, 2, 3]` |
| Pipe | `x \|> f \|> g` |
| Interpolation | `` `Hello {name}` `` |
| Match | `match x { p -> e }` |
| Comment | `-- text --` |

## Next / 接下来

- [Tutorial](tutorial.md) — deeper dive / 深入学习
- [Spec](../reference/spec.md) — language reference / 语言规范
- [API](../reference/api.md) — standard library / 标准库参考

---

> Now go build something cool! / 去构建点酷的东西吧！
