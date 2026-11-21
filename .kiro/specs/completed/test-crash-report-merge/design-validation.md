# 設計レビュー: test-crash-report-merge

- 対象: `design.md`（2026-11-21・`ab65d6fd3`）・`requirements.md`（承認済み）・`research.md`
- 手順: `kiro-validate-design`（Analysis → Critical Issues → Strengths → GO/NO-GO）・対話なし（議題は設計の討議へ回す）

## Design Review Summary

設計は承認済みの決定 1〜8 にそのまま沿い、「本体 1 つ＋各テストの実行ファイルの根に据え付けの静的変数 1 つ」という、`areka-input` と本体で既に動いている形を wintf・areka-placement・kit 自身へ当てるだけの小さい直しになっている。主な主張はコードで確かめ、すべて正しかった（下の「確かめた事実」）。食い違いは 1 件で、ほかに議題が 1 件ある。どちらも文書の直しか数行の置き場の違いで済み、実装の道を変えるものではない。

## 確かめた事実（コードとの照合）

| 設計の主張 | 確かめ | 結果 |
|---|---|---|
| runtime の写しは wintf・placement の写しの上位集合 | コメントと空行を除いて 3 本を `diff`。wintf と placement は一致（345 行）。runtime（419 行）は `installed`・`pub`・持ち手の表（`HOLDERS`・`MISSED`・`hold`・`release`・報告の `AREKA-CRASH-HOLDER` の 3 形）・`#[cfg(test)]` 2 か所（`RaiseException` 110 行・`INSTALL` 127〜130 行）・較正の `GpuPermit`／`GraphicsCore` を足しただけ | 正しい |
| `.CRT$XCU` の `#[used]` の静的変数は各テストの実行ファイルの根の crate に置く | `crates/areka-input/src/crash_report_tests.rs`・`crates/areka/src/runtime_tests/mod.rs:11-12` が、静的変数を自分の crate に置いて `areka_runtime::crash_report_test_support::install` を指している。runtime の `INSTALL` は `#[cfg(test)]` なので、依存先として組まれる回には入らない＝1 つの実行ファイルに静的変数は 1 つ | 正しい |
| optional＋dev の二重の依存を cargo が許す | `crates/areka-runtime/Cargo.toml` に `shiori-host32-host` が `[dependencies]`（`optional = true`・`test-support = ["dep:shiori-host32-host", …]`）と `[dev-dependencies]` の両方で載り、今も組めている | 正しい（前例どおり） |
| kit のテストの実行ファイルには kit 自身の据え付けが要る | kit のテストの実行ファイルの根は kit なので、`lib_tests.rs` の `#[cfg(test)]` の静的変数がそれに当たる（research 8.3 R3 も同旨） | 正しい |
| 根の `Cargo.toml` の members が新しい crate を含む | `members = ["crates/*"]` | 正しい・根は触らなくてよい |
| `windows = { workspace = true }` だけで組める | ワークスペースの表に `Win32_System_Com`・`Win32_System_LibraryLoader`・`Win32_System_Threading` がある。`Win32_Foundation` は表に無いが、windows 0.62.2 の `Win32 = ["Win32_Foundation"]` から付く。placement も今 `workspace = true` だけで同じ本文を組めている | 正しい |
| `log-capture-kit` の見張りを壊さない | `with_default_guard_test.rs` の依存の向きの検査は `KIT_PACKAGE`（`log-capture-kit`）だけを見る。runtime の `[dependencies]` に `crash-report-kit` が載っても赤にならない | 正しい |
| `-kit` の crate が windows-rs の API の数えでテストの道具に入る | `q9-api-count/src/count.rs` の `is_test_tool` に `krate.ends_with("-kit")` がある | 正しい |
| `THIRD-PARTY-NOTICES.md` にワークスペースの member が載る | `log-capture-kit 0.1.0`（3065 行）・`temp-path-kit 0.1.0`（3077 行）が載っている | 正しい・1 行増える |
| `code_only` の 2 本は同じ働き | placement の `zorder_pair_deferred::code_only`（423〜429 行）と runtime の `source_text_test_support::code_only`（3〜10 行）は同じ本文。runtime は `test-support` でも自分のテストでも placement の `test-support` を付けている | 正しい・`pub use` で届く |
| `window_move_lines` の移し | 私有の写しは 169〜176 行。呼び手は 7 か所（`only_window_move_line` の中を含む）。`CapturedEvent::message(&self) -> &str`。`diag::WINDOW_MOVE_RECORD_TAG` は crate の中から読める | 正しい |
| 申し送りウの形 | 前例 `frame_work_area_resnap_hold_tests.rs:375-405` が `capture(|| catch_unwind(…))` で数えてから戻している。placement の的の 1 本（148〜153 行）は `should_panic(expected = "DpiSyncHold")`＋debug だけ | 正しい・前例どおり組める |
| 古い道は 21 ファイル 44 行・すべてコメント | `git grep -nE '(^|[^_a-z])runtime::' -- crates/areka-placement` で `areka_runtime::` を除くと 44 行・21 ファイル。コードの行は 0 | 正しい |
| `spawn.rs` は 350 行だけ | 当たりは 350 行の 1 件（ファイルは 788 行・末尾は `audit-areka-input-menu` の持ち場） | 正しい・行は離れている |
| 同じウェーブの約束のファイルに触らない | `frame_test_support.rs`・`crates/areka-input/`・`runtime_tests/mod.rs`・`file_length_guard_test.rs` は、どれも設計の Modified Files に無い。`window-resetwindowpos-hold` の触るファイル（`frame/move_hold.rs` ほか）と `session-module-gather` の触るファイル（本体の crate だけ）とも重ならない | 正しい（下の Critical Issue 1 の 1 行を除く） |

