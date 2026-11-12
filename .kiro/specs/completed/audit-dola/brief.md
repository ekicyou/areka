# Brief: audit-dola

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **dola（絶対時刻の台本の再生）** を受け持つ。

## Current State

- 受け持つファイル: `crates/dola/` 全部。
- 大きさ: 109 ファイル・コード 8,778 行・テスト 15,932 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
- まだ audit をかけていない。指摘はギャップ分析で洗い出す（起票のときに出さなかったのは、着手までにコードが変わって指摘が古びるため）。

## Desired Outcome

- 受け持つ範囲に `/ponytail:ponytail-audit` をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。
- 直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。
- 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑。直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Approach

`/kiro-start` の各段で、次のように進める（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

- **要件**: 「受け持つ範囲の audit の指摘を、3 つの組とも全部直す」という形だけを書く。指摘の中身はまだ書かない。
- **ギャップ分析**: 受け持つ範囲だけを対象にして `/ponytail:ponytail-audit` をかける。指摘の一覧はギャップ分析の報告に残す。番号は audit の番号をそのまま使う。audit の作法のうち 2 つは外す。
  - 「最大 20 件」は外す。全件を出し、省いた件を残さない。
  - 「読まなかった所（Not checked）」は残さない。範囲のファイルは全部読む。
- **要件の討議**: 指摘の一覧を要件に落として、直す対象を確定する。直さずに残す 3 種類（「Constraints」）は、ここで議題にする。
- **設計**: 指摘ごとの直し方を決める（何を消すか・どこにまとめるか・範囲の外の呼び出し元への影響）。直し方は ponytail の「いちばん小さな完全な変更」に従う。消せるものは消し、呼び出し元・テスト・fixture まで直し切る。
- **タスク・実装**: 直す。最後に同じ範囲へ audit をかけ直して、0 件を確かめる（検証の段のタスク）。

## Scope

- **In**: 受け持つファイルの指摘の直し・そのためのテストの追加と直し・直しのために要る範囲の外の呼び出し元の直し（下の「Constraints」の約束に従う）。
- **Out**: 範囲の外の見直し（ほかの 32 本の持ち物）・新しい機能・`vendors/`・clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。

## Boundary Candidates

- 指摘の組（必ず直す・直したほうがよい・できれば）ごと、またはファイルのまとまりごとにタスクを切る。

## Out of Boundary

- ほかの 32 本の範囲のファイル（直しが及ぶときの約束は「Constraints」）。
- 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。

## Upstream / Downstream

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝なし（すぐ着手できる）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `clippy-199-lints`（同じクレートに触る）。同じ範囲に触る未着手の spec は、着手のときに roadmap の spec 台帳で確かめる。

## Constraints

- **範囲の外へ直しが及ぶとき**: 関数の形を変えたら、ツリー全体で呼び出し元を探して全部直す。相手の範囲の見直しの spec が走っていれば、そのセッションと申し合わせてから触る。
- **直さずに残してよいのは次の 3 つだけ**。どれも要件の討議で開発者の裁定を仰ぐ。
  1. 開発者の裁定に反する直し。
  2. わざと置いた先送りの印。`#[allow(dead_code)]` に実在の理由の doc を付けた語彙の口（実装規律「語彙完備・配線ゼロの追跡」）と、`shortcut:`・`ponytail:` の注釈で限界が今の負荷を越えていないものがこれに当たる。
  3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。ukadoc で確かめてから決める。
- **守る決まり**:
  - 1 ファイル 1,000 行の番人を越えない。足すなら新しいファイルへ。
  - 決定論のテストで網羅する。
  - 記録の無い失敗の道を作らない（`error!`＋`Err`）。
  - 終了の道は正規の実装にする。
  - 実機の根と一時フォルダは、ワークツリーの `target` の下に置く。
  - GPU のテストは実 GPU で回す。
  - 文字化けをコンソールの文字コードで逃げない。
  - 依存を足すなら `tech.md` へ登記して、開発者の承認を得る。
- 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
- 1 spec 20 タスクを越えそうなら、要件の討議で、指摘の組ごとに同じ範囲の 2 本目として切ってよい。

## audit-parsers-sakura の申し送り（2026-11-06 同 spec のタスク 8.1 の `/kiro-discovery` で追記）

小さい 2 件は新しい spec にせず、同じ dola を直す本 spec へ「件 N」で足した（触る場所が同じものを 1 本にまとめる決まり）。どちらも区分 B 相当の持ち越し。範囲のファイルは本文の範囲に入っている。

