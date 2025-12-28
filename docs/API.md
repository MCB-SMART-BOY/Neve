# Neve 标准库 API 文档 / Standard Library API Reference

本文档列出了 Neve 标准库提供的核心函数和类型。

## 目录 / Table of Contents

- [核心函数](#核心函数)
- [列表操作](#列表操作)
- [字符串操作](#字符串操作)
- [Option 类型](#option-类型)
- [Result 类型](#result-类型)
- [数学函数](#数学函数)
- [I/O 操作](#io-操作)

---

## 核心函数

### `id<A>(x: A) -> A`

恒等函数,返回输入值本身。

```neve
id(42)  -- => 42
id("hello")  -- => "hello"
```

### `const<A, B>(x: A, y: B) -> A`

常量函数,返回第一个参数,忽略第二个参数。

```neve
const(1, 2)  -- => 1
const("hello", 42)  -- => "hello"
```

### `compose<A, B, C>(f: B -> C, g: A -> B) -> A -> C`

函数组合。

```neve
let addOne = fn(x) x + 1;
let double = fn(x) x * 2;
let addOneThenDouble = compose(double, addOne);

addOneThenDouble(3)  -- => 8  (先 +1 得 4, 再 *2 得 8)
```

### `flip<A, B, C>(f: A -> B -> C) -> B -> A -> C`

翻转函数参数顺序。

```neve
let subtract = fn(x, y) x - y;
let subtractFrom = flip(subtract);

subtract(10, 3)  -- => 7
subtractFrom(10, 3)  -- => -7  (3 - 10)
```

---

## 列表操作

### `map<A, B>(f: A -> B, xs: List<A>) -> List<B>`

对列表中每个元素应用函数。

```neve
map(fn(x) x * 2, [1, 2, 3])  -- => [2, 4, 6]
```

### `filter<A>(pred: A -> Bool, xs: List<A>) -> List<A>`

过滤列表,保留满足谓词的元素。

```neve
filter(fn(x) x > 2, [1, 2, 3, 4])  -- => [3, 4]
```

### `fold<A, B>(init: B, f: B -> A -> B, xs: List<A>) -> B`

从左到右折叠列表。

```neve
fold(0, fn(acc, x) acc + x, [1, 2, 3, 4])  -- => 10
fold(1, fn(acc, x) acc * x, [1, 2, 3, 4])  -- => 24
```

### `foldRight<A, B>(init: B, f: A -> B -> B, xs: List<A>) -> B`

从右到左折叠列表。

```neve
foldRight([], fn(x, acc) [x] ++ acc, [1, 2, 3])  -- => [1, 2, 3]
```

### `length<A>(xs: List<A>) -> Int`

返回列表长度。

```neve
length([1, 2, 3, 4])  -- => 4
length([])  -- => 0
```

### `head<A>(xs: List<A>) -> Option<A>`

返回列表第一个元素。

```neve
head([1, 2, 3])  -- => Some(1)
head([])  -- => None
```

### `tail<A>(xs: List<A>) -> Option<List<A>>`

返回列表除第一个元素外的其余部分。

```neve
tail([1, 2, 3])  -- => Some([2, 3])
tail([])  -- => None
```

### `reverse<A>(xs: List<A>) -> List<A>`

反转列表。

```neve
reverse([1, 2, 3])  -- => [3, 2, 1]
```

### `take<A>(n: Int, xs: List<A>) -> List<A>`

获取列表前 n 个元素。

```neve
take(2, [1, 2, 3, 4])  -- => [1, 2]
```

### `drop<A>(n: Int, xs: List<A>) -> List<A>`

丢弃列表前 n 个元素。

```neve
drop(2, [1, 2, 3, 4])  -- => [3, 4]
```

### `zip<A, B>(xs: List<A>, ys: List<B>) -> List<(A, B)>`

将两个列表配对。

```neve
zip([1, 2, 3], ["a", "b", "c"])  -- => [(1, "a"), (2, "b"), (3, "c")]
```

### `concat<A>(xss: List<List<A>>) -> List<A>`

连接列表的列表。

```neve
concat([[1, 2], [3, 4], [5]])  -- => [1, 2, 3, 4, 5]
```

---

## 字符串操作

### `length(s: String) -> Int`

返回字符串长度。

```neve
length("hello")  -- => 5
```

### `concat(xs: List<String>) -> String`

连接字符串列表。

```neve
concat(["hello", " ", "world"])  -- => "hello world"
```

### `split(sep: String, s: String) -> List<String>`

按分隔符分割字符串。

```neve
split(",", "a,b,c")  -- => ["a", "b", "c"]
```

### `trim(s: String) -> String`

去除字符串首尾空白。

```neve
trim("  hello  ")  -- => "hello"
```

### `toUpper(s: String) -> String`

转换为大写。

```neve
toUpper("hello")  -- => "HELLO"
```

### `toLower(s: String) -> String`

转换为小写。

```neve
toLower("HELLO")  -- => "hello"
```

---

## Option 类型

### 定义

```neve
enum Option<T> {
    Some(T),
    None,
};
```

### `map<A, B>(f: A -> B, opt: Option<A>) -> Option<B>`

对 Option 中的值应用函数。

```neve
map(fn(x) x * 2, Some(21))  -- => Some(42)
map(fn(x) x * 2, None)  -- => None
```

### `flatMap<A, B>(f: A -> Option<B>, opt: Option<A>) -> Option<B>`

链式 Option 操作。

```neve
let divide = fn(x, y) if y == 0 then None else Some(x / y);

flatMap(fn(x) divide(x, 2), Some(10))  -- => Some(5)
flatMap(fn(x) divide(x, 0), Some(10))  -- => None
```

### `withDefault<A>(default: A, opt: Option<A>) -> A`

提供默认值。

```neve
withDefault(0, Some(42))  -- => 42
withDefault(0, None)  -- => 0
```

### `isSome<A>(opt: Option<A>) -> Bool`

检查是否为 Some。

```neve
isSome(Some(42))  -- => true
isSome(None)  -- => false
```

---

## Result 类型

### 定义

```neve
enum Result<T, E> {
    Ok(T),
    Err(E),
};
```

### `map<T, E, U>(f: T -> U, res: Result<T, E>) -> Result<U, E>`

对 Ok 值应用函数。

```neve
map(fn(x) x * 2, Ok(21))  -- => Ok(42)
map(fn(x) x * 2, Err("error"))  -- => Err("error")
```

### `mapErr<T, E, F>(f: E -> F, res: Result<T, E>) -> Result<T, F>`

对 Err 值应用函数。

```neve
mapErr(fn(e) `Error: {e}`, Err("bad"))  -- => Err("Error: bad")
```

### `withDefault<T, E>(default: T, res: Result<T, E>) -> T`

提供默认值。

```neve
withDefault(0, Ok(42))  -- => 42
withDefault(0, Err("error"))  -- => 0
```

---

## 数学函数

### `abs(x: Int) -> Int`

绝对值。

```neve
abs(-42)  -- => 42
abs(42)  -- => 42
```

### `min(x: Int, y: Int) -> Int`

最小值。

```neve
min(3, 7)  -- => 3
```

### `max(x: Int, y: Int) -> Int`

最大值。

```neve
max(3, 7)  -- => 7
```

### `pow(base: Int, exp: Int) -> Int`

幂运算。

```neve
pow(2, 10)  -- => 1024
```

### `sqrt(x: Float) -> Float`

平方根。

```neve
sqrt(16.0)  -- => 4.0
```

---

## I/O 操作

### `print(s: String) -> Unit`

打印字符串到标准输出(不换行)。

```neve
print("Hello")
```

### `println(s: String) -> Unit`

打印字符串到标准输出(带换行)。

```neve
println("Hello, World!")
```

### `readLine() -> String`

从标准输入读取一行。

```neve
let input = readLine();
```

### `readFile(path: String) -> Result<String, String>`

读取文件内容。

```neve
match readFile("config.txt") {
    Ok(content) -> process(content),
    Err(msg) -> println(`Error: {msg}`),
}
```

### `writeFile(path: String, content: String) -> Result<Unit, String>`

写入文件。

```neve
writeFile("output.txt", "Hello, World!")
```

---

## 路径操作

### `fetchurl`

下载文件。

```neve
fetchurl {
    url = "https://example.com/file.tar.gz",
    hash = "sha256-...",
}
```

### `fetchGit`

克隆 Git 仓库。

```neve
fetchGit {
    url = "https://github.com/user/repo.git",
    rev = "main",
    hash = "sha256-...",
}
```

---

## Derivation 函数

### `mkDerivation`

创建包derivation。

```neve
mkDerivation {
    name = "mypackage",
    version = "1.0.0",
    src = fetchurl { ... },
    buildInputs = [ gcc, make ],
    buildPhase = "make",
    installPhase = "make install PREFIX=$out",
}
```

---

## 注意事项

1. 所有函数都是纯函数,没有副作用(除了 I/O 函数)
2. 列表操作会创建新列表,不会修改原列表
3. 字符串是不可变的
4. 使用管道 `|>` 可以让函数调用更自然

---

## 示例

### 综合示例

```neve
-- 处理用户列表
let users = [
    #{ name = "Alice", age = 30 },
    #{ name = "Bob", age = 25 },
    #{ name = "Charlie", age = 35 },
];

let adults = users
    |> filter(fn(u) u.age >= 18)
    |> map(fn(u) u.name)
    |> concat(", ");

println(`Adults: {adults}`);
-- => "Adults: Alice, Bob, Charlie"
```

---

*完整的标准库文档将在未来版本中扩展*
