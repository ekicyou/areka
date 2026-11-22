# 設計の検証: dola-cue-test-builder

- 対象: `.kiro/specs/completed/dola-cue-test-builder/design.md`（コミット `1287f1206`）
- 照らした文書: `requirements.md`・`research.md`・`spec.json`（`language: ja`・`phase: design-generated`）・steering（`product.md`・`tech.md`・`structure.md`）
- 進め方: `kiro-validate-design`（Analysis → Critical Issues → Strengths → GO/NO-GO）。対話は行わず、議題は設計の討議へ渡す。

## 設計の検証の要約

設計は、ワークスペースに既にある `test-support` 機能の前例（`areka-placement`・`areka-input`・`areka-runtime`）と同じ形を `dola` に足し、5 つの写しを再輸出・`use`・1 行の関数で取り次ぐだけの小さな変更で、要件 1〜5 の受け入れ基準をすべて受け持つ先が表で示されている。設計の主張をコードに当てて確かめた範囲（下の「コードでの確かめ」）では、呼び手 22 ファイルと本体の名の経路が無変更で組め、`Cargo.lock` も変わらない見込みで、実装を止める誤りは見つからなかった。

## コードでの確かめ（読みと grep のみ・ビルドは回していない）

1. **runtime の再輸出で呼び手が壊れない**: `consumer_ledger_test_support.rs` は `consumer_ledger.rs:390-393` の `#[cfg(any(test, feature = "test-support"))] pub mod test_support;` の下にある。`pub use dola::cue::test_support::carrier_cue as actor_carrier_cue;` は同じ名・同じ引数の並び（`&str, &str, &[&str]`）の関数を持ち込むので、同じモジュールの `pub fn carrier_cue(name, args)` の本文 `actor_carrier_cue("0", name, args)` も、本体の `areka_runtime::consumer_ledger::test_support::carrier_cue`（`install_cue_tests.rs:18` ほか 5 ファイル）も、そのまま組める。再輸出の別名は `actor_carrier_cue` だけを持ち込むので、同じモジュールの `carrier_cue` とはぶつからない。
2. **runtime の cfg(test) のビルドが dola の機能を見る**: 根の `Cargo.toml` は `resolver = "2"`。`cargo test -p areka-runtime` では自分の dev の依存の機能が本番の依存の `dola` に合わさるので、`[dev-dependencies]` の `dola = { path = "../dola", features = ["test-support"] }` で開く。本体のテストでは runtime は非テストの lib として `test-support` 付きで組まれ、dev の依存は効かないので、`[features] test-support` への `"dola/test-support"` の前送りが要る。設計の「2 つの開き方の両方が要る」は正しい。同じ依存を本番と dev の両方に書く形は、runtime の `areka-placement`・`shiori-host32-host`・`crash-report-kit` に前例がある。runtime の `test-support` を本番の依存で付ける crate は無い（本体・`areka-input` とも `[dev-dependencies]`）。
3. **外す取り込みが正しい**: `frame_window_link_drain_tests.rs`（`frame.rs:365` の `cfg(all(test, target_pointer_width = "64"))` の下）の `ActorKey`・`CueCommand` は `carrier` の本文（31-38 行）でしか使われず、`TalkCue`（`move_scope1` の戻り）と `CueSink` は残る。ghost `prop_sink.rs` の `mod tests` の `ActorKey`・`CueCommand`・`TalkCue` は `non_carrier_cue_is_skipped_no_write`（264-266 行）が使い続ける。ghost `prop_get_sink_tests.rs` の取り込みは 165-168 行の `CueCommand::Text` の合図が使う。seriko `actor_test_support.rs` の `CueCommand`・`TalkCue` は `emote_cue`・`noncanonical_custom_cue` が使う。runtime `consumer_ledger_test_support.rs` の `ActorKey`・`CueCommand`・`TalkCue` は `broken_custom_cue` と `carrier_cue` の戻りが使う。設計の「残る／外す」の記述はどれも合っている。
4. **seriko のグロブ**: 呼び手 5 ファイルはどれも `use super::test_support::*;`（`actor_parts_tests.rs` 以外は `use super::*;` も）で引く。`actor.rs` にも seriko の他のどこにも `carrier_cue` の名は無いので、`pub(super) use` の再輸出でグロブ同士のあいまい（E0659）は起きない。
5. **型が同じ**: ghost と seriko が使う `areka_sakura::contract::{ActorKey, CueCommand, TalkCue}` は `dola::cue` の再輸出（`contract.rs:59-60`）なので、`dola` の組み立ての戻りをそのまま渡せる。
6. **`Cargo.lock`**: ghost は既に dev で、seriko・runtime は既に本番で `dola` に依存しているので、足すのは機能だけで、依存の並びは変わらない見込み。
7. **13 ファイル**: File Structure Plan の数（dola 5・ghost 3・seriko 2・runtime 3）は要件 5.1 の許す範囲に収まる。`#[path = "test_support_tests.rs"]` を `cue/test_support.rs` に置く形は、`command.rs:304` の `#[path = "command_tests.rs"]` と同じで、同じフォルダの兄弟に解ける。
8. **lint**: `dola`・使い手 3 crate の `lib.rs` に `deny(missing_docs)` などの crate 全体の lint は無く、ワークスペースの `[lints]` も無い。

