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


> **📌 2026-09-17 `areka-P0-sylphya-set-ledger` が **完了**（PR#151・`.kiro/specs/completed/areka-P0-sylphya-set-ledger/`）**——サウンドプロパティ名 18 葉（2.8.72 の 10 葉＋2.8.73 の `meta.` 8 葉）は `dotted.rs` の記録用の表 `SOUND_PROP_NAMES`（書き込みの仕分けは読まない・各要素に正典 URL）として登記済みで、その表の持ち主は `sylphya-set-ledger`（`doc/COMPAT_ARCHITECTURE.md` §8 の【所有の相互参照】行）。SET 有効の 3 葉 `pause`／`playing`／`position` は `SET_EFFECTIVE` へ末尾形で登記済み。本 spec に残るのは `currentghost.sound.*` の**値の導出**（音再生基盤の解禁ゲート下）で、In の「サウンド語彙族の登記」は済み。調査台帳のサウンド 18 行の宛先は本 spec のまま。アンカーのずれ: `dotted.rs:106-109` → `pub const EXT_EVENT_GET`／`EXT_EVENT_SET` の定義行（`.ext.*` の運搬は `areka-P0-property-ipc-transport`）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 低〜中。**そのままでは 20 タスクを超える**＝要件の段で ⒜ `system.*`（時計・モニタ・OS の値）と ⒝ 一覧（`ghostlist`・`balloonlist`）に切る。
- **崩れた前提**: 「一覧の列挙は M2 の基盤でまだ無い」は誤りになった＝`areka_ghost::catalog::{list_ghosts, list_balloons, list_shells}` が在る。`ghostlist`・`balloonlist` は今日出せる。`activeghostlist` は 1 体だけなので今のゴーストそのもの。音・履歴・利用率・プラグインは登記だけのまま。網羅台帳で本 spec が持つ行は 120（最多）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: 網羅台帳 `property.toml` で本 spec が持ち主の行は 120（最多）。全部では 25 を大きく超える＝**切る**。案: ⒜ `system.*`（時計・モニタ・カーソル・OS／メモリ／CPU／電源／ディスク・テーマ・ドラッグの様子）と Win32 の採り口（偽の値を差せる形）＝M〜L（14〜18）／⒝ 一覧（`ghostlist`・`balloonlist`・`activeghostlist`＝今出せる）と、基盤待ちの枝（`headlinelist`・`pluginlist`・`history`・`rateofuselist`・`currentghost.sound.*`・`.ext.*`）の語彙と縮退の登記＝M（10〜14）。順はどちらからでもよいが、⒜ の時計が「動く値の口」を要するので、`currentghost-property-tree` の ⒜ の後に置くのが素直。**brief は今分けてよい**（20 を大きく超え、⒜ と ⒝ は値の源が別）。
- 前提の状態: 読む道（`property-query-channels`・`mcp-get-property`）はまだ。出すことと決定論のテストは単独でできる。
- 崩れた前提／古くなった位置:
  - 一覧の源は在る: `crates/areka-ghost/src/catalog.rs` の `list_ghosts`・`list_balloons`・`list_shells`（前回どおり）。`activeghostlist` は 1 体だけ＝今のゴースト。
  - モニタの値は `crates/wintf/src/ecs/window/monitor.rs` の `Monitor`（`bounds`・`work_area`・`dpi`・`is_primary`）。`bpp` だけ源が無い（前回どおり）。
  - **動く値を出す口が sylphya に無い**（`SylphyaPublisher` は静的・SHIORI 照会・永続の 3 つだけ・`BackingLayer::SystemEnv` は縮退のまま）。時計（毎秒変わる）・カーソルの位置・メモリは「読む時に問い合わせる」口でないと出し直しが追いつかない＝`currentghost-property-tree` の議題と同じ口。
  - サウンドの 18 葉は `dotted.rs` の `SOUND_PROP_NAMES` に登記済み（`sylphya-set-ledger`）。本 spec に残るのは値の導出だけ（前回どおり）。
