# Brief: areka-P0-property-catalog-lists

> 起票: 2026-08-27（bvc 要件ディスカッション議題 4 の開発者指示による `/kiro-discovery` 再入・プロパティ系 3 spec 分割の 3 本目）
> **本 spec はプロパティ木のうち「ゴースト自身の状態」ではないもの**——OS/環境メトリクス・インストール済みカタログ・使用履歴——**を所有する。大半が areka 未保有の基盤（マルチゴースト運用・カタログ列挙・音再生）に依存するため、最も M2 色が濃い。**

## Problem

SSP プロパティ木の過半は `currentghost` の外にある——`system.*`（時計・CPU・メモリ・モニタ…）・`ghostlist`/`balloonlist` 等のインストール済みカタログ・`history`・`rateofuselist`。これらが無いと、環境依存の演出（メモリ残量トーク・モニタ寸法参照・隣のゴースト検出）を行う既存ゴースト資産が areka で動かない。ただし多くはカタログ列挙・多重ゴースト・音再生など **areka が M1 で持たない基盤**の上にしか立たない。

## Current State

2026-08-27 サーベイ（snapshot 2.8.80・詳細は `areka-P0-property-query-channels/brief.md` と同一調査）。

| 枝 | ≈項目数 | 依存する基盤 |
|---|---|---|
| `system.*` | 25 | OS メトリクス採取（clock ×8・`cpu.(キー)`・`memory.(キー)`・os/network/power・`disk.count`/`.index(ID)`・`monitor.count`/`.index(ID).{bpp,dpi,primary,rect,work}`・`cursor.pos`・`dnd.mode`・`theme.{app,os}.mode`）——**monitor 系は areka の DPI 追従基盤に実データあり＝最初に立てられる島** |
| `ghostlist` ×5・`activeghostlist` ×5＋`.ext` | 12 | インストール済みゴーストのカタログ列挙・多重ゴースト運用（M2） |
| `balloonlist` ×3・`headlinelist` ×2・`pluginlist` ×4＋`.ext` | 11 | 同カタログ＋HEADLINE/Plugin ホスティング（M2 予約） |
| `history.*` | 8 | balloon/ghost/headline/plugin × {(名), .index(ID)} の使用履歴の永続化 |
| `rateofuselist.*` | 24 | 12 葉 × {(名前), .index(順位)}——使用率統計の永続化 |
| 汎用プロパティ名（共有葉） | 17 | `GENERIC_PROP_NAMES`（sylphya 登記済み・17 で一致）が各カタログ根の下で乗算される |
| `currentghost.sound.*` ×3＋サウンド語彙族 | ≈21 | 音再生基盤（2.8.72/73・`playing`/`pause`/`position` は SET 有効）——**currentghost 配下だが基盤依存ゆえ本 spec 所有**（tree spec から明示的に切り出し） |
| `.ext.拡張プロパティ名`（逆方向） | — | ベースウェアが SHIORI/PLUGIN イベント `property.get`/`property.set` を発生（2.7.85）。名前は sylphya に予約済み（`vocab/dotted.rs:106-109`）・発火条件が activeghostlist/pluginlist に依存 |

- sylphya の `Selector::ByName`（`key.rs:140`）が `ghostlist(名前)` 型の構文を既にカバーしている（機構は待ち構えている・値が無いだけ）。

## Desired Outcome

依存基盤が存在する枝から順に実導出され、基盤が無い枝は**完全語彙＋縮退シーム（NotFound）＋依存基盤の明示**で登記されている——「黙って無い」枝が 1 つも残らない。

## Approach

島ごとの段階着地。第 1 の島＝`system.monitor.*`／`system.clock 系`（実データが既にある）。カタログ系・履歴系・音系は依存基盤の解禁ゲートとして登記し、基盤 spec が着地したとき just-in-time で実導出タスクを起こす（`status-execution-states` の台帳 spec 方式と同型）。

## Scope

- **In**:
  - `system.*` 25 項目の実導出（monitor/clock から着手・cpu/memory/os/network/power/disk は OS API 採取の設計込み）。
  - カタログ 5 根（ghostlist/activeghostlist/balloonlist/headlinelist/pluginlist）＋`history`＋`rateofuselist` の**完全語彙登記と縮退シーム**（実導出は依存基盤の解禁ゲート下・汎用 17 葉の乗算規則込み）。
  - `currentghost.sound.*`＋サウンド語彙族の登記（音再生基盤の解禁ゲート下・SET 3 葉の扱いは channels spec の台帳追随と整合させる）。
  - `.ext.*` 逆方向イベントの発火条件の登記（`property.get`/`property.set`・実装は activeghostlist/pluginlist の実導出と同時）。
- **Out**:
  - 照会経路（`areka-P0-property-query-channels`）・`currentghost.*` 本体（`areka-P0-currentghost-property-tree`）。
  - カタログ列挙・多重ゴースト・HEADLINE/Plugin・音再生の各基盤そのもの（M2 の各機能 spec）。

## Boundary Candidates

- 「実データが既にある島」（monitor/clock）と「基盤待ちの登記」（カタログ・履歴・音）の 2 相。
- `rateofuselist`/`history` は永続化層（areka.* persist の先例）と接続する独立スライス。

## Out of Boundary

- SSTP・FMO 経由の外部照会（M2 予約）。

## Upstream / Downstream

- **Upstream**: `areka-P0-property-query-channels`（照会の成立）・DPI 追従基盤（monitor 実データ）・sylphya。
- **Downstream**: 環境依存演出を使う既存ゴースト資産の互換・M2 の多重ゴースト/プラグイン運用。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-currentghost-property-tree`（sound 族の所有分界を本 brief どおりに保つこと）・`areka-P0-status-execution-states`（just-in-time 台帳方式の先例）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（プロパティ 3 spec の最後尾・島単位で前倒し可）。
- 正典参照はライブ ukadoc（snapshot のプロパティ節は 2.8.80——本 spec の範囲では balloon.scope 系ほどの既知逆転は無いが、設計前にライブ突合を必須とする）。
- 値の捏造禁止・決定論テスト必達（OS メトリクスは採取層を抽象して偽値注入で檻に入れる）。

---

> **📌 2026-09-02 棚卸⑫**——アンカー **ドリフト 0**（`key.rs:140`・`dotted.rs:106-109`・`GENERIC_PROP_NAMES` 17＝`:37-55`）。**今日 M2 基盤なしで実装できる島**（実測）: `system.monitor.*`（`count`・`index(ID).{dpi,primary,rect,work}`＝`wintf/ecs/window/monitor.rs` の `dpi: u32` :72・`is_primary`・`work_area`・`bounds` に全部ある・**`bpp` のみ実データなし＝別途 Win32 採取**）／`system.clock` 8 葉／`system.cursor.pos`／`system.{os,memory,cpu,power,disk}`／`system.{dnd.mode,theme.*}`＝いずれも Win32 採取層の新設のみで M2 サブシステムに依存しない（「M2 待ち」は採取層未作成の意味）。真に M2 基盤待ち＝`ghostlist`／`activeghostlist`／`balloonlist`／`headlinelist`／`pluginlist`／`history`／`rateofuselist`／`currentghost.sound.*`／`.ext.*` 逆方向。編成＝W14 裁定枠（channels→tree の後・publish のみなら前倒し可）。wintf 直近コミット（visual/draw・aabb）とは非交差（Monitor データは `ecs/window/monitor.rs`）。