## 重大な指摘（≤3）

なし。

実装を止める、または要件を外す誤りは見つからなかった。次の 3 件は軽い指摘で、設計の討議で一言確かめれば足りる（どれもコードの形は変わらない）。

- **軽 1（数の食い違い）**: 「Revalidation Triggers」の「写しの取り次ぎ 5 か所（再輸出 3・1 行の関数 1・`use` 1）」と、Requirements Traceability の 3.1 の「再輸出 2・`use` 2・1 行の関数 1」が食い違う。数え直すと、再輸出は seriko の `pub(super) use` と runtime の `pub use … as actor_carrier_cue` の 2、`use` は ghost `prop_sink.rs` と runtime `frame_window_link_drain_tests.rs`（`as carrier`・私有なので再輸出ではない）の 2、1 行の関数は ghost `prop_get_sink_tests.rs` の 1 で、3.1 の行が正しい。Revalidation の行を 3.1 にそろえる。
- **軽 2（README の言い方）**: README の下書きは「helpers that build cues for tests」と複数で書くが、置くのは `carrier_cue` 1 つ。今後足す余地を残す言い方なら今のままでよく、1 つだけを言うなら「a helper」にする（議題 5 の中で決める）。
- **軽 3（doc の例）**: `carrier_cue` の `///` に doc テストの例を付けないこと。`cargo test -p dola` の doc テストは lib を `cfg(test)` 無し・機能無しで組むので、`dola::cue::test_support` が見えず赤になる。設計の Service Interface は例を持たないので今のままで足りるが、実装の注意として「Implementation Notes」に一言あるとよい。

## 設計の強み

1. **前例どおりの小さな形**: `test-support` 機能・`cfg(any(test, feature = "test-support"))` の `test_support` モジュール・dev の依存に機能を足すだけ、というワークスペースの既にある型をそのまま使い、新しい crate も `Cargo.lock` の変更も持ち込まない。本文は `dola` の 1 式だけになり、写しの位置には名だけが残るので、正準形を変えるときに直す所が 1 つになる（要件 1.1・3.1・5.3）。
2. **機能が開く経路と確かめ方が具体的**: どのビルドで組み立てが組まれ、どのビルドで組まれないかを図にし、`--workspace` では機能が合わさって書き忘れが隠れるので crate ごとに単独で回す、という確かめの肝を押さえている。`cargo tree -p areka -e normal,features -i dola`・`tools/crates-io.ps1`・`git diff --exit-code Cargo.lock`・`--list` の数の比べ、と受け入れ基準ごとに機械で見られる確かめを当てている（要件 2.3・2.4・2.6・4.2）。

## 最終の判断

- **判断: GO**
- **理由**: 既存の前例に沿った形で要件 1〜5 をすべて受け持ち、コードに当てて確かめた範囲で呼び手 22 ファイル・本体の名の経路・取り込みの整理・`Cargo.lock` に誤りが無い。残るのは文書の数のそろえと言い方の軽い指摘だけで、実装の形は変わらない。
- **次の段**:
  1. 設計の討議（`/kiro-design-discussion dola-cue-test-builder`）で、設計の 8 つの議題と上の軽 1〜3 を確かめる。
  2. 承認の後、`/kiro-spec-tasks dola-cue-test-builder` でタスクを作る。

## 確かめていないこと

- ビルド・テストは回していない（読みと grep のみ）。`Cargo.lock` が変わらないことと `tools/crates-io.ps1` が通ることは、実装の後の確かめに任せる。
