# Brief: areka-P0-currentghost-property-tree

> 起票: 2026-08-27（bvc 要件ディスカッション議題 4 の開発者指示による `/kiro-discovery` 再入・プロパティ系 3 spec 分割の 2 本目）
> **本 spec は `currentghost.*` 枝（≈65 項目）の最初の実導出であり、bvc（`areka-P0-balloon-vertical-canon`）が縮退登記した `currentghost.balloon.scope(ID).*` 族——`.vertical` を含む——の指名追跡先である。**

## Problem

ゴーストスクリプトは自分自身の状態（サーフェス番号・窓座標・バルーンの寸法や書字方向・シェル一覧…）をプロパティ `currentghost.*` で照会するが、areka の sylphya には `currentghost.*` の行が **0 件**しか無い（ルート枝名の予約のみ）。sylphya の M1 設計は「`baseware.*` のみ実導出・他ルート枝は NOT_FOUND 縮退（差替シーム付き）」と明文宣言しており（`vocab/dotted.rs:3-9`）、本 spec がその差替シームを `currentghost` 枝で初めて使う。

## Current State

- 掲示機構は完備: 解決は正準文字列の map 参照 1 本（`reader.rs:80-83`・`:127-146`）・`scope(N)` は `Selector::ByIndex`、`ghostlist(名前)` 型は `Selector::ByName`（`key.rs:129-141`）も既存・値なしは `NotFound`→SHIORI 側 `SHIORI_E_PROPERTY_NOT_FOUND`（捏造しない縮退が既定で成立）・`currentghost` はルート枝ゆえ SET は自動で `NotSettable`。
- 本番の publish 縫い目は 1 箇所: `emo2_boot/mod.rs:430-465`（`BootAssets.balloons` が生きていて `sylphya_publisher()` も取れる区間）。sylphya は最下層 crate で `areka-parsers`/`areka-emo-text` へ依存できない＝値は `areka` bin 側で解決して文字列で渡す形（bvc research §3.6-2）。
- `scope(ID)` は「ID ごとに 1 行 publish」を意味する（セレクタ分岐解決は存在しない）＝scope 集合の列挙規則が本 spec の設計事項（`areka.balloon.offset.scope(N)` と同形の先例あり・`persist/mod.rs:150-161`）。

## Desired Outcome

`currentghost.*` の各項目が、実際のゴースト状態から導出されて掲示され、経路 spec（`areka-P0-property-query-channels`）の照会で読める。未解決スコープ・未導出項目は値なしのまま（捏造しない）。

## Scope

- **In**（snapshot 2.8.80 実測の枝別内訳・≈65 項目）:
  - **`currentghost.balloon.scope(ID).*` ×17＋`balloon.汎用`＋`balloon.count`＝19 項目（bvc 縮退登記の指名受け皿・全列挙）**: `background.color`／`basepos.x`／`basepos.y`／`char_width`／`count`／`lines`／`lines.initial`／`num`／`rect`／`scaling`／`validheight`／`validheight.initial`／`validwidth`／`validwidth.initial`／**`vertical`**／`x`／`y`（＋`mousecursor` 系 4 は SET 有効側）。**⚠2.8.83 改訂の適用必須**——`validwidth`＝列が並ぶ方向の幅／`validheight`＝1 列の長さ／`lines`＝収まる列数＝いずれも**画面上の向き**基準（2.8.80 と役割が逆・bvc requirements SC3/SC4/SC13 が正本・ukadoc-mcp snapshot のプロパティ節は旧意味論なので裏取りに使わない）。
  - **`.vertical` の導出規則は bvc が確定済み**——スコープに実際に適用されている書字方向（bvc Requirement 2 の共存規則の確定結果）から導く・`vertical_lr`（areka 拡張）も `1`・未解決スコープは値なし（bvc Requirement 7 の語彙登記が正本・書字方向の確定は起動時 1 回＝bvc Requirement 9）。
  - **`basepos.x`／`.y` の導出規則は `areka-P0-balloon-origin-outside-validrect` が確定済み**——値は解決後の文字描画開始点（`origin` の宣言が文字を描いてよい範囲の外にある場合は、その宣言を使わず範囲の書き始めの角へ落とした後の点）から導けば足り、あちらでは実装しない（同 spec 要件 7.2・2026-09-19）。
  - `currentghost.scope(ID).*`＋`.scope.count` ×17: `animation.num`・`currentmonitor` ×5・`name`・`rect`・`scaling`・`seriko.defaultsurface`・`surface(ID).rect`・`surface.num`/`x`/`y` 等（SET 有効 3 件を含む）。
  - `currentghost.mousecursor.*` ×6（全 SET 有効）・`currentghost.seriko.*` ×14（cursor/tooltip の当たり判定名セレクタ・**`zorder`**・`sticky-window`・surfacelist）・`currentghost.shelllist.*` ×4・`.status`・`.汎用`。
  - scope ID 集合の列挙規則と未解決スコープの表現（publish の不在 vs 明示——bvc research §6 項目 8 を引受け）。
  - `vertical` 等を `GENERIC_PROP_NAMES` に登録するかの裁定（bvc research §6 項目 9 を引受け——登録すれば件数檻 4 箇所更新・GET 経路は台帳を読まないため目的は語彙の第一級保持の側）。
