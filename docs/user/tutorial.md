<div align="center">

<img src="../../assets/logo.svg" width="120" alt="n3v3 logo">

<h1>Complete Tutorial</h1>

<p><em>完整教程</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

---

> Learn the language from basics to modules. / 从基础到模块的完整教程。

## 1. Basics / 基础


### Values and Bindings

All bindings are immutable:

```n3v3
x = 42
name = "Alice"
valid = true
```

### Functions

```n3v3
-- Named function
add(x: Int, y: Int) -> Int = x + y

-- Lambda
multiply = |x, y| x * y

-- With string interpolation
greet(name) = `Hello, {name}!`
```

### Records

```n3v3
user = {
    name = "Bob",
    age = 30,
}

-- Access
n = user.name

-- Update (creates new record)
older = user & { age = 31 }

-- Shorthand
name = "Alice"
u = { name, age = 25 }  -- same as { name = name, age = 25 }
```

### Lists

```n3v3
nums = [1, 2, 3, 4, 5]

-- Concatenate
combined = [1, 2] ++ [3, 4]

-- Comprehension
doubled = [x * 2 | x <- nums]
filtered = [x | x <- nums, x > 2]
```

### Blocks

```n3v3
result = {
    a = 10
    b = 20
    a + b   -- last expression is returned
}
```

---


### 值和绑定

所有绑定都是不可变的：

```n3v3
x = 42
name = "Alice"
valid = true
```

### 函数

```n3v3
-- 命名函数
add(x: Int, y: Int) -> Int = x + y

-- Lambda
multiply = |x, y| x * y

-- 带字符串插值
greet(name) = `你好，{name}！`
```

### 记录

```n3v3
user = {
    name = "小明",
    age = 30,
}

-- 访问字段
n = user.name

-- 更新（创建新记录）
older = user & { age = 31 }

-- 简写
name = "小红"
u = { name, age = 25 }  -- 等价于 { name = name, age = 25 }
```

### 列表

```n3v3
nums = [1, 2, 3, 4, 5]

-- 拼接
combined = [1, 2] ++ [3, 4]

-- 推导
doubled = [x * 2 | x <- nums]
filtered = [x | x <- nums, x > 2]
```

### 代码块

```n3v3
result = {
    a = 10
    b = 20
    a + b   -- 最后一个表达式作为返回值
}
```

---


## 2. Type System / 类型系统


### Basic Types

```n3v3
-- Primitive types include: Int, Float, Bool, Char, String, Unit
```

### Compound Types

```n3v3
-- Tuple
type Point = (Int, Int)

-- List
type Numbers = List<Int>

-- Record type
type User = { name: String, age: Int }
```

### Generics

```n3v3
first<T>(xs: List<T>) -> Option<T> = match xs {
    [] -> None,
    [h, ..] -> Some(h),
}

identity<T>(x: T) -> T = x
```

### Type Inference

n3v3 uses Hindley-Milner:

```n3v3
double = |x| x * 2     -- inferred: Int -> Int
id = |x| x             -- inferred: forall a. a -> a
```

---


### 基本类型

```n3v3
-- 基本类型包括：Int、Float、Bool、Char、String、Unit
```

### 复合类型

```n3v3
-- 元组
type Point = (Int, Int)

-- 列表
type Numbers = List<Int>

-- 记录类型
type User = { name: String, age: Int }
```

### 泛型

```n3v3
first<T>(xs: List<T>) -> Option<T> = match xs {
    [] -> None,
    [h, ..] -> Some(h),
}

identity<T>(x: T) -> T = x
```

### 类型推导

n3v3 用的是 Hindley-Milner 算法：

```n3v3
double = |x| x * 2     -- 推导出：Int -> Int
id = |x| x             -- 推导出：forall a. a -> a
```

---


## 3. Pattern Matching / 模式匹配


### Basics

```n3v3
describe(x) = match x {
    0 -> "zero",
    1 -> "one",
    n -> `other: {n}`,
}
```

### Lists

```n3v3
sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
}
```

### Records

```n3v3
getName(user) = match user {
    { name, .. } -> name,
    _ -> "unknown",
}

isAdult(user) = match user {
    { age, .. } if age >= 18 -> true,
    _ -> false,
}
```

### Option and Result

