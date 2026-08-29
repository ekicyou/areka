# 実機サインオフの試行記録（task 10.1・2026-08-28）——**未完了・設計の欠陥 2 件を検出**

判定器は 1 度も回していない。手順どおりに進めようとして、**手順では到達できない**ことが
2 段階で判明したためである。以下は「なぜ実施できなかったか」の記録であり、合否ではない。

## 環境（要件 8.1 は満たしている）

- DISPLAY1（主）: mon=L0,T0,R2880,B1800  work=L0,T0,R2880,B1704  **dpi=192**
- DISPLAY2      : mon=L-2560,T195,R0,B1795  work=同  **dpi=144**
- 実効 DPI は Per-Monitor v2 aware なプロセスから読んだ（要件 8.1 の但し書き）。
- areka: `target/debug/areka.exe`（本ブランチのビルド）
- ゴースト: `crates/pilot/examples/shiori-host-32/fixtures/emo2`
- バルーン: `crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-offsetdpi`（本作業で新設）
- 観測: `RUST_LOG=wintf::transition=debug` を点灯。行頭タグ `[transition]` は 31 行 → 最終 54 行採れた。

## 検出 1（手順の穴・本作業で是正済み）——手順を満たせる検体が 1 つも無かった

手順書 §3 の必須手は **両方**を要求する——手 2「素の追従スコープ（キーワード指定でない
バルーン）を最低 1 つ」／手 3・4「キーワード指定のバルーン」。ところが既存の検体は

- `fixtures/emo2/emo2-kakukaku`     : 両スコープとも**数値指定**（キーワードが無い）
- `fixtures/emo2-kakukaku-wplimit`  : 両スコープとも**キーワード**（素の追従が無い）

で、**どちらでも手順を満たせない**。手順書は「キーワード指定のバルーンを」と書くだけで
どの検体を使うかを言っていなかった。混成の検体 `emo2-kakukaku-offsetdpi`
（scope 0 = `center` / scope 1 = 数値 −190）を新設して塞いだ。

## 検出 2（設計の欠陥・**未是正・開発者の裁定が要る**）

### 2a. ゴーストを合成マウス入力で掴めない＝手順の手 5 が実行できない

areka のゴースト窓は 4 つとも `WS_EX_TRANSPARENT` が立ったままで、カーソルを立ち絵の
上へ運んでも外れない（5 水準 × 3 列の 15 点で確認。`WindowFromPoint` は 1 点も areka を
返さず、非透過へ切り替わる窓も 0 個）。ゆえに**ドラッグでモニタ間を往復させられない**。

代替として `SetWindowPos` で char 窓を外から DISPLAY2 へ移した（利用者のドラッグ経路は
通らないので、これ自体はサインオフの代用にならない）。窓は dpi=144 へ移り、**バルーンは
追従して一緒に移った**（char L=-1600 → balloon L=-1566・相対 x=34 を維持）。追随の相も
走った（下記のとおり門の行が 1 行増えた）。

### 2b. **判定器の起点レコードが、要件 8.2 が指示する操作では 1 行も出ない**

これが本命の欠陥である。

- design.md:606 は判定器の切り出しを「**`kind=monitor` を起点に**」と定め、
  design.md:594 は「`kind=monitor` と同一時系列に並ぶため、判定器は遷移の切り出しに
  既存の起点をそのまま使える」と述べる。task 8.1 はそのとおり実装した。
- ところが `kind=monitor` を出すのは `crates/wintf/src/ecs/layout/systems/monitor_systems.rs`
  の `detect_display_change_system` であり、**Monitor エンティティ自身の DPI／作業領域が
  変わったとき**——すなわち**表示設定の変更**——にしか出ない。
- 要件 8.2 が指示する操作は「**ゴーストをモニタ間で往復させる**」であり、これは
  **窓の** DPI が変わる（`Changed<DPI>`）だけで、モニタ表は 1 つも変わらない。

実測: 外部移動で char 窓は 192 → 144 へ移り追随の相も走ったのに、ログの
`kind=monitor` は **0 行**のままだった（`kind=offset` は 3 行）。

**帰結**: 手順どおりに実機を回しても、判定器は遷移を 1 本も切り出せない。
`split_transitions` が最初の起点より前を捨てるため `kind=offset` 行は全て判定の外へ落ち、
「往復が 1 度も観測されていない」「低い拡大率側で追随が出ていない」「揃えを 1 度も
測れていない」が**製品の欠陥でないのに**立つ。**手順の並べ替えでは塞げない構造的な穴**である。

## 採れたログ（`real-run-attempt-2026-08-28.log`・抜粋）

```
[transition] frame=1     kind=offset scope=1 base_dpi=192 new_dpi=192 base_offset=292,-150 old_offset=292,-150 new_offset=292,-150 verdict=unchanged
[transition] frame=1     kind=offset scope=0 base_dpi=192 new_dpi=0   base_offset=34,-448  old_offset=34,-448  new_offset=34,-448  verdict=keyword-pending
[transition] frame=28688 kind=offset scope=0 base_dpi=192 new_dpi=0   base_offset=34,-448  old_offset=34,-448  new_offset=34,-448  verdict=keyword-pending
```

いずれも起点より前にあり判定の外（手順書 §6.3 の限界 5 がまさにこの形を予告していた）。

## 裁定が要る点（開発者へ）

1. **判定器の起点をどう採るか。** ⑴ `kind=monitor` のほかに「窓の DPI が変わった」を表す
   起点レコードを新設する ⑵ 実機手順を「モニタ間の移動」ではなく「**表示設定で拡大率を
   変える**」へ改める（`kind=monitor` は出るが、要件 8.2 の字面「ゴーストをモニタ間で
   往復させる」と食い違う） ⑶ 判定器を起点非依存の切り出しへ作り直す——のいずれか。
2. **実機でゴーストを掴めない件**（2a）。合成入力の限界なのか製品の欠陥なのかは
   本記録では切り分けていない。人の手でのドラッグなら掴める可能性がある。

いずれも本仕様の要件 8 の充足条件そのものに関わるため、実装側で勝手に決めない。