- **Out**:
  - 照会経路（`areka-P0-property-query-channels` 所有）。
  - `currentghost.sound.*` ×3＋サウンド語彙族 ≈18 葉（音再生基盤に依存＝`areka-P0-property-catalog-lists` へ・SET 経路の台帳追随は channels spec）。
  - `system.*`・カタログ群・`.ext.*`（同上）。
  - SET 有効項目の**書込効果の実装**（mousecursor 差替・tooltip 文言変更等は各機能基盤が要る——値の保持と GET までを本 spec・効果は機能側 spec が解禁時に接続）。

## Boundary Candidates

- publish シーム（`emo2_boot` の 1 箇所・値の解決は bin 側）と枝別の導出タスク（balloon／scope 幾何／seriko／shelllist）が自然な分割。
- balloon.scope 19 項目は bvc の座標意味論テスト資産（R3）と同じ数値源＝先行スライス候補。

## Out of Boundary

- 書字方向の解決規則そのもの（bvc 所有・確定済み）。
- バルーン定義の解析（bvc）・窓配置（placement 系 spec）。

## Upstream / Downstream

- **Upstream**: `areka-P0-property-query-channels`（照会の end-to-end 証明はこれ待ち・**publish と決定論テストは単独で可能**）・bvc（`.vertical` 導出規則＋2.8.83 意味論の登記・balloon.scope 族の縮退登記元）・`ghost-window-zorder`／`scope-zorder-pinning`（seriko.zorder の値源）。
- **Downstream**: ゴーストスクリプトの状態照会全般・bvc の「ゴーストが縦書きを判定して字数計算を変える」ユースケースの最終成立。

## Existing Spec Touchpoints

- **Extends**: sylphya の M1 縮退宣言（`vocab/dotted.rs:3-9`）の差替シームを初行使＝宣言文の改訂を伴う（完了 spec 正典の追随規律）。
- **Adjacent**: **⚠合流裁定必須＝`areka-P0-zorder-property`**（2026-08-27 に並走 zsp ブランチで起票・`currentghost.seriko.zorder` 単独 spec・本ブランチには不在）は本 spec の `seriko.*` 範囲の真部分集合——**マージ時に ⑴ 本 spec へ吸収 or ⑵ 本 spec から `seriko.zorder` を切り出しの二択を、クロス spec 合流セッションで裁定すること**（二重所有のまま放置しない・記憶 portfolio-convergence-decided-in-separate-session）。`areka-P0-balloon-canon-residue`（M2 ゲート・系列解決と表示寿命＝プロパティ族は収載外で非交差）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（channels spec の後段・bvc 完了後が自然）。
- 正典参照はライブ ukadoc（2.8.83 現行）——**ukadoc-mcp snapshot のプロパティ節は 2.8.80 意味論で逆**（bvc Requirement 11.7 登記済みの罠）。
- 値の捏造禁止（未導出は NotFound のまま）・決定論テスト必達（scope 2 体・縦書き/横書き双方の `.vertical` 一致を含む）。

---

> **📌 2026-09-02 棚卸⑫**——アンカー再測定: `dotted.rs:3-9`・`reader.rs:80-83/:127-146`・`key.rs:129-141`・`persist/mod.rs:153/:156` ＝命中。**ずれ**: publish 縫い目 `emo2_boot/mod.rs:430-465`→**:458-502**（zsp が +35 行）。Adjacent 節の「`zorder-property` は本ブランチには不在」は**現在は偽**（09-02 の zsp 合流で `.kiro/specs/areka-P0-zorder-property/brief.md` 実在）。**三重所有**（channels も SET 台帳で `seriko.zorder` を主張）＝棚卸⑫の推奨は「本 spec は `seriko.*` から `zorder` を除外」（roadmap 干渉台帳・裁定は着手前）。編成＝W13 裁定枠（channels と `dotted.rs`／`emo2_boot/mod.rs` を共有＝同居不可・直列）。先行スライス可＝balloon.scope 19 項目（bvc R3 の数値源を再利用・publish と決定論檻は channels 抜きで単独着地可）。規模 L。


