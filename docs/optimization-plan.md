# Neve 项目优化计划

> **更新于 v0.6.0**: 标记 ✅ 的项目已完成

## 项目设计哲学总结

Neve 的核心理念：**"我不讨厌 Nix。我想成为 Nix —— 但是那个如果今天从头开始的话，本应该成为的 Nix。"**

### 六大设计原则

1. **零歧义** - `#{ x = 1 }` 永远是记录，`fn(x) x + 1` 永远是函数
2. **语法统一** - 相似概念使用相似语法
3. **不依赖缩进** - 使用显式分隔符
4. **纯函数式** - 无副作用，引用透明
5. **极简性** - 仅 17 个关键字
6. **Unix 哲学** - 做好一件事，通过组合构建复杂系统

---

## 一、语言核心优化

### 1.1 尾调用优化 (TCO) - 高优先级 ✅ 已完成

**状态**: 已在 v0.6.0 实现

**问题**: 递归函数可能导致栈溢出

```neve
-- 当前这样的代码会栈溢出
fn factorial(n, acc) = match n {
    0 -> acc,
    n -> factorial(n - 1, n * acc),  -- 尾调用，应该优化
};
```

**优化方案**:

```rust
// crates/neve-eval/src/eval.rs

pub enum TcoResult {
    Value(Value),
    TailCall { func: Value, args: Vec<Value> },
}

pub fn eval_tco(&mut self, expr: &Expr) -> Result<Value, EvalError> {
    let mut current = TcoResult::Value(self.eval_inner(expr)?);
    
    loop {
        match current {
            TcoResult::Value(v) => return Ok(v),
            TcoResult::TailCall { func, args } => {
                current = self.apply_tco(&func, args)?;
            }
        }
    }
}

fn is_tail_position(&self, expr: &Expr, in_tail: bool) -> bool {
    match &expr.kind {
        ExprKind::Call { .. } if in_tail => true,
        ExprKind::If { then_branch, else_branch, .. } => {
            self.is_tail_position(then_branch, true) &&
            self.is_tail_position(else_branch, true)
        }
        ExprKind::Match { arms, .. } => {
            arms.iter().all(|arm| self.is_tail_position(&arm.body, true))
        }
        ExprKind::Let { body, .. } => self.is_tail_position(body, true),
        ExprKind::Block { expr: Some(e), .. } => self.is_tail_position(e, true),
        _ => false,
    }
}
```

### 1.2 惰性求值增强

**当前状态**: 基础 Thunk 实现

**优化方案**:

```rust
// 添加惰性参数支持
pub struct Param {
    pub pattern: Pattern,
    pub ty: Type,
    pub is_lazy: bool,  // 已有但未充分使用
}

// 优化 Thunk 共享
pub struct Thunk {
    inner: Arc<RwLock<ThunkState>>,  // 改用 Arc<RwLock> 支持并发
}
```

### 1.3 模式匹配编译优化

**当前状态**: 线性匹配

**优化方案**: 使用决策树编译

```rust
// crates/neve-eval/src/pattern.rs

pub enum DecisionTree {
    Leaf(usize),  // 匹配到的分支索引
    Switch {
        scrutinee: Path,
        cases: Vec<(Constructor, DecisionTree)>,
        default: Option<Box<DecisionTree>>,
    },
}

pub fn compile_patterns(arms: &[MatchArm]) -> DecisionTree {
    // 使用 Maranget 算法编译模式
}
```

---

## 二、类型系统优化

### 2.1 改进类型错误信息 ✅ 已完成

**状态**: 已在 v0.6.0 增强

**优化方案**:

