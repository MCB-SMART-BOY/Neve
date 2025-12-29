# Neve 标准库 API

## 核心函数

```neve
id<A>(x: A) -> A                              -- 恒等函数
const<A, B>(x: A, y: B) -> A                  -- 常量函数
compose<A, B, C>(f: B -> C, g: A -> B) -> A -> C  -- 函数组合
flip<A, B, C>(f: A -> B -> C) -> B -> A -> C  -- 翻转参数
```

## 列表操作

```neve
map<A, B>(f: A -> B, xs: List<A>) -> List<B>
filter<A>(pred: A -> Bool, xs: List<A>) -> List<A>
fold<A, B>(init: B, f: B -> A -> B, xs: List<A>) -> B
foldRight<A, B>(init: B, f: A -> B -> B, xs: List<A>) -> B

length<A>(xs: List<A>) -> Int
head<A>(xs: List<A>) -> Option<A>
tail<A>(xs: List<A>) -> Option<List<A>>
reverse<A>(xs: List<A>) -> List<A>
take<A>(n: Int, xs: List<A>) -> List<A>
drop<A>(n: Int, xs: List<A>) -> List<A>
zip<A, B>(xs: List<A>, ys: List<B>) -> List<(A, B)>
concat<A>(xss: List<List<A>>) -> List<A>
```

## 字符串操作

```neve
length(s: String) -> Int
concat(xs: List<String>) -> String
split(sep: String, s: String) -> List<String>
trim(s: String) -> String
toUpper(s: String) -> String
toLower(s: String) -> String
```

## Option 类型

```neve
enum Option<T> { Some(T), None }

map<A, B>(f: A -> B, opt: Option<A>) -> Option<B>
flatMap<A, B>(f: A -> Option<B>, opt: Option<A>) -> Option<B>
withDefault<A>(default: A, opt: Option<A>) -> A
isSome<A>(opt: Option<A>) -> Bool
```

## Result 类型

```neve
enum Result<T, E> { Ok(T), Err(E) }

map<T, E, U>(f: T -> U, res: Result<T, E>) -> Result<U, E>
mapErr<T, E, F>(f: E -> F, res: Result<T, E>) -> Result<T, F>
withDefault<T, E>(default: T, res: Result<T, E>) -> T
```

## 数学函数

```neve
abs(x: Int) -> Int
min(x: Int, y: Int) -> Int
max(x: Int, y: Int) -> Int
pow(base: Int, exp: Int) -> Int
sqrt(x: Float) -> Float
```

## I/O 操作

```neve
print(s: String) -> Unit
println(s: String) -> Unit
readLine() -> String
readFile(path: String) -> Result<String, String>
writeFile(path: String, content: String) -> Result<Unit, String>
```

## 源码获取

```neve
fetchurl #{ url: String, hash: String } -> Path
fetchGit #{ url: String, rev: String, hash: String } -> Path
```

## Derivation

```neve
mkDerivation #{
    name: String,
    version: String,
    src: Path,
    buildInputs: List<Derivation>,
    buildPhase: String,
    installPhase: String,
} -> Derivation
```

## 示例

```neve
let users = [
    #{ name = "Alice", age = 30 },
    #{ name = "Bob", age = 25 },
];

let names = users
    |> filter(fn(u) u.age >= 18)
    |> map(fn(u) u.name)
    |> concat(", ");

-- => "Alice, Bob"
```
