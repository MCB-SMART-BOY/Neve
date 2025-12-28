# 5 分钟入门 Neve / 5-Minute Quickstart

---

## 安装 / Installation

```bash
# 从源码构建
git clone https://github.com/mcbgaruda/neve.git
cd neve
cargo build --release

# 添加到 PATH
export PATH="$PWD/target/release:$PATH"

# 验证安装
neve --version
```

---

## 第一步：Hello World / Step 1: Hello World

```bash
neve eval '"Hello, Neve!"'
```

输出：
```
"Hello, Neve!"
```

---

## 第二步：基本表达式 / Step 2: Basic Expressions

```bash
# 算术
neve eval "1 + 2 * 3"
# => 7

# 字符串插值
neve eval '`2 + 2 = {2 + 2}`'
# => "2 + 2 = 4"

# 列表
neve eval "[1, 2, 3] ++ [4, 5]"
# => [1, 2, 3, 4, 5]

# 记录
neve eval '#{ name = "Neve", version = 1 }'
# => #{ name = "Neve", version = 1 }
```

---

## 第三步：REPL 交互 / Step 3: REPL

```bash
neve repl
```

```neve
> let x = 42
42

> x * 2
84

> let double = fn(n) n * 2
<function>

> double(21)
42

> [1, 2, 3] |> double
-- 错误: double 期望 Int，但 [1,2,3] 是 List<Int>

> let doubleAll = fn(xs) [x * 2 | x <- xs]
<function>

> doubleAll([1, 2, 3])
[2, 4, 6]
```

输入 `Ctrl+D` 退出 REPL。

---

## 第四步：编写文件 / Step 4: Write a File

创建 `hello.neve`:

```neve
-- hello.neve

-- 定义一个函数
fn greet(name) = `Hello, {name}!`;

-- 定义一个记录
let config = #{
    name = "World",
    count = 3,
};

-- 使用列表推导
let greetings = [greet(config.name) | _ <- [1, 2, 3]];

-- 最后一个表达式是返回值
greetings
```

运行：

```bash
neve run hello.neve
```

输出：
```
["Hello, World!", "Hello, World!", "Hello, World!"]
```

---

## 第五步：类型检查 / Step 5: Type Checking

创建 `typed.neve`:

```neve
-- 带类型注解的函数
fn add(x: Int, y: Int) -> Int = x + y;

fn factorial(n: Int) -> Int = {
    if n <= 1 then 1
    else n * factorial(n - 1)
};

-- Neve 会推导类型
let result = factorial(5);

result
```

类型检查：

```bash
neve check typed.neve
# 无输出表示类型检查通过
```

运行：

```bash
neve run typed.neve
# => 120
```

---

## 第六步：模式匹配 / Step 6: Pattern Matching

```neve
-- match.neve

-- 定义一个处理 Option 的函数
fn describe(opt) = match opt {
    Some(x) -> `Got: {x}`,
    None -> "Nothing",
};

-- 使用模式匹配处理列表
fn sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
};

-- 记录解构
fn getName(user) = match user {
    #{ name, age } if age >= 18 -> `Adult: {name}`,
    #{ name, .. } -> `Minor: {name}`,
};

-- 测试
let tests = [
    describe(Some(42)),
    describe(None),
    sum([1, 2, 3, 4, 5]),
    getName(#{ name = "Alice", age = 25 }),
    getName(#{ name = "Bob", age = 15 }),
];

tests
```

```bash
neve run match.neve
# => ["Got: 42", "Nothing", 15, "Adult: Alice", "Minor: Bob"]
```

---

## 第七步：管道和组合 / Step 7: Pipes and Composition

```neve
-- pipes.neve

-- 定义一些转换函数
fn double(x) = x * 2;
fn addOne(x) = x + 1;
fn square(x) = x * x;

-- 使用管道组合
let result = 5
    |> double      -- 10
    |> addOne      -- 11
    |> square;     -- 121

-- 对列表使用管道
fn process(xs) = xs
    |> filter(fn(x) x > 0)
    |> map(fn(x) x * 2)
    |> fold(0, fn(acc, x) acc + x);

-- 测试
#{
    single = result,
    list = process([-1, 2, -3, 4, 5]),
}
```

```bash
neve run pipes.neve
# => #{ single = 121, list = 22 }
```

---

## 语法速查 / Syntax Cheat Sheet

| 概念 | 语法 | 示例 |
|------|------|------|
| 记录 | `#{ }` | `#{ x = 1, y = 2 }` |
| 列表 | `[ ]` | `[1, 2, 3]` |
| Lambda | `fn(x) expr` | `fn(x) x + 1` |
| 函数 | `fn name(x) = expr;` | `fn add(a, b) = a + b;` |
| If | `if c then a else b` | `if x > 0 then "pos" else "neg"` |
| Match | `match x { ... }` | `match x { 0 -> "zero", n -> "other" }` |
| 管道 | `\|>` | `x \|> f \|> g` |
| 插值 | `` `{expr}` `` | `` `sum = {1 + 2}` `` |
| 注释 | `-- --` | `-- 这是注释 --` |
| 合并 | `//` | `a // b` |
| 拼接 | `++` | `[1] ++ [2]` |

---

## 下一步 / Next Steps

- 阅读 [语言规范](../neve-spec-v2.md)
- 阅读 [设计哲学](../PHILOSOPHY.md)
- 查看 [项目路线图](../ROADMAP.md)
- 尝试编写自己的程序！

---

## 常见问题 / FAQ

### Q: Neve 和 Nix 有什么关系？

Neve 继承了 Nix 的核心理念（纯函数式、可复现、声明式），但用现代设计从零实现，不兼容 nixpkgs。

### Q: 为什么用 `#{ }` 而不是 `{ }`？

消除歧义。在 Nix 中 `{ x = 1; }` 可能是记录也可能是函数，在 Neve 中 `#{ }` 永远是记录。

### Q: 类型注解是必须的吗？

不是。Neve 使用 Hindley-Milner 类型推导，大多数情况下类型会自动推导。

### Q: 如何报告 bug？

在 GitHub Issues 中提交：https://github.com/mcbgaruda/neve/issues

---

*Happy Hacking with Neve!*