```rust
// crates/neve-typeck/src/errors.rs

pub struct TypeMismatch {
    pub expected: Ty,
    pub found: Ty,
    pub span: Span,
    pub context: ErrorContext,
}

pub enum ErrorContext {
    FunctionArg { func_name: String, arg_index: usize },
    BinaryOp { op: BinOp, side: Side },
    IfCondition,
    IfBranches,
    MatchArms,
    FieldAccess { field: String },
    ListElement { index: usize },
}

pub fn format_type_error(err: &TypeMismatch) -> Diagnostic {
    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        err.span,
        format!("type mismatch"),
    );
    
    // 添加上下文信息
    match &err.context {
        ErrorContext::FunctionArg { func_name, arg_index } => {
            diag = diag.with_note(format!(
                "in argument {} of function '{}'",
                arg_index + 1, func_name
            ));
        }
        // ... 其他上下文
    }
    
    // 添加修复建议
    if let Some(suggestion) = suggest_fix(&err.expected, &err.found) {
        diag = diag.with_help(suggestion);
    }
    
    diag
}

fn suggest_fix(expected: &Ty, found: &Ty) -> Option<String> {
    match (&expected.kind, &found.kind) {
        (TyKind::Int, TyKind::Float) => 
            Some("use `toInt(value)` to convert Float to Int".into()),
        (TyKind::Float, TyKind::Int) => 
            Some("use `toFloat(value)` to convert Int to Float".into()),
        (TyKind::String, _) => 
            Some("use `toString(value)` to convert to String".into()),
        (TyKind::List(_), TyKind::Tuple(_)) =>
            Some("use `toList(tuple)` to convert Tuple to List".into()),
        _ => None,
    }
}
```

### 2.2 完善关联类型

**当前状态**: 有声明但实现不完整

**优化方案**:

```rust
// crates/neve-typeck/src/traits.rs

pub struct AssocTypeBinding {
    pub trait_: DefId,
    pub name: String,
    pub ty: Ty,
}

impl TraitResolver {
    pub fn resolve_assoc_type(
        &self,
        receiver: &Ty,
        trait_: DefId,
        name: &str,
    ) -> Result<Ty, TypeError> {
        // 1. 查找 receiver 的 impl
        let impl_ = self.find_impl(receiver, trait_)?;
        
        // 2. 在 impl 中查找关联类型
        for assoc in &impl_.assoc_types {
            if assoc.name == name {
                return Ok(assoc.ty.clone());
            }
        }
        
        Err(TypeError::AssocTypeNotFound { ... })
    }
}
```

### 2.3 高阶类型 (HKT) 基础设施

**目标**: 为未来的 HKT 支持做准备

```rust
// crates/neve-typeck/src/kind.rs

pub enum Kind {
    Type,                           // *
    Arrow(Box<Kind>, Box<Kind>),   // * -> *
}

pub struct TyParam {
    pub name: String,
    pub kind: Kind,
    pub bounds: Vec<TraitBound>,
}

// 支持 Functor, Monad 等
trait Functor<F> where F: * -> * {
    fn map<A, B>(f: A -> B, fa: F<A>) -> F<B>;
}
```

---

## 三、存储系统优化

### 3.1 完成 NAR 格式实现 ✅ 已完成

**状态**: 已在 v0.6.0 实现完整的 NAR 读写器

**优化方案**:

