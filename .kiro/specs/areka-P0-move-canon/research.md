# ギャップ分析: areka-P0-move-canon

> 2026-10-17 `kiro-validate-gap`。入力は確定した `requirements.md`（要件 1〜10）・`brief.md`・steering（product・tech・structure・logging）。答えは決めず、案と決めごとの種を並べる。

## 1. 分析の要約

- **読み・計算・基準の 3 つは `move_cue.rs` の中で閉じる。** 名前付きの形の読み手と参照点の計算は純粋な関数で足せ、受け口（`MoveCueSink`）から窓へ届く線（`move_rx` → `run_move_drain_phase` → `apply_move_directive` → `move_window_to`）はそのまま使える。`move_cue.rs` は 683 行なので、新しい中身は兄弟のファイルへ出す。
- **固定は wintf の `DragConfig.enabled` で止められる。** ドラッグの準備は `enabled` を見てから始まり、クリック・ダブルクリック・右クリックの記録はその前に済んでいるので、要件 7.4 は道具の上では満たせる。ただし「固定」を何で表すか、ドラッグ以外の窓の書き手（表情の切替での置き直し・拡大率の遷移・作業領域の変化）に固定を効かせるかは決まっていない。
- **並べ直し（`resetwindowpos`）がいちばん大きい穴。** 実行中に既定の位置を計算し直す関数が無い。材料（配置の設定と主モニタの作業領域）も、走っている間は手元に残っていない。近い前例はシェルの切替の `placement/reseed.rs`（走っている窓の寸法で `resolve_placement` を解き直す）。
- **`OnResetWindowPos` は許可表に無く、返事を待つ口も UI スレッド向けには無い。** 返事なしの扱いは更新の手続き（`update/procedure.rs` の `Raised::NoReply`）が前例だが、あちらは別スレッドで待つ。メニューの照会（`menu/trigger.rs` の `PendingQuery`）は tick ごとに覗く前例である。
- **調べ物**: 窓の大きさの定義（areka は全着せ替え層の和集合で窓を作る）と SSP の参照点のずれ、固定中の窓が表情の切替で大きさを変えたときの扱い、同梱の他の検体の `\![move]` の用法。

## 2. 今の資産の地図

### 2.1 `\![move]` の道（`crates/areka/src/emo2_boot/move_cue.rs`・683 行）

| 部品 | 場所 | 今の振る舞い | 本 spec との関係 |
|---|---|---|---|
| `MoveDirective` | :53-68 | scope・x・y・duration_ms・base・base_offset・move_offset を持つ。`--option` の欄は無い | 欄を足す（要件 1.8・1.9）。構造体の直書きはテストに 8 か所 |
| `parse_move_directive` | :234-262 | 引数に `--` で始まるものが 1 つでもあれば `Err(NamedForm)`（:240-242） | 名前付きの形の読み手へ分岐させる（要件 1.1・1.13） |
| `parse_axis`・`parse_duration`・`parse_base`・`parse_ref_point` | :265-317 | 省略＝`Fix`／0／`Screen`／`left.top`。裸の 1 語は両軸へ（:313） | 名前付きの形でもそのまま使い回せる。要件 1.3・1.7 は今の既定と同じ |
| `MoveBase` | :106-117 | `Scope`・`Screen`・`PrimaryScreen`・`Me`・`Global`。他のゴーストの名前は `Err(UnknownBase)` | 型は足りる。他のゴーストの名前の記録の文言だけ要件 3.8 に合わせる |
| `m1_degradations` | :77-88 | 非スコープ基準を `UnsupportedBase`、time>0 を `TimedMoveImmediate` として記録 | `UnsupportedBase` は役目を終える。time>0 の記録は残す（要件 5.1） |
| `BaseposResolver`／`CanonDefaultBasepos` | :332-350 | basepos＝幅÷2・下端 | 参照点 `base` の解決にそのまま使う（要件 2.3） |
| `resolve_move_target_position` | :386-429 | **参照点を読まない**。式は常に `基準窓の左上＋basepos(基準窓)＋k·d − basepos(対象窓)` | 参照点 9 点＋`base` と、基準の矩形（窓・モニタ・点）を入れて書き直す（要件 2.1・2.2） |
| `MoveCueSink::emit` | :480-553 | 名前 `move` だけを自分で選び、`cue.actor` を scope にして送る | 名前付きの形が通るようになる。set/reset position をここで受けるかは案による |
| `apply_move_directive` | :585-667 | 基準が `Scope` 以外なら warn して `false`（:587-594）。窓の矩形を読み、`move_window_to` だけを呼ぶ | 基準の解決（モニタの表を読む）と固定の確かめを足す |

拡大率 k は `frame/drain_resnap.rs:96-99` で**動かす窓のスコープ**の `applied_ratio` から取っている（要件 2.4 と一致）。旧い emo2 の着地は `move_cue_apply_move_tests.rs:91-117` が固定している（要件 2.6 の檻）。

### 2.2 窓を動かす口（`placement/follow/window_move.rs`・1,221 行＝例外表に載った超過）

- `move_window_to`（:45）は経路 `MoveCue` で位置を書き、バルーンを `BalloonFollow` のずらしのまま随伴させる（要件 2.7・8.9 はここで満たされる）。
- 位置の記憶には触らない。既定の位置の追随（`track_default_char_pos`・:816-）はシステム由来の経路にだけ効く。
- このファイルは `file_length_guard_test.rs:87` の例外表に「着手前から超過」として載っている。**ここへ足すと超過が増える**ので、新しい中身は外に置くのが前提になる。

### 2.3 モニタの表（`placement/follow/work_area.rs`）

- `MonitorSnapshot`（作業領域の列）と `MonitorDpiTable`（モニタ全体の矩形 `bounds`＋拡大率）が Resource として毎フレーム同期される（`emo2_boot/frame/work_area_sync.rs`）。
- 窓の属するモニタを決める規則が **2 つある**。
  1. `work_area_for_window`（:78）: 窓の中心・半開区間・どこにも属さなければ最近傍へ寄せる。作業領域が相手。
  2. `MonitorDpiTable::dpi_for_point`（:223）: wintf の `monitor_containing` をそのまま呼ぶ。`bounds` が相手で、最近傍は無い。
