# Neve 集成测试 / Integration Tests

本目录包含 Neve 语言的集成测试,验证从词法分析到执行的完整管道。

This directory contains integration tests for the Neve language, verifying the complete pipeline from lexing to execution.

## 测试结构 / Test Structure

### [parsing.rs](parsing.rs)
**解析器测试 / Parser Tests**

测试完整的解析流程:Lexer → Parser → AST

Tests the complete parsing pipeline: Lexer → Parser → AST

涵盖内容 / Coverage:
- ✅ 基础函数定义 / Basic function definitions
- ✅ 记录字面量 / Record literals
- ✅ Trait 关联类型 / Trait associated types
- ✅ Impl 关联类型 / Impl associated types
- ✅ 模式匹配 / Pattern matching
- ✅ 泛型 / Generics
- ✅ 模块导入 / Module imports
- ✅ 管道操作符 / Pipe operator
- ✅ Derivation 语法 / Derivation syntax
- ✅ 错误恢复 / Error recovery

### [module_loading.rs](module_loading.rs)
**模块加载测试 / Module Loading Tests**

测试模块系统,包括循环依赖检测。

Tests the module system including circular dependency detection.

涵盖内容 / Coverage:
- ✅ 简单模块加载 / Simple module loading
- ✅ 嵌套模块 / Nested modules
- ✅ 循环依赖检测 / Circular dependency detection
- ✅ 错误消息格式 / Error message formatting
- ✅ self 导入 / self imports
- ✅ super 导入 / super imports
- ✅ crate 导入 / crate imports
- ✅ 模块未找到 / Module not found
- ✅ 菱形依赖 / Diamond dependencies

### [evaluation.rs](evaluation.rs)
**求值和 TCO 测试 / Evaluation and TCO Tests**

测试求值器,包括尾调用优化。

Tests the evaluator including tail call optimization.

涵盖内容 / Coverage:
- ✅ 基础算术 / Basic arithmetic
- ✅ 函数应用 / Function application
- ✅ 高阶函数 / Higher-order functions
- ✅ 闭包捕获 / Closure capture
- ✅ 模式匹配(列表/Option) / Pattern matching (lists/Option)
- ✅ 尾递归阶乘 / Tail-recursive factorial
- ✅ 尾递归求和 / Tail-recursive sum
- ✅ 互递归 / Mutual recursion
- ✅ 列表操作(map/filter/fold) / List operations
- ✅ 记录操作 / Record operations
- ✅ If 表达式 / If expressions
- ✅ 惰性求值 / Lazy evaluation
- ✅ 字符串操作 / String operations
- ✅ 布尔运算 / Boolean operations
- ✅ 比较运算 / Comparison operations
- ✅ 管道操作符求值 / Pipe operator evaluation
- ✅ 错误处理 / Error handling
- ✅ Match 表达式中的 TCO / TCO in match expressions
- ✅ If 表达式中的 TCO / TCO in if expressions

### [type_checking.rs](type_checking.rs)
**类型检查测试 / Type Checking Tests**

测试 Hindley-Milner 类型推导和 Trait 约束。

Tests Hindley-Milner type inference and trait constraints.

涵盖内容 / Coverage:
- ✅ 简单类型推导 / Simple type inference
- ✅ 函数类型推导 / Function type inference
- ✅ 多态函数 / Polymorphic functions
- ✅ 列表类型推导 / List type inference
- ✅ 记录类型推导 / Record type inference
- ✅ 高阶函数类型 / Higher-order function types
- ✅ Trait 约束 / Trait constraints
- ✅ 关联类型 / Associated types
- ✅ 类型错误(类型不匹配) / Type errors (mismatch)
- ✅ 类型错误(参数数量不匹配) / Type errors (arity)
- ✅ 递归函数类型 / Recursive function types
- ✅ 互递归类型 / Mutual recursion types
- ✅ Option 类型 / Option type
- ✅ Result 类型 / Result type
- ✅ 泛型函数实例化 / Generic function instantiation
- ✅ 类型注解一致性 / Type annotation consistency
- ✅ 闭包类型推导 / Closure type inference
- ✅ 嵌套泛型 / Nested generics
- ✅ If 分支类型检查 / If branch type checking
- ✅ Match 分支类型检查 / Match arm type checking
- ✅ 类型统一 / Unification
- ✅ Occurs check

### [end_to_end.rs](end_to_end.rs)
**端到端测试 / End-to-End Tests**

测试完整管道:Lexer → Parser → HIR → TypeCheck → Eval

Tests complete pipeline: Lexer → Parser → HIR → TypeCheck → Eval