```rust
// crates/neve-store/src/nar.rs

use std::io::{Read, Write};
use xz2::read::XzDecoder;
use tar::Archive;

pub struct NarWriter<W: Write> {
    writer: W,
}

impl<W: Write> NarWriter<W> {
    pub fn write_path(&mut self, path: &Path) -> io::Result<()> {
        self.write_str("nix-archive-1")?;
        self.write_entry(path)
    }
    
    fn write_entry(&mut self, path: &Path) -> io::Result<()> {
        self.write_str("(")?;
        
        if path.is_file() {
            self.write_str("type")?;
            self.write_str("regular")?;
            
            let meta = path.metadata()?;
            if meta.permissions().mode() & 0o111 != 0 {
                self.write_str("executable")?;
                self.write_str("")?;
            }
            
            self.write_str("contents")?;
            let contents = std::fs::read(path)?;
            self.write_bytes(&contents)?;
        } else if path.is_dir() {
            self.write_str("type")?;
            self.write_str("directory")?;
            
            let mut entries: Vec<_> = std::fs::read_dir(path)?
                .filter_map(|e| e.ok())
                .collect();
            entries.sort_by_key(|e| e.file_name());
            
            for entry in entries {
                self.write_str("entry")?;
                self.write_str("(")?;
                self.write_str("name")?;
                self.write_str(&entry.file_name().to_string_lossy())?;
                self.write_str("node")?;
                self.write_entry(&entry.path())?;
                self.write_str(")")?;
            }
        } else if path.is_symlink() {
            self.write_str("type")?;
            self.write_str("symlink")?;
            self.write_str("target")?;
            let target = std::fs::read_link(path)?;
            self.write_str(&target.to_string_lossy())?;
        }
        
        self.write_str(")")?;
        Ok(())
    }
    
    fn write_str(&mut self, s: &str) -> io::Result<()> {
        let bytes = s.as_bytes();
        let len = bytes.len() as u64;
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(bytes)?;
        // 8字节对齐
        let padding = (8 - (len % 8)) % 8;
        self.writer.write_all(&vec![0u8; padding as usize])?;
        Ok(())
    }
}

pub struct NarReader<R: Read> {
    reader: R,
}

impl<R: Read> NarReader<R> {
    pub fn extract(&mut self, dest: &Path) -> io::Result<()> {
        let magic = self.read_str()?;
        if magic != "nix-archive-1" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid NAR"));
        }
        self.extract_entry(dest)
    }
    
    fn extract_entry(&mut self, dest: &Path) -> io::Result<()> {
        self.expect_str("(")?;
        self.expect_str("type")?;
        
        let ty = self.read_str()?;
        match ty.as_str() {
            "regular" => {
                let mut executable = false;
                loop {
                    let tag = self.read_str()?;
                    match tag.as_str() {
                        "executable" => {
                            self.read_str()?;
                            executable = true;
                        }
                        "contents" => {
                            let contents = self.read_bytes()?;
                            std::fs::write(dest, &contents)?;
                            if executable {
                                use std::os::unix::fs::PermissionsExt;
                                let perms = std::fs::Permissions::from_mode(0o755);
                                std::fs::set_permissions(dest, perms)?;
                            }
                        }
                        ")" => break,
                        _ => return Err(io::Error::new(
                            io::ErrorKind::InvalidData, 
                            format!("unexpected tag: {}", tag)
                        )),
                    }
                }
            }
            "directory" => {
                std::fs::create_dir_all(dest)?;
                loop {
                    let tag = self.read_str()?;
                    match tag.as_str() {
                        "entry" => {
                            self.expect_str("(")?;
                            self.expect_str("name")?;
                            let name = self.read_str()?;
                            self.expect_str("node")?;
                            self.extract_entry(&dest.join(&name))?;
                            self.expect_str(")")?;
                        }
                        ")" => break,
                        _ => return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unexpected tag: {}", tag)
                        )),
                    }
                }
            }
            "symlink" => {
                self.expect_str("target")?;
                let target = self.read_str()?;
                std::os::unix::fs::symlink(&target, dest)?;
                self.expect_str(")")?;
            }
            _ => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown type: {}", ty)
            )),
        }
        
        Ok(())
    }
}
```

### 3.2 并行缓存查询

**优化方案**:

```rust
// crates/neve-store/src/cache.rs

use tokio::sync::Semaphore;
use futures::stream::{self, StreamExt};

impl BinaryCache {
    pub async fn query_parallel(&self, paths: &[StorePath]) -> Vec<Option<CachedPath>> {
        let semaphore = Arc::new(Semaphore::new(8));  // 限制并发
        
        stream::iter(paths)
            .map(|path| {
                let sem = semaphore.clone();
                let caches = self.caches.clone();
                async move {
                    let _permit = sem.acquire().await.unwrap();
                    for cache in &caches {
                        if let Ok(Some(cached)) = self.query_cache(cache, path).await {
                            return Some(cached);
                        }
                    }
                    None
                }
            })
            .buffer_unordered(paths.len())
            .collect()
            .await
    }
}
```

### 3.3 签名验证

**优化方案**:

```rust
// crates/neve-store/src/signature.rs

use ed25519_dalek::{PublicKey, Signature, Verifier};

pub struct NarInfo {
    pub store_path: StorePath,
    pub url: String,
    pub compression: Compression,
    pub file_hash: Hash,
    pub file_size: u64,
    pub nar_hash: Hash,
    pub nar_size: u64,
    pub references: Vec<StorePath>,
    pub signatures: Vec<NarSignature>,
}

pub struct NarSignature {
    pub key_name: String,
    pub signature: Vec<u8>,
}

pub struct TrustedKeys {
    keys: HashMap<String, PublicKey>,
}

impl TrustedKeys {
    pub fn verify(&self, info: &NarInfo) -> Result<(), SignatureError> {
        // 需要至少一个有效签名
        for sig in &info.signatures {
            if let Some(key) = self.keys.get(&sig.key_name) {
                let message = self.fingerprint(info);
                let signature = Signature::from_bytes(&sig.signature)?;
                
                if key.verify(message.as_bytes(), &signature).is_ok() {
                    return Ok(());
                }
            }
        }
        
        Err(SignatureError::NoValidSignature)
    }
    
    fn fingerprint(&self, info: &NarInfo) -> String {
        format!(
            "1;{};{};{};{}",
            info.store_path,
            info.nar_hash,
            info.nar_size,
            info.references.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",")
        )
    }
}
```