---

> **📌 2026-09-11 棚卸⑬（三重所有の仮裁定・前提の再測定）**——本 spec の `currentghost.seriko.*` は **`zorder` を除外**した 13 項（`sticky-window` は残す）。`zorder` の値は `areka-P0-zorder-property`、SET 台帳行は `areka-P0-sylphya-set-ledger`。Adjacent 節の「`zorder-property` は本ブランチに不在」は偽（実在・棚卸⑫追記どおり）。publish 縫い目は `crates/areka/src/emo2_boot/mod.rs` の `GhostBootOptions` 構築（:442 付近）と sinks vec（:448 付近）＝行番号は再びずれているが実体は健在。編成＝**W15**（`property-query-channels` W14 の後・`property-catalog-lists` とは `dotted.rs`／`key.rs`／`mod.rs` を共有＝直列で W16 へ）。先行スライス「balloon.scope 19 項目」は要件段階で L→M に縮める選択肢として保持。要件定義は Opus（scope ID 集合と未解決スコープの表現の 2 議題）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 中。規模 L（**そのままでは 20 タスクを超える**＝要件の段で `balloon.scope` の 19 項目を先に切る）。
- **崩れた前提**: 「値を出す場所は `emo2_boot` の 1 か所」は古い。α でゴースト・シェル・バルーンの切替が入り、ゴーストを起こすたびに実行環境が新しくなる＝値は実行環境ごと・切替のたびに出し直す（前例 `install/names.rs` の種の入れ直し・`ghost_switch.rs`）。`shelllist.*` は `areka_ghost::catalog::{list_shells, list_all_shells}` から出せる。`currentghost.status` は `ExecutionStatus::render` を使い回す。
- 読む道は `property-query-channels` か `mcp-get-property`（開発者向け）。出すだけなら単独で作って試験できる。`property-catalog-lists` とは同じファイル（sylphya の語彙・`emo2_boot/mod.rs`）を触る＝直列。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: 網羅台帳 `property.toml` で本 spec が持ち主の行は 64。全部では 25〜32 タスク＝**切る**。案: ⒜ 動く値を出す仕組み（下記）＋`currentghost.balloon.scope(ID).*` の 19 項目＝M（13〜16）／⒝ `currentghost.scope(ID).*`（面・窓の位置と大きさ・モニタ）・`seriko.*`（`zorder` を除く 13）・`mousecursor.*`・`shelllist.*`・`status`＝M〜L（14〜18）。順は ⒜→⒝（⒝ は ⒜ の仕組みに乗る）。**brief は今分けてよい**: ⒜ は bvc（`.vertical`）の指名の受け皿で単独の価値があり、20 を大きく超える（25 以上）ので「一度切り出した spec は削らない」の目安の外。
- 前提の状態: 読む道の `property-query-channels`・`mcp-get-property`（C3-⑦）はどちらもまだ。出すことと決定論のテストは単独でできる（前回どおり）。
- 崩れた前提／古くなった位置:
  - **動く値を出す口が sylphya に無い**: `SylphyaPublisher`（`crates/areka-sylphya/src/actor.rs`）の出す口は `publish_static`（静的構成）・`publish_shiori`（SHIORI 照会）・`persist_put`（永続）だけで、`BackingLayer::RuntimeState`（面・scope 系）は「縮退のまま層の存在を型で表す」と書かれたまま。読む側は `SharedMirror` の写し（`mirror.rs`）を引くだけ。面の番号・窓の位置・`status` は刻々と変わる＝「変わるたびに出し直す」か「読む時に問い合わせる」かを決め、口を足す（⒜ の最初の仕事）。
  - `currentghost.status` を `ExecutionStatus::render` で作る案（前回）は、その値が kanade のスレッドの `State` から毎回導かれる点に注意（UI の側に写しは無い）。
  - 切替: 値はゴーストの実行環境ごと（`ghost_switch.rs` の `runtime.sylphya_publisher()`・`crates/areka/src/emo2_boot/frame/switch.rs` の `record_memory`）。シェル・バルーンの切替は kanade を作り直さない＝`balloon.*`・`shelllist.*` は切替の後に出し直す。
  - `shell-balloon`（PR#227）で、シェルの絵の中の箱（`balloon.名前`ブレス・`\b[名前]`）が入った。これは `currentghost.balloon.scope(ID)` の scope ではない＝19 項目の対象に入れないことを要件で書く。
  - `emo2_boot/mod.rs` は 883 行・`ghost_switch.rs` は 891 行（どちらも上限に近い）。
