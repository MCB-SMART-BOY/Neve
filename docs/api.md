<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Standard Library API</h1>

<p><em>标准库接口文档</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong>
</p>

</div>

---

> *Your toolkit. Everything you need, nothing you don't.*  
> 标准库工具箱：只包含你需要的东西。


## Using the stdlib / 标准库用法


Neve's standard library is namespaced. Import modules to use `list.*`, `string.*`,
`math.*`, `io.*`, `path.*`, `Map.*`, and `Set.*`. You can also bring selected
names into scope.

```neve
import std.list;
import std.list (map, filter, fold);
import std.string as Str;
import std.Map;
import std.Set;
```


Neve 的标准库是命名空间形式。通过导入模块使用 `list.*`、`string.*`、
`math.*`、`io.*`、`path.*`、`Map.*`、`Set.*`。也可以选择性导入需要的名字。

```neve
import std.list;
import std.list (map, filter, fold);
import std.string as Str;
import std.Map;
import std.Set;
```


## Core Builtins (global) / 核心内置函数（全局）


```neve
id<A>(x: A) -> A
const<A, B>(x: A, y: B) -> A
print(x: A) -> Unit
len(x: A) -> Int
typeOf(x: A) -> String
toString(x: A) -> String
toInt(x: A) -> Int
toFloat(x: A) -> Float
assert(cond: Bool) -> Unit
assertEq<A>(a: A, b: A) -> Unit
trace(label: A, value: B) -> B
force(x: A) -> A
isEvaluated(x: A) -> Bool
```


```neve
id<A>(x: A) -> A
const<A, B>(x: A, y: B) -> A
print(x: A) -> Unit
len(x: A) -> Int
typeOf(x: A) -> String
toString(x: A) -> String
toInt(x: A) -> Int
toFloat(x: A) -> Float
assert(cond: Bool) -> Unit
assertEq<A>(a: A, b: A) -> Unit
trace(label: A, value: B) -> B
force(x: A) -> A
isEvaluated(x: A) -> Bool
```


## List Module (std.list) / 列表模块（std.list）


```neve
list.len<A>(xs: List<A>) -> Int
list.isEmpty<A>(xs: List<A>) -> Bool
list.head<A>(xs: List<A>) -> Option<A>
list.tail<A>(xs: List<A>) -> List<A>
list.append<A>(xs: List<A>, ys: List<A>) -> List<A>
list.map<A, B>(f: A -> B, xs: List<A>) -> List<B>
list.filter<A>(pred: A -> Bool, xs: List<A>) -> List<A>
list.fold<A, B>(init: B, f: B -> A -> B, xs: List<A>) -> B
list.range(start: Int, end: Int) -> List<Int>
```


```neve
list.len<A>(xs: List<A>) -> Int
list.isEmpty<A>(xs: List<A>) -> Bool
list.head<A>(xs: List<A>) -> Option<A>
list.tail<A>(xs: List<A>) -> List<A>
list.append<A>(xs: List<A>, ys: List<A>) -> List<A>
list.map<A, B>(f: A -> B, xs: List<A>) -> List<B>
list.filter<A>(pred: A -> Bool, xs: List<A>) -> List<A>
list.fold<A, B>(init: B, f: B -> A -> B, xs: List<A>) -> B
list.range(start: Int, end: Int) -> List<Int>
```


## String Module (std.string) / 字符串模块（std.string）


```neve
string.len(s: String) -> Int
string.split(s: String, sep: String) -> List<String>
string.join(xs: List<String>, sep: String) -> String
string.trim(s: String) -> String
string.upper(s: String) -> String
string.lower(s: String) -> String
string.contains(s: String, needle: String) -> Bool
```


```neve
string.len(s: String) -> Int
string.split(s: String, sep: String) -> List<String>
string.join(xs: List<String>, sep: String) -> String
string.trim(s: String) -> String
string.upper(s: String) -> String
string.lower(s: String) -> String
string.contains(s: String, needle: String) -> Bool
```


## Option Module (std.option) / Option 模块（std.option）


```neve
enum Option<T> { Some(T), None }

option.some<A>(x: A) -> Option<A>
option.none -> Option<A>
option.is_some<A>(opt: Option<A>) -> Bool
option.is_none<A>(opt: Option<A>) -> Bool
option.unwrap<A>(opt: Option<A>) -> A
option.unwrap_or<A>(opt: Option<A>, default: A) -> A
```


