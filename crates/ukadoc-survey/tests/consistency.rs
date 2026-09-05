//! 常時走る整合検査の入口（要件 6.1）。
//!
//! ワークスペースの標準テスト実行（`cargo test`）にそのまま乗る一群である。ネットワーク
//! にも実機にも触れず、ファイルを 1 つも作らず、一時ディレクトリも使わない。読むのは
//! repo に置かれたカタログ・台帳 4 本・テーマ定義・ソース・ドメイン別報告 4 本だけで、
//! スナップショットには構造として届かない（要件 6.2。理由は [`consistency`] の冒頭）。
//!
//! ここは入口なので**テストの本体は 1 つも置かない**（`structure.md:129`）。読み込みは
//! `consistency/mod.rs` にあり、テストの本体はその兄弟——実データへの主張は
//! `consistency/checks.rs`、対象が 0 件でないことの主張は `consistency/non_vacuity.rs`、
//! 自前の道具の較正は `consistency/values_md.rs`、記入例が実在することの主張は
//! `consistency/examples.rs` にある。`consistency/perturb.rs` は `checks.rs` の摂動が
//! 使う道具だけを持ち、テストは 1 つも持たない。

#[cfg(test)]
#[path = "consistency/mod.rs"]
mod consistency;