- **どちらの表も主モニタの印を持たない。** wintf の `Monitor` は `is_primary` を持つが（`wintf/src/ecs/window/monitor.rs:73`）、表へ写すときに落としている。起動の並べ方は `placement/mod.rs:848` で `is_primary` を選んでいる。

### 2.4 ドラッグ（wintf）

- `DragConfig { enabled, move_window, … }`（`wintf/src/ecs/drag/mod.rs:39-54`）。`enabled` はドラッグの準備の入口で見られる（`window_proc/mouse_click.rs:153`・`mouse_dblclick_wheel.rs:130`・`mouse_move.rs:226`）。ボタンの押し離しの記録は `enabled` の前に済む。
- areka が `DragConfig` を書き換えるのは `reseed.rs:145-147` の `move_window` だけで、`enabled` に触る者はいない。キャラ窓は `spawn.rs:493` で付与される。

### 2.5 既定の位置と位置の記憶

| 部品 | 場所 | 中身 |
|---|---|---|
| `resolve_placement` | `placement/resolver.rs:243` | 配置の設定・作業領域・スコープの寸法から既定の位置を解く純粋な関数（連鎖 P2・クランプ P4） |
| 起動の準備 | `placement/mod.rs:794-` | 主モニタの作業領域で解く |
| `GhostWindows.default_char_pos` | `placement/spawn.rs:291` | スコープごとの既定の位置。**記憶から戻したスコープは `None`**。システム由来の置き直しで追随する |
| 連鎖の確定 | `placement/chain_finalize.rs` | 起動直後に実表示の寸法で横の並びだけを 1 度解き直す（`ChainFinalized`） |
| シェルの切替の解き直し | `placement/reseed.rs:55-170` | 走っている窓の寸法で `resolve_placement` を解き、揃え方・バルーンのずらしだけを取り、**キャラ窓の位置は捨てる**。材料（descript の値）は背景の資産づくりが読んだもの |
| 記憶への送り口 | `placement/persist.rs:500`（`persist_entries`） | 書く時機はドラッグの確定と、起動で並べ終えた時点（`persist_unremembered_char_positions`・:537）の 2 つ |
| 記憶の鍵 | `areka-sylphya/src/persist/mod.rs:125` | `WindowPos { scope, axis }`。**消す口は無い**（`persist_put` だけ）。空の値は起動時に `parse_px` が `None` に読み、警告の上で既定へ倒れる |

### 2.6 受け口の台帳とイベント

- 台帳（`emo2_boot/consumer_ledger.rs`・943 行）の登記は `move`・`(set,zorder)`・`(reset,zorder)`・`(execute,install)` ほか。`(set,position)`・`(reset,position)`・`(execute,resetwindowpos)` は無い。
- `ZOrderCueSink`（`zorder_cue.rs:65-68`）は、開けない荷物の宛名が `set`／`reset` なら**名前だけで自分宛**と見なす。モジュールの注記は「将来 `\![set,他]` に別の担当が付いたら見直すこと」と書く。本 spec がその「将来」にあたる。
- 台帳と 9 つの受け口の一致は `consumer_ledger_agreement_tests.rs` が固定している（表の行・見本・受け口の数）。`check_script`（MCP の作者の道具）も同じ表を引くので、行を足すと「知らない `\!`」の答えが変わる。
- `ALLOWED_EVENT_IDS`（`areka-kanade/src/schedule/events.rs:113-`）に `OnResetWindowPos` は無い。件数は `events_change_tests.rs:41` が `53` と逐語で固定している。
- リソースの照会の許可表 `ALLOWED_RESOURCE_IDS`（`areka-kanade/src/schedule/resources.rs:37`）に `switchmovetodefaultpositionbutton.caption` は無い（語彙は `areka-sylphya/src/vocab/shiori_resource.rs:193` にある）。
- イベントを送って返事を受ける形は `KanadeMsg::RaiseEvent { reply: Some(..) }` → `RaiseOutcome`（`areka-kanade/src/change.rs:111`: `Script`・`NoReply`〔空・空白だけ・エラーの返事を含む〕・`NotSteady`・`NotAllowed`・`Failed`）。

### 2.7 メニュー（`crates/areka/src/menu/`）

- 枠は `Frame` の 10 種で、並びは `Frame::ORDER: [Frame; 10]`（`menu/mod.rs:66-79`）。項目名の表は `captions.rs:30-53` の `FRAME_CAPTIONS`（枠・リソース名・既定名）。
- 葉 1 つの枠の前例は `update_frame.rs`・`install_frame.rs`（30 行前後）。登記は `ghost_session.rs:799-803` でゴーストを起こすたびに行う。
- 選ばれた動作は UI スレッドで `&mut World` を持って 1 回呼ばれる（`MenuAction`）。ここで返事を待つと画面が止まる（`captions.rs` の注記）。

## 3. 要件と資産の対応

凡例: **在る**＝そのまま使える／**足す**＝穴（Missing）／**不明**＝調べ物（Unknown）／**縛り**＝既存の決まりによる制約（Constraint）

