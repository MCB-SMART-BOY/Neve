```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                            COMPLETE TUTORIAL                                  ║
║                               完整教程                                         ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  Basics / Types / Patterns / Traits / Modules   │
│  [中文]     #chinese   ──→  基础 / 类型 / 匹配 / Trait / 模块              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

<a name="english"></a>

# English

## 1. Basics

### Values and Bindings

All bindings are immutable:

```neve
let x = 42;
let name = "Alice";
let valid = true;
```

### Functions

```neve
-- Named function
fn add(x: Int, y: Int) -> Int = x + y;

-- Lambda
let multiply = fn(x, y) x * y;

-- With string interpolation
fn greet(name) = `Hello, {name}!`;
```

### Records

```neve
let user = #{
    name = "Bob",
    age = 30,
};

-- Access
let n = user.name;

-- Update (creates new record)
let older = user // #{ age = 31 };

-- Shorthand
let name = "Alice";
let u = #{ name, age = 25 };  -- same as #{ name = name, age = 25 }
```

### Lists

```neve
let nums = [1, 2, 3, 4, 5];

-- Concatenate
let combined = [1, 2] ++ [3, 4];

-- Comprehension
let doubled = [x * 2 | x <- nums];
let filtered = [x | x <- nums, x > 2];
```

### Blocks

```neve
let result = {
    let a = 10;
    let b = 20;
    a + b   -- last expression is returned
};
```

---

## 2. Type System

### Basic Types

```neve
Int, Float, Bool, Char, String, Unit
```

### Compound Types

```neve
-- Tuple
type Point = (Int, Int);

-- List
type Numbers = List<Int>;

-- Record type
type User = #{ name: String, age: Int };
```

### Generics

```neve
fn first<T>(xs: List<T>) -> Option<T> = match xs {
    [] -> None,
    [h, ..] -> Some(h),
};

fn identity<T>(x: T) -> T = x;
```

### Type Inference

Neve uses Hindley-Milner:

```neve
let double = fn(x) x * 2;     -- inferred: Int -> Int
let id = fn(x) x;             -- inferred: forall a. a -> a
```

---

## 3. Pattern Matching

### Basics

```neve
fn describe(x) = match x {
    0 -> "zero",
    1 -> "one",
    n -> `other: {n}`,
};
```

### Lists

```neve
fn sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
};
```

### Records

```neve
fn getName(user) = match user {
    #{ name, .. } -> name,
};

fn isAdult(user) = match user {
    #{ age } if age >= 18 -> true,
    _ -> false,
};
```

### Option and Result

```neve
fn divide(a, b) = {
    if b == 0 then Err("div by zero")
    else Ok(a / b)
};

match divide(10, 2) {
    Ok(n) -> `Got: {n}`,
    Err(e) -> `Error: {e}`,
}
```

---

## 4. Traits

### Define

```neve
trait Show {
    fn show(self) -> String;
};

trait Eq {
    fn eq(self, other: Self) -> Bool;
};
```

### Implement

```neve
struct Point { x: Int, y: Int };

impl Show for Point {
    fn show(self) = `Point({self.x}, {self.y})`;
};

impl Eq for Point {
    fn eq(self, other) = self.x == other.x && self.y == other.y;
};
```

### Bounds

```neve
fn print_all<T: Show>(items: List<T>) = {
    -- T must implement Show
};
```

---

## 5. Modules

### Define

```neve
-- utils.neve
pub fn add(x, y) = x + y;
fn helper() = 42;  -- private
```

### Import

```neve
import utils;
let r = utils.add(1, 2);

-- Or selective
import utils (add);
let r = add(1, 2);
```

---

## 6. Best Practices

1. **Use type annotations** for public APIs
2. **Prefer immutable data** (it's the only option anyway)
3. **Use tail recursion** for large iterations
4. **Use pipes** for data transformation chains
5. **Match exhaustively** — handle all cases

```neve
-- Good: clear data flow
let result = data
    |> filter(valid)
    |> map(transform)
    |> fold(0, add);
