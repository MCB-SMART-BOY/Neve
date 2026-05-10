/-
  Neve 核心语法形式化
  Formalization of Neve's core syntax: types, values, expressions
-/

namespace Neve

-- === Effect 标注 / Effect Annotation ===
inductive Effect : Type where
  | Pure
  | Effectful
  deriving BEq, Repr, Inhabited

-- === 类型 / Types ===
inductive Ty : Type where
  | Int | Float | Bool | Char | String | Unit
  | List (elem : Ty)
  | Tuple (elems : List Ty)
  | Record (fields : List (String × Ty))
  | Fn (param : Ty) (ret : Ty) (effect : Effect := Effect.Pure)
  | Option (inner : Ty)
  | Bytes | Command | Pipeline | ProcessResult
  | Task (inner : Ty)
  deriving BEq, Repr, Inhabited

-- === 二元运算符 / Binary Operators ===
inductive BinOp : Type where
  | Add | Sub | Mul | Div | Mod
  | Eq | Neq | Lt | Le | Gt | Ge
  | And | Or
  | Pipe
  deriving BEq, Repr, Inhabited

-- === 模式 / Patterns ===
inductive Pattern : Type where
  | wildcard
  | var (x : String)
  | lit_int (n : Int)
  | lit_bool (b : Bool)
  | lit_string (s : String)
  | tuple (pats : List Pattern)
  | list (pats : List Pattern) (rest : Bool)
  | record (fields : List (String × Pattern))
  deriving BEq, Repr, Inhabited

-- === 表达式 / Expressions (forward-declare) ===
-- Use a pre-declaration to break mutual recursion
inductive Expr : Type where
  | lit_int (n : Int)
  | lit_float (f : Float)
  | lit_bool (b : Bool)
  | lit_char (c : Char)
  | lit_string (s : String)
  | lit_unit
  | var (x : String)
  | app (f : Expr) (arg : Expr)
  | lam (x : String) (body : Expr)
  | letIn (x : String) (val : Expr) (body : Expr)
  | binop (op : BinOp) (l : Expr) (r : Expr)
  | matchOn (scrutinee : Expr) (arms : List (Pattern × Expr))
  | builtin (name : String) (args : List Expr)
  deriving BEq, Repr, Inhabited

-- === 运行时值 / Runtime Values ===
inductive Value : Type where
  | int (n : Int)
  | float (f : Float)
  | bool (b : Bool)
  | char (c : Char)
  | string (s : String)
  | unit
  | list (elems : List Value)
  | tuple (elems : List Value)
  | record (fields : List (String × Value))
  | bytes (data : List Nat)
  | processResult (code : Int) (stdout : String) (stderr : String)
  | closure (x : String) (body : Expr) (env : List (String × Value))
  | someVal (v : Value)
  | noneVal
  deriving Repr, Inhabited, BEq

end Neve