```n3v3
divide(a, b) = {
    if b == 0 -> Err("div by zero")
    else Ok(a / b)
}

match divide(10, 2) {
    Ok(n) -> `Got: {n}`,
    Err(e) -> `Error: {e}`,
}
```

---


### 基础

```n3v3
describe(x) = match x {
    0 -> "零",
    1 -> "一",
    n -> `其他：{n}`,
}
```

### 列表匹配

```n3v3
sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
}
```

### 记录匹配

```n3v3
getName(user) = match user {
    { name, .. } -> name,
    _ -> "未知",
}

isAdult(user) = match user {
    { age, .. } if age >= 18 -> true,
    _ -> false,
}
```

### Option 和 Result

```n3v3
divide(a, b) = {
    if b == 0 -> Err("除以零了")
    else Ok(a / b)
}

match divide(10, 2) {
    Ok(n) -> `结果：{n}`,
    Err(e) -> `出错：{e}`,
}
```

---


## 4. Traits / Trait


### Define

```n3v3
trait Show {
    fn show(self) -> String;
}

trait Eq {
    fn eq(self, other: Self) -> Bool;
}

type Point = Int;

impl Show for Point {
    fn show(self) -> String = `Point({self})`;
}

impl Eq for Point {
    fn eq(self, other: Self) -> Bool = self == other;
}
```

### Bounds

```n3v3
print_all<T: Show>(items: List<T>) = {
    -- T must implement Show
}
```

---


### 定义

```n3v3
trait Show {
    fn show(self) -> String;
}

trait Eq {
    fn eq(self, other: Self) -> Bool;
}

type Point = Int;

impl Show for Point {
    fn show(self) -> String = `Point({self})`;
}

impl Eq for Point {
    fn eq(self, other: Self) -> Bool = self == other;
}
```

### 约束

```n3v3
print_all<T: Show>(items: List<T>) = {
    -- T 必须实现 Show
}
```

---


## 5. Modules / 模块


### Define

```n3v3
-- utils.n3v3
add(x, y) = x + y
```
Since v4.0, n3v3 has no private-binding syntax: all bindings are public.

### Import

```n3v3
use utils
r = utils.add(1, 2)

-- Or selective
use utils (add)
r = add(1, 2)
```

---


### 定义

```n3v3
-- utils.n3v3
add(x, y) = x + y
```
从 v4.0 起，n3v3 不再有私有绑定语法：所有绑定都是 public。

### 导入

```n3v3
use utils
r = utils.add(1, 2)

-- 或者只导入需要的
use utils (add)
r = add(1, 2)
```

---


## 6. Best Practices / 写代码的建议


1. **Use type annotations** for public APIs
2. **Prefer immutable data** (it's the only option anyway)
3. **Use tail recursion** for large iterations
4. **Use pipes** for data transformation chains
5. **Match exhaustively** — handle all cases

```n3v3
use std.list = list

let data = [1, 2, 3, 4]
let valid = |x| x % 2 == 0
let transform = |x| x * 2
let add = |acc, x| acc + x

filter_valid(xs) = list.filter(valid, xs)
map_transform(xs) = list.map(transform, xs)
fold_sum(xs) = list.fold(0, add, xs)

result = data
    |> filter_valid
    |> map_transform
    |> fold_sum

```
---


1. **公开 API 加上类型注解**，方便别人用
2. **数据都是不可变的**，习惯就好
3. **大循环用尾递归**，不然栈会炸
4. **数据变换用管道**，看着清楚
5. **匹配要穷尽**，别漏情况

```n3v3
use std.list = list

let data = [1, 2, 3, 4]
let valid = |x| x % 2 == 0
let transform = |x| x * 2
let add = |acc, x| acc + x

filter_valid(xs) = list.filter(valid, xs)
map_transform(xs) = list.map(transform, xs)
fold_sum(xs) = list.fold(0, add, xs)

result = data
    |> filter_valid
    |> map_transform
    |> fold_sum

```
---


## Next / 接下来


- [Spec](../reference/spec.md) — full language reference
- [API](../reference/api.md) — standard library
- [Philosophy](../project/philosophy.md) — why these design choices

---


- [语言规范](../reference/spec.md) — 完整语法参考
- [标准库](../reference/api.md) — API 文档
- [设计哲学](../project/philosophy.md) — 为什么这样设计

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                           Happy hacking! 写代码愉快！
═══════════════════════════════════════════════════════════════════════════════
```

</div>