```

---

## Next

- [Spec](spec.md) — full language reference
- [API](api.md) — standard library
- [Philosophy](philosophy.md) — why these design choices

---

<a name="chinese"></a>

# 中文

## 1. 基础

### 值和绑定

所有绑定都是不可变的：

```neve
let x = 42;
let name = "Alice";
let valid = true;
```

### 函数

```neve
-- 命名函数
fn add(x: Int, y: Int) -> Int = x + y;

-- Lambda
let multiply = fn(x, y) x * y;

-- 带字符串插值
fn greet(name) = `你好，{name}！`;
```

### 记录

```neve
let user = #{
    name = "小明",
    age = 30,
};

-- 访问字段
let n = user.name;

-- 更新（创建新记录）
let older = user // #{ age = 31 };

-- 简写
let name = "小红";
let u = #{ name, age = 25 };  -- 等价于 #{ name = name, age = 25 }
```

### 列表

```neve
let nums = [1, 2, 3, 4, 5];

-- 拼接
let combined = [1, 2] ++ [3, 4];

-- 推导
let doubled = [x * 2 | x <- nums];
let filtered = [x | x <- nums, x > 2];
```

### 代码块

```neve
let result = {
    let a = 10;
    let b = 20;
    a + b   -- 最后一个表达式作为返回值
};
```

---

## 2. 类型系统

### 基本类型

```neve
Int, Float, Bool, Char, String, Unit
```

### 复合类型

```neve
-- 元组
type Point = (Int, Int);

-- 列表
type Numbers = List<Int>;

-- 记录类型
type User = #{ name: String, age: Int };
```

### 泛型

```neve
fn first<T>(xs: List<T>) -> Option<T> = match xs {
    [] -> None,
    [h, ..] -> Some(h),
};

fn identity<T>(x: T) -> T = x;
```

### 类型推导

Neve 用的是 Hindley-Milner 算法：

```neve
let double = fn(x) x * 2;     -- 推导出：Int -> Int
let id = fn(x) x;             -- 推导出：forall a. a -> a
```

---

## 3. 模式匹配

### 基础

```neve
fn describe(x) = match x {
    0 -> "零",
    1 -> "一",
    n -> `其他：{n}`,
};
```

### 列表匹配

```neve
fn sum(xs) = match xs {
    [] -> 0,
    [h, ..t] -> h + sum(t),
};
```

### 记录匹配

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
fn divide(a, b) = {
    if b == 0 then Err("除以零了")
    else Ok(a / b)
};

match divide(10, 2) {
    Ok(n) -> `结果：{n}`,
    Err(e) -> `出错：{e}`,
}
```

---

## 4. Trait

### 定义

```neve
trait Show {
    fn show(self) -> String;
};

trait Eq {
    fn eq(self, other: Self) -> Bool;
};
```

### 实现

```neve
struct Point { x: Int, y: Int };

impl Show for Point {
    fn show(self) = `Point({self.x}, {self.y})`;
};

impl Eq for Point {
    fn eq(self, other) = self.x == other.x && self.y == other.y;
};
```

### 约束

```neve
fn print_all<T: Show>(items: List<T>) = {
    -- T 必须实现 Show
};
```

---

## 5. 模块

### 定义

```neve
-- utils.neve
pub fn add(x, y) = x + y;
fn helper() = 42;  -- 私有的
```

### 导入

```neve
import utils;
let r = utils.add(1, 2);

-- 或者只导入需要的
import utils (add);
let r = add(1, 2);
```

---

## 6. 写代码的建议

1. **公开 API 加上类型注解**，方便别人用
2. **数据都是不可变的**，习惯就好
3. **大循环用尾递归**，不然栈会炸
4. **数据变换用管道**，看着清楚
5. **匹配要穷尽**，别漏情况

```neve
-- 这样写清楚
let result = data
    |> filter(valid)
    |> map(transform)
    |> fold(0, add);
```

---

## 接下来

- [语言规范](spec.md) — 完整语法参考
- [标准库](api.md) — API 文档
- [设计哲学](philosophy.md) — 为什么这样设计

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                           Happy hacking! 写代码愉快！
═══════════════════════════════════════════════════════════════════════════════
```

</div>
