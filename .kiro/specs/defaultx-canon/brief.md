# Brief: defaultx-canon

> 2026-11-15 `/kiro-discovery` で起票（`audit-placement-core` のタスク 8.2 の完了の申し送りから・同 spec の `research.md` 18.10「ukadoc と食い違う所」と 19.2 の起票の候補）。開発者の承認は同じ日（「推奨どおりで」）。開発者の方針＝意味論は ukadoc から輸入する・ukadoc で読めない所は SSP で測る（SSP は参考で、正しいとは限らない）・未対応は範囲の外でも全部起票する。行の番号は起票のとき（枝 `claude/kiro-start-audit-placement-core-61b071` の `8f1a8394f`＝`audit-placement-core` の着地の前）の実測＝着手のときに `main` で測り直す。

## Problem

areka は `sakura.defaultx`（`kero.defaultx`・`char*.defaultx`）を「鎖の前の窓（本体は作業領域の右端）から左へ空ける量」と読んでいる。ukadoc は、画面の X 座標と書いている。ゴーストの作者が ukadoc のとおりに `defaultx` を書くと、areka は違う所へ窓を出す。

ukadoc（2.9.13 の版）の書き方:

- `sakura.defaultx`（SHIORI リソース https://ukatech.shillest.net/ukadoc-spec/list_shiori_resource.html#sakura.defaultx ）: 「本体側(\0)の既定位置のX座標。サーフェスの基準点(通常はサーフェスの中央下)をこの位置に置く。」値はプライマリモニタの左上を原点とするスクリーン座標（マルチモニタでは仮想デスクトップ全体での位置）。負の値はプライマリモニタの作業領域の右端からの距離で、`-0` は右端そのもの [2.9.13]。負の座標そのものは `--100` のように `-` を 2 つ重ねる（X 座標 −100）[2.9.13]。同じ軸に `sakura.defaultleft` もあれば `defaultx` が優先。ゴースト・シェルの descript.txt にも書ける [2.9.13]・SHIORI リソースのほうが優先。デスクトップの左端／右端に沿う設定のときは X がその設定で決まるので無効。
- descript の `sakura.defaultx`（https://ukatech.shillest.net/ukadoc-spec/descript_ghost.html#sakura.defaultx ）: 基準点をこの位置に置く文に [2.9.13] が付く。書式と意味は SHIORI リソースと同じ。優先は SHIORI リソース＞シェルの descript＞ゴーストの descript。
- `kero.defaultx`（https://ukatech.shillest.net/ukadoc-spec/list_shiori_resource.html#kero.defaultx ）・`char*.defaultx`（https://ukatech.shillest.net/ukadoc-spec/list_shiori_resource.html#char-star-.defaultx ）: 「sakura.defaultxと同様」。指定しなければ「本体側の左隣」「1つ前のキャラクターの左隣」。
- `sakura.defaultleft`（https://ukatech.shillest.net/ukadoc-spec/descript_ghost.html#sakura.defaultleft ）: 「キャラクターウィンドウの左端をこの位置に置く。」負の値は右端からの余白 [2.9.13]。同じ descript に `defaultx` もあれば `defaultx` が優先。
- Y の軸（`sakura.defaulty`・`sakura.defaulttop`・https://ukatech.shillest.net/ukadoc-spec/list_shiori_resource.html#sakura.defaulty ）も同じ形（負は作業領域の下端からの距離・`-0` は下端・`--` の重ね書き・上端／下端に沿う設定のときは無効）。

## Current State

