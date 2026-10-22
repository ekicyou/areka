# Brief: areka-P0-real-run-log-noise

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の実機の確かめ〔タスク 9.2〕で気付いた件。範囲の外の問題として、完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 9.2 の ⑥ と「参考」。行数は、その実機の確かめ（2026-10-10）の記録を数えたもの（記録の置き場 `target\` は git に残らないので、数はこの brief に書いてある）。どれも `seriko-trigger-intervals` より前から在る記録で、動きの誤りではない。区分 C（バグでない持ち越し）。

## Problem

- **開発者**: 実機の確かめで文字の層の記録を `debug` まで開けると、毎フレーム出る 2 種類の `debug!` が記録のほとんどを占める。判定に要る行（どの枝を通ったか）が埋もれて探しにくく、記録のファイルも大きくなる。実機の確かめは「判定の分岐の水準まで記録を開ける」のが決まりなので、文字の層を見る確かめのたびに同じことが起きる。
- **利用者・開発者**: 同梱の emo2 をそのまま起こすだけで、起動のたびに `WARN` が出る。同梱物が警告を出す状態だと、作者の誤りを知らせる本物の警告が目立たない。

## Current State

2026-10-10 時点（`seriko-trigger-intervals` のブランチ・コミット `84ca70e6`・debug 版の `areka.exe`・記録の絞りは `RUST_LOG=info,areka_seriko=debug,areka_emo_text=debug`）。

1. **毎フレームの `debug!` 2 種類**（文字の層・スコープごと）。
   - `crates/areka-emo-text/src/region.rs` の `resolve_origin_component`:「未指定の origin 成分を書字開始角へ寄せる」。バルーンの定義が `origin.x`・`origin.y` を書いていないと、成分ごとに 1 行。同じ関数のコメントに「本関数は毎フレーム呼ばれる」と在る。
   - `crates/areka-emo-text/src/actor_attach.rs` の再追従の口:「文字層の再追従: 適用 k・面実寸・文字描画領域がいずれも同値のため再結線・再生成を行わない」。何も変わっていないフレームで、スコープごとに 1 行。
   - 実測: スコープ 2 つ（`\0`・`\1`）で、1 フレームに 6 行（スコープごとに `origin.x`・`origin.y`・再追従）。
     - 検体つきの走行（約 51 秒）: 全 32,194 行のうち 30,724 行（「未指定の origin」20,484・「文字層の再追従」10,240）＝約 95%。
     - 別の検体つきの走行（約 32 秒）: 全 21,389 行のうち 20,446 行。
     - 同梱の emo2 そのままの走行（約 46 秒）: 全 42,845 行のうち 32,008 行。
   - 再追従の口には、作り直したときだけ出る `info!` が別に在る。毎フレーム出るのは「何も変わらなかった」側の `debug!`。origin の側は、解決の関数そのものが毎フレーム呼ばれる。
2. **同梱の emo2 が起動のたびに出す `WARN`（絵の焼き込み）**: `areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） … rel_path="purple/a/null.png" original_w=382 original_h=547` が 2 件（同じ絵について、1 回の起動のうちに約 0.2〜0.35 秒あけて 2 回。同梱の emo2 そのままの走行でも同じ）。
   - 出どころは `crates/areka-emo-atlas/src/lib.rs` の焼き込みの入口の「0 寸／全透明の element の早期の警告」。作者の誤りの見込みが高い、という理由で `warn!` にしてある。
   - `purple/a/null.png` は α を持たない PNG で、左上の色を抜き色として抜くと全部が透明になる（`crates/areka-emo-atlas/src/emo2_e2e.rs` の説明）。名前のとおり「何も着けない」ための絵と読めるが、作者の意図は確かめていない。
3. **本 spec に入れないもの**: 起動時のもう 1 件の `WARN`（`areka_emo_text::actor::attach: 折返し基準が描画範囲の外に解決された … balloon="emo2-kakukaku" … wrap_threshold=254.0 inline_limit=240.0`）は、`emo-text-canon-residue` の項目 14（折返し基準が描画範囲の外に解決されるバルーン定義の扱い）の件。同 spec は roadmap で「据え置き・取り下げ予定」。ここでは扱わない（下の Constraints に、落とさないための一言を置いた）。
4. 有界の自動終了つきの走行では、終わり際に `WARN` がさらに出る（`kanade: 強制終了指示`・`MCP: areka が終了の途中で答えなかった`）。これは確かめの道具の側が起こしているもので、ここでは扱わない。

## Desired Outcome

- 文字の層の記録を `debug` で開けた実機の確かめで、「何も変わらなかった」だけの行が毎フレーム出ない。状態が変わった行と判定の行は今までどおり読める。
- 同梱の emo2 をそのまま起こしたとき、焼き込みの `WARN` が 0 件になる。または、出すと決めた理由が記録に残る。
- 作者の誤り（本当に空の絵・0 寸の絵）を知らせる警告は弱めない。

## Approach

要件で選ぶ。

- **毎フレームの 2 種類**: ⑴ 状態が変わったときだけ出す（前と同じ結果なら出さない。前の結果を覚える場所が要る）／⑵ `trace!` へ下げる／⑶ 今のまま。⑵ は 1 行ずつの直しで済むが、`trace` まで開けた確かめでは同じ量が出る。⑴ は「同値だった」ことを 1 回だけ知らせる形にできる。実機の確かめの記録の検索語として使われていないかを先に引く。
- **`null.png` の `WARN`**: ⑴ 同梱の検体の側を直す（絵を α つきの透明な PNG に替えても、全部が透明なら同じ警告が出るので、結局は警告の条件の側を見ることになる）／⑵ 「全部が透明な element」を、参照のされ方（着せ替えの「何も着けない」選択肢として置かれている、など）で作者の意図と見分けられるなら、見分けて水準を下げる／⑶ 今のまま（作者が意図して空の絵を置くのは稀、と決める）。見分けの材料が焼き込みの入口に届いているかをコードで確かめてから選ぶ。
- 直した後、同じ手順の実機の走行で行数を数え直して記録する。

## Scope

- **In**: `region.rs` と `actor_attach.rs` の毎フレームの `debug!` 2 種類の出し方。焼き込みの入口の全透明の警告の条件（同梱の emo2 の `null.png` で出る件）。直した後の行数の確かめ。記録の文言を固定しているテストの追随。
- **Out**: 折返し基準の `WARN`（`emo-text-canon-residue` の項目 14）。文字の描画範囲が潰れたときの記録の洪水（`balloon-text-bugs` の件 2）。背景のスレッドの記録がテストの捕捉に届かない件（`actor-thread-log-capture`）。seriko の記録（`talk` の開始と終わりは `seriko-trigger-intervals` が `debug!` に留めた）。確かめの道具が終わり際に起こす `WARN`。記録の仕組みそのもの（`LogSink`・MCP の `get_log`）。

## Boundary Candidates

- 「何も変わらなかった」を記録する水準と回数（文字の層の再追従・origin の解決）。
- 「作者の誤りの見込み」の警告の条件（`areka-emo-atlas` の焼き込みの入口）。

## Out of Boundary

- 再追従の判定そのもの（同値なら作り直さない＝変えない）。origin の解決の結果（変えない）。
- 焼き込みの結果（索引表・配置・トリムの矩形＝変えない。変えるのは記録だけ）。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: 文字の層の記録を開けて回す実機の確かめすべて。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**（読んで、境界を確かめた）:
  - `balloon-text-bugs` の件 2: 文字の描画範囲が 0 以下に潰れたとき、供給面を作る失敗が毎フレーム `WARN` 1 行と `ERROR` 2 行を出し続ける件。あちらは失敗の経路で、直しは「0 寸なら作らない」。こちらは正常の経路の `debug!`。触るファイルも別（あちらは `actor_present.rs`）。
  - `actor-thread-log-capture`: テストの記録の捕捉を子のスレッドへ届ける件（`areka-actor`）。記録の量や水準は変えない。重なり無し。
  - 文字とバルーンの列（`crates/areka-emo-text/src/actor*.rs`）の spec とは、`actor_attach.rs` を分け合う＝同じ時に走らせない。

## Constraints

- 段は**その他**・区分 C。先に要るものは無い。
- ログの無い失敗の経路を作らない（下げる・減らすのは正常の経路の記録だけ）。検査は表示するだけでなく判定させる（直した後の行数は、数えて上限と比べる）。
- 触るファイルの行数は余裕がある（`region.rs` 486 行・`actor_attach.rs` 251 行・`areka-emo-atlas` の `lib.rs` 686 行）。テストは兄弟のファイルへ。
- 折返し基準の `WARN` は、持ち主の `emo-text-canon-residue` が取り下げ予定なので、同 spec を片付けるとき（`coverage-roadmap-refresh` と一緒）に「同梱のバルーンが起動のたびに警告を出す」ことが落ちないよう、要件の段で 1 行確かめる（本 spec へ入れるかは、そのとき開発者に聞く）。
- 規模の見込み: S（3〜5 タスク。`trace!` へ下げるだけなら 2〜3）。
- 要件定義のモデル: Opus。

## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）

- **前提の変化**: 無い。毎フレームの `debug!` 2 種類（`crates/areka-emo-text/src/region.rs`・`actor_attach.rs`）と焼き込みの警告（`crates/areka-emo-atlas/src/lib.rs`）は main に同じ形で在る。行数も同じ（486・251・686）。
- **触るファイル・規模・優先度の区分・要件定義のモデル**: 変わらない。
- **見つけた穴・古くなった記述**（ほかの spec との重なりの書き足し）:
  - Adjacent の「`balloon-text-bugs` の件 2 とは触るファイルも別」は条件つき。あちらは 0 寸を見分ける層の候補に `region.rs` を挙げている＝あちらが `region.rs` で止める設計を選ぶと重なる。
  - `balloon-font-file` とは `actor_attach.rs` が重なる（あちらは普通のバルーンを登録する所を触る）＝同じウェーブに置けない。`text-typesetting` の前半とは `region.rs` が重なる。
  - 検査を兄弟の新しいファイルに置くと、emo-text の `lib.rs` の一覧（今 76 本）の席を使う。席を使う spec（`range-choice-tag` など）と並べるなら、今ある検査ファイルへ足す。
  - 並べられる相手: `range-choice-tag`・`choice-ranges-one-function`・`font-size-keywords`・`choice-marker-styling`（重なり 0）。

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）

- **前提の変化**: `balloon-text-bugs` が着地した（10-16）。あちらは潰れた描画範囲の警告を、毎フレーム呼ばれる `TextRegion::resolve`（`crates/areka-emo-text/src/region.rs`）から外し、登録の口（`actor_attach.rs` の `note_region_collapse`）で「潰れた」「戻った」の変わり目に 1 回だけ出す形にした。本 spec の「状態が変わったときだけ出す」案は、この先例と同じ形で作れる（origin を書いていないことの記録を、登録の口の隣の「範囲外ゆえ無視した origin」の警告と同じ場所へ移す）。
  - 毎フレームの `debug!` 2 種類は main に同じ形で残っている（`region.rs` の `resolve_origin_component` の「未指定の origin 成分を書字開始角へ寄せる」・`actor_attach.rs` の `refresh_actor_binding` の「同値のため再結線・再生成を行わない」）。焼き込みの警告（`crates/areka-emo-atlas/src/lib.rs` の全透明の element）も同じ。
  - 上の棚卸㉔の「`balloon-text-bugs` の件 2 が `region.rs` で止める設計を選ぶと重なる」は解けた（あちらは着地済み）。
- **触るファイル**（行数）: `crates/areka-emo-text/src/region.rs`（495）・`actor_attach.rs`（310）・origin の記録を登録の口へ移すなら `actor_decoration.rs`・`crates/areka-emo-atlas/src/lib.rs`（686）。
- **新しいファイルと依存**: 検査を今ある `region_tests.rs`・`actor_region_warn_tests.rs`（登録の口の警告の回数を数えている）と `areka-emo-atlas` の `lib.rs` の中の検査へ足せば、emo-text の `lib.rs` の一覧の席を使わない。依存の追加は無い。
- **規模**: 3〜5 タスク（`trace!` へ下げるだけなら 2〜3）。
- **人の手・机**: 人の手の確かめは要らない。行数は決定論の検査（記録を捕まえて数え、上限と比べる）で固定できる。直した後の数え直しは、有界の自動終了つきの短い実走（約 50 秒・記録を絞り込んで数える）で AI が回せる。静かな机・前後の全体テストの比べ・長い負荷の走行は要らない。
- **先に要るもの**: 無い。v0.0.3 の 2 本とは触るファイルが重ならない。同じウェーブに置けない相手: `region.rs` を触る `bottom-row-text-clip`（v0.1.0）と `text-typesetting` の前半、`actor_attach.rs` を触る `balloon-font-file`。`range-choice-tag`・`choice-ranges-one-function`・`font-size-keywords` とは重なり 0。
- **優先度の区分**: C のまま（バグでない持ち越し）。
- **要件定義のモデル**: Opus のまま。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - 同じ `TextRegion::resolve` の中に、毎フレーム出うる `debug!` がほかに 2 つある（`region.rs` の `resolve_or` の「未指定座標成分を既定辺へ縮退する」＝validrect を書いていないとき、`resolve_origin_component` の「宣言された origin 成分が文字を描いてよい範囲の外にある」）。実測の記録に出ていないのは同梱のバルーンが書いているから。同じ形の洪水になるので、一緒に扱うかを要件で決める。
  - 上の棚卸㉔の行数（486・251）は古い。
  - `crates/areka-emo-atlas/src/lib.rs` の 164〜166 行目と 167〜169 行目に、同じ 3 行の説明が 2 回続いている（その場で直せる）。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 10-16 の後、触るファイルに変更は 0（`crates/areka-emo-text/src/region.rs` 495 行・`actor_attach.rs` 310 行・`actor_decoration.rs` 208 行は同じ。`crates/areka-emo-atlas/src/lib.rs` は 686 → 683 行）。毎フレームの `debug!` 2 種類（`region.rs` 479 行の「未指定の origin 成分を書字開始角へ寄せる」・`actor_attach.rs` 274 行の「文字層の再追従 … 同値のため」）と、焼き込みの全透明の警告（emo-atlas の `lib.rs` 184 行）はそのまま在る。
- 触るファイル: 上の 3〜4 つと、記録の文言を固定している検査 `crates/areka-emo-text/tests/attach_wiring_test.rs`（788 行・749 行が「文字層の再追従」の記録を**ちょうど 1 回**と数える＝出し方を変えたら直す）・`region_tests.rs`・`actor_region_warn_tests.rs`。記録の文言は道具（`tools/`）と `doc/` には使われていない（引いて 0 件）。
- 規模: 3〜5 タスク（`trace!` へ下げるだけなら 2〜3）。
- 先に要るもの: 無し。
- v0.1.0 との重なり: 0。v0.1.0 の brief に `region.rs`・`actor_attach.rs`・emo-atlas の `lib.rs` を触ると書くものは無い（`placement-measure-bake-once` は焼き込みの中身を範囲の外に置く・`offline-surface-dump` が触るのは emo-atlas の `decode/wic_arm.rs`）。`emo2-real-run-stability` が待つ記録 `wrap=BudouxWordWrap` は `actor_present.rs` の `info!` で、この spec が変える `debug!` とは別。
- 1 本だけの席: 取らない（今ある検査へ足せば emo-text の `lib.rs` の一覧に触らない・依存の追加なし）。
- 人の待ち・机: 要らない。直した後の数え直しは、有界の自動終了つきの短い実走（約 50 秒）を AI が回す。静かな机・前後の全体テストの比べ・長い負荷の走行は要らない。
- 版の席: 版に縛らない列。上限 5 は小さく、v0.1.0 のウェーブの空き席に入れられる見込み（そのウェーブの v0.1.0 の一番大きい spec の下限が 5 以上なら）。`choice-ranges-one-function` とも重なり 0。
- 優先度の区分: C（バグでない持ち越し）。
- 要件定義のモデル: Opus。
- 分割の案: 切らない。
- 見つけた穴・古くなった記述:
  - 棚卸㉖の「emo-atlas の `lib.rs` の 164〜166 行目と 167〜169 行目に同じ説明が 2 回」は、もう直っている（今は 164〜166 行目に 1 回だけ）。
  - v0.1.0 の後に取る見直しの spec（`audit-emo-text-layout-draw` が `region*`、`audit-emo-text-state-actor` が `actor*`、`audit-emo-atlas` が emo-atlas の全部を持つ）と同じファイル＝この spec を v0.1.0 の間に取れば先に済む。後に回すなら同じウェーブに置かない。
