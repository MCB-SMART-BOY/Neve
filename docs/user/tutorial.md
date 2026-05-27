<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

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

```neve
x = 42
name = "Alice"
valid = true
```

### Functions

```neve
-- Named function
add(x: Int, y: Int) -> Int = x + y

-- Lambda
multiply = |x, y| x * y

-- With string interpolation
greet(name) = `Hello, {name}!`
```

### Records

```neve
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

```neve
nums = [1, 2, 3, 4, 5]

-- Concatenate
combined = [1, 2] ++ [3, 4]

-- Comprehension
doubled = [x * 2 | x <- nums]
filtered = [x | x <- nums, x > 2]
```

### Blocks

```neve
result = {
    a = 10
    b = 20
    a + b   -- last expression is returned
}
```

---


### 值和绑定

所有绑定都是不可变的：

```neve
x = 42
name = "Alice"
valid = true
```

### 函数

```neve
-- 命名函数
add(x: Int, y: Int) -> Int = x + y

-- Lambda
multiply = |x, y| x * y

-- 带字符串插值
greet(name) = `你好，{name}！`
```

### 记录

```neve
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

```neve
nums = [1, 2, 3, 4, 5]

-- 拼接
combined = [1, 2] ++ [3, 4]

-- 推导
doubled = [x * 2 | x <- nums]
filtered = [x | x <- nums, x > 2]
```

### 代码块

```neve
result = {
    a = 10
    b = 20
    a + b   -- 最后一个表达式作为返回值
}
```

---


## 2. Type System / 类型系统


### Basic Types

```neve
Int, Float, Bool, Char, String, Unit
```

### Compound Types

```neve
-- Tuple
type Point = (Int, Int)

-- List
type Numbers = List<Int>

-- Record type
type User = { name: String, age: Int }
```

### Generics

```neve
first<T>(xs: List<T>) -> Option<T> = match xs {
    [] -> None,
    [h, ..] -> Some(h),
}

identity<T>(x: T) -> T = x
```

### Type Inference

Neve uses Hindley-Milner:

```neve
double = |x| x * 2     -- inferred: Int -> Int
id = |x| x             -- inferred: forall a. a -> a
```

---


### 基本类型

```neve
Int, Float, Bool, Char, String, Unit
```

### 复合类型

```neve
-- 元组
type Point = (Int, Int)

-- 列表
type Numbers = List<Int>

-- 记录类型
type User = { name: String, age: Int }
```

### 泛型

```neve
first<T>(xs: List<T>) -> Option<T> = match xs {
    [] -> None,
    [h, ..] -> Some(h),
}

identity<T>(x: T) -> T = x
```

### 类型推导

Neve 用的是 Hindley-Milner 算法：

```neve
double = |x| x * 2     -- 推导出：Int -> Int
id = |x| x             -- 推导出：forall a. a -> a
```

---


## 3. Pattern Matching / 模式匹配


### Basics

```neve
describe(x) = match x {
    0 -> "zero",
    1 -> "one",
    n -> `other: {n}`,
}
```

### Lists

```neve
sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
}
```

### Records

```neve
getName(user) = match user {
    { name, .. } -> name,
}

isAdult(user) = match user {
    { age } if age >= 18 -> true,
    _ -> false,
}
```

### Option and Result

```neve
divide(a, b) = {
    if b == 0 then Err("div by zero")
    else Ok(a / b)
}

match divide(10, 2) {
    Ok(n) -> `Got: {n}`,
    Err(e) -> `Error: {e}`,
}
```

---


### 基础

```neve
describe(x) = match x {
    0 -> "零",
    1 -> "一",
    n -> `其他：{n}`,
}
```

### 列表匹配

```neve
sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
}
```

### 记录匹配

```neve
getName(user) = match user {
    { name, .. } -> name,
}

isAdult(user) = match user {
    { age } if age >= 18 -> true,
    _ -> false,
}
```

### Option 和 Result

```neve
divide(a, b) = {
    if b == 0 then Err("除以零了")
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

```neve
trait Show {
    show(self) -> String
}

trait Eq {
    eq(self, other: Self) -> Bool
}
```

### Implement

```neve
type Point = { x: Int, y: Int }

impl Show for Point {
    show(self) = `Point({self.x}, {self.y})`
}

impl Eq for Point {
    eq(self, other) = self.x == other.x && self.y == other.y
}
```

### Bounds

```neve
print_all<T: Show>(items: List<T>) = {
    -- T must implement Show
}
```

---


### 定义

```neve
trait Show {
    show(self) -> String
}

trait Eq {
    eq(self, other: Self) -> Bool
}
```

### 实现

```neve
type Point = { x: Int, y: Int }

impl Show for Point {
    show(self) = `Point({self.x}, {self.y})`
}

impl Eq for Point {
    eq(self, other) = self.x == other.x && self.y == other.y
}
```

### 约束

```neve
print_all<T: Show>(items: List<T>) = {
    -- T 必须实现 Show
}
```

---


## 5. Modules / 模块


### Define

```neve
-- utils.neve
pub add(x, y) = x + y
helper() = 42  -- private
```

### Import

```neve
use utils
r = utils.add(1, 2)

-- Or selective
use utils (add)
r = add(1, 2)
```

---


### 定义

```neve
-- utils.neve
pub add(x, y) = x + y
helper() = 42  -- 私有的
```

### 导入

```neve
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

```neve
use std.list (filter, map, fold)

-- Good: clear data flow
result = data
    |> filter(valid)
    |> map(transform)
    |> fold(0, add)
```

---


1. **公开 API 加上类型注解**，方便别人用
2. **数据都是不可变的**，习惯就好
3. **大循环用尾递归**，不然栈会炸
4. **数据变换用管道**，看着清楚
5. **匹配要穷尽**，别漏情况

```neve
use std.list (filter, map, fold)

-- 这样写清楚
result = data
    |> filter(valid)
    |> map(transform)
    |> fold(0, add)
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