## Critical Issues

🔴 **Critical Issue 1**: 「差分を作らないファイル」の表と古い道の書き換えが食い違っている（`crates/areka-placement/src/lib.rs`）
**Concern**: 表は `crates/areka-placement/src/lib.rs` を差分 0 の的に挙げる。一方で Modified Files の「古い道の説明 21 ファイル 44 行」にはこの `lib.rs` が入っていて、実測でも 4 行ある（114・138・181・187 行の `runtime::frame::run_dpi_phase`・`runtime::frame::work_area_sync`・`runtime::wire_runtime`・`runtime::frame`）。
**Impact**: 書いたとおりに実装すると、要件 6.2 の直しが 4 行漏れるか、完了の記録の「差分 0」の確かめが赤になる。どちらでも検証の記録（6.7・8.2）が崩れる。
**Suggestion**: 表の行を「wintf・runtime の `lib.rs` は差分 0。placement の `lib.rs` は `mod test_crash_report;` の行と 85 行の説明は差分 0、差分は古い道のコメント 4 行（114・138・181・187）だけ」に直す。`session-module-gather` の brief（119 行）は、この spec が両方の `lib.rs` に触る前提で「触らない」と約束しているので、相手との衝突は無い。
**Traceability**: 6.2・6.7・8.2
**Evidence**: design.md「差分を作らないファイル（確かめの的）」の表の 5 行目／「Modified Files」の古い道の説明 21 ファイル 44 行の項

🔴 **Critical Issue 2**: `code_only` の 1 つの定義を、Z 順の専用のモジュールに置いている
**Concern**: 設計は、runtime と本体の汎用の下拵え `areka_runtime::source_text_test_support::code_only` の正本を、`areka_placement::test_support::zorder_pair_deferred::code_only` の再公開にする。このモジュールは「Z 順のペアの先送りの語彙」のためのもので（`test_support.rs:379-384`）、名前と中身が合わない。そのため、Z 順の語彙を片づける将来の直し（モジュールを消す・移す）で、runtime・本体・`audit-areka-app` の寄せ先が黙って巻き込まれる。設計はこの危うさを Revalidation Triggers の 5 番目に自分で書いている。
**Impact**: 定義を 1 つにすること自体（6.3）は満たす。ただ、依存を名前から読めない道に置くので、再検証の引き金が増える。
**Suggestion**: 定義を `areka_placement::test_support::code_only`（`test_support` の上の段）へ上げ、`zorder_pair_deferred` には `pub use super::code_only;` の 1 行を残す（placement の呼び手は差分 0）。runtime は `pub use areka_placement::test_support::code_only;` にする。増えるのは数行で、`log-capture-kit` を引かない規律も保てる。今の仮定のまま進めるなら、その理由を設計の討議で確かめる。
**Traceability**: 6.3
**Evidence**: design.md「PlacementSupportMerge」の Service Interface／「Revalidation Triggers」の 5 番目

## 軽微な注記（GO の妨げではない）

- 檻 4 本（MTA・最初の受け手・1 行の溜め・据え付け）は今、3 つの実行ファイルでそれぞれ走っている。直した後は、MTA・最初の受け手・1 行の溜めの 3 本が kit の実行ファイルでだけ走る（wintf・placement・runtime に残るのは据え付けの確かめだけ）。本体は 1 つで、要件 5.1〜5.4 もこの配置なので受け入れてよい。設計の Testing Strategy に「据え付けの関数が同じなので、MTA の檻は kit の 1 本で足りる」と 1 句あると、読む人が迷わない。
- 申し送りウで「0 件」の 3 本が空虚でなくなるのは、数える側（`#[cfg_attr(not(debug_assertions), ignore)]`）が走る debug の走行だけ。今の全体テストは debug なので実害は無い。
- 鳴る側のもう 1 本 `the_keep_position_resize_of_a_waiting_balloon_trips_the_watch`（174〜176 行）は今のままになる。檻は 1 本で足りるので、直さなくてよい。

## Design Strengths

- **入口の名を保つ再公開**: `pub use crash_report_kit::{hold, install, installed, release};` にすることで、`areka-input`・本体・`spine_wait.rs` と説明 5 か所が差分 0 のまま新しい本体を指す。並走の約束（要件 7.2）を、コードの形そのものが守っている。静的変数が指す関数も同じなので、呼び出しが 1 段増えるわけでもない。
- **既存の前例だけで組んでいる**: leaf の `-kit`（`temp-path-kit`）・optional＋dev の二重の依存（`shiori-host32-host`）・根の crate の据え付け（`areka-input`）・数えてから panic を戻す檻（`frame_work_area_resnap_hold_tests.rs`）。新しい仕組みも外の crate も無く、`Cargo.lock` と NOTICES の差分も前もって決まった形に収まる。

## Final Assessment

- **Decision**: **GO**
- **Rationale**: 依存の向き（wintf が上位に依存しない・kit は leaf）・据え付けの置き場・並走の約束・数えと NOTICES の扱いは、どれもコードで裏づけが取れた。残る 2 件は、表の 1 行の直しと、数行の置き場の選び直し（または仮定の確認）で済む。
- **Next Steps**:
  1. 設計の討議（`kiro-design-discussion`）で Critical Issue 1（表の直し）と Critical Issue 2（`code_only` の置き場）を、設計の Open Questions 1〜6 と一緒に決める。
  2. 決まった分を `design.md` に反映してから `/kiro-spec-tasks test-crash-report-merge` へ進む。