- 触るファイル（並走の照合用）:
  - `crates/areka-sylphya/src/{actor.rs, mirror.rs, vocab/dotted.rs, vocab/mod.rs}`
  - `crates/areka-ghost/src/sylphya_wiring.rs`・`crates/areka-ghost/src/runtime.rs`
  - `crates/areka/src/emo2_boot/{mod.rs, ghost_switch.rs, shell_balloon_switch.rs, frame/switch.rs}`＋新規（値を集める所・例 `crates/areka/src/property/`）
  - ⒝ で `crates/areka-ghost/src/catalog.rs`（`list_shells`・`list_all_shells` を読むだけ）
  - `doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: 動く値を「変わるたびに出す」か「読む時に問い合わせる」か（前者は出す点が多く、後者は sylphya に読む時の問い合わせ口を足す）。brief を今分けるか（上の案）。
- 見つけた穴: なし。並走の照合: `property-catalog-lists`（同じ sylphya の口・`dotted.rs`・`emo2_boot/mod.rs`）・`zorder-property`（同じ動く値の口）・`mcp-get-property`（`areka-ghost/src/runtime.rs`）とは同時に走らせない。動く値の口は 3 本（本 spec・`property-catalog-lists` の `system.clock` など・`zorder-property`）が共通に要る＝最初に着手する 1 本が作り、残りが使う。

## 2026-10-04 棚卸㉑で切った後の範囲

- 残した範囲: ⒜ 動く値を出す口（sylphya に足す・「変わるたびに出す」か「読む時に問い合わせる」かは本 spec の議題）と、`currentghost.balloon.scope(ID).*`・`balloon.汎用`・`balloon.count` の 19 項目・scope ID の列挙と未解決スコープの表し方。**動く値の口の持ち主は本 spec**（`system-property-values`・`currentghost-property-others`・`zorder-property` はこれを使う）。
- 規模: M（13〜16 タスク）。
- 移した先: `currentghost.scope(ID).*`・`mousecursor.*`・`seriko.*`（`zorder` を除く）・`shelllist.*`・`status`・`汎用` は新しい spec `areka-P0-currentghost-property-others` へ（本 spec が前提）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（13〜16）のまま。切る: なし（一度切り出した spec）。
- 前提の状態: **読む道ができた**＝`mcp-get-property`（✅ 10-04・PR#230）が sylphya の読み手 `GhostRuntime::sylphya_reader()` で値を読む。出した値を実機で端から端まで確かめられる（MCP の `get_property` に `currentghost.balloon.scope(0).validwidth` などを聞く）。`property-query-channels` を待つ理由は無くなった＝中身の前提は満たす。待つのは `emo2_boot` の結線の列の順（`balloon-font-file`〔C4〕ほか）だけ。
- 崩れた前提／古くなった位置:
  - sylphya の出す口は今も `publish_static`・`publish_shiori`・`persist_put` の 3 つ（`crates/areka-sylphya/src/actor.rs`・C3 は説明文の「M2」→「α 後」の言い換えだけ）。動く値の口は無い（前回どおり）。
  - `RuntimeCommandSink` の届け先は未登録のまま（`actor.rs` の `run_actor` の `RuntimeCommandReserved` の腕）。
  - 行数: `emo2_boot/mod.rs` 883・`ghost_switch.rs` 891・`shell_balloon_switch.rs` 442・`frame/switch.rs` 635・`areka-ghost/src/runtime.rs` 788（+5）・`sylphya_wiring.rs` 415・`mirror.rs` 248。
  - `mcp-get-property` の実行系のテスト（`crates/areka/src/mcp/get_property_tests.rs` の `reads_values_through_the_ghost_own_asker_on_a_real_runtime`）は `currentghost.name` が値なしになることを期待する。本 spec の 19 項目には入らないので赤にならない（`currentghost-property-others` の側で効く）。
- 触るファイル: `crates/areka-sylphya/src/{actor.rs, mirror.rs, vocab/dotted.rs, vocab/mod.rs}`・`crates/areka-ghost/src/{sylphya_wiring.rs, runtime.rs}`・`crates/areka/src/emo2_boot/{mod.rs, ghost_switch.rs, shell_balloon_switch.rs, frame/switch.rs}`＋新規（例 `crates/areka/src/property/`）・`doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 議題（答えで作業が変わるものだけ）: 動く値を「変わるたびに出す」か「読む時に問い合わせる」か（前回どおり・後続 3 本の形が決まる）。
- 見つけた穴: なし。並走の照合: `property-name-case-fold`（`key.rs`・`reader.rs`・`actor.rs` の分類）とは `actor.rs` を分け合う見込み＝同時に走らせない。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - **働きの上では先頭に立った**。読む道は `mcp-get-property`（✅ 10-04）で足り、未完了の spec への働きの依存は無い（`property-query-channels` は待たない）。待つのはファイルの重なりだけ＝⑴ `property-name-case-fold`（同じ sylphya の `actor.rs`・列の先頭）⑵ `emo2_boot` の結線の列（下の「先に要るもの」）。
  - sylphya は C4 で変更 0。出す口は今も `publish_static`・`publish_shiori`・`persist_put` の 3 つで、動く値の口は無く、運行の値の届け先も未登録のまま（`actor.rs` 754・`mirror.rs` 248・`vocab/dotted.rs` 407・`vocab/mod.rs` 96）。
  - `emo2_boot` は C4 の 4 本が触った: `mod.rs` 883 → **912**・`ghost_switch.rs` 891 → **902**（どちらも上限の近く＝値を集める所は新しいファイルに置き、ここへは呼び出しの数行だけ）。`shell_balloon_switch.rs` 442・`frame/switch.rs` 635 は同じ。
  - `ghost-standard-balloon`（✅ 10-07）で、起動と切替のバルーンは「ゴーストの descript の指定 → 同梱の最初の 1 個」で決まるようになった（`crates/areka/src/boot_resolve.rs` の `resolve_balloon`）。19 項目の値の源（今のバルーンの定義）はゴーストの切替でも替わりうる＝出し直しの点に入れる。
  - `mcp-get-status`（✅ 10-06）が kanade へ状態を聞く `KanadeMsg::StatusQuery`（返事が後から来る形）を足した。「読む時に問い合わせる」口を選ぶなら、SHIORI の呼び出しの最中の読み取りで待ち合いにならないかを先に確かめる（口の形を決める材料）。
