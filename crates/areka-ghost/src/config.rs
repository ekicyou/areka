//! 運行設定（`KanadeConfig`）の値源解決。
//!
//! shell descript からシェル名を読み取り、読取不能・欠落時はディレクトリ名へ
//! 安全側にフォールバックする純関数と、areka 固有のベースウェア定数を提供する
//! （design.md「ghost::config」）。task 2.3 で実装する。
