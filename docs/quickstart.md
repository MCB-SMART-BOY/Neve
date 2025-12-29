# 5 分钟入门 Neve

## 安装

```bash
git clone https://github.com/mcbgaruda/neve.git
cd neve
cargo build --release
export PATH="$PWD/target/release:$PATH"
neve --version
```

## 基本表达式

```bash
neve eval "1 + 2 * 3"           # => 7
neve eval '`2 + 2 = {2 + 2}`'   # => "2 + 2 = 4"
neve eval "[1, 2] ++ [3, 4]"    # => [1, 2, 3, 4]
neve eval '#{ name = "Neve" }'  # => #{ name = "Neve" }
```

## REPL 交互

```bash
neve repl
```

```neve
> let x = 42
> x * 2
84

> let double = fn(n) n * 2
> double(21)
42

> [x * 2 | x <- [1, 2, 3]]
[2, 4, 6]
```

按 `Ctrl+D` 退出。

## 编写文件

创建 `hello.neve`:

```neve
fn greet(name) = `Hello, {name}!`;

let config = #{ name = "World", count = 3 };

[greet(config.name) | _ <- [1, 2, 3]]
```

运行：
```bash
neve run hello.neve
# => ["Hello, World!", "Hello, World!", "Hello, World!"]
```

## 类型检查

```neve
fn factorial(n: Int) -> Int = {
    if n <= 1 then 1
    else n * factorial(n - 1)
};

factorial(5)  -- => 120
```

```bash
neve check file.neve  # 无输出表示通过
```

## 模式匹配

```neve
fn describe(opt) = match opt {
    Some(x) -> `Got: {x}`,
    None -> "Nothing",
};

fn sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
};
```

## 管道

```neve
let result = 5
    |> fn(x) x * 2    -- 10
    |> fn(x) x + 1    -- 11
    |> fn(x) x * x;   -- 121
```

## 语法速查

| 概念 | 语法 | 示例 |
|------|------|------|
| 记录 | `#{ }` | `#{ x = 1 }` |
| 列表 | `[ ]` | `[1, 2, 3]` |
| Lambda | `fn(x) expr` | `fn(x) x + 1` |
| 函数 | `fn name(x) = expr;` | `fn add(a, b) = a + b;` |
| 管道 | `\|>` | `x \|> f \|> g` |
| 插值 | `` `{expr}` `` | `` `sum = {1 + 2}` `` |
| 注释 | `-- --` | `-- 注释 --` |

## 下一步

- [完整教程](tutorial.md)
- [语言规范](spec.md)
- [API 参考](api.md)