---

## 四、构建系统优化

### 4.1 seccomp 支持

**优化方案**:

```rust
// crates/neve-builder/src/seccomp.rs

#[cfg(target_os = "linux")]
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, SeccompRule};

pub fn create_build_filter() -> Result<BpfProgram, SeccompError> {
    let allowed_syscalls = vec![
        // 文件操作
        "read", "write", "open", "close", "stat", "fstat", "lstat",
        "poll", "lseek", "mmap", "mprotect", "munmap", "brk",
        "ioctl", "access", "pipe", "select", "sched_yield",
        "dup", "dup2", "pause", "nanosleep", "getpid", "socket",
        "connect", "accept", "sendto", "recvfrom", "sendmsg",
        "recvmsg", "shutdown", "bind", "listen", "getsockname",
        "getpeername", "socketpair", "setsockopt", "getsockopt",
        "clone", "fork", "vfork", "execve", "exit", "wait4",
        "kill", "uname", "fcntl", "flock", "fsync", "fdatasync",
        "truncate", "ftruncate", "getdents", "getcwd", "chdir",
        "fchdir", "rename", "mkdir", "rmdir", "creat", "link",
        "unlink", "symlink", "readlink", "chmod", "fchmod",
        "chown", "fchown", "lchown", "umask", "gettimeofday",
        "getrlimit", "getrusage", "sysinfo", "times", "getuid",
        "getgid", "setuid", "setgid", "geteuid", "getegid",
        "setpgid", "getppid", "getpgrp", "setsid", "setreuid",
        "setregid", "getgroups", "setgroups", "setresuid",
        "getresuid", "setresgid", "getresgid", "getpgid",
        "setfsuid", "setfsgid", "getsid", "capget", "capset",
        "rt_sigpending", "rt_sigtimedwait", "rt_sigqueueinfo",
        "rt_sigsuspend", "sigaltstack", "utime", "mknod",
        "uselib", "personality", "statfs", "fstatfs", "getpriority",
        "setpriority", "sched_setparam", "sched_getparam",
        "sched_setscheduler", "sched_getscheduler",
        "sched_get_priority_max", "sched_get_priority_min",
        "sched_rr_get_interval", "mlock", "munlock", "mlockall",
        "munlockall", "prctl", "arch_prctl", "setrlimit",
        "sync", "acct", "settimeofday", "mount", "umount2",
        "swapon", "swapoff", "reboot", "sethostname", "setdomainname",
        "ioperm", "iopl", "create_module", "init_module",
        "delete_module", "quotactl", "nfsservctl", "getpmsg",
        "putpmsg", "afs_syscall", "tuxcall", "security",
        "gettid", "readahead", "setxattr", "lsetxattr", "fsetxattr",
        "getxattr", "lgetxattr", "fgetxattr", "listxattr",
        "llistxattr", "flistxattr", "removexattr", "lremovexattr",
        "fremovexattr", "tkill", "time", "futex", "sched_setaffinity",
        "sched_getaffinity", "set_thread_area", "io_setup",
        "io_destroy", "io_getevents", "io_submit", "io_cancel",
        "get_thread_area", "lookup_dcookie", "epoll_create",
        "getdents64", "set_tid_address", "restart_syscall",
        "semtimedop", "fadvise64", "timer_create", "timer_settime",
        "timer_gettime", "timer_getoverrun", "timer_delete",
        "clock_settime", "clock_gettime", "clock_getres",
        "clock_nanosleep", "exit_group", "epoll_wait", "epoll_ctl",
        "tgkill", "utimes", "mbind", "set_mempolicy", "get_mempolicy",
        "mq_open", "mq_unlink", "mq_timedsend", "mq_timedreceive",
        "mq_notify", "mq_getsetattr", "kexec_load", "waitid",
        "add_key", "request_key", "keyctl", "ioprio_set",
        "ioprio_get", "inotify_init", "inotify_add_watch",
        "inotify_rm_watch", "migrate_pages", "openat", "mkdirat",
        "mknodat", "fchownat", "futimesat", "newfstatat", "unlinkat",
        "renameat", "linkat", "symlinkat", "readlinkat", "fchmodat",
        "faccessat", "pselect6", "ppoll", "unshare", "set_robust_list",
        "get_robust_list", "splice", "tee", "sync_file_range",
        "vmsplice", "move_pages", "utimensat", "epoll_pwait",
        "signalfd", "timerfd_create", "eventfd", "fallocate",
        "timerfd_settime", "timerfd_gettime", "accept4", "signalfd4",
        "eventfd2", "epoll_create1", "dup3", "pipe2", "inotify_init1",
        "preadv", "pwritev", "rt_tgsigqueueinfo", "perf_event_open",
        "recvmmsg", "fanotify_init", "fanotify_mark", "prlimit64",
        "name_to_handle_at", "open_by_handle_at", "clock_adjtime",
        "syncfs", "sendmmsg", "setns", "getcpu", "process_vm_readv",
        "process_vm_writev", "kcmp", "finit_module", "sched_setattr",
        "sched_getattr", "renameat2", "seccomp", "getrandom",
        "memfd_create", "kexec_file_load", "bpf", "execveat",
        "userfaultfd", "membarrier", "mlock2", "copy_file_range",
        "preadv2", "pwritev2", "pkey_mprotect", "pkey_alloc",
        "pkey_free", "statx", "rseq",
    ];
    
    // 禁止危险的系统调用
    let denied_syscalls = vec![
        "ptrace",       // 进程追踪
        "kexec_load",   // 加载内核
        "reboot",       // 重启
        "swapon",       // 交换分区
        "swapoff",
        "mount",        // 挂载（沙箱外已处理）
        "umount2",
    ];
    
    SeccompFilter::new(
        allowed_syscalls.into_iter()
            .map(|name| (name.to_string(), SeccompAction::Allow))
            .chain(
                denied_syscalls.into_iter()
                    .map(|name| (name.to_string(), SeccompAction::Errno(libc::EPERM)))
            )
            .collect(),
        SeccompAction::Log,  // 未知系统调用记录日志
    )
    .and_then(|f| f.try_into())
}
```