- **件 1: トークの終わりの時刻が ms の格子から 1 ulp 外れうる**。`audit-parsers-sakura` は台本の開始時刻を整数 ms で積み、命令ごとに 1 回だけ秒へ直す形にした（同 spec の design.md「開始時刻の積み（8）」）。けれど終わりの時刻は dola が f64 の和で出す＝`crates/dola/src/cue/sheet.rs` の `absolute_end_time`（85〜91 行目・`cue.start_time + cue.duration` の max）と、canonical 変換が焼き込む占有の終わり（`crates/dola/src/cue/schedule.rs` 69 行目の `max(start_time + duration)`）。150 ms＋800 ms が 0.9500000000000001 になり、Tick の式 `(now-base) as f64 / 1000.0` と比べて `TalkDone` と後始末が 1 刻み遅れうる（同 spec の design.md「残る危うさ」・research.md 12 節で測った）。直し方（終わりを ms の格子で出す・合図の長さを整数 ms で持つ、など）は要件の段で選ぶ。呼び手は `crates/areka-sakura/src/compile.rs` の `append_epilogue`（`sheet.absolute_end_time()`）。
- **件 2: `CuePlayer::stop` を呼ぶ本番のコードが 0**。`crates/dola/src/cue/runtime.rs` 361 行目の `stop` は、`audit-parsers-sakura` の指摘 78 の直しの後、呼ぶのが dola のテストだけになった（同 spec の tasks.md の Implementation Notes「8.1 への申し送り（6.1 のレビュー）」）。消すか、語彙の口として理由の doc を付けて残すかを、この spec の見直しで決める（消す前にツリー全体の参照を確かめる＝本文の Constraints）。

## audit-ghost-catalog の申し送り（2026-11-06 同 spec の完了の棚卸で追記）

- **件 3: 今の clippy で `-D warnings` が赤**。`cargo clippy -p dola --all-targets --no-deps -- -D warnings` が lib で 21 件・lib test で 23 件を出す（`collapsible_if` が `crates/dola/src/validate/rules.rs`・`validate/mod.rs` ほかに多数、`too_many_arguments`〔`runtime/instance_manager/mod.rs` 71 行目ほか〕、`manual_range_contains`、`doc_lazy_continuation`〔`runtime/conflict_resolver.rs` 42 行目〕）。振る舞いを変えずに直すか、決めて許す。起票の訳: 持ち主のこの spec の受け持ちの範囲と重なる。

## 2026-11-06 棚卸㉟の再測定（main `18e4cb30`）

- **前提の変化**: C17 で `crates/dola/` に触ったのは `audit-parsers-sakura` だけ（`src/cue/schedule.rs` の `notify_barrier_resolved` で集めた束を空にする 3 行と説明 1 行）。件 1〜3 はそれぞれの申し送りの節のとおり。
- **触るファイル**: `crates/dola/` の 109 ファイル・合わせて 24,944 行（うちテスト 18,237）。大きいのは `tests/cue/schedule_test.rs` 960・`src/cue/command_tests.rs` 830＝1,000 行に近い本番のファイルは無い。dola を使う外の所は、本体 `crates/areka` の 54 ファイル（多くは `emo2_boot` のテスト）・`areka-sakura`・`areka-emo-text` の `sink.rs`／`actor_lookahead*`・wintf の `ecs/dola/mod.rs`・`ecs/mod.rs`・`ecs/world/mod.rs`・`ecs/graphics/core.rs`。
- **規模**: 指摘の数で決まる。C17 の見直しは 18〜28 タスクになった＝20 を越えたら要件の討議で同じ範囲の 2 本目へ切る。
- **先に要るもの**: 無い。C18 では、dola の公開の形を変えて外の呼び手を直す直しは取らない（本体 `crates/areka/` は `placement-crate-split`、`areka-emo-text` の `sink*`・`actor*` は `audit-emo-text-state-actor`、wintf の `ecs/mod.rs`・`ecs/world/mod.rs` は `wintf-visual-tree` が触る）。件 1 は dola の中で閉じる直し方（終わりを ms の格子で出す）なら並べてよい。合図の長さを整数 ms で持つ形にするなら呼び手が多いので、相手の着地の後に回す。
- **優先度の区分**: A（開発者 10-19「とにかく指摘事項は全部直す方針です」）。件 1〜3 は B 相当の持ち越し。
- **要件定義のモデル**: Opus（受け持つ範囲の見直しで、新しい境目も骨組みの分かれ道も無い）。
- **分割の案**: 今は無し（上の規模のとおり）。
- **見つけた穴・古くなった記述**: 件 2 の「呼ぶのは dola のテストだけ」は違う＝本体の `crates/areka/src/emo2_boot/frame/window_state_tests.rs:496` も `CuePlayer::stop` を呼ぶ。消すならこのテストも直す（C18 では本体に触らない約束＝消すのは相手の着地の後か、理由の doc を付けて残す）。件 1 の「`schedule.rs` 69 行目の `max(start_time + duration)`」は欄の説明で、焼き込みの計算は `src/cue/sheet.rs` の `to_talk_schedule`（説明は 154 行目）。Current State の「コード 8,778 行・テスト 15,932 行」は上の数に読み替える。