```neve
enum Option<T> { Some(T), None }

option.some<A>(x: A) -> Option<A>
option.none -> Option<A>
option.is_some<A>(opt: Option<A>) -> Bool
option.is_none<A>(opt: Option<A>) -> Bool
option.unwrap<A>(opt: Option<A>) -> A
option.unwrap_or<A>(opt: Option<A>, default: A) -> A
```


## Result Module (std.result) / Result 模块（std.result）


```neve
enum Result<T, E> { Ok(T), Err(E) }

result.ok<T, E>(x: T) -> Result<T, E>
result.err<T, E>(e: E) -> Result<T, E>
result.is_ok<T, E>(res: Result<T, E>) -> Bool
result.is_err<T, E>(res: Result<T, E>) -> Bool
result.unwrap<T, E>(res: Result<T, E>) -> T
result.unwrap_err<T, E>(res: Result<T, E>) -> E
```


```neve
enum Result<T, E> { Ok(T), Err(E) }

result.ok<T, E>(x: T) -> Result<T, E>
result.err<T, E>(e: E) -> Result<T, E>
result.is_ok<T, E>(res: Result<T, E>) -> Bool
result.is_err<T, E>(res: Result<T, E>) -> Bool
result.unwrap<T, E>(res: Result<T, E>) -> T
result.unwrap_err<T, E>(res: Result<T, E>) -> E
```


## Math Module (std.math) / 数学模块（std.math）


Math helpers accept Int or Float where it makes sense. In this doc, `Number`
means Int or Float.

```neve
math.abs(x: Number) -> Number
math.floor(x: Number) -> Int
math.ceil(x: Number) -> Int
math.round(x: Number) -> Int
math.sqrt(x: Number) -> Float
math.pow(base: Number, exp: Number) -> Number
math.max(x: Number, y: Number) -> Number
math.min(x: Number, y: Number) -> Number
```


数学函数在合适的情况下支持 Int 和 Float。这里的 `Number`
表示 Int 或 Float。

```neve
math.abs(x: Number) -> Number
math.floor(x: Number) -> Int
math.ceil(x: Number) -> Int
math.round(x: Number) -> Int
math.sqrt(x: Number) -> Float
math.pow(base: Number, exp: Number) -> Number
math.max(x: Number, y: Number) -> Number
math.min(x: Number, y: Number) -> Number
```


## I/O Module (std.io) / I/O 模块（std.io）


I/O helpers are impure and raise runtime errors on failure.

```neve
io.readFile(path: String) -> String
io.readDir(path: String) -> List<String>
io.writeFile(path: String, content: String) -> Unit
io.appendFile(path: String, content: String) -> Unit
io.createDirAll(path: String) -> Unit
io.removeDirAll(path: String) -> Unit
io.pathExists(path: String) -> Bool
io.isDir(path: String) -> Bool
io.isFile(path: String) -> Bool
io.getEnv(name: String) -> Option<String>
io.currentDir() -> String
io.homeDir() -> Option<String>
io.hashFile(path: String) -> String
io.hashString(content: String) -> String
io.currentSystem() -> String
io.exec(program: String, args: List<String>) -> #{ code: Int, success: Bool, stdout: String, stderr: String }
io.execShell(command: String) -> #{ code: Int, success: Bool, stdout: String, stderr: String }
io.execWith(opts: #{
  program: String,
  args?: List<String>,
  cwd?: String,
  env?: #{ ...String },
  stdin?: String
}) -> #{ code: Int, success: Bool, stdout: String, stderr: String }
```


I/O 函数是非纯的，失败会抛出运行时错误。

```neve
io.readFile(path: String) -> String
io.readDir(path: String) -> List<String>
io.writeFile(path: String, content: String) -> Unit
io.appendFile(path: String, content: String) -> Unit
io.createDirAll(path: String) -> Unit
io.removeDirAll(path: String) -> Unit
io.pathExists(path: String) -> Bool
io.isDir(path: String) -> Bool
io.isFile(path: String) -> Bool
io.getEnv(name: String) -> Option<String>
io.currentDir() -> String
io.homeDir() -> Option<String>
io.hashFile(path: String) -> String
io.hashString(content: String) -> String
io.currentSystem() -> String
io.exec(program: String, args: List<String>) -> #{ code: Int, success: Bool, stdout: String, stderr: String }
io.execShell(command: String) -> #{ code: Int, success: Bool, stdout: String, stderr: String }
io.execWith(opts: #{
  program: String,
  args?: List<String>,
  cwd?: String,
  env?: #{ ...String },
  stdin?: String
}) -> #{ code: Int, success: Bool, stdout: String, stderr: String }
```