### 4.2 增量构建支持

**优化方案**:

```rust
// crates/neve-builder/src/incremental.rs

use std::collections::HashMap;
use std::time::SystemTime;

pub struct BuildCache {
    cache_dir: PathBuf,
    entries: HashMap<Hash, CacheEntry>,
}

pub struct CacheEntry {
    pub drv_hash: Hash,
    pub output_hashes: HashMap<String, Hash>,
    pub build_time: SystemTime,
    pub inputs_hash: Hash,
}

impl BuildCache {
    pub fn check(&self, drv: &Derivation) -> Option<CacheHit> {
        let drv_hash = drv.hash();
        
        if let Some(entry) = self.entries.get(&drv_hash) {
            // 验证所有输入仍然有效
            let current_inputs_hash = self.hash_inputs(drv)?;
            if current_inputs_hash == entry.inputs_hash {
                return Some(CacheHit {
                    outputs: entry.output_hashes.clone(),
                });
            }
        }
        
        None
    }
    
    pub fn record(&mut self, drv: &Derivation, outputs: &HashMap<String, StorePath>) {
        let entry = CacheEntry {
            drv_hash: drv.hash(),
            output_hashes: outputs.iter()
                .map(|(k, v)| (k.clone(), v.hash()))
                .collect(),
            build_time: SystemTime::now(),
            inputs_hash: self.hash_inputs(drv).unwrap(),
        };
        
        self.entries.insert(drv.hash(), entry);
        self.persist();
    }
    
    fn hash_inputs(&self, drv: &Derivation) -> Option<Hash> {
        let mut hasher = Hasher::new();
        
        for (input_drv, outputs) in &drv.input_drvs {
            hasher.update(input_drv.hash().as_bytes());
            for output in outputs {
                hasher.update(output.as_bytes());
            }
        }
        
        for input_src in &drv.input_srcs {
            hasher.update(input_src.hash().as_bytes());
        }
        
        Some(hasher.finalize())
    }
}
```