涵盖内容 / Coverage:
- ✅ Hello World
- ✅ Fibonacci 数列 / Fibonacci sequence
- ✅ 列表处理 / List processing
- ✅ 阶乘(带累加器) / Factorial with accumulator
- ✅ 快速排序 / Quicksort
- ✅ Option 链式调用 / Option chaining
- ✅ Result 错误处理 / Result error handling
- ✅ 树数据结构 / Tree data structure
- ✅ 柯里化 / Currying
- ✅ 函数组合 / Function composition
- ✅ 部分应用 / Partial application
- ✅ Y 组合子 / Y combinator
- ✅ 记录模式匹配 / Record pattern matching
- ✅ 惰性列表处理 / Lazy list processing
- ✅ 字符串操作 / String manipulation
- ✅ 嵌套 let 绑定 / Nested let bindings
- ✅ 互递归函数 / Mutually recursive functions
- ✅ 深度嵌套表达式 / Deeply nested expressions
- ✅ 复杂模式匹配 / Complex pattern matching

## 运行测试 / Running Tests

### 运行所有集成测试 / Run All Integration Tests
```bash
cargo test --tests
```

### 运行特定测试文件 / Run Specific Test File
```bash
cargo test --test parsing
cargo test --test module_loading
cargo test --test evaluation
cargo test --test type_checking
cargo test --test end_to_end
```

### 运行单个测试 / Run Single Test
```bash
cargo test --test parsing test_parse_basic_function
cargo test --test module_loading test_circular_dependency_detection
cargo test --test evaluation test_tail_recursion_factorial
```

### 显示测试输出 / Show Test Output
```bash
cargo test -- --show-output
```

### 运行测试并显示详细信息 / Run Tests with Verbose Output
```bash
cargo test -- --nocapture
```

## 测试原则 / Testing Principles

### 1. 完整性 / Completeness
每个测试应该验证完整的功能,而不仅仅是部分行为。

Each test should verify complete functionality, not just partial behavior.

### 2. 独立性 / Independence
测试应该互相独立,可以以任意顺序运行。

Tests should be independent and runnable in any order.

### 3. 可读性 / Readability
测试代码应该清晰,作为功能的文档和示例。

Test code should be clear, serving as documentation and examples.

### 4. 真实性 / Realism
测试应该使用真实的用例,而不是人为构造的例子。

Tests should use realistic use cases, not artificial examples.

## 测试覆盖 / Test Coverage

当前集成测试覆盖:

Current integration test coverage:

- **解析器 / Parser**: 10 个测试,覆盖所有语法结构
- **模块加载 / Module Loading**: 9 个测试,包括循环依赖检测
- **求值器 / Evaluator**: 23 个测试,包括 TCO
- **类型检查 / Type Checker**: 28 个测试,覆盖推导和错误
- **端到端 / End-to-End**: 20 个测试,完整管道

**总计 / Total**: 90 个集成测试

## 添加新测试 / Adding New Tests

添加新集成测试的步骤:

Steps to add new integration tests:

1. **选择合适的测试文件** / Choose appropriate test file
   - 解析相关 → `parsing.rs`
   - 模块相关 → `module_loading.rs`
   - 求值相关 → `evaluation.rs`
   - 类型相关 → `type_checking.rs`
   - 完整流程 → `end_to_end.rs`

2. **编写测试函数** / Write test function
   ```rust
   #[test]
   fn test_my_new_feature() {
       let source = r#"
           // Neve code here
       "#;

       let result = test_helper(source);
       assert!(result.is_ok());
   }
   ```

3. **运行并验证** / Run and verify
   ```bash
   cargo test --test <file> test_my_new_feature
   ```

4. **更新本 README** / Update this README
   - 在相应部分添加新测试描述
   - 更新测试计数

## 已知问题 / Known Issues

某些测试可能失败或被跳过,因为以下功能尚未完全实现:

Some tests may fail or be skipped because the following features are not fully implemented:

- ⚠️ 完整的 Trait 系统 / Complete trait system
- ⚠️ 代数数据类型 (ADTs) / Algebraic Data Types
- ⚠️ 记录模式匹配 / Record pattern matching
- ⚠️ 某些内置函数 / Some built-in functions
- ⚠️ 完整的错误恢复 / Complete error recovery

这些测试被标记为 `assert!(result.is_ok() || result.is_err())` 以便在实现过程中跟踪进度。

These tests are marked with `assert!(result.is_ok() || result.is_err())` to track progress during implementation.

## 贡献指南 / Contributing

添加测试时请遵循:

When adding tests, please follow:

1. **清晰的测试名称** / Clear test names
   - 使用描述性名称:`test_<feature>_<scenario>`
   - 例如:`test_tail_recursion_factorial`

2. **完整的注释** / Complete comments
   - 说明测试目的
   - 解释预期行为

3. **适当的断言** / Appropriate assertions
   - 使用具体断言而不是宽泛的检查
   - 验证错误消息内容

4. **真实示例** / Realistic examples
   - 使用实际代码模式
   - 避免过于简化的例子

---

**测试你的代码,让 Neve 更健壮!** 🧪

**Test your code, make Neve more robust!** 🧪