## Path Module (std.path) / 路径模块（std.path）


```neve
path.join(a: String, b: String) -> String
path.parent(path: String) -> Option<String>
path.filename(path: String) -> Option<String>
path.extension(path: String) -> Option<String>
path.is_absolute(path: String) -> Bool
```


```neve
path.join(a: String, b: String) -> String
path.parent(path: String) -> Option<String>
path.filename(path: String) -> Option<String>
path.extension(path: String) -> Option<String>
path.is_absolute(path: String) -> Bool
```


## Map / Set Namespaces (Map.*, Set.*) / Map / Set 命名空间（Map.*、Set.*）


```neve
Map.empty -> Map<K, V>
Map.singleton(key: K, value: V) -> Map<K, V>
Map.fromList(items: List<(K, V)>) -> Map<K, V>
Map.get(key: K, map: Map<K, V>) -> Option<V>
Map.contains(key: K, map: Map<K, V>) -> Bool

Set.empty -> Set<A>
Set.singleton(value: A) -> Set<A>
Set.fromList(items: List<A>) -> Set<A>
Set.contains(value: A, set: Set<A>) -> Bool
Set.size(set: Set<A>) -> Int
```


```neve
Map.empty -> Map<K, V>
Map.singleton(key: K, value: V) -> Map<K, V>
Map.fromList(items: List<(K, V)>) -> Map<K, V>
Map.get(key: K, map: Map<K, V>) -> Option<V>
Map.contains(key: K, map: Map<K, V>) -> Bool

Set.empty -> Set<A>
Set.singleton(value: A) -> Set<A>
Set.fromList(items: List<A>) -> Set<A>
Set.contains(value: A, set: Set<A>) -> Bool
Set.size(set: Set<A>) -> Int
```


## Package System (in progress) / 包管理（开发中）


```neve
derivation #{
    name: String,
    system: String,
    builder: String,
    args: List<String>,      -- optional
    version: String,         -- optional (defaults to 0.0.0)
    ...                      -- other string fields become env vars
} -> Derivation
```

```neve
fetch.path(path: String) -> #{ path: String, hash: String, cached: Bool }
fetch.pathWithHash(path: String, hash: String) -> #{ path: String, hash: String, cached: Bool }
fetch.url(url: String) -> #{ path: String, hash: String, cached: Bool }
fetch.urlWithHash(url: String, hash: String) -> #{ path: String, hash: String, cached: Bool }
fetch.git(url: String, rev: String) -> #{ path: String, hash: String, cached: Bool }
fetch.gitWithHash(url: String, rev: String, hash: String) -> #{ path: String, hash: String, cached: Bool }
```

Note: Fetch helpers are impure and can access local filesystem/network. Prefer
`*WithHash` variants for reproducible builds.


```neve
derivation #{
    name: String,
    system: String,
    builder: String,
    args: List<String>,      -- 可选
    version: String,         -- 可选（默认 0.0.0）
    ...                      -- 其它字符串字段会变成环境变量
} -> Derivation
```

```neve
fetch.path(path: String) -> #{ path: String, hash: String, cached: Bool }
fetch.pathWithHash(path: String, hash: String) -> #{ path: String, hash: String, cached: Bool }
fetch.url(url: String) -> #{ path: String, hash: String, cached: Bool }
fetch.urlWithHash(url: String, hash: String) -> #{ path: String, hash: String, cached: Bool }
fetch.git(url: String, rev: String) -> #{ path: String, hash: String, cached: Bool }
fetch.gitWithHash(url: String, rev: String, hash: String) -> #{ path: String, hash: String, cached: Bool }
```

注意：fetch 函数是带副作用的，可能访问本地文件系统或网络。为了可复现构建，优先使用带 `WithHash` 的版本。


## Example / 示例


```neve
import std.list (filter, map);
import std.string;

let users = [
    #{ name = "Alice", age = 30 },
    #{ name = "Bob", age = 25 },
];

let names = users
    |> filter(fn(u) u.age >= 18)
    |> map(fn(u) u.name);

let joined = string.join(names, ", ");

-- => "Alice, Bob"
```

---


```neve
import std.list (filter, map);
import std.string;

let users = [
    #{ name = "小明", age = 30 },
    #{ name = "小红", age = 25 },
];

let names = users
    |> filter(fn(u) u.age >= 18)
    |> map(fn(u) u.name);

let joined = string.join(names, "、");

-- => "小明、小红"
```

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                    Build something. Break something. Learn.
═══════════════════════════════════════════════════════════════════════════════
```

</div>