- **今の読みの持ち主**: 完了した `areka-P0-window-placement` 要件 2.10（「`alignmenttodesktop` が `bottom` のとき `defaultx` を work area 右端からの左方向オフセットとして解釈する（de-facto。`defaultx=0`＝右端に密着）」）と、完了した `areka-P0-scope-chain-gap` 要件 2.8（「`defaultx` の意味論を変更しない」）。`areka-P0-window-placement` の `research.md`（「`defaultx` は ukadoc に実在する」の節）は、ukadoc の意味が基準点のアンカー指定で `defaultleft` とは別のキーだと知ったうえで、要件の討議（2026-07-10）の de-facto の読みを採り、正典の意味を「転記のみの将来シーム」として残した。同じ spec の要件 2.7 は `defaultx`⇔`defaultleft`・`defaulty`⇔`defaulttop` を同じ枠へ寛容に受ける（`crates/areka-placement/src/config.rs:227-229`・同じ層の中で `defaultx` ＞ `defaultleft`）。
- **SSP で確かめた範囲**: `areka-P0-scope-chain-gap` の brief は今の読みを「別途 SSP 確認済み」と書く。けれども同 spec の `ssp-oracle-notes.md`（23 行・101 行）で SSP を測ったのは emo2 の `sakura.defaultx,0`・`kero.defaultx,0` だけで、本体が作業領域の右端に付くことを見た。0 以外の値は測っていない。ukadoc の読みでは `0` は画面の X 座標 0（左端）、右端は `-0` なので、`0` で右端に付いた SSP の振る舞いは ukadoc の字面と合わない（`0` を「指定なし」と見たのか、SSP 2.9.13 で意味が変わったのか）。2.9.13 の印は、負の値・`--`・descript に書けることの文に付く。いつ・どう変わったかは、この spec で ukadoc の履歴（`ukatech/ukadoc-spec` の git の履歴）と SSP の更新履歴で確かめる。
- **今の読みの上に立つもの**（`audit-placement-core` で決めた鎖の決まり・`crates/areka-placement/src/chain_rule.rs`・同 spec の `research.md` 18.10「この仕様で行く」の 2）: 組（鎖の前の窓, n）の隙間は n の `defaultx`（下端・上端だけ・Free と辺は 0・DPI で換算しない）。resolver は `x(n) = x(鎖の前の窓) − w(n) − 隙間` で置く。この読みを使う道:
  - 既定の並び（`resolver.rs` の `resolve_placement`・`joins_chain`・`chain_gap`）。鎖の頭（スコープ 0）は右の辺から `defaultx` だけ左（18.10 の T41）。
  - 起動の確定（`chain_rule.rs` の `boot_chain`・`chain_finalize.rs` の `boot_default_decision`・`JoinLeftOffset`）。
  - 隣り合いの印の保存（`persist_save.rs`・左の隙間が `defaultx` なら `left`）。
  - 拡大率の後の詰め直し（`chain_realign.rs`）。
  - 本体の `runtime/frame/drain_resnap.rs`（`boot_chain` を呼ぶ 1 行）と、追随の `follow/work_area_clamp.rs` の `place_chain_in_work_area`（`JoinLeftOffset` を解く）。
  - 記憶の戻し（`persist.rs` の `restore_and_judge`）。`ChainRule::from_terms` で全スコープの揃え方から `default_x: None` の鎖を組み、`follow/work_area_clamp.rs` の `keep_restored_groups` が `ChainView::Saved` の `prev`・`joins` だけを読む（隙間は読まない）。申し送り（audit-placement-follow-residue より・2026-11-17）: 読みを変えるときはこの道も見直す（`audit-placement-follow-residue` の design.md の RestoreGroup・決め 1）。
- **今の読みの上に立つ裁定**: `audit-placement-core` の 485（起動の確定で `defaultx` の隙間を残す）・494 ⑵⑶（左の組は `defaultx` の隙間も隣り合い・覚えた `left` は `defaultx` の隙間で付ける）・513（大きく負の `defaultx` でも左の組）・514（±1・±2 を 0 に丸めない）。18.10 の C2・C3・C4・C11・C12・C14 の整理と、表の T9〜T13・T15・T41。読みを ukadoc に合わせると、これらは全部開け直しになる（たとえば `defaultx` が画面の座標なら、組の「隙間」という考えそのものが無くなる）。
- **材料**: `audit-placement-core` の `research.md` 18.10 の「根拠（ukadoc）」U1〜U7（とくに U3＝`sakura.defaultx` の「表示位置の記憶が無いとき」「X座標はその設定で決まるため無効」・U4＝`kero.defaultx`／`char*.defaultx` の左隣）、「ukadoc と食い違う所」、「SSP で測るなら」S1〜S4（S2 が `kero.defaultx,36` で X 座標か本体からの量かを分ける・S1・S3・S4 は鎖の決まりの裏付け）。
- **SHIORI リソース**: ukadoc は SHIORI リソースの `defaultx` を descript より先に置く。areka が SHIORI リソースの `sakura.defaultx` を配置に使っているかは、起票のときは確かめていない（`crates/areka-sylphya/src/vocab/shiori_resource.rs:57-65` に語の一覧はある）。

## Desired Outcome

- `defaultx`・`defaultleft`（と Y の軸の `defaulty`・`defaulttop`）の読みが ukadoc に合っている: 画面の座標（プライマリモニタの左上が原点・仮想デスクトップ）・負の値は作業領域の右端（下端）からの距離・`-0` は端・`--` は負の座標そのもの・`defaultx` は基準点（通常は中央下）・`defaultleft` は窓の左端・同じ軸では `defaultx` が優先・優先の順は SHIORI リソース＞シェルの descript＞ゴーストの descript・辺に沿う設定のときは無効・指定の無いスコープは前のキャラクターの左隣。
- 鎖の決まり（`chain_rule.rs`）の隙間の読み ⑵ と、それを使う道（既定の並び・起動の確定・印の保存・詰め直し）と、485・494・513・514 の裁定が、新しい読みに合わせて決め直されている。決め直した仕様が `chain_rule.rs` の説明と `doc/COMPAT_ARCHITECTURE.md` に書かれている。
- ukadoc の読みに合わない今までのゴースト（emo2 の `defaultx,0` ほか）がどう出るかが分かっていて、変わるなら開発者が承認している。
- 読みの変化には、それが壊れたら落ちる決定論のテストがある（鎖の決まりの表の T9〜T15・T41 の形の行を、新しい読みで引き直す）。
- 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑。emo2 の実走で本体と相方の位置が今までと同じか、変わるなら変わった位置で開発者が承認している。