### 4.3 分布式构建基础

**优化方案**:

```rust
// crates/neve-builder/src/remote.rs

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct RemoteBuilder {
    host: String,
    port: u16,
    ssh_key: Option<PathBuf>,
    max_jobs: usize,
}

pub struct BuildJob {
    pub drv: Derivation,
    pub priority: i32,
}

pub struct RemoteBuildResult {
    pub outputs: HashMap<String, StorePath>,
    pub log: String,
    pub duration: Duration,
}

impl RemoteBuilder {
    pub async fn build(&self, job: BuildJob) -> Result<RemoteBuildResult, RemoteError> {
        // 1. 连接远程机器
        let stream = TcpStream::connect(format!("{}:{}", self.host, self.port)).await?;
        
        // 2. 发送派生
        let drv_json = serde_json::to_string(&job.drv)?;
        self.send_message(&stream, &drv_json).await?;
        
        // 3. 传输缺失的输入
        let missing = self.query_missing(&stream, &job.drv).await?;
        for path in missing {
            self.upload_path(&stream, &path).await?;
        }
        
        // 4. 触发构建
        self.send_command(&stream, "build").await?;
        
        // 5. 等待结果
        let result = self.receive_result(&stream).await?;
        
        // 6. 下载输出
        for (name, path) in &result.outputs {
            if !self.store.exists(&path) {
                self.download_path(&stream, &path).await?;
            }
        }
        
        Ok(result)
    }
}
```

---

## 五、CLI 优化

### 5.1 进度显示 ✅ 已完成

**状态**: 已在 v0.6.0 实现 ProgressBar、Status、Table 等

**优化方案**:

```rust
// neve-cli/src/progress.rs

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct BuildProgress {
    multi: MultiProgress,
    bars: HashMap<String, ProgressBar>,
}

impl BuildProgress {
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: HashMap::new(),
        }
    }
    
    pub fn add_package(&mut self, name: &str, total_steps: u64) {
        let style = ProgressStyle::default_bar()
            .template("{prefix:.bold.dim} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  ");
        
        let bar = self.multi.add(ProgressBar::new(total_steps));
        bar.set_style(style);
        bar.set_prefix(format!("[{}]", name));
        
        self.bars.insert(name.to_string(), bar);
    }
    
    pub fn update(&self, name: &str, step: u64, message: &str) {
        if let Some(bar) = self.bars.get(name) {
            bar.set_position(step);
            bar.set_message(message.to_string());
        }
    }
    
    pub fn finish(&self, name: &str) {
        if let Some(bar) = self.bars.get(name) {
            bar.finish_with_message("done");
        }
    }
}
```

### 5.2 交互式搜索

**优化方案**:

```rust
// neve-cli/src/commands/search.rs

use dialoguer::{theme::ColorfulTheme, FuzzySelect};

pub fn run_interactive(query: &str) -> Result<(), String> {
    let registry = PackageRegistry::open()?;
    let results = registry.search(query)?;
    
    if results.is_empty() {
        println!("No packages found matching '{}'", query);
        return Ok(());
    }
    
    let items: Vec<String> = results.iter()
        .map(|pkg| format!(
            "{} ({}) - {}",
            pkg.name,
            pkg.version,
            pkg.description.chars().take(50).collect::<String>()
        ))
        .collect();
    
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select package to install")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| e.to_string())?;
    
    let selected = &results[selection];
    
    println!("\nPackage: {}", selected.name);
    println!("Version: {}", selected.version);
    println!("Description: {}", selected.description);
    println!("Homepage: {}", selected.homepage.as_deref().unwrap_or("N/A"));
    println!("License: {}", selected.license.as_deref().unwrap_or("N/A"));
    
    let confirm = dialoguer::Confirm::new()
        .with_prompt("Install this package?")
        .interact()
        .map_err(|e| e.to_string())?;
    
    if confirm {
        install::run(&selected.name)?;
    }
    
    Ok(())
}
```

### 5.3 配置文件支持

**优化方案**:

