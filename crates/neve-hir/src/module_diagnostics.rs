//! Internal diagnostics storage for module loading.
//! 模块加载内部诊断存储。

use neve_diagnostic::Diagnostic;

/// Compatibility-layer diagnostic storage.
///
/// This intentionally preserves the current flat diagnostic behavior while
/// providing a dedicated seam for future module/file attribution work.
/// 兼容层诊断存储。
///
/// 当前仍保持扁平诊断行为，但为后续模块/文件归属拆分提供独立边界。
#[derive(Debug, Clone, Default)]
pub(crate) struct ModuleDiagnostics {
    diagnostics: Vec<Diagnostic>,
}

impl ModuleDiagnostics {
    /// Borrow all currently collected diagnostics.
    /// 借用当前已收集的所有诊断。
    pub(crate) fn as_slice(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Remove and return all currently collected diagnostics.
    /// 取出并返回当前所有已收集的诊断。
    pub(crate) fn take(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Record a single diagnostic.
    /// 记录单条诊断。
    pub(crate) fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Record multiple diagnostics while preserving order.
    /// 按原顺序记录多条诊断。
    pub(crate) fn extend<I>(&mut self, diagnostics: I)
    where
        I: IntoIterator<Item = Diagnostic>,
    {
        self.diagnostics.extend(diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neve_common::{BytePos, Span};
    use neve_diagnostic::DiagnosticKind;

    #[test]
    fn diagnostics_store_preserves_order_and_take() {
        let mut store = ModuleDiagnostics::default();
        store.push(Diagnostic::error(
            DiagnosticKind::Module,
            Span::new(BytePos(1), BytePos(2)),
            "first",
        ));
        store.extend([Diagnostic::error(
            DiagnosticKind::Module,
            Span::new(BytePos(3), BytePos(4)),
            "second",
        )]);

        assert_eq!(store.as_slice().len(), 2);
        assert!(store.as_slice()[0].message.contains("first"));
        assert!(store.as_slice()[1].message.contains("second"));

        let taken = store.take();
        assert_eq!(taken.len(), 2);
        assert!(store.as_slice().is_empty());
    }
}