## Approach

- 要件の段の最初に、ukadoc の履歴と SSP の更新履歴で、2.9.13 で何が変わったかを確かめる（URL と原文の短い引用を残す）。ukadoc で読めない所（`0` の扱い・2.9.13 より前のゴーストとの互換）だけを、議題にする前に SSP で測る（S2 を第一に。測りは参考にとどめる）。SSP の `ssp_shiori_log.txt` と MCP の `get_property`（`currentghost.scope(ID).x`）が使える。
- 読みを ukadoc に合わせたとき、`defaultx` を書いていないスコープ（多くのゴースト）は鎖（前のキャラクターの左隣・隙間 0）のままで、`defaultx` を書いたスコープだけが画面の座標になるはず。この形で、鎖の決まりの ⑵ が「隙間」から「`defaultx` のあるスコープは鎖の外（位置で置く）」へ変わるかを設計で決める。今の Free の x あり（T4＝位置で置き、次のスコープの基準になる）の扱いが手本になる。
- 記憶の印（`left`／`right`／`none`）の保存の形は変えないのが第一（変えると sylphya の読みと今までの記憶が全部読み替えになる）。

## Scope

- **In**: `defaultx`・`defaultleft`・`defaulty`・`defaulttop` の読み（`config.rs` の読み取り・`resolver.rs` の解き）、SHIORI リソースの同じ語の扱い、鎖の決まり ⑵ とそれを使う 4 つの道と追随の `place_chain_in_work_area` の `JoinLeftOffset`、485・494・513・514 の裁定の決め直し、`doc/COMPAT_ARCHITECTURE.md` の判断の表、そのためのテストの追加と直し、SSP の測り（参考）。
- **Out**: 鎖の決まりのうち `defaultx` に関わらない所（辺のスコープ・鎖の前の窓・起動の見方・520 の ⑷・残す限り 1〜8 のうち `defaultx` に関わらないもの）・記憶の印の語の追加・バルーンの位置・`windowposition.limit`。

## Boundary Candidates

- 読み取り（キーと値の書式・優先の順）・既定の並び（resolver）・鎖の道（起動の確定・保存・詰め直し）の 3 つの塊。読み取りは純粋な関数で切れる。

## Out of Boundary

- 開発者の裁定を、討議なしに覆す直し。485・494・513・514 と `areka-P0-window-placement` 要件 2.10・`areka-P0-scope-chain-gap` 要件 2.8 は、要件の討議で開発者に開け直しを仰いでから変える。

## Upstream / Downstream

- **Upstream**: `audit-placement-core`（鎖の決まり `chain_rule.rs` を置いた・着地を待つ）・`audit-placement-follow-residue`（同じ `chain_rule.rs` と追随の鎖の寄せを通る＝先に着地させ、同じウェーブに並べない）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: 完了した `areka-P0-window-placement`（要件 2.7・2.10）・`areka-P0-scope-chain-gap`（要件 2.8・7）・`dpi-realign-remembered-chain`（隣り合いの印）・`audit-placement-core`（鎖の決まり）。
- **Adjacent**: `audit-placement-follow-residue`（`chain_rule.rs`・`follow/work_area_clamp.rs`＝直列）・`descript-window-keys` の系の descript の読み取り（`config.rs`）・`sylphya-sqlite-kvs`（記憶の保存の形・触らないのが第一）。

## Constraints

- **意味論は ukadoc から**。ukadoc が沈黙している所だけ、完了した spec の決めと開発者の裁定に従う。SSP の振る舞いは参考で、正しいとは限らない。測る前に SSP を動かすことを開発者に断る（reload を打つのは毎回許可を取る）。
- **今までのゴーストの見え方が変わる直し**は、要件の討議で開発者の承認を得てから。emo2（`sakura.defaultx,0`・`kero.defaultx,0`）が今と同じ位置に出るかを、最初に確かめる。
- 1 ファイル 1,000 行の番人を越えない・決定論のテストで網羅する・記録の無い失敗の道を作らない（読めない値は `warn!`）・実機の根と一時フォルダはワークツリーの `target` の下。
- **規模の見立て**: M〜L（12〜20 タスク）。読みの変化が鎖の道の全部に及ぶので、要件の討議で 20 を越えそうなら、読み取りと既定の並びを 1 本目、鎖の道の決め直しを 2 本目に切ってよい。
- **要件定義のモデル**: Opus（今ある配置の決まりの決め直しで、新しい境界や公開の契約は引かない）。
- **優先度の区分**: B（ukadoc と食い違うバグ）。