- 触るファイル（並走の照合用）:
  - `crates/areka-sylphya/src/{actor.rs, vocab/dotted.rs, key.rs}`
  - 新規 Win32 の採り口（例 `crates/areka/src/property/system_env.rs`）・`crates/wintf/src/ecs/window/monitor.rs`（読むだけ）
  - `crates/areka-ghost/src/catalog.rs`（読むだけ）・`crates/areka-ghost/src/sylphya_wiring.rs`
  - `crates/areka/src/emo2_boot/mod.rs`（883）
  - `doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: 動く値の出し方（`currentghost-property-tree` と一度で決める）。brief を今分けるか（上の案）。
- 見つけた穴: なし。並走の照合: `currentghost-property-tree` とは sylphya の口・`dotted.rs`・`emo2_boot/mod.rs` を分け合う＝直列（前回どおり）。`.ext.*` の運搬は `property-ipc-transport` と発火条件を分け合う。

## 2026-10-04 棚卸㉑で切った後の範囲

- 残した範囲: ⒝ 一覧の枝＝`ghostlist`・`balloonlist`・`activeghostlist` の値の導出（`areka_ghost::catalog` から）と、基盤待ちの枝（`headlinelist`・`pluginlist`・`history`・`rateofuselist`・`currentghost.sound.*`・`.ext.*` の発火条件）の完全な語彙と縮退の登記。
- 規模: M（10〜14 タスク）。
- 移した先: `system.*` と Win32 の採り口は新しい spec `areka-P0-system-property-values` へ（前提は `currentghost-property-tree` の動く値の口）。網羅台帳の `system.*` の行の持ち主は向こうが着地するときに直す。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（10〜14）のまま。切る: なし（一度切り出した spec）。
- 前提の状態: roadmap の並びでは `currentghost-property-tree` の後（同じ sylphya の口と `emo2_boot/mod.rs`）。読む道は `mcp-get-property`（✅ 10-04）でできた。
- 崩れた前提／古くなった位置: なし。一覧の源 `crates/areka-ghost/src/catalog.rs`（377）の `list_ghosts`・`list_shells`・`list_all_shells`・`list_balloons` は同じ名前で在る。網羅台帳 `property.toml` で本 spec が持ち主の行は今も 120（うち `system.*` の行は `system-property-values` が着地するときに移す）。`dotted.rs` の `EXT_EVENT_GET`／`EXT_EVENT_SET` の定義行は説明文の「M2」→「α 後」の言い換えだけ。
- 触るファイル: `crates/areka-sylphya/src/{actor.rs, vocab/dotted.rs, key.rs}`・`crates/areka-ghost/src/{catalog.rs（読むだけ）, sylphya_wiring.rs}`・`crates/areka/src/emo2_boot/mod.rs`（883）・`doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 議題（答えで作業が変わるものだけ）: なし（動く値の出し方は `currentghost-property-tree` が決める）。
- 見つけた穴: なし。並走の照合: `property-name-case-fold` が括弧の中の名前（`ghostlist(名前)`）を畳む範囲を決めると、本 spec の名前での引き当てに効く＝向こうが先なら、その裁定に従う。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - 順は前回のまま（`currentghost-property-tree` の後）。sylphya は C4 で変更 0。
  - 一覧の源 `crates/areka-ghost/src/catalog.rs` は `ghost-standard-balloon`（✅ 10-07）で 377 → 414 行になった（ゴーストの descript のバルーンの指定を読む `standard_balloon_keys` が増えた）。`list_ghosts`・`list_shells`・`list_all_shells`・`list_balloons` は同じ名前・同じ形で在る。
  - **源の側に未完了の spec ができた**: `baseware-root-list`（10-05 起票・その他）は、ゴーストとバルーンを複数の根から数える形に変え、「その列挙が `ghostlist`・`balloonlist` の源になる」と書く。どちらが先でも作れる。後から着地する側が合わせる。
  - 名前で選ぶ形（`ghostlist(名前)`）の英字の大小は `property-name-case-fold` が決める。手本は `mcp-ghost-name-match`（✅ 10-06）の「半角の英字の大小だけ同じとみなす」。`ghostlist(0)` を番号と読むか名前と読むか（SSP は名前と読む＝`doc/ssp-mcp/survey.md` 7.3 節の 5）は本 spec の議題として残る。
- **触るファイル**: 前回のまま＝`crates/areka-sylphya/src/{actor.rs, vocab/dotted.rs, key.rs}`（754・407・411）・`crates/areka-ghost/src/{catalog.rs（読むだけ）, sylphya_wiring.rs}`・`crates/areka/src/emo2_boot/mod.rs`（**912**）・台帳 `property.toml`（本 spec が持ち主の 120 行のうち `system.*` の 25 行を除く 95 行）・`doc/COMPAT_ARCHITECTURE.md` §8。一覧はゴーストの切替とインストールの後に変わる＝出し直しの点（`crates/areka/src/emo2_boot/ghost_switch.rs` **902**・`crates/areka/src/install/`）にも呼び出しの数行が要る見込み（前回の一覧に無い・口が「読む時に問い合わせる」形なら要らない）。
- **規模**: M（10〜14）のまま。**分割の案**: なし（一度切り出した spec）。
- **先に要るもの**: `currentghost-property-tree`（その前に `property-name-case-fold`）。
- **ファイルの重なり**: `currentghost-property-others`・`system-property-values`・`zorder-property`（`dotted.rs`・`actor.rs`・`emo2_boot/mod.rs`）・`property-name-case-fold`（`actor.rs`・`key.rs`）・`emo2_boot` の結線の列の全員・`baseware-root-list`（向こうが `catalog.rs` を書き換える）。
- **優先度の区分**: C（ukadoc の一覧のプロパティの拾い残し）。**要件定義のモデル**: Opus。
- **見つけた穴・古くなった記述**: 上の「出し直しの点」と `baseware-root-list` との順。ほかは無し。