```rust
// neve-cli/src/config.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NeveConfig {
    #[serde(default)]
    pub store: StoreConfig,
    
    #[serde(default)]
    pub build: BuildConfig,
    
    #[serde(default)]
    pub network: NetworkConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StoreConfig {
    pub path: Option<PathBuf>,
    pub auto_gc: bool,
    pub gc_keep_generations: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    pub max_jobs: usize,
    pub cores_per_job: usize,
    pub backend: String,
    pub keep_failed: bool,
    pub timeout: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub binary_caches: Vec<String>,
    pub trusted_keys: Vec<String>,
    pub proxy: Option<String>,
}

impl NeveConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content).map_err(ConfigError::Parse)
        } else {
            Ok(Self::default())
        }
    }
    
    fn config_path() -> Result<PathBuf, ConfigError> {
        let home = dirs::home_dir().ok_or(ConfigError::NoHome)?;
        Ok(home.join(".config").join("neve").join("config.toml"))
    }
}
```

---

## 六、超越 Nix 的独特特性

### 6.1 编译期配置验证

**目标**: 在编译期捕获配置错误

```neve
-- neve-config/examples/validated.neve

type PortNumber = Int where fn(p) p >= 1 && p <= 65535;
type Hostname = String where fn(s) s.len() > 0 && s.len() <= 253;

struct ServerConfig {
    host: Hostname,
    port: PortNumber,
    workers: Int where fn(w) w >= 1,
}

-- 编译期验证
let config = ServerConfig {
    host = "localhost",
    port = 8080,
    workers = 4,
};

-- 这会在编译期失败
-- let bad_config = ServerConfig {
--     host = "",            -- 错误: 空字符串
--     port = 70000,         -- 错误: 超出范围
--     workers = 0,          -- 错误: 必须 >= 1
-- };
```

### 6.2 配置组合器

**目标**: 类型安全的配置组合

```neve
-- neve-config/examples/combinators.neve

-- 基础配置单元
let withSSL = fn(config) config // #{
    ssl = true,
    ssl_cert = "/etc/ssl/cert.pem",
    ssl_key = "/etc/ssl/key.pem",
};

let withLogging = fn(level) fn(config) config // #{
    logging = #{
        level = level,
        format = "json",
    },
};

let withMetrics = fn(port) fn(config) config // #{
    metrics = #{
        enabled = true,
        port = port,
    },
};

-- 组合配置
let productionConfig = 
    baseConfig
    |> withSSL
    |> withLogging("info")
    |> withMetrics(9090);

-- 类型系统确保组合的正确性
```

### 6.3 配置差异追踪

**目标**: 自动追踪配置变更

```rust
// crates/neve-config/src/diff.rs

pub struct ConfigDiff {
    pub added: Vec<ConfigChange>,
    pub removed: Vec<ConfigChange>,
    pub modified: Vec<ConfigModification>,
}

pub struct ConfigChange {
    pub path: String,
    pub value: Value,
}

pub struct ConfigModification {
    pub path: String,
    pub old_value: Value,
    pub new_value: Value,
}

impl ConfigDiff {
    pub fn compute(old: &Value, new: &Value) -> Self {
        let mut diff = Self::default();
        Self::diff_recursive("", old, new, &mut diff);
        diff
    }
    
    fn diff_recursive(path: &str, old: &Value, new: &Value, diff: &mut ConfigDiff) {
        match (old, new) {
            (Value::Record(old_rec), Value::Record(new_rec)) => {
                // 检查新增的字段
                for (key, value) in new_rec.iter() {
                    let new_path = format!("{}.{}", path, key);
                    if let Some(old_value) = old_rec.get(key) {
                        Self::diff_recursive(&new_path, old_value, value, diff);
                    } else {
                        diff.added.push(ConfigChange {
                            path: new_path,
                            value: value.clone(),
                        });
                    }
                }
                
                // 检查删除的字段
                for (key, value) in old_rec.iter() {
                    if !new_rec.contains_key(key) {
                        diff.removed.push(ConfigChange {
                            path: format!("{}.{}", path, key),
                            value: value.clone(),
                        });
                    }
                }
            }
            _ if old != new => {
                diff.modified.push(ConfigModification {
                    path: path.to_string(),
                    old_value: old.clone(),
                    new_value: new.clone(),
                });
            }
            _ => {}
        }
    }
}
```

### 6.4 智能包建议

**目标**: 基于使用模式的智能建议