- **触るファイル**: `crates/areka-sylphya/src/{actor.rs, mirror.rs, vocab/dotted.rs, vocab/mod.rs}`・`crates/areka-ghost/src/{sylphya_wiring.rs, runtime.rs}`（415・788）・`crates/areka/src/emo2_boot/{mod.rs, ghost_switch.rs, shell_balloon_switch.rs, frame/switch.rs}`＋新規（例 `crates/areka/src/property/`）・台帳 `property.toml`（本 spec が持ち主の 64 行のうち `currentghost.balloon` の 23 行）・`doc/COMPAT_ARCHITECTURE.md` §8。口を「読む時に問い合わせる」にすると sylphya の `reader.rs`・`key.rs` にも触れうる。
- **規模**: M（13〜16）のまま。**分割の案**: なし（一度切り出した spec）。
- **先に要るもの**: 働きは無し。ファイルで `property-name-case-fold`（先に着地させる）。`emo2_boot` の結線の列では `balloon-font-file`・`balloon-canon-residue`・`shell-companion-balloon`・`mcp-reload`・`extra-character-windows` の後ろ（`mod.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`・`frame/switch.rs` を分け合う）。`placement-measure-bake-once`（`emo2_boot/mod.rs` を触る案のとき）・`property-query-channels`（`mod.rs`・`areka-ghost/src/runtime.rs`）とも同時に走らせない。
- **本 spec を待つ spec**: `currentghost-property-others`・`system-property-values`・`property-catalog-lists`・`zorder-property`（4 本とも動く値の口を使う）。
- **優先度の区分**: C（ukadoc の `currentghost.*` の拾い残し・完了 `balloon-vertical-canon` が先送りした `.vertical` の受け皿・台帳の段は「その他」）。
- **要件定義のモデル**: Fable（動く値の口の形で後ろの 4 本の形が決まる・スレッドをまたぐ）。
- **見つけた穴・古くなった記述**: 棚卸㉒の「待つのは `balloon-font-file`〔C4〕ほか」は古い（`balloon-font-file` は C5 へ回った）。roadmap の台帳の行の「前提」の欄（`property-query-channels`（読む道）か `mcp-get-property`）は満たされた＝今の前提は `property-name-case-fold`。本文の行番号つきの位置（`emo2_boot/mod.rs:430-465` など）はどれも古い（受け口の並びは今 `emo2_boot/mod.rs` の `GhostBootOptions` を組む所）。