| 要件 | 使える資産 | 判定 |
|---|---|---|
| 1.1・1.2・1.13 名前付きの形・大小・順・判定 | `parse_*` の各関数 | **足す**: `--key=value` の読み手（キー名は小文字にそろえて引く） |
| 1.3・1.4・1.5・1.6・1.7・1.12 既定・`fix`・負の数・裸の 1 語・旧い形 | `parse_axis`・`parse_ref_point`（:313） | **在る**（旧い形は無変更） |
| 1.8・1.9 `--option` の繰り返し・`ignore-sticky-window` | — | **足す**: `MoveDirective` に語彙の欄（効き目なし） |
| 1.10・1.11 知らないキー・知らない値・重複 | — | **足す**: 無視と記録、後勝ちと記録 |
| 2.1・2.2 参照点 9 点＋`base` | `RefPoint`・`RefX`・`RefY`（型だけ） | **足す**: `resolve_move_target_position` の式の書き直し |
| 2.3 `base`＝既定の basepos | `CanonDefaultBasepos` | **在る** |
| 2.4・2.5 k 倍・実寸から | `scale_signed`・`WindowPos.size` | **在る**／**不明**: 窓の大きさの定義（調べ物 R1） |
| 2.6 旧い emo2 の着地 | `move_cue_apply_move_tests.rs:91-117` | **在る**（檻はそのまま） |
| 2.7・2.8・2.9 バルーン随伴・寄せない・記憶に書かない | `move_window_to` | **在る** |
| 3.1・3.2 `screen`＝今のモニタの全体 | `MonitorDpiTable.bounds` | **足す**＋**縛り**: 属するモニタの規則が 2 つある（決めごと 3） |
| 3.3 `primaryscreen` | wintf `Monitor.is_primary` | **足す**: 実行時の表に主モニタの印が無い（決めごと 4） |
| 3.4 `global` | — | **足す**（原点 0,0 の点。式だけ） |
| 3.5・3.7 スコープ番号・窓が無い | `apply_move_directive` の既存の分岐 | **在る** |
| 3.6 `me` | 対象窓の `WindowPos` | **足す**（式だけ） |
| 3.8 他のゴーストの名前 | `Err(UnknownBase)` の warn | **足す**: 記録の文言（解決先が無い旨） |
| 4.1〜4.3 使えない組み合わせ | — | **足す**: 基準の参照点だけを検める判定 |
| 4.4・4.5 読めない値・黙って諦めない | 各 `Err` の warn | **在る**（新しい分類を足すだけ） |
| 5.1・5.2 time | `TimedMoveImmediate`・sink は `duration` に触れない | **在る** |
| 6.1・6.2 emo2 の位置調整（100%・200%） | 上の全部 | 計算の上では `x1 = x0 − w1`・`y1 = y0 + h0 − h1`（X=Y=0 なので k に依らない） |
| 7.1・7.2・7.6 `set,position` の位置と scope | `move_window_to` | **足す**: 受け口・`global`／`left.top` での配置・cue.actor の既定 |
| 7.3・7.4 ドラッグだけを止める | `DragConfig.enabled` | **在る**（道具）／**足す**: 固定の印と切り替え（決めごと 6） |
| 7.5・8.7 固定中の `\![move]`・並べ直し | — | **足す**: 適用の段で印を見る |
| 7.7 `reset,position` は全部の窓 | `GhostWindows.scopes()` | **足す** |
| 7.8 読めない値 | — | **足す** |
| 7.9 終わるまで有効・持ち越さない | ゴーストの切替で窓ごと作り直す（`app_exit::close_windows_for_restart`） | **在る**（印を窓に付ければ自然に消える） |
| 7.10 記憶に書かない | `move_window_to` は記憶に触れない。ドラッグ確定は固定中は起きない | **在る** |
| 8.1・8.2 メニュー項目と caption | `Frame`・`FRAME_CAPTIONS`・葉の枠の前例 | **足す**: 11 番目の枠＋`ALLOWED_RESOURCE_IDS` に 1 語 |
| 8.3・8.4・8.5 `OnResetWindowPos` と返事なし | `RaiseEvent { reply }`・`RaiseOutcome` | **足す**: 許可表に 1 語＋UI を止めない返事の待ち（決めごと 9） |
| 8.6 既定の位置へ並べ直す | `resolve_placement`・`reseed.rs` の前例 | **足す**: 実行時の並べ直しの関数と、その材料（決めごと 8） |
| 8.8 次の起動で既定の位置 | `char_pos_entries`・`persist_entries` | **足す**: 記憶の扱い（決めごと 10） |
| 9.1・9.2 記録 | `apply_move_directive` の info の行 | **足す**: 基準・参照点の欄、固定・解除・並べ直しの行 |
| 9.3 決定論のテスト | 純粋な関数の兄弟テストの流儀 | **在る**（流儀） |
| 9.4・9.5 実機 100%・200%・`target\` の下 | 既存の実機の手順 | **在る**（手順）／emo2 の 10-17 版は PR#326 で差し替え済み |
| 10.1〜10.4 COMPAT §8・台帳 | COMPAT :131-135・:155、台帳 :1773・:1465・:2798・:3150、shiori.toml :5191・:14967 | **足す**: 書き改め＋`ukadoc-survey report`／`check` の作り直し |

## 4. 実装の案

### 案 A: 今の受け口と道を広げる

- `MoveCueSink` の担当を `move` に加えて `(set,position)`・`(reset,position)`・`(execute,resetwindowpos)` へ広げ、線の型を列挙型（移動・固定・解除・並べ直し）にする。UI 側の `run_move_drain_phase` が種類ごとに振り分ける。
- 名前付きの形の読み手と参照点の計算は `move_cue.rs` の兄弟（例: `move_cue_named.rs`・`move_cue_geometry.rs`）へ出す。
- 並べ直しは `reseed.rs` に隣の関数として足す（走っている窓の寸法で解き、位置まで書く版）。
- ✅ 受け口の数・`emo2_boot/mod.rs`（916 行）と `spine.rs`（937 行）の sink の列が増えない（上限に近い 2 ファイルを太らせない）
- ✅ 窓の位置を書く道が 1 本に集まり、固定の確かめを 1 か所に置ける
- ❌ `MoveCueSink` の「名前 `move` だけを自分で選ぶ」という単純さが崩れる。`ZOrderCueSink` の宛名の規律（`set`／`reset` を名前だけで自分宛）と衝突するので、両方の受け口の宛名の述語を直す必要がある
- ❌ 型の名前（`MoveDirective`）と中身がずれる

### 案 B: 位置の指令の受け口を新しく作る

- `move` は今の `MoveCueSink` のまま（読み手と計算だけ広げる）。`(set,position)`・`(reset,position)`・`(execute,resetwindowpos)` は新しい受け口（例: `PositionCueSink`）と新しい線で UI へ運ぶ。
- 固定・解除・並べ直しの UI 側の処理と、メニュー・`OnResetWindowPos` の処理を新しいファイル（例: `placement/window_pin.rs`・`placement/reset_pos.rs`）に置く。
- ✅ 責務が分かれ、テストも分けやすい。`move` の受け口の既存テストに触れずに済む
- ❌ `emo2_boot/mod.rs`（916）・`spine.rs`（937）・`frame/wiring.rs`（368）に sink と受信端を 1 本ずつ足す。前の 2 つは 1,000 行に近い
- ❌ 台帳の一致の檻（9 つの受け口）が 10 になり、見本も増える

### 案 C: 混ぜる

- `move` の読み・計算・基準は案 A のとおり `MoveCueSink` と兄弟ファイルで（線の型は変えない）。
- 固定と並べ直しは新しいファイルに「UI 側の関数」として置き、タグからの入口だけを案 A か B のどちらかで選ぶ。メニューの「シェル位置初期化」は葉の枠の前例どおり `menu/reset_pos_frame.rs`（仮）を足し、返事なしのときに同じ並べ直しの関数を呼ぶ。
- ✅ 段階に分けやすい（1. 読みと計算〔emo2 の位置調整が直る〕→ 2. 基準 → 3. 固定 → 4. 並べ直し）
- ❌ 入口の選び方を設計の段で決める必要が残る

## 5. 規模と危うさ

- **規模: M〜L（5〜8 日）**。読み・計算・基準は既存の型の上の純粋な関数で S。固定は S〜M。並べ直し（材料の持ち方・返事の待ち・記憶）と、台帳・COMPAT・`ukadoc-survey` の作り直し、100%／200% の実機が M を押し上げる。
- **危うさ: 中**。読み・計算は前例どおりで低い。並べ直しは「実行中に既定の位置を解き直す」新しい形で、連鎖の確定・既定の位置の追随・記憶の 3 つの決まりと噛み合わせる必要がある。固定の効く範囲によっては窓の書き手（window_move.rs の多くの経路）に広く手が入る。

## 6. 1,000 行の決まりに近いファイル（太らせない）

| ファイル | 行数 | 触る見込み |
|---|---:|---|
| `crates/areka/src/main.rs` | 1,000 ちょうど | 触らない（1 行でも足すと超える） |
| `crates/areka/src/placement/follow/window_move.rs` | 1,221（例外表） | 触らない前提（超過を増やさない） |
| `crates/areka/src/placement/transition_judge.rs` | 996 | 関係なし |
| `crates/areka/src/emo2_boot/consumer_ledger.rs` | 943 | 登記 3 行（＋注記）。注記を厚くすると近づく |
| `crates/areka/src/emo2_boot/spine.rs` | 937 | 案 B なら sink 1 本 |
| `crates/areka/src/placement/follow/drag_follow.rs` | 937 | 固定をここで扱うなら注意 |
| `crates/areka/src/emo2_boot/mod.rs` | 916 | 案 B なら sink 1 本＋長い注記（既存の流儀は 1 sink につき 5〜6 行の注記） |
| `crates/areka/src/placement/mod.rs` | 902 | 主モニタの選び方を使い回すなら注意 |
| `crates/areka/src/ghost_session.rs` | 878 | メニューの枠の登記 1 行 |
| `crates/areka-kanade/src/schedule/events_tests.rs` | 858 | 許可表の件数の見張りは `events_change_tests.rs`（255）側 |
| `crates/areka-kanade/src/schedule/events.rs` | 845 | 許可表に 2 行（URL のコメント＋名前） |
| `crates/areka/src/emo2_boot/move_cue.rs` | 683 | 新しい中身は兄弟へ（brief の方針） |

## 7. 設計の段への申し送り

### 7.1 推奨の方向（決定ではない）

- 段階に分けた案 C が扱いやすい。最初の段（名前付きの形の読み＋参照点の計算）だけで emo2 の「エモの位置調整」（要件 6）が直るので、実機の確かめを早く始められる。
- 並べ直しは `reseed.rs` の「走っている窓の寸法で `resolve_placement` を解く」形を下敷きにでき、新しい配置の規則は要らない。足りないのは材料と、位置まで書く版の関数である。

### 7.2 設計で決めること

> 2026-10-17 要件ディスカッションで、SSP の実測（§8）により次が片付いた: 3（作業領域の規則〔中心・最近傍あり〕に決まった・要件 3.1〜3.3）・6 と 7（窓の固定は受けない・要件 7）。2 の新しい分類は「断る」ではなく「SSP と同じ置き換え＋記録」になった（要件 3.7・3.8・4）。1 の宛名の見直しから `(set,position)`・`(reset,position)` は外れる。
>
> 追加の決めごと 13（議題 4・要件 2.10〜2.12）: `\![move]` の移動先を `project_anchor`（`placement/follow/anchor.rs:104`）に通す場所。`move_window_to` は `window_move.rs`（例外表の超過ファイル）にあるので、当て直しは `apply_move_directive` 側（`move_cue.rs` の兄弟）で `Anchored` と `MonitorSnapshot` を読んで行うのが前提。今は `move_window_to` が `Anchored` を通らず、後の大きさの変化（`window_move.rs:311`）で吸い戻される。

1. **受け口の分け方**: `MoveCueSink` を広げる（案 A）か、位置の指令の受け口を新しく作る（案 B）か。どちらでも `ZOrderCueSink` の宛名の述語（`set`／`reset` を名前だけで自分宛とする・`zorder_cue.rs:65-68`）と、台帳の一致の檻（`consumer_ledger_agreement_tests.rs`）の見直しが要る。
2. **縮退の型の改め**: `NamedForm`・`UnsupportedBase` は役目を終える（固定しているテスト `move_cue_tests.rs:85-133` は陳腐化テストとして改める）。新しい分類（使えない組み合わせ・他のゴーストの基準・知らないキー・重複・固定中）を「読む段で `Err`」にするか「適用の段で弾く」にするか。使えない組み合わせは基準の語と参照点だけで決まるので読む段で弾ける。固定中の判定は World を要するので適用の段。
3. **窓の属するモニタの規則**（要件 3.2 の「今の決まり」がどちらか）: 作業領域の表の規則（中心・最近傍あり）か、拡大率の表の規則（中心・`bounds`・最近傍なし）か。`screen` はモニタ全体の矩形が要る（要件 3.1）ので `bounds` を持つのは拡大率の表の方。どのモニタにも属さない窓（画面の外へ出た窓）を `screen` 基準で動かすときの扱いが分かれる。
4. **主モニタの見分け方**: 実行時の表に主モニタの印を足す（`MonitorDpiEntry` に 1 欄）か、「原点 (0,0) を含むモニタ」と読む（Windows では主モニタの左上が常に原点）か。
5. **`center` の丸め**: 奇数の幅・高さの半分を切り捨てるか。今の basepos（:346）と resolver の P5（`resolver.rs` の注記「整数除算＝0 方向切り捨て」）は切り捨て。負の座標に掛かる場合の向きもそろえる。
6. **固定の持ち方と効く範囲**: 印をキャラ窓の component にして `DragConfig.enabled` を連動させるか、`enabled` そのものを印にするか。要件 7 が止めるのはドラッグ・`\![move]`・並べ直しの 3 つ。ほかの窓の書き手——表情の切替で窓の大きさが変わるときの下端の置き直し（`resize_window_to`）・拡大率の遷移・作業領域の変化・起動の連鎖の確定・シェルの切替の置き直し——で固定の窓を動かすかは要件に無い（固定の窓が表情の切替で上下にずれてよいか）。効かせるなら `window_move.rs`（例外表の超過ファイル）に手が入る。
7. **`\![set,position]` の座標に k を掛ける帰結**: 要件 7.1 の同値（`--base=global` の `\![move]` と同じ位置）と要件 2.4（X・Y に k を掛ける）から、拡大率 200% で `\![set,position,100,100]` は物理の (200,200) に置かれる。画面の絶対座標を書いたゴーストとは食い違うので、COMPAT の行に意図した差として書く。
8. **既定の位置の材料**: 走っている間は配置の設定（`PlacementConfig`）が手元に無い。
   - (a) 起動とシェルの切替のたびに配置の設定（か、記憶を重ねる前の既定の位置）を Resource として持っておく
   - (b) 並べ直しのたびに `descript.txt` を読み直す（UI スレッドでのファイル読み・バルーンの `windowposition` も要る）
   - (c) `GhostWindows.default_char_pos` を使う（記憶から戻したスコープは `None` なので、それだけでは足りない）
   
   作業領域は起動と同じ**主モニタ**（要件 8.6「起動で並べるのと同じ」）か、シェルの切替と同じ**窓の今のモニタ**か。並べ直した後に `default_char_pos` を `Some` に戻すか（戻すと以後の連鎖の解き直し・拡大率の遷移の追随の対象に戻る）。
9. **`OnResetWindowPos` の返事の待ち方**: UI を止めない形として、(a) メニューの照会と同じく受け手を Resource に置き tick ごとに `try_recv` で覗く、(b) 更新の手続きと同じく別スレッドで待ち、返事なしなら並べ直しの指令を線へ流す。`NotSteady`（起動中・切替中・終了中）と `Failed` のときに並べ直すか（要件 8.5 が挙げるのは空・返事なし・エラーの返事）。返事が来ないまま待つ上限。
10. **並べ直しの後の位置の記憶**（要件 8.8）: 並べ直した位置を記憶に書く（`char_pos_entries` を使い回せる）か、記憶を消す（sylphya に消す口が無く、空の値は起動時に警告になる）か。書く場合、モニタの構成が変わった次の起動では「記憶の位置」として扱われ、既定の位置とは別物になる。バルーンのずらしの記憶には触らない（`resetballoonpos` は範囲外）。
11. **メニューの項目の置き場**: `Frame::ORDER` のどこに 11 番目を置くか。選べない状態（灰色）にする条件（窓が全部固定されているとき・並べ直しの返事待ちの間など）を持つか。
12. **記録の水準と語彙**: 動かさなかった各理由（使えない組み合わせ・固定中・基準の窓が無い・他のゴーストの基準）を `warn!` にするか `info!` にするか。作者の書き間違いは warn、固定中は正常な拒否として info、など。`event = "…"` の名前をそろえる。

### 7.3 調べ物（Research Needed）

- **R1 窓の大きさの定義**: areka のキャラ窓の大きさは、面の全要素と**全着せ替えの pattern0 の和集合**（`areka-emo-compose/src/plan_extent.rs` の `compute_extent`）。SSP が参照点に使う矩形がこれと同じか。着せ替えが面の外へはみ出すシェルでは `right`・`bottom`・`center` がずれる可能性がある（emo2 では影響しない見込み）。
- **R2 固定中の窓の大きさの変化**: SSP で `\![set,position]` 中の窓が表情の切替で大きさを変えたとき、左上を保つのか下端を保つのか（ukadoc は沈黙）。決めごと 6 の材料。
- **R3 同梱の他の検体の `\![move]`**: 旧い位置の形で参照点を省いた用法（例 `\![move,100,0,0,0]`）は、今は basepos どうしで合わせているが、本 spec の後は `left.top` どうしになる（正典どおりへの改め）。`vendors/sample_ghost/` の claudia・emily4・konnoyayame・R_POST_and_KOMAINU が使っていないかを点検する。
- **R4 台帳の作り直しの手順**: 台帳の行を改めた後に `cargo run -p ukadoc-survey -- report` と `check` が要る（`doc/ukadoc-coverage/README.md:42-50`）。`check` はソースの正典 URL のコメントを証拠として数えるので、新しい受け口・許可表の行に ukadoc の URL を添える。
- **R5 並走との重なり**: 同じ v0.0.3 の `emily-ghost-verification` は brief の上では `areka-nar`・`install/`・`sample-ghost-kit` を触り、本 spec と重ならない。ただし両方が kanade の許可表（`events.rs`）と件数の見張り（`events_change_tests.rs:41` の `53`）に手を入れる可能性があり、そうなると後から着地した側が数を合わせ直す。

## 8. SSP での実測（2026-10-17）

> ukadoc が黙っている点を、開発者の机で動いている SSP で確かめた。台本は MCP の `sakurascript`（`strict=true`）で送り、窓の矩形は `currentghost.scope(n).rect`（左,上,右,下）で読み、毎回エラーログ（`get_log log_type=error`）を確かめた。最後に窓を測る前の状態へ戻し、読み直して一致を確かめた（`\0` (726,610)-(1490,1704)・`\1` (54,904)-(726,1704)・面 1000／2100・`\1` の吸着も既定へ）。

### 8.1 条件

- **SSP**: `SSP/2.9.18 (20261017-5; Windows NT 10.0.26300)`（`baseware.version`）
- **ゴースト**: 「えも2DEBUG」（emo2 の開発版）。`\0`＝むらさき（面 1000・着せ替えあり）、`\1`＝エモ（面 2100）。
- **窓の大きさ**: `\0` 764×1094、`\1` 672×800（面 2100）。シェルの拡大率 `currentghost.scope(n).scaling` は両方 `200,200`。
- **モニタ**（SSP の `currentmonitor.rect`／`.work` と、DPI を意識した PowerShell の問い合わせで一致を確かめた。単位は物理の画素）:
  - 主モニタ DISPLAY1: 全体 (0,0)-(2880,1800)、作業領域 (0,0)-(2880,1704)（下にタスクバー）、DPI 192（200%）
  - 副モニタ DISPLAY2: 全体＝作業領域 (-2560,195)-(0,1795)、DPI 144（150%）
- **測る前の位置**: `\0` (726,610)-(1490,1704)、`\1` (54,904)-(726,1704)。基準位置（`currentghost.scope(n).x`／`.y`）は `\0` (1108,1704)、`\1` (390,1704)＝幅の半分・下端。
- **SSP の座標は物理の画素**: `\1\![move,--base=global,--X=100,--Y=100]` の後、SSP の矩形 (100,100)-(772,900) と、外から DPI を意識して `GetWindowRect` で読んだ矩形が一致した。DPI を意識しない読み方なら (50,50) に見える位置である。

### 8.2 前提として見つかったこと: 下端への吸着

- SSP の既定では、キャラクターの窓はデスクトップの下端に吸着している（`seriko.alignmenttodesktop` の既定は `bottom`）。この状態では **`\![move]` の Y は効かない**（M1-b: Y=-900 を書いても下端 1704 のまま）。
- 縦の参照点を測るため、`\1` だけ `\1\![set,alignmenttodesktop,free]`（ukadoc「ゴースト終了まで有効」）で吸着を外して測り、最後に `\1\![set,alignmenttodesktop,default]` で戻した。戻した後に `\1\![move,--base=me,--X=0,--Y=-100]` が効かないことで、吸着が戻ったことを確かめた。
- 下の表は、M1-a・M1-b を除き、`\1` の吸着を外した状態の値である。
- **エラーログ**: 下の `\![move]` と `\![execute,resetwindowpos]` はどれも、`strict=true` でもエラーログに 1 行も出さなかった（使えない組み合わせ・知らない語・知らないキーを含む）。出たのは `set,position`／`reset,position` の 3 行だけ（M7・M10）。

### 8.3 測った結果

`\1` の矩形は「左,上,右,下」。「前」は台本を送る直前の `\1` の矩形。

| 番号 | 台本 | 前 | 後 | 読み取れること |
|---|---|---|---|---|
| M1-a | `\1\![move,--X=0,--Y=0,--base=0,--base-offset=left.bottom]`（吸着あり） | 54,904,726,1704 | 726,904,1398,1704 | 横は `\1` の左端が `\0` の左端へ。縦は吸着で下端のまま |
| M1-b | 同じで `--Y=-900`（吸着あり） | 726,904,1398,1704 | 変わらず | 吸着中は Y が効かない |
| M1-c | 同じで `--Y=-900`（吸着なし） | 726,904,1398,1704 | 726,804,1398,1604 | 上端 804＝1704−900。**`move-offset` の省略＝`left.top`**（`\1` の左上が `\0` の左下から (0,−900)）。Y に拡大率を掛けない |
| M1-d | `\1\![move,--X=0,--Y=0,--base=0,--base-offset=left.bottom]` | 726,804,1398,1604 | 726,1704,1398,2504 | 画面の外（下）へ出ても内へ寄せない |
| M1-e | `\1\![move,--X=0,--Y=0,--base=0,--move-offset=right.bottom]` | 726,1704,1398,2504 | 54,-190,726,610 | `\1` の右下が `\0` の左上 (726,610) へ。**`base-offset` の省略＝`left.top`**。上へはみ出しても寄せない |
| M1-f | `\1\![move,--X=0,--Y=0,--base=0,--base-offset=left.bottom,--move-offset=right.bottom]`（emo2 の位置調整） | 54,-190,726,610 | 54,904,726,1704 | `\1` の右下＝`\0` の左下（200% で成立） |
| M2-a | `\1\![move,100,0,0,0]` | 54,904,726,1704 | 826,610,1498,1410 | `\1` の左上＝`\0` の左上＋(100,0)。**旧い形で参照点を省いても `left.top` どうし**（今の areka の「基準位置どうし」とは違う） |
| M2-b | `\1\![move,-353,,,0,base,base]` | 826,610,1498,1410 | 419,610,1091,1410 | `\1` の基準位置の x＝755＝1108−353（拡大率を掛けない）。Y の空欄は今のまま |
| M3-a | `\1\![move,--base=screen,--base-offset=right.bottom,--move-offset=right.bottom]`（X・Y 省略） | 419,610,1091,1410 | 変わらず | 名前付きの形で X・Y を省くと今のまま |
| M3-b | 同じに `--X=0,--Y=0` | 419,610,1091,1410 | 2208,904,2880,1704 | 下端 1704＝**作業領域の下端**（タスクバーの上）。モニタ全体の下端 1800 ではない。M1-d のとおり寄せは無いので、基準の矩形そのものが作業領域 |
| M3-c | `\1\![move,--X=-100,--Y=-100,--base=primaryscreen,--base-offset=right.bottom,--move-offset=right.bottom]` | 2208,904,2880,1704 | 2108,804,2780,1604 | `primaryscreen` も**作業領域**の右下 (2880,1704) から |
| M3-d | `\1\![move,--X=0,--Y=0]` | 2108,804,2780,1604 | 0,0,672,800 | 基準の省略＝`screen`、参照点は `left.top` どうし |
| M3-e | `\1\![move,--base=global,--X=-2000,--Y=400]` の後に M3-b と同じ | -2000,400,-1328,1200 | -672,995,0,1795 | `screen`＝**窓が今いるモニタ**（副）。副は作業領域＝全体なので、ここでは区別できない。副へ移っても窓の大きさ・拡大率は変わらなかった |
| M3-f | 副から `\1\![move,--X=10,--Y=10,--base=primaryscreen]` | -672,995,0,1795 | 10,10,682,810 | `primaryscreen` は今いるモニタに依らず主モニタ |
| M3-g | `global` で (5000,400)／(-5000,400) へ出してから M3-b と同じ | — | 2208,904,2880,1704／-672,995,0,1795 | どのモニタにもいない窓は**一番近いモニタ**（右の外→主、左の外→副） |
| M4-a | (1000,500) へ置いてから `\1\![move,--X=0,--Y=0,--base=screen,--base-offset=base.base]` | 1000,500,1672,1300 | 1440,852,2112,1652 | 動かさないのではなく、**`base` を `center` として扱った**（作業領域の中央 (1440,852)） |
| M4-b | 同じで `--base=primaryscreen` | 1000,500,1672,1300 | 1440,852,2112,1652 | 同じ（作業領域の中央） |
| M4-c | `\1\![move,--X=0,--Y=0,--base=screen,--base-offset=left.base]` | 1440,852,2112,1652 | 0,852,672,1652 | 軸ごとに `base`→`center` |
| M4-d | (1000,500) から `--base=global,--base-offset=right.bottom` | 1000,500,1672,1300 | 0,0,672,800 | **参照点を無視して原点 (0,0)** |
| M4-e | 同じで `center.center` | 1000,500,1672,1300 | 0,0,672,800 | 同上 |
| M4-f | 同じで `base.base` | 1000,500,1672,1300 | 0,0,672,800 | 同上 |
| M4-g | (1000,500) から `--base=0,--base-offset=xyz.xyz` | 1000,500,1672,1300 | 726,610,1398,1410 | 知らない語は `left`／`top` として動かす |
| M4-h | 同じで `foo.bar` | 1000,500,1672,1300 | 726,1704,1398,2504 | 横は `left`、縦は `bottom`（か `base`。`\0` ではどちらも 1704）に読まれた。語の頭だけを見ている疑い |
| M4-i | `\1\![move,--X=abc,--Y=0,--base=0]` | 84,804,756,1604 | 84,610,756,1410 | 読めない X は `fix` 扱い（Y は効いた・止まらない） |
| M4-j | `\1\![move,--X=10,--X=20,--Y=fix,--base=me]` | 54,804,726,1604 | 74,804,746,1604 | 同じキーの重複は**後勝ち** |
| M4-k | `\1\![move,--zzz=1,--X=10,--Y=fix,--base=me,bogus]` | 74,804,746,1604 | 84,804,756,1604 | 知らないキーと `--キー=値` でない引数は黙って無視し、残りで動く |
| M4-l | `\1\![move,--X=0,--Y=0,--base=5]`（無いスコープ） | 84,610,756,1410 | 0,0,672,800 | 動かさないのではなく **`screen` へ倒れた** |
| M4-m | (1000,500) から `--X=0,--Y=0,--base=NoSuchGhostXYZ` | 1000,500,1672,1300 | 726,610,1398,1410 | 無いゴーストの名前は**スコープ 0 として動いた**（`\0` の左上）。名前を数として読んで 0 になった疑い |
| M5 | `\1\![move,--base=global,--X=100,--Y=100]` | 0,0,672,800 | 100,100,772,900 | 左上が物理の (100,100)。SSP は DPI で仮想化されない物理の画素で座標を扱う |
| M6-a | `\1\![move,--base=me,--X=10,--Y=0]` | 726,610,1398,1410 | 736,610,1408,1410 | `me`＝動かす前の自分の矩形 |
| M6-b | `\1\![move,--X=fix,--Y=50,--base=0]` | 736,610,1408,1410 | 736,660,1408,1460 | X は今のまま、上端＝610＋50 |
| M7 | `\0\![set,position,100,100,1]`、`\1\![set,position,200,200]` | 736,660,1408,1460 | 変わらず（`\0` も変わらず） | エラーログ `unknown subcommand (set,position)`。**SSP 2.9.18 は `\![set,position]` を知らない指令として扱い、何もしない** |
| M8 | （固定ができないため代わりに）吸着なしの `\1` で `\1\s[1000]`、後で `\1\s[2100]` | 736,660,1408,1460 | 690,366,1454,1460 → 736,660,1408,1460 | 面の大きさが変わると、**下端と横の中央（基準位置）を保つ**（左上ではない） |
| M9 | `\0` を `\0\![move,--base=me,--X=-100,--Y=0]` で (626,610)-(1390,1704) へずらしてから `\0\![execute,resetwindowpos]` | `\0` 626,610,1390,1704・`\1` 736,660,1408,1460 | `\0` 726,610,1490,1704・`\1` 54,904,726,1704 | 両方の窓が既定の位置（＝この机の測る前の位置）へ戻った。スクリプトログにゴーストの台本は出ず、**タグからは `OnResetWindowPos` が送られない**（ukadoc の「メニューが選ばれた際に通知」と合う）。`\1` の吸着の外しは戻らなかった（直後に Y=-100 が効いた） |
| M10 | `\![reset,position]` | — | 変わらず | エラーログ `unknown subcommand (reset,position)`。M7 と同じく SSP に無い |
| M11 | 負の X・Y | — | — | M1-c（−900）・M2-b（−353）・M3-c（−100）で効いた。奇数の幅の丸めは下の「測れなかったこと」 |

### 8.4 測れなかったこと

- **固定の振る舞い全般**（M7 の固定の点・M8 の固定中の面の大きさの変化・M9 の固定中の並べ直し・M10 の解除の範囲）: SSP 2.9.18 に `\![set,position]`／`\![reset,position]` が無いため。
- **`center` の丸め**: 手元の窓（764×1094・672×800）もモニタの作業領域（2880×1704・2560×1600）もすべて偶数で、奇数の半分が現れない。
- **メニューの「シェル位置初期化」と `OnResetWindowPos`**: 右クリックのメニューをマウスで操作する必要がある。ゴーストが台本を返したときに SSP が並べ直しを止めるかは未確認。
- **拡大率 100% のとき**: 画面の設定を変える必要がある。
- **作業領域の上端・左端**: タスクバーが下にあるため、`screen` の上端・左端がモニタ全体と作業領域のどちらかは区別できない（下端・右端では作業領域）。
- **`--time` とドラッグ**: 範囲外。

### 8.5 要件への示唆

| 要件 | SSP の実測 | 合う／食い違う |
|---|---|---|
| 1.3 | 名前付きの形で X・Y の省略＝今のまま（M3-a）、`base` の省略＝`screen`（M3-d）、`base-offset`・`move-offset` の省略＝`left.top`（M1-c・M1-e） | **合う** |
| 1.4 | 旧い形で参照点を省くと `left.top` どうし（M2-a）。Y の空欄は今のまま（M2-b） | **合う**（今の areka の「基準位置どうし」からの改めは SSP どおり） |
| 1.5・1.6 | `fix`（M6-b）・負の値（M11）が効く | 合う |
| 1.10 | 知らないキー・形の違う引数は無視して残りで動く（M4-k）。SSP は記録しない | 合う（記録は areka の足し） |
| 1.11 | 重複は後勝ち（M4-j） | 合う |
| 2.4 | SSP は X・Y に拡大率（DPI 200%・シェル 200%）を掛けない（M1-c・M2-b） | 意図した差（COMPAT §8 の既存の決まり）であることを確かめた |
| 2.8 | 画面の外へ出ても寄せない（M1-d・M1-e） | 合う |
| 3.1 | `screen`＝**作業領域**（下端 1704、M3-b） | **食い違う**（要件は「全体の矩形・作業領域ではない」） |
| 3.2 | 今いるモニタ。どこにもいない窓は一番近いモニタ（M3-e・M3-g） | 決めごと 3 の材料（最近傍ありの規則が SSP に近い） |
| 3.3 | `primaryscreen`＝主モニタの**作業領域**（M3-c・M3-f） | **食い違う**（3.1 と同じ） |
| 3.4 | `global`＝主モニタの左上 (0,0)、物理の画素（M5） | 合う |
| 3.6 | `me`＝動かす前の自分の矩形（M6-a） | 合う |
| 3.7 | 無いスコープの基準は `screen` へ倒れて動く（M4-l） | 食い違う（要件は動かさず記録） |
| 3.8 | 無いゴーストの名前はスコープ 0 として動く（M4-m） | 食い違う（要件は動かさず記録） |
| 4.1 | `screen`／`primaryscreen` の `base` は軸ごとに `center` として動く（M4-a〜c） | **食い違う**（要件は動かさず記録） |
| 4.2 | `global` は基準の参照点を無視して原点で動く（M4-d〜f） | **食い違う**（要件は動かさず記録） |
| 4.4 | 読めない X は `fix` 扱い（M4-i）、知らない参照点の語は `left`／`top` 扱い（M4-g・h）で、どちらも動く | 食い違う（要件は動かさず記録） |
| 4.5 | SSP は `strict` でも何も記録しない | areka の記録は SSP にない足し |
| 6.1 | 200% で `\1` の右下＝`\0` の左下（M1-f） | 合う。吸着中は Y が効かないので、SSP では縦は吸着の結果でもそろう |
| 7.1・7.2・7.5〜7.8 | SSP 2.9.18 は `set,position`／`reset,position` を「知らない指令」として何もしない（M7・M10） | **SSP では確かめられない**。正典の 1 行（「指定スコープを XY 座標に固定する」）だけが根拠で、座標の意味・スコープの省略・固定中の `\![move]` と並べ直し・解除の範囲は areka が決めることになる |
| 8.3 | タグの `\![execute,resetwindowpos]` からは `OnResetWindowPos` が送られない（M9） | 合う（イベントはメニューからだけ） |
| 8.6 | `\![execute,resetwindowpos]` で全部の窓が既定の位置へ（M9）。吸着の設定は戻さない | 合う |
| 8.7 | SSP に固定が無く測れない | — |
| 固定中の面の大きさの変化（決めごと 6・R2） | 固定は測れない。参考に、吸着なしの窓は面の大きさが変わると基準位置（横の中央・下端）を保つ（M8） | 固定中も「基準位置を保つ」のが SSP の普段の動きに近い |

**新しく見つかった論点（下端への吸着）**: SSP の既定は `alignmenttodesktop`＝`bottom` で、吸着中のキャラクターの窓には `\![move]` の Y が効かない（8.2・M1-b）。本 spec は吸着を範囲外（`areka-P0-window-link-directives`）とし、areka の窓は吸着を持たないので、たとえば `\1\![move,100,0,0,0]` は SSP では `\1` が下端に残るのに、areka では `\1` の上端が `\0` の上端にそろい、窓が浮く。emo2 の位置調整（要件 6）は下端どうしをそろえるので影響しないが、R3 の同梱の検体の点検と合わせて、Y の扱い（吸着を先に入れるか、COMPAT に意図した差として書くか）を要件か設計で決める必要がある。