```rust
// crates/neve-store/src/suggestions.rs

pub struct PackageSuggester {
    usage_data: UsageDatabase,
    patterns: Vec<UsagePattern>,
}

pub struct UsagePattern {
    pub packages: Vec<String>,
    pub often_used_with: Vec<String>,
    pub confidence: f64,
}

impl PackageSuggester {
    pub fn suggest_for(&self, installed: &[String]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        
        for pattern in &self.patterns {
            let matches: Vec<_> = pattern.packages.iter()
                .filter(|p| installed.contains(p))
                .collect();
            
            if matches.len() >= pattern.packages.len() / 2 {
                for pkg in &pattern.often_used_with {
                    if !installed.contains(pkg) {
                        suggestions.push(Suggestion {
                            package: pkg.clone(),
                            reason: format!(
                                "Often used with {}",
                                matches.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                            ),
                            confidence: pattern.confidence,
                        });
                    }
                }
            }
        }
        
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        suggestions.truncate(5);
        suggestions
    }
}
```

### 6.5 配置时间旅行

**目标**: 查看任意时间点的配置状态

```rust
// crates/neve-config/src/history.rs

use chrono::{DateTime, Utc};

pub struct ConfigHistory {
    generations: Vec<GenerationSnapshot>,
}

pub struct GenerationSnapshot {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub config_hash: Hash,
    pub packages: Vec<StorePath>,
    pub diff_from_prev: Option<ConfigDiff>,
}

impl ConfigHistory {
    pub fn at_time(&self, time: DateTime<Utc>) -> Option<&GenerationSnapshot> {
        self.generations.iter()
            .rev()
            .find(|g| g.timestamp <= time)
    }
    
    pub fn diff_between(&self, from: u64, to: u64) -> ConfigDiff {
        let from_gen = self.generations.iter().find(|g| g.id == from);
        let to_gen = self.generations.iter().find(|g| g.id == to);
        
        match (from_gen, to_gen) {
            (Some(f), Some(t)) => {
                // 累积差异
                let mut diff = ConfigDiff::default();
                for gen in self.generations.iter()
                    .filter(|g| g.id > f.id && g.id <= t.id)
                {
                    if let Some(d) = &gen.diff_from_prev {
                        diff.merge(d);
                    }
                }
                diff
            }
            _ => ConfigDiff::default(),
        }
    }
    
    pub fn search_changes(&self, path: &str) -> Vec<&GenerationSnapshot> {
        self.generations.iter()
            .filter(|g| {
                g.diff_from_prev.as_ref()
                    .map(|d| d.affects_path(path))
                    .unwrap_or(false)
            })
            .collect()
    }
}
```

---

## 七、实施优先级

### 高优先级（v0.6.0）✅ 已完成

1. ✅ **尾调用优化 (TCO)** - 解决递归栈溢出
2. ✅ **改进类型错误信息** - 提升开发体验
3. ✅ **完成 NAR 格式** - 启用二进制缓存
4. ✅ **进度显示** - 改善 CLI 体验
5. ✅ **安全增强** - SecurityProfile 支持

### 中优先级（v0.7.0）

1. **seccomp 支持** - 完整的系统调用过滤
2. **增量构建** - 提升构建速度
3. **签名验证** - 安全的远程缓存
4. **并行缓存查询** - 提升缓存性能

### 低优先级（v0.8.0+）

1. **HKT 基础设施** - 为未来高级类型做准备
2. **分布式构建** - 支持大规模项目
3. **智能建议** - 提升用户体验
4. **配置时间旅行** - 高级调试功能

---

## 八、总结

Neve 项目已经具备了坚实的基础：

✅ **清晰的语法设计** - 17 个关键字，零歧义
✅ **类型安全** - Hindley-Milner 类型推导
✅ **现代工具链** - REPL、格式化器、LSP
✅ **内容寻址存储** - 可重现、可验证
✅ **沙箱构建** - 隔离、安全

通过以上优化，Neve 将真正实现"继承并超越 Nix"的目标：

1. **性能更好** - TCO、增量构建、并行缓存
2. **更安全** - seccomp、签名验证
3. **体验更好** - 更好的错误信息、进度显示
4. **功能更强** - 配置验证、时间旅行、智能建议

这些优化将使 Neve 成为一个真正独立的、不依赖 nixpkgs 的、在设计哲学上超越 Nix 的现代系统配置语言。
