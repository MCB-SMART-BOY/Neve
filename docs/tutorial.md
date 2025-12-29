# Neve 完整教程 / Complete Tutorial

本教程将带你深入了解 Neve 语言的各个方面。

## 目录 / Table of Contents

1. [基础语法](#基础语法)
2. [类型系统](#类型系统)
3. [模式匹配](#模式匹配)
4. [Traits 和多态](#traits-和多态)
5. [模块系统](#模块系统)
6. [包管理](#包管理)
7. [最佳实践](#最佳实践)

---

## 基础语法

### 值和变量

Neve 中的所有绑定都是不可变的:

```neve
let x = 42;
let name = "Alice";
let isValid = true;
```

### 函数定义

函数是一等公民:

```neve
-- 简单函数
fn add(x: Int, y: Int) -> Int = x + y;

-- Lambda 表达式
let multiply = fn(x, y) x * y;

-- 多参数和类型推导
fn greet(name) = `Hello, {name}!`;
```

### 记录 (Records)

记录是结构化数据的基础:

```neve
let user = #{
    name = "Bob",
    age = 30,
    email = "bob@example.com",
};

-- 访问字段
let name = user.name;

-- 记录更新 (创建新记录)
let updated = user // #{ age = 31 };
```

### 列表

列表是同质集合:

```neve
let numbers = [1, 2, 3, 4, 5];
let names = ["Alice", "Bob", "Charlie"];

-- 列表拼接
let combined = [1, 2] ++ [3, 4];

-- 列表推导
let doubled = [x * 2 | x <- numbers];
let filtered = [x | x <- numbers, x > 2];
```

---

## 类型系统

### 基本类型

```neve
Int      -- 整数
Float    -- 浮点数
Bool     -- 布尔值
Char     -- 字符
String   -- 字符串
Unit     -- 单元类型 ()
```

### 复合类型

```neve
-- 元组
type Point = (Int, Int);
let p: Point = (10, 20);

-- 列表
type Numbers = List<Int>;
let nums: Numbers = [1, 2, 3];

-- 记录类型
type User = #{
    name: String,
    age: Int,
};
```

### 泛型

```neve
-- 泛型函数
fn first<T>(xs: List<T>) -> Option<T> = match xs {
    [] -> None,
    [h, ..] -> Some(h),
};

-- 泛型类型别名
type Pair<A, B> = (A, B);

-- 泛型结构体
struct Container<T> {
    value: T,
};
```

### 类型推导

Neve 使用 Hindley-Milner 类型推导:

```neve
-- 类型会自动推导
let double = fn(x) x * 2;  -- Int -> Int

-- 多态函数
let identity = fn(x) x;  -- forall a. a -> a

-- 组合函数
let compose = fn(f, g) fn(x) f(g(x));
-- forall a b c. (b -> c) -> (a -> b) -> a -> c
```

---

## 模式匹配

### 基础匹配

```neve
fn describe(x) = match x {
    0 -> "zero",
    1 -> "one",
    n -> `other: {n}`,
};
```

### 列表模式

```neve
fn sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
};

fn length(xs) = match xs {
    [] -> 0,
    [_, ..t] -> 1 + length(t),
};
```

### 记录模式

```neve
fn getName(user) = match user {
    #{ name, .. } -> name,
};

fn isAdult(user) = match user {
    #{ age } if age >= 18 -> true,
    _ -> false,
};
```

### Option 和 Result

```neve
fn parseInt(s: String) -> Option<Int> = -- ...

fn divide(a: Int, b: Int) -> Result<Int, String> = {
    if b == 0 then
        Err("division by zero")
    else
        Ok(a / b)
};

-- 使用
let result = match divide(10, 2) {
    Ok(n) -> `Success: {n}`,
    Err(msg) -> `Error: {msg}`,
};
```

---

## Traits 和多态

### 定义 Trait

```neve
trait Show {
    fn show(self) -> String;
};

trait Eq {
    fn eq(self, other: Self) -> Bool;
};
```

### 实现 Trait

```neve
struct Point {
    x: Int,
    y: Int,
};

impl Show for Point {
    fn show(self) -> String = `Point({self.x}, {self.y})`;
};

impl Eq for Point {
    fn eq(self, other: Point) -> Bool =
        self.x == other.x && self.y == other.y;
};
```

### 关联类型

```neve
trait Iterator {
    type Item;
    fn next(self) -> Option<Self.Item>;
};

impl<T> Iterator for List<T> {
    type Item = T;

    fn next(self) -> Option<T> = match self {
        [] -> None,
        [h, ..] -> Some(h),
    };
};
```

### Trait 边界

```neve
fn print_all<T: Show>(items: List<T>) -> Unit = {
    for item in items {
        println(item.show());
    }
};
```

---

## 模块系统

### 定义模块

```neve
-- utils.neve
pub fn add(x: Int, y: Int) -> Int = x + y;

pub fn multiply(x: Int, y: Int) -> Int = x * y;

-- 私有函数
fn helper() = 42;
```

### 导入模块

```neve
-- main.neve
import utils;

let result = utils.add(1, 2);
```

### 选择性导入

```neve
-- 导入特定项
import utils (add, multiply);

let sum = add(1, 2);
let product = multiply(3, 4);
```

### 路径前缀

```neve
-- 相对导入
import self.utils;      -- 当前模块
import super.common;    -- 父模块
import crate.helpers;   -- crate 根模块
```

### 可见性控制

```neve
pub fn publicFunc() = 1;           -- 公开
pub(crate) fn crateFunc() = 2;     -- crate 内可见
pub(super) fn superFunc() = 3;     -- 父模块可见
fn privateFunc() = 4;               -- 私有
```

---

## 包管理

### 包结构

```
myproject/
├── neve.toml         # 包配置
├── lib.neve          # 库入口
├── main.neve         # 二进制入口
└── src/
    ├── utils.neve
    └── helpers.neve
```

### neve.toml 配置

```toml
[package]
name = "myproject"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]

[dependencies]
# 依赖将在这里定义
```

### Derivations (包定义)

```neve
-- package.neve
{
    name = "hello",
    version = "1.0.0",

    src = fetchurl {
        url = "https://example.com/hello-1.0.tar.gz",
        hash = "sha256-...",
    },

    buildInputs = [ gcc, make ],

    buildPhase = ''
        make
    '',

    installPhase = ''
        mkdir -p $out/bin
        cp hello $out/bin/
    '',
}
```

---

## 最佳实践

### 1. 使用类型注解增强可读性

```neve
-- 好
fn process(items: List<String>) -> Result<Int, String> = -- ...

-- 也可以,但不太清晰
fn process(items) = -- ...
```

### 2. 优先使用不可变数据

```neve
-- 好: 创建新记录
let updated = user // #{ age = user.age + 1 };

-- 避免: 在 Neve 中不存在可变状态
```

### 3. 使用尾递归避免栈溢出

```neve
-- 好: 尾递归
fn factorial_tail(n: Int, acc: Int) -> Int = {
    if n <= 1 then acc
    else factorial_tail(n - 1, n * acc)
};

-- 可能栈溢出
fn factorial(n: Int) -> Int = {
    if n <= 1 then 1
    else n * factorial(n - 1)
};
```

### 4. 使用管道提高可读性

```neve
-- 好: 清晰的数据流
let result = data
    |> filter(isValid)
    |> map(transform)
    |> fold(0, add);

-- 不太清晰
let result = fold(0, add, map(transform, filter(isValid, data)));
```

### 5. 合理使用模式匹配

```neve
-- 好: 穷尽所有情况
fn handle(opt) = match opt {
    Some(x) -> process(x),
    None -> defaultValue,
};

-- 避免: 遗漏情况
fn handle(opt) = match opt {
    Some(x) -> process(x),
    -- 编译器会警告
};
```

---

## 下一步

- 查看 [语言规范](../neve-spec-v2.md) 了解完整语法
- 阅读 [设计哲学](../PHILOSOPHY.md) 理解设计决策
- 参考 [API 文档](API.md) 了解标准库

---

*祝你在 Neve 中编程愉快!*
