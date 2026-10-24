# Brief: real-run-log-noise

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

## どっとさくらで見えた事実（2026-10-23 `areka-P0-dot-sakura-specimen` の実機の 1 回）

- 同梱バルーン `bottle` を使うと、`cursor.blendmethod ROP 系は M1 未対応` の WARN が**毎フレーム**出た。2 分 12 秒で 20,764 行、記録の 98% を占めた。
- `crates/areka-emo-text/src/choice.rs` の注記は「`cursor.blendmethod` が ROP 系（`none` 以外）→ warn-once（解決時 1 回）」。解決が毎フレーム走り、1 回だけ出すはずの警告が毎回出ている見込み（areka の側）。`mergepennot` を描けないこと自体は決めた縮退で、直すのは警告の回数だけ。
- 根拠: `.kiro/specs/completed/areka-P0-dot-sakura-specimen/verification.md` の 7 節と 8 節の件 2。
- 開発者（2026-10-23）: 「次のリリースまでに治るように」。この spec の版の段は「版に縛らない」のまま（roadmap はこのセッションでは触っていない）＝次の棚卸で v0.1.0 に入れるかを決める。

## 2026-10-23 `pasta-in-windows-verification` からの申し送り（タスク 5.3・開発者 「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」）

- ぱすたの走行で見つかった `apply(ShowSurface): 表示・マスクを更新` の INFO 行の量（`crates/areka-emo-present/src/presenter/show.rs:402`・記録の 9 割）は、開発者の決めで本 spec へまとめず、v0.1.0 の新しい spec `animated-surface-log-flood` が持つ（`verification.md` 5 節の 21）。本 spec の範囲は変わらない。

## 2026-10-23 棚卸㉙の追記
- **棚卸㉙で本 spec を v0.1.0 に入れた**。10-23 に足した件（どっとさくらのバルーン `bottle` で毎フレーム出る `cursor.blendmethod` の警告）に、開発者の言葉「次のリリースまでに治るように」が付いたため。`shiori2-protocol`・`seriko-pattern-calls` と同じ決まり（開発者が「次のリリースまでに」と言った件を持つ spec は、その版の 1 本にする）。
- もう「版に縛らない列」の埋め草ではない。席が空いたときに挟む扱いはやめ、v0.1.0 のウェーブの 1 本として並べる。優先度の区分は A（10-23 の件）。
- 範囲・規模（4〜7）・触るファイル・同じウェーブに置けない相手（`choice.rs` を直す形なら `vertical-choice-hover-band`）は、すぐ上の再測定のまま。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: `vertical-choice-hover-band`（#375）が着地し、`crates/areka-emo-text/src/choice.rs` が 748 → 766 行になった。選択肢の飾りを決める `ResolvedChoiceStyle::resolve` は 651 行目、`cursor.blendmethod` の警告は 664 行目、`cursor.style` の下線の警告は 674 行目、「warn-once（解決時 1 回）」の説明は 640〜645 行目へ動いた。描くたびに呼ぶ道（`crates/areka-emo-text/src/draw.rs` 283 行目）と登録の口（`actor_decoration.rs` 155 行目）はそのまま。ほかの触るファイルは動いていない（`region.rs` 495・`actor_attach.rs` 310・`actor_decoration.rs` 208・emo-atlas の `lib.rs` 683。毎フレームの `debug!` は `region.rs` 479 行目と `actor_attach.rs` 274 行目、同じ関数の中の毎フレーム出うる `debug!` は `region.rs` 432 行目と 471 行目、焼き込みの全透明の警告は emo-atlas の `lib.rs` 184 行目）。
- **触るファイル**: 上の 4 つ・`choice.rs`（766）か `draw.rs`（761）・検査 `crates/areka-emo-text/tests/attach_wiring_test.rs`（788・749 行目が「文字層の再追従」をちょうど 1 回と数える）・`region_tests.rs`（520）・`actor_region_warn_tests.rs`（894＝上限に近い）。新しいファイルは足さない見込み。
- **規模**: 4〜7 のまま。
- **先に要るもの**: 働きの上で無し。相手の入れ替わり: `vertical-choice-hover-band` は着地した＝`choice.rs` を直す形でも重ならなくなった。代わりに v0.1.0 の `emo-text-dwrite-fidelity` が `draw.rs` を触る＝描くたびの警告を `draw.rs` 283 行目の呼び手で止めると重なる。`resolve` から警告を外して登録の口（`actor_decoration.rs`）で 1 回だけ出す形にすれば `draw.rs` に触らず 0。`balloon-scroll-arrows` とは、あちらが印の登録を `actor_decoration.rs` に置くときだけ重なる（あちらは `choice.rs` に触らない約束）。`animated-surface-log-flood` とは emo-atlas の `lib.rs` で重ならない（あちらの件 25 の直しはアトラスの展開でなく重ね直しの側・あちらが emo-atlas の `lib.rs` に触らない約束）。版の外: `balloon-font-file`（`actor_attach.rs`）・`text-typesetting` の前半（`region.rs`）・`emo-text-line-gap`（`choice.rs`）。1 本だけの席・依存の追加は無い。静かな机・前後の全体テストの比べは要らない。直した後の数え直しは有界の自動終了つきの短い実走（約 50 秒・同梱の emo2 と、どっとさくらのバルーン `bottle`）を AI が回す＝その数分だけ排他テストの机を借りる。
- **優先度の区分**: A（10-23 の件に開発者「次のリリースまでに治るように」・棚卸㉙で v0.1.0 に入れた）。
- **要件定義のモデル**: Opus のまま（先例＝`actor_attach.rs` の `note_region_collapse`〔変わり目に 1 回だけ出す〕）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**（前の再測定のうち今も正しいもの）: ⑴ 同じ `TextRegion::resolve` の中の毎フレーム出うる `debug!` 2 つ（`region.rs` 432 行目「未指定座標成分を既定辺へ縮退する」・471 行目「宣言された origin 成分が … 範囲の外にある」）を一緒に扱うかを要件で決める。⑵ `choice.rs` 640〜645 行目の説明（「warn-once（解決時 1 回）」）は今の呼ばれ方と合わない＝直すのはこの spec の仕事（説明だけ先に直すと事実と合わなくなる）。⑶ Constraints の行数（486・251・686）は古い（495・310・683）。⑷ v0.1.0 の後の見直し（`audit-emo-text-layout-draw`・`audit-emo-text-state-actor`・`audit-emo-atlas`）と同じファイル＝この spec を v0.1.0 の間に取れば先に済む。⑸ 記録の文言は道具（`tools/`）と `doc/` には使われていない（棚卸㉘で引いて 0 件）。
