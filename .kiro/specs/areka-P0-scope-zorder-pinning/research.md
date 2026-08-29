# ギャップ分析: areka-P0-scope-zorder-pinning

**作成**: 2026-08-27 / **入力**: requirements.md（確定済）・brief.md・`.kiro/steering/`（product/tech/structure/logging/roadmap）・ukadoc（ライブ照合済）
**file:line は 2026-08-27 に本ワークツリー（`claude/areka-p0-zorder-pinning-8e3e7c`）で実測**。design 前に再実測すること。

---

## 0. 要約（3〜5 行）

- **土台は 8 割揃っており、欠けているのは「N 窓の列」という語彙 1 つと、その列を保つ発火経路である。** ペア機構（`zorder_pair*.rs` 5 本）・実測層・指令の組立・記録の書式・診断・決定論檻の作法はすべて再利用できる。
- **最大のギャップは 3 つ**: ⑴ `KeepDirectlyAbove.peer` が**単数**＝2 窓固定で N 窓の列を表せない、⑵ 維持系が **1 巡 1 本**しか是正指令を出さない（N 窓グループは N−1 本要る）、⑶ **是正の引き金を出す供給者が確立系 1 つしかない**（利用者操作・窓の出現・再表示に対する引き金が本番に存在しない）。
- **タグの入口はゼロ工事で通る**（parsers/sakura/dola は無改変で `\![set,zorder,…]` が sink まで届く）。ただし汎用キャリアは**第 1 トークンだけ**を名前にするため、届くのは `name="set"` / `params=["zorder","1","0"]` であり、**消費者は `set` ＋第 1 引数 `zorder` で二段自己選別する**必要がある。
- **descript は転記済み**（`config.rs:104` `zorder_raw`）だが、KV は**後勝ちの単一値**なので descript で複数グループは作れない（要件 5.3 の「1 つの指定＝1 グループ」と整合するが、正典の沈黙箇所として登記が要る）。
- **要注意の意味論の食い違い**: 「可視」が 2 つある。実測層の隣接判定は **Win32 の可視**（`is_window_visible`）で測るが、バルーンの表示・非表示は**合成層の可視**（`VisualMount::set_visible`）であって窓は WS_VISIBLE のまま——**要件 7.3 の「再表示」は Win32 の可視性遷移ではない**。ここを取り違えると檻も実機判定も空振りする。

---

## 1. 現状調査（既存資産と慣行）

### 1.1 wintf 側 — ペア機構（`crates/wintf/src/ecs/window/`）

| ファイル | 役割 | 本 spec から見た位置づけ |
|---|---|---|
| `zorder_pair.rs`（51KB） | 語彙・純判断・実測層・記録の出力点 | **拡張元の中心**。1,000 行目安に迫っており、これ以上の増築は分割前提 |
| `zorder_pair_diag.rs` | 記録の**行を組む純関数**（`*_line`）とタグ定数 | 語彙保存（要件 9.5）の正本。**タグ 6 種で閉じている** |
| `zorder_pair_establish.rs` | 案 A の owner 確立（`Added<WindowHandle>` 駆動） | 現状**唯一の**`ReassertZOrder` 供給者（`:180`） |
| `zorder_pair_maintain.rs` | 維持系（トリガ→観測→判断→適用→次巡検証）＋破棄時の owner 切離し＋沈降観測の呼び出し | **改組の主戦場**。dlp の起床旗 1 行が同居（`:373-375`） |
| `zorder_pair_sink.rs` | 非活性化の 1 巡遅延観測（読み取り専用） | 要件 7.5／9.3 の先例。書込を一切しない形の手本 |

**主要な型と関数（実測アンカー）**

- `KeepDirectlyAbove { peer: Entity }`（`zorder_pair.rs:47`）— **バルーン窓 1 枚に付く片側宣言・peer は単数**。doc に「スコープ間には宣言を張らない——これが要件 3.1／3.4 の構造的根拠」と明記されている（＝**本 spec が正面から書き換える対象**）。
- `ReassertZOrder { pending_verify: Option<ExpectedOrder> }`（`:124`）— **他クレートから挿せる公開の契約点**。未適用（`None`）→ 適用済み検証待ち（`Some`）の 2 段階。
- `ExpectedOrder { above, below }`（`:66`）— 期待隣接 1 形。
- `OwnerLink { owner_hwnd }`（`:142`）／`ZOrderPairStrategy`（`:165`・既定＝案 A・`raise_assist: false`）。
- `decide_pair_fix(&PairObservation) -> PairFixDecision`（`:335`）— **Win32 も World も触らない純判断**。歯止めの順は 生存 → ハンドル → 隣接（同値ガード）→ トリガ×ストラテジ。本 spec の判断もこの形に倣うのが素直。
- `SkipReason`（`:212`・5 語）／`InsertSpec`（`:279`・`After`/`TopEdge`/`TopOfNormalBand`）／`PairFixDecision`（`:303`）。
- 実測層: `measure_window_above`（`:564`）・`measure_window_below`（`:575`）・`measure_window_is_always_on_top`（`:592`）・`measure_windows_in_front`（`:635`）。走査上限 `SIBLING_SCAN_LIMIT = 512`（`:511`）。**いずれも「最も近い可視の隣」で測る**（不可視窓＝既定 IME 窓を読み飛ばす）。
- 適用側: `plan_pair_fix`（`maintain.rs:157`）→ `pair_fix_command`（`:188`。`InsertSpec` → `ZOrder` の写像は `:190-194`。`TopEdge`／`TopOfNormalBand` はともに `ZOrder::Top`＝`HWND_TOP`）→ `SetWindowPosCommand::enqueue`（`:497`）。フラグは `WindowPos` から自動導出され、常に `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`。観測札は `WriteTag { origin: ORIGIN_ZORDER_PAIR }`。
- **記録タグは 6 種で閉じている**（`zorder_pair_diag.rs:30-40`）: `owner-established` / `fix` / `skip` / `verify-failed` / `owner-establish-failed` / `owner-detached`（＋`sink-observed`）。実機サインオフの grep 判定語がこの表と 1:1。

**制約として効く既存の設計判断（読み落とすと後で覆る）**

1. **1 巡に実際に指令を出すのは高々 1 ペア**（`maintain.rs:396`・`:483-489`）。理由は「実測はこの巡・書込は巡の後の flush」ゆえ、同じ巡に 2 本積むと 2 本目の挿入位置が既に古い、という実機で確定した欠陥（2 スコープ起動で毎回 `verify-failed`）。**N 窓グループはここに正面からぶつかる。**
2. **収束の論証は「ペアどうしがキャラ窓を共有しない」「キャラ窓が可視」の 2 前提に依存**（module doc `:40-59`）。グループはこの前提の外にあるので、収束の論証をやり直す必要がある。
3. **要求は 1 巡限りで消える**（見送りでも検証でも `remove`）。「再試行の輪を作らない」が明示の設計方針。
4. **owner（案 A）は各ペア内にのみ張る**。OS は被 owner 窓を owner の手前に保つので、**キャラ窓が浮上すると自分のバルーンも連れて上がる**（ゲート G6）。逆に**別スコープの窓を owner 一組の間に挟むことはできない**（記憶 `windows-setwindowpos-insert-after-pulls-into-topmost-band` と同根の制約）。

### 1.2 areka 側 — 宣言・結線・scope の正本

- `placement/spawn.rs`
  - `ScopeWindows { char_window, balloon_window, default_char_pos }`（`:251`）／`GhostWindows`（`:291`）＝**scope → 窓 Entity の唯一の正本**。`char_window(scope)`（`:299`）・`balloon_window(scope)`（`:304`）・`scopes()`（`:309`）。**`sN`／`bN` はこの 2 本の引きで即解決できる。**
  - バルーン窓へ `KeepDirectlyAbove { peer: char_window }` を付与（`:532-533`）＋`log_zorder_pair_declared`（`:538`・記録は `placement/diag.rs:485`）。
  - `wire_zorder_pair(world)`（`:631`）— ストラテジ明示挿入＋`FrameFinalize` へ `(establish_owner_links, apply_zorder_pair_maintenance).chain()`（`:640-643`）。呼び手は `main.rs:633`。
- `placement/config.rs` — `PlacementConfig.zorder_raw: Option<String>`（`:104`）を **shell descript KV から生転記**（`:133`・`shell_kv.get("seriko.zorder")`）。**実挙動なし。読む口が無いだけ。** ghost 側 descript には無い（`:677-679` のテストが「ghost 側では None」を固定）。
- `emo2_boot/consumer_ledger.rs` — `ConsumerLedger::canonical()`（`:96-105`）が `move`→`MoveSink`・`bind`→`Seriko` を登記。**粒度はコマンド名のみ**。
- `emo2_boot/move_cue.rs` — 消費者の同型実例。`MoveCueSink`（`:470`）・`impl CueSink::emit`（`:487`〜`:553`）＝ ①キャリア抽出 → ②名前自己選別（`name != "move"` は debug スキップ）→ ③`cue.actor` の scope 解釈 → ④純関数 parse → ⑤mpsc 送出 → ⑥`tick_wake::mark(PRESENT)`。
- `emo2_boot/mod.rs` — 本番結線。move チャネル生成（`:322-323`）・`sinks: vec![…]` の 4 本（`:420-425`）・`Emo2Wiring::new`（`:458`）。**sink 追加は「チャネル 1 組＋`sinks` へ 1 行＋`Emo2Wiring` へ受信端 1 本＋frame 相 1 つ」で閉じる。**
- `emo2_boot/frame.rs` — `emo2_frame_system`（`:158`・`FrameFinalize` の排他 system）が相を順に回す。`run_move_drain_phase`（`:210`・実体は `frame/drain_resnap.rs:79`）が UI スレッド適用の手本（`GhostWindows` 未挿入の間はチャネルが保留バッファを兼ね、取りこぼさない）。

### 1.3 `\!` 汎用キャリア（parsers → sakura → dola）

- `areka-parsers/src/sakura/decode.rs:321-326` `decode_passthrough_bang` — **第 1 引数だけを `name`、残りを `raw_args`** にする。
- `areka-sakura/src/compile.rs:174-181` — `Instruction::GenericCommand` → `CueCommand::command_carrier(name, raw_args)`（typed variant を新設しない）。
- **したがって `\![set,zorder,1,0]` は `name="set"` / `params=["zorder","1","0"]` として sink へ届く。** `\![reset,zorder]` も同様に `name="reset"` / `params=["zorder"]`。**parsers/sakura/dola は無改変でよい**（brief の主張は実測で裏が取れた）。
- **`set,balloonwait` の allowlist は本番コードに実在しない**（`crates/` 全域 grep で 0 件）。COMPAT §8 に「compile 側時間指令 allowlist は M1 非実導出」と登記されているだけであり、**現時点で `"set"` を巡る実コードの衝突は無い**。

### 1.4 tick の門・観測資産（dlp 由来・保存義務あり）

- `tick_wake::ZORDER`（`crates/wintf/src/ecs/world/tick_wake.rs:117`）。生産者は「`zorder_pair_maintain`／`ReassertZOrder`」と module doc に明記。
- 実行は `maintain.rs:373-375`——**この巡の頭に要求が 1 つでもあれば旗を立てる**。doc が「後ろに置く生産者を足すときは、その側で旗を立てること」と名指しで警告している。
- `[tick]` 相別行の書式は `crates/wintf/src/ecs/world/tick_diag.rs:132`（`format_window_line`）。相名（`SCHEDULE_NAMES`）は pwc／dlp の観測資産が読む前提。
- **`FrameFinalize` 内で `emo2_frame_system` と `(establish, maintain).chain()` の相対順は未拘束**。新しい生産者を emo2 相の側に置くと、旗と 1 巡遅延の両方が問題になる（→ 設計判断 #7）。

### 1.5 先送り語彙檻（両肺）

- wintf 側: `zorder_pair_deferred_vocabulary_tests.rs` — `PRODUCTION_FILES: [&str; 5]`（`:64-70`）・`DEFERRED_NEEDLES: [(&str,&str); 9]`（`:76-101`）。行頭 `//` を落としてから走査。
- areka 側: `crates/areka/src/placement/spawn_zorder_pair_deferred_tests.rs` — `PRODUCTION_FILES: [&str; 2]`（`:48`＝`spawn.rs`・`diag.rs`）・同じ 9 語。
- **新設本番ファイルは該当する側の `PRODUCTION_FILES` へ足す**（要件 10.4）。件数定数（`[&str; 5]` / `[&str; 2]`）も同時に直す＝**暗黙に増えない形**。

### 1.6 COMPAT 記録

- `doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」（`:122`〜）は `| 項目 | 裁量 | 根拠 | 出典 spec |` の 4 列表。**z-order の行は現在 0 件**（grep で `zorder` は §8 内に不在）。
- 訂正対象の誤記は `.kiro/specs/completed/areka-P0-ghost-window-zorder/brief.md:10`（「`zorder_raw`＝SERIKO レイヤ順」）。正しい解釈は `completed/areka-P0-window-placement/design.md:67` と ukadoc（`seriko.zorder` は「`\![set,zorder,…]` の descript 版」）。**完了 spec 配下の文書を書き換えるか、§8 の行で上書きするかは裁定事項**（先例＝scg が `window-placement` R2.9 を §8 で上書きした形）。

### 1.7 プロパティ側（要件 13 の先送り先）

- `crates/areka-sylphya/src/vocab/dotted.rs` — `currentghost` はルート枝として存在（`:22`・`:130`）。SET 有効群は 21 項（件数檻 `:188`）で `seriko.defaultsurface`・`seriko.cursor.*`・`seriko.tooltip.*` はあるが **`seriko.zorder` は無い**。M1 では `currentghost` 配下は NOT_FOUND 縮退。
- **要件 13.2 の「現行どおりの応答」は既に成立している**（何もしなければ NOT_FOUND）。実装ではなく**語彙と追跡先の記録**が仕事。

---

## 2. 要件 → 資産 対応表（Missing / Unknown / Constraint）

| 要件 | 依存する既存資産 | ギャップ | タグ |
|---|---|---|---|
| 1.1 数値モードの左＝手前 | `GhostWindows`（scope→窓）・`pair_fix_command` | 窓 N 枚を順に並べる語彙・計画・指令列がすべて無い | **Missing** |
| 1.2 スコープ単位のかたまり（バルーンはキャラ直上のまま） | `KeepDirectlyAbove`・owner 保証 | 数値モードのグループ要素をスコープ→窓 2 枚へ展開する規則が無い | **Missing** |
| 1.3 利用者操作で崩れたら是正 | `decide_pair_fix` の `RaisedAbove`／`RaisedBelow` 分岐は**存在するが供給者が居ない** | **z 変化の検知が本番未結線**（`PairTrigger` の 3 種は `#[allow(dead_code)]`）。`WM_WINDOWPOSCHANGED` ハンドラ（`window_proc/window_pos.rs:41`）は `WINDOWPOS`（`flags`／`hwndInsertAfter`）を手にしており**検知点そのものは在る** | **Missing** |
| 1.4 未出現の窓はグループから外さない | — | グループの永続化（窓が無くても宣言が残る）＝現行の Component 宣言（窓に付く）では表せない → **Resource 側に持つ設計が要る** | **Constraint** |
| 1.5 ゴースト終了まで／持ち越さない | 永続層（`sylphya::persist`）に触れなければ自動で成立 | 無し（**非目標として明記するだけ**） | — |
| 1.6 要素 2 個未満は無視＋記録 | `SkipReason` の形（理由必須） | 新しい理由語が要る | Missing（小） |
| 1.7 実行スコープに依らない解釈 | `cue.actor` を**見ない**ことで成立（`move` は逆に actor を見る） | move_cue と挙動が違う点を設計で明示 | **Constraint** |
| 2.1〜2.2 明示モード `balloonN`/`surfaceN`・省略形 | `GhostWindows::balloon_window`/`char_window` | トークン解釈の純関数が無い | Missing（小） |
| 2.3 モード混在の全体拒否 | — | 同上（純関数内の分岐） | Missing（小） |
| 2.4 明示指定がスコープ内隣接と矛盾 → 隣接優先 | `KeepDirectlyAbove` は**永続宣言**であり、owner が OS 側で保証 | **調停規則が無い**。しかも案 A の owner 制約では「キャラをバルーンより手前」は**そもそも OS が許さない**（構造的に成立済み）——記録だけが仕事の可能性 | **Unknown** |
| 2.5 グループ外の窓を動かさない | 現行の是正が「宣言ペアの 2 枚のいずれか」に限られる型の形 | グループ版でも同じ型の形を作れるか要設計 | Constraint |
| 3.1 複数グループ併存 | — | グループ集合を持つ Resource が無い | Missing |
| 3.2 再指定の全体拒否 | `ConsumerLedger` の `try_register` が**同型の先例**（重複を `Err` で観測可能化） | 判定は純関数で書ける | Missing（小） |
| 3.3 `sN`／`bN` と `N` を同一スコープとみなす（拒否判定） | — | 同上 | Missing（小） |
| 3.4／3.5 タグ内重複 / `bN` と `sN` は別窓 | — | 同上 | Missing（小） |
| 3.6 グループ間は非規定 | 現行が既に非強制 | 「何もしない」ことの檻をどう書くか | Constraint |
| 3.7 個数上限なし | `SIBLING_SCAN_LIMIT=512` は**走査**の上限であってグループ長ではない | 実質無し（`Vec` で足りる） | — |
| 4.1〜4.3 `\![reset,zorder]` | `\!` キャリア（`name="reset"`） | 消費者が無い。descript 既定の**保持**（解除後に戻す先）が要る | Missing |
| 4.4 zorder 以外の reset は無変更 | 自己選別で自然に成立 | 記録の要否が裁定事項 | Constraint |
| 5.1〜5.5 descript `seriko.zorder` | `config.rs:104` `zorder_raw` が**転記済み** | parse＋起動時適用が無い。`zorder_raw` を `spawn`／`main.rs` の起動経路へ運ぶ配線も無い | Missing |
| 5.3 1 指定＝1 グループ | KV は**後勝ちの単一値** | descript で複数グループは**作れない**（正典沈黙・登記対象） | **Constraint** |
| 6.1〜6.4 既定＝非強制の保持 | 完了 spec の要件 3 がそのまま既定 | 「グループが空なら現行と 1 命令も変わらない」ことを構造で示す形が要る | Constraint |
| 7.1 窓の出現へ追随 | `Added<WindowHandle>` パターン（`establish_owner_links:97`・`register_ghost_windows_click_through`） | 供給者を足すだけ（先例あり） | Missing（小） |
| 7.2 破棄への追随 | `detach_owner_links_for_lost_peers`・`GhostWindows::remove_entry_of`（`spawn.rs:375`） | グループ側の掃除が無い（**要件 1.4 と対＝エントリは残す／窓だけ落とす**） | Missing |
| 7.3 再表示直後の確認と是正 | **W6 申し送り⑴の未消費シーム**。`ReassertZOrder` は公開契約点として在るが挿す側が居ない | **「再表示」は Win32 の可視性遷移ではない**（`presenter/visibility.rs:69` `show_target` → `mount.set_visible`＝合成層）。窓は WS_VISIBLE のまま。**発火点は `balloon_visibility` 相の `shown` エッジ**（`balloon_visibility.rs` の `ContentDecisions.shown`） | **Unknown** |
| 7.4 省略され得る状態でも是正が届く | `tick_wake::ZORDER`（`:117`）＋`maintain.rs:373-375` | 新しい生産者を後ろに置くならその側で旗を立てる必要（doc の名指し警告） | **Constraint** |
| 7.5 全窓が背面へ回っても相対順を保つ | `zorder_pair_sink.rs`（1 巡遅延の沈降観測）が手本 | グループ版の観測／保持の扱い | Missing（小） |
| 8.1〜8.4 不正入力・失敗 | `log-first`（error!+Err）・`SkipReason` の形 | 新しい理由語彙 | Missing（小） |
| 9.1〜9.2 診断（指令と実測を同一行） | `fix_line`（`diag.rs:131`）が**まさにその形** | グループ用の行の新設 or 既存行の拡張 | Missing |
| 9.3 不可視を読み飛ばす | `measure_nearest_visible`（`:525`） | そのまま流用可 | — |
| 9.4 有界 auto-exit＋ログ grep | `AREKA_APP_SMOKE_EXIT_MS`・`RUST_LOG`（記憶 `areka-real-machine-signoff-bounded-auto-exit`） | 手順書の新設 | Missing（小） |
| 9.5 既存の観測語彙を保つ | タグ 6 種（`diag.rs:30-40`）＋`[tick]` 相名（`tick_diag.rs:132`）＋起床旗 | **改組で落とさないことが義務**（roadmap 干渉台帳 zsp⇄pwc の裏返し義務） | **Constraint** |
| 10.1〜10.3 決定論檻 | `log-capture-kit`／`temp-path-kit`（cage 着地物）・`decide_pair_fix` 檻の作法 | 純関数境界の切り方次第 | — |
| 10.4 先送り語彙檻へ新ファイルを含める | `PRODUCTION_FILES` 両肺（`:64-70` / `:48`） | 追加＋件数定数の更新 | Missing（小） |
| 11.1 位置・寸法を変えない | `pair_fix_command` が `WindowPos` 経由でフラグを自動導出＝**型の形で保証** | 同じ形を踏襲すれば自動 | — |
| 11.2〜11.3 他コマンドの扱い不変・高々 1 消費者・将来余地 | `ConsumerLedger`（粒度＝コマンド名のみ） | **`"set"` を登記すると全 `\![set,*]` の分配点になる**（→ 設計判断 #4） | **Constraint** |
| 11.4 窓状態指定を扱わない | 先送り語彙檻 9 語 | 新ファイルを檻へ入れれば自動 | — |
| 12.1〜12.4 COMPAT 記録 | §8 の 4 列表（`:122`） | z-order 行 0 件＝新規追記。誤記訂正の置き場が裁定事項 | Missing |
| 13.1〜13.4 プロパティ縮退シーム | sylphya `dotted.rs`（SET 有効群 21・件数檻 `:188`） | **何も実装しないことが要件**。記録のみ。SET 有効群 21→22 の要否は裁定 | Constraint |

---

## 3. 主要ギャップの掘り下げ

### G1. ペア語彙は 2 窓固定（要件 1／2／3 の土台）

`KeepDirectlyAbove.peer` は単数であり、doc 自身が「スコープ間には宣言を張らない」を**要件の構造的根拠**として書いている（`zorder_pair.rs:41-45`）。N 窓の列は現行語彙では表せない。
**かつ**、グループは「窓がまだ存在しなくても宣言は残る」（要件 1.4）ので、**窓 Entity に付く Component では表現しきれない**——グループの正本は Resource（あるいは areka 側の台帳）に置く必要がある。

### G2. 「1 巡 1 本」の壁（要件 1.1／7.4 の実現方式を決める）

維持系は**実測の陳腐化**を理由に 1 巡 1 本しか出さない（`maintain.rs:483-489`）。N 窓グループを素朴に組むと N−1 本要るので、次のどれかを選ぶ必要がある。

- **(a) 踏襲**: 1 巡 1 本のまま N−1 巡かけて収束させる。安全だが、120Hz でも数フレーム、**tick の門が入ると心拍まで遅れる**（要件 7.4 と相性が悪い）。
- **(b) 一括**: グループ内は**自己参照の連鎖**（`w[i]` を `w[i-1]` のすぐ背後へ）で指令を組む。**この連鎖は「測った隣」に依存しない**——挿入位置がグループ自身の HWND だからである。陳腐化するのは**先頭 1 枚の挿入位置だけ**であり、そこだけ 1 巡 1 本の規律を残せば論証は保てる。
  - 追加確認が要る（→ Research R1）: `flush` は `DeferWindowPos` の**一括投入**（`command.rs:757` `apply_as_batch`）であり、`EndDeferWindowPos` の時点で解決される。**バッチ内で連鎖する z 指令の最終順序が逐次適用と一致するか**は実測で確かめる必要がある。縮退経路（逐次適用・`:771`）では順序どおりに効く。
  - 合流（coalesce）は無害: z を動かす指令は `is_coalescible`（`command.rs:513`）が偽になるので**畳まれず、相対順も保たれる**（`REQUIRED_FOR_COALESCE` に `SWP_NOZORDER` が要る＝`:507`）。
- **(c) 混成**: グループの是正は 1 巡 1 **グループ**（グループ内は一括連鎖）。ペア機構の「1 巡 1 ペア」を「1 巡 1 まとまり」へ一般化する形。

### G3. 引き金の供給者が居ない（要件 1.3／7.1／7.3／7.5）

本番で `ReassertZOrder` を挿すのは `establish_owner_links:180` のみ。要件が求める 4 つの契機に対し、供給者は次のとおり。

| 契機 | 既存の検知点 | 状態 |
|---|---|---|
| 窓の出現（7.1） | `Added<WindowHandle>`（確立系・clickthrough 登録の先例） | **在る**（使うだけ） |
| 利用者操作で崩れた（1.3） | `WM_WINDOWPOSCHANGED`（`window_proc/window_pos.rs:41`。`WINDOWPOS.flags` の `SWP_NOZORDER` 欠落＋`hwndInsertAfter` が読める。`is_self_initiated()` でエコー判別済み `:49`） | **検知点は在るが結線ゼロ**。案 B 用に設計だけされて実装されていない |
| 破棄（7.2） | `detach_owner_links_for_lost_peers`／`GhostWindows::remove_entry_of` | 部分的に在る |
| 再表示（7.3） | **Win32 の可視性遷移は起きない**。合成層の `shown` エッジのみ | **要設計**（→ G7） |
| 全窓が背面へ（7.5） | `WM_ACTIVATE(WA_INACTIVE)` → `SinkObservationPending`（`zorder_pair_sink.rs:65`）で 1 巡遅延観測 | 観測は在る。**是正はしない**設計 |

**一石二鳥候補**（brief ⑦・roadmap 申し送り⑴）: 7.1／7.3 の供給者を本 spec が入れると、`ghost-window-zorder` が公開契約点として用意したまま未消費だったシームが埋まる。ただし**そのシームの想定（Win32 の hide/show）は現実と食い違っている**（G7）ので、埋め方は設計判断。

### G4. `\![set,…]` の消費者が居ない／台帳の粒度（要件 11.2／11.3）

- キャリアは第 1 トークンだけを名前にするので、**`"set"` の消費者は必然的に全 `\![set,*]` の入口になる**。台帳（`consumer_ledger.rs:96-105`）は名前のみの粒度なので、`try_register("set", …)` と書いた瞬間に「1 名前＝高々 1 消費者」の不変条件が **`set` サブコマンド空間全体**に対する独占を意味してしまう。
- 現時点で実コードの競合は無い（`balloonwait` は 0 件）。しかし要件 11.3 は「将来別の担当が扱える余地を残す」と明示しているので、**台帳の形か消費者の形のどちらかを決める必要がある**（→ 設計判断 #4）。
- `\![reset,zorder]` は `name="reset"` なので**別の名前**である。`"set"` と `"reset"` の 2 名を同じ消費者へ登記するのか、消費者を 2 つに分けるのかも決めどころ。

### G5. descript を読む口が無い（要件 5）

`zorder_raw` は `PlacementConfig` に載っているが、**起動経路のどこにも運ばれていない**（消費者 grep 0 件）。`main.rs:645-664` の起動窓シームで `placements`／`titles` を渡す形と同じ流儀で、`zorder_raw` から解釈したグループを `spawn_ghost_windows` の後に適用する経路を足すのが素直。
**KV は後勝ちの単一値**なので descript から作れるグループは高々 1 つ（要件 5.3 と整合するが、SSP が複数行をどう扱うかは正典沈黙 → §8 登記候補）。

### G6. 層の分界（scope を知るのは areka だけ）

`wintf → areka` の import は禁止（`zorder_pair.rs:5-7` に明記）。したがって **wintf 側は Entity の列しか受け取れない**。`sN`／`bN` → Entity の解決は areka（`GhostWindows`）で行い、wintf へは「手前から順の Entity 列」を渡す——既存 `KeepDirectlyAbove` と同じ分界であり、**トークン解釈の純関数は areka 側に置くのが一貫している**。
逆に「窓がまだ存在しないスコープ」（要件 1.4）は Entity へ解決できないので、**グループの正本は scope／窓種別のまま持ち、Entity への解決は毎巡やり直す**必要がある。これは areka 側にグループ台帳を置く強い理由になる（→ 案 B／C）。

### G7. 「可視」が 2 つある（要件 7.3／9.3 の落とし穴）

- 実測層の隣接判定は **Win32 の可視**（`is_window_visible`）で不可視窓を読み飛ばす（`zorder_pair.rs:525-553`）。
- バルーンの表示・非表示は **合成層**（`presenter/visibility.rs:134` `mount.set_visible(world, true)`／`t.visible = true`）であり、**Win32 の窓は WS_VISIBLE のまま**。`WindowStyle` を触る経路は emo-present に無い。
- 帰結:
  1. **非表示のバルーンも隣接判定では「可視の隣」として数えられる**。グループの実測照合を組むとき、これは「見えていない窓が順序に効く」ことを意味する。要件 9.3 の文言（「不可視の窓を読み飛ばす」）は Win32 の意味であり、**利用者の目に映る可視性とは別物**——ここを設計で明記しないと、実機サインオフの読み方が食い違う。
  2. **要件 7.3 の「再表示」の発火点は `balloon_visibility` 相**（`ContentDecisions.shown`）であって窓のイベントではない。areka 側の相から要求を挿す形になる（→ 設計判断 #7 の順序問題に直結）。

### G8. 記録語彙（要件 9.1／9.2／9.5）

既存のタグ 6 種は**実機サインオフの grep 判定語**であり、行の組立は純関数（`*_line`）に閉じ、マクロを呼ぶ側は `zorder_pair.rs` に置く（tracing の target が module path 既定なので分けると grep 対象が分裂する）という**明示の規律**がある。
グループ用の記録を足すときも同じ規律に従う必要がある。既存 6 種の**文言・フィールド名・出力先を変えない**ことが要件 9.5 の実体。

### G9. 干渉（W6.95 同居 4 本）

- **zsp⇄pwc**: ファイル素。**本 spec 側の義務＝`zorder_pair_maintain.rs` の起床旗 1 行（`:373-375`）と相名の語彙を改組後も保存する**。
- **zsp⇄bod**: `enqueue_window_set_pos`（`placement/follow/window_move.rs`・`SWP_NOZORDER` ハードコード）を**本 spec は触らない**を設計不変条件にする（維持系は従来どおり `SetWindowPosCommand` を直接発行）。
- **zsp⇄bvc**: sylphya 語彙表の行隣接のみ（要件 13 で行を足す場合）。
- **共有の追記先**: `doc/COMPAT_ARCHITECTURE.md` §8（各自の節のみ）・sylphya 語彙表・`file_length_guard_test.rs` の例外表（**4 spec とも触らない＝新規ファイルは 1,000 行未満で作る**）。

---

## 4. 実装アプローチの選択肢

### 選択肢 A — 既存 `zorder_pair*` を N 窓へ一般化（拡張）

`KeepDirectlyAbove.peer: Entity` を列へ広げ（あるいは `KeepOrder { below: Entity }` の連鎖へ組み替え）、維持系を「ペア」ではなく「まとまり」で回す。

- **触るファイル**: `zorder_pair.rs`（語彙・判断）・`zorder_pair_maintain.rs`（適用）・`zorder_pair_establish.rs`・`zorder_pair_diag.rs`・areka `spawn.rs`（宣言の付与）。
- ✅ 判断・実測・記録・檻が 1 箇所に集まったまま。ペアとグループの調停（要件 2.4）が**同じ関数の中**で決まるので、二重帳簿にならない。
- ✅ 既存の 8 本のテストファイル（合計 20 万字超）がそのまま回帰網になる。
- ❌ `zorder_pair.rs` は既に 51KB（1,000 行目安の直上）。**分割が前提**になる。
- ❌ **完了 spec の要件 3.1／3.4 の構造的根拠（宣言を片側単数に閉じる）を書き換える**——doc の主張と実装がずれると、既存 doc の記述が静かに嘘になる（記憶 `revise-design-not-just-requirements`）。
- ❌ 既存 8 テストファイルの前提（ペア 2 窓・1 巡 1 ペア）を広く書き換える手が要る。**pwc が観測対象として名指ししているファイル**なので、改組の規模がそのまま干渉の大きさになる。

### 選択肢 B — グループ層を新設し、ペア機構は不変（新規）

`zorder_group.rs`（語彙・純判断）／`zorder_group_maintain.rs`（適用）などを新設し、既存ペア機構には触れない。areka 側に `zorder_cue.rs`（sink）＋グループ台帳を置く。

- **触るファイル**: wintf 新設 2〜3 本＋areka 新設 1〜2 本。既存は結線 1 行（`wire_zorder_pair`）と `PRODUCTION_FILES`／件数定数のみ。
- ✅ **既存の 6 種の記録・起床旗・1 巡 1 ペアの論証をそのまま保存できる**（要件 9.5 と干渉台帳の義務が構造で満たされる）。
- ✅ 既定＝非強制の保持（要件 6.4）が「グループが空なら新 system は 1 命令も出さない」で自明に示せる。
- ✅ pwc／bod との衝突面が最小。
- ❌ **調停（要件 2.4）が 2 箇所に分かれる**——ペア機構は owner で隣接を保証し、グループ機構は順序を保証する。**両者が同じ巡に別々の指令を出すと往復しうる**（ペアの是正がグループ順を壊し、グループの是正がペア隣接を壊す）。ここを塞ぐ規則（例: グループに属する窓のペア是正は見送る／同じ巡には片方しか出さない）を明示的に設計する必要がある。
- ❌ 実測層・記録の書式を共有するための `pub(crate)` の口が要る（既存を触らずには済まない小さな露出）。

### 選択肢 C — ハイブリッド（推奨候補）

- **グループの正本と解釈は areka 側**（`GhostWindows` が scope→窓の唯一の正本であり、未出現スコープを scope のまま持てる。トークン解釈は純関数で檻に入れやすい）。
- **wintf 側は「手前から順の Entity 列」を受ける薄い語彙 1 つ＋維持系 1 本を新設**し、実測層（`measure_*`）・指令組立（`pair_fix_command` 相当）・記録の行組立は**既存の純関数を共有**する（`zorder_pair_diag.rs` にグループ用の `*_line` を足す＝タグ表の増設であって既存 6 種は不変）。
- **調停は wintf 側 1 箇所に閉じる**（brief の「グループ⇄ペアの調停は wintf 側 1 箇所」）——具体的には、**グループに属する窓についてはペアの是正を見送り、グループ側の計画がスコープ内隣接を内包して組む**。これで案 B の往復リスクを消しつつ、案 A の大改造を避ける。
- **是正の発行は G2 の (c)**: 1 巡 1 まとまり・グループ内は自己参照の連鎖で一括。
- ✅ 干渉最小・語彙保存・調停一元・檻が書きやすい。
- ❌ 「見送る」規則のぶんだけ既存 `decide_pair_fix` へ入力が 1 つ増える（グループ所属の有無）＝既存判断の純関数署名が変わる。**既存テストの機械的な追随が要る**（署名変更なので漏れはコンパイラが捕まえる）。
- ❌ 計画（3 者: sink／グループ台帳／維持系）の境界設計を先に固める必要がある。

**評価軸のまとめ**

| 観点 | A 拡張 | B 新設 | C 混成 |
|---|---|---|---|
| 既存記録語彙の保存（要件 9.5） | 危うい | 強い | 強い |
| 調停（要件 2.4）の一元性 | 強い | 弱い | 強い |
| pwc／bod との干渉 | 大 | 小 | 小 |
| 既存テストの手当て | 大 | 小 | 中 |
| 1,000 行目安 | 分割必須 | 余裕 | 余裕 |
| 「既定＝非強制」の自明性（要件 6.4） | 論証が要る | 自明 | ほぼ自明 |

---

## 5. 規模と危険度

- **Effort: L（1〜2 週間）**。新設ファイル 4〜6 本＋既存 3〜5 本への追記。判断分岐が多く（モード判定・混在拒否・重複拒否・再指定拒否・2 個未満・調停・解除・descript・解釈失敗＝要件 10.2 が 9 分岐を名指し）、**檻の本数が実装量を上回る**。
- **Risk: Medium**。
  - 技術的な未知は少ない（Win32 の `SetWindowPos` 連鎖・既存の実測層・既存の sink パターンがすべて実在）。
  - 危険の本体は**既存の設計判断との衝突**——1 巡 1 本の規律・owner の構造保証・完了 spec の要件 3・「可視」の二重定義。**いずれも「コードは動くが doc が嘘になる」種類**であり、記憶 `revise-design-not-just-requirements`／`doc-claims-need-file-line-verification` が名指しする形。
  - 実機側の危険は低い（位置・寸法を動かさない・常時最前面を指さない・owner を張り替えない、が型の形で守られている限り）。

---

## 6. Research Needed（design フェーズへ持ち越す）

| # | 項目 | なぜ要るか |
|---|---|---|
| R1 | **`DeferWindowPos` 一括投入の内側で、複数の z 指令を連鎖させたときの最終順序**（`command.rs:757` `apply_as_batch`）。逐次適用（`:771`）と一致するか | G2 (b)/(c) の前提。一致しないなら 1 巡 1 本へ倒すか、グループだけ縮退経路を通すかの分岐になる |
| R2 | **owner 関係とグループ順の相互作用**。⑴ 別スコープの窓を owner 一組（キャラ＋バルーン）の**間**へ挿せるか ⑵ 利用者がキャラ窓をクリックして owner ごと浮上したとき、グループ順は何巡で戻るか | 要件 1.2／1.3／2.4 の実現可能性そのもの。案 A の G6 保証が**グループの敵にもなる** |
| R3 | **SSP の「再ペアはエラー」の実挙動**（画面に何か出るのか・黙って無視か・グループはどうなるか） | 要件 3.2 は「全体不採用＋記録」で確定済みだが、§8 へ「areka 裁量」として登記する際の根拠になる |
| R4 | **`\![reset]`（引数なし）の正典上の位置づけ**。ukadoc の `set,zorder` 本文は「組み合わせを変える場合は `\![reset]` タグを使ってからやり直す」と書くが、別項は `\![reset,zorder]`（2.3.77）である | 要件 4 は `\![reset,zorder]` のみ In scope・引数なし `\![reset]` は Out of scope。**正典の記述が食い違っている**ことを §8 へ登記する候補 |
| R5 | **descript に `seriko.zorder` が複数行あるときの SSP 挙動** | areka の KV は後勝ち＝1 行しか残らない（`config.rs:133`）。正典沈黙なら §8 登記 |
| R6 | **明示モードで同一スコープの `sN`／`bN` が非隣接に並ぶ指定**（例 `b1,s0,s1,b0`）の扱い | 要件 2.4 は「キャラをバルーンより手前」の反転だけを名指しする。**間に他スコープが挟まる形**は文面が沈黙している |
| R7 | **`balloon_visibility` の `shown` エッジを z 順の引き金にしてよいか**（完了 spec の相の内側から wintf の要求を挿す形） | 要件 7.3 の唯一の現実的な発火点。相の順序と起床旗（要件 7.4）に直結 |

---

## 7. design フェーズへの推奨

1. **境界を 4 つに切る**: ⑴ トークン解釈（純関数・areka・実機不要）⑵ グループ台帳（areka・scope／窓種別のまま保持・Entity 解決は毎巡）⑶ 是正の計画と適用（wintf・既存実測層と指令組立を共有）⑷ 記録（既存 `*_line` の隣へ増設）。**⑴⑵は wintf を 1 行も触らずに檻へ入る。**
2. **調停は 1 箇所に閉じる**（グループ所属の窓についてはペア是正を見送り、グループ計画がスコープ内隣接を内包する）。二重帳簿を作らない。
3. **起床旗と相名を先に固定する**——改組の前に「保存すべき既存語彙」の一覧（タグ 6 種・`ZORDER` 旗・`SCHEDULE_NAMES`）を design に明記し、着地時に逐語で照合する。
4. **檻は 9 分岐（要件 10.2）＋調停＋グループ収束**を最小単位に。純関数を先に決めれば全部が実機不要で書ける。
5. **1,000 行**: 新設ファイルはすべて 1,000 行未満で作る（`file_length_guard_test.rs` の例外表は 4 spec とも触らない約束）。
6. **file:line は design 直前に再実測**（本ワークツリーは 4 本並走中）。

---

## 8. 設計判断項目（要件ディスカッションへ）

> いずれも「情報と選択肢」であり、決定ではない。

1. **是正の発行戦略**（G2）: (a) 1 巡 1 本を踏襲し N−1 巡で収束／(b) グループ内は自己参照の連鎖で 1 巡一括／(c) 1 巡 1 グループ・グループ内は一括。**要件 7.4（省略され得る状態でも是正が届く）とどこまで両立させるか**が判断の軸。R1 の実測結果に依存する。
2. **グループ語彙の住処**（G1／G6・選択肢 A/B/C）: 既存 `zorder_pair*` を N 窓へ一般化するか、グループ層を新設して既存を不変に保つか、混成か。**完了 spec の doc（「スコープ間には宣言を張らない」）をどう扱うか**が付随する（書き換える／§8 で上書きする／既定状態の記述として残す）。
3. **調停の方式**（要件 2.4）: 案 A の owner 制約により「キャラをバルーンより手前」は**そもそも OS が許さない**可能性が高い（R2）。この場合、要件 2.4 は「拒否して記録する」ではなく「**構造的に起き得ないことを記録で示す**」になる。実装として何を書くか（明示的な検査を置くか、owner の保証に委ねて記録だけ残すか）。
4. **`consumer_ledger` の粒度**（G4・要件 11.3）: (i) `"set"` を 1 消費者へ登記し、その消費者が第 1 引数で内部分配する／(ii) 台帳のキーを「名前＋第 1 引数」へ拡張する／(iii) `zorder` 専用の消費者を作り台帳には `"set"` を登記しない（自己選別のみ）。**将来 `\![set,*]` の別サブコマンドを別担当が扱う余地**をどう残すか。`"reset"` の扱い（同一消費者か別か）も同時に決まる。
5. **`\![reset,zorder]` の記録水準と `\![reset,他]` の扱い**（要件 4.4）: 担当外として黙って読み飛ばす（現行の良性スキップと同じ debug 水準）か、`reset` の名前は自分宛として warn を出すか。move_cue の「宛名規律 D8④」（自分宛の壊れ物は warn・他人宛は debug）が先例。
6. **descript の適用点と失敗時の水準**（要件 5.1／5.4）: `zorder_raw` を `main.rs` の起動窓シームで解釈するか、`spawn_ghost_windows` の後の相で解釈するか。解釈失敗は warn か error か（要件 5.4 は「起動を継続」のみ規定）。また **descript で複数グループを作れない**（KV 後勝ち）ことを縮退として登記するか、正典沈黙として §8 へ入れるか（R5）。
7. **要件 7.3／7.4 の発火経路と相の順序**（G3／G7）: 「再表示」は合成層の `shown` エッジであり、発火点は `emo2_frame_system` の内側になる。**`FrameFinalize` 内で `emo2_frame_system` と `(establish, maintain).chain()` の相対順は現在未拘束**。⒜ 順序を明示して維持系を後ろへ置く／⒝ 生産者側で起床旗を立てる（`maintain.rs` の doc が名指しする作法）／⒞ 両方。**1 巡遅延を許容するかどうか**が要件 7.3 の「表示された直後」の解釈に効く。
8. ~~**要件 9.3 の「不可視」の定義**（G7）~~ **→ 裁定済み（2026-08-27 要件ディスカッション議題 1）**: Win32 基準で確定。絵を消しただけのバルーン窓は実測上「可視」として数える。要件 9.3／7.3 に注記反映済み。
9. ~~**要件 12.3 の訂正の置き場**~~ **→ 裁定済み（2026-08-27 要件ディスカッション議題 2）**: COMPAT §8 への訂正行で上書き・完了アーカイブは書き換えない（scg 先例踏襲）。要件 12.3 に反映済み。
10. ~~**要件 13 の sylphya への露出**~~ **→ 裁定済み（2026-08-27 要件ディスカッション議題 3）**: 語彙表には触れない（名前だけの先行登録はしない）。追跡先の実在検証で拾い手ゼロと判明したため、追跡 spec `areka-P0-zorder-property` を即日起票（brief＋roadmap M2 ゲート棚 6 本目）。要件 13.3〜13.5 に反映済み。bvc との語彙表行隣接ウォッチは消滅。
11. **グループの上限と性能**（要件 3.7 / Out of scope の「性能最適化の計測はしない」）: 上限を設けないと、1 グループ N 窓 × M グループで 1 巡に出す指令数が増える。**「常識的に減らす」の下限（例: 既に順序どおりなら 1 本も出さない同値ガード）をどこまで設計で書くか**。既存 `decide_pair_fix` の同値ガード（`zorder_pair.rs:346`）が手本。
12. **要件 1.7（実行スコープに依らない）と `cue.actor`**: `move` は `cue.actor` を scope として使う（`move_cue.rs:522`）が、本 spec は actor を**見ない**。同じキャリアで挙動が違うことを、消費者の doc とテストで明示的に固定するかどうか。

---

## 9. 設計前再実測（2026-08-27）とシンセシス結果

### 9.1 ドリフト訂正（§1〜§3 の記述に対する実測差・design.md はこちらを正とする）

| # | 本文の記述 | 実測 |
|---|---|---|
| ① | 収束の論証は `zorder_pair.rs:40-59` | **`zorder_pair_maintain.rs:40-59`**（前提 2 つは `:56-59`） |
| ② | 確立系は `Added<WindowHandle>` 駆動 | `Query<Option<Ref<WindowHandle>>>`＋`Ref::is_added`（`establish.rs:107,118-126`） |
| ③ | 記録タグは 6 種（`owner-detached` 含む）＋`sink-observed` | **`[zorder-pair]` タグは 6 本＝`owner-established`/`fix`/`skip`/`verify-failed`/`owner-establish-failed`/`sink-observed`**。`owner-detached` はタグ無しの素文（warn） |
| ④ | `window_proc/window_pos.rs` | `crates/wintf/src/ecs/window_proc/window_pos.rs`（`ecs/` 直下） |
| ⑤ | `WINDOWPOS.flags`／`hwndInsertAfter` が「読める＝検知点は在る」 | **現行が読むのは `wp.x/y/cx/cy` のみ**（`flags`/`hwndInsertAfter` 参照 0 件）。z 変化検知は新規実装になる |
| ⑥ | `apply_as_batch`（`command.rs:757`） | 定義は `:385-446`・`:757` は呼出・縮退は `:771`（`BatchDegrade` 3 種のみ） |
| ⑦ | `tick_diag.rs:132` | `:131` |
| ⑧ | `KeepDirectlyAbove` 付与 `:532-533` | `:531-533` |
| ＋ | `Emo2Wiring::new` は `mod.rs:458` | 定義は `frame/wiring.rs:122`・sink 追加は**5 点**（`new` 署名変更を含む） |
| ＋ | 記録水準の正典は steering `logging.md` | `logging.md` にあるのは水準表（`:23-29`）のみ。良性スキップ debug／不正入力 warn の実質規範はコード先例（宛名規律 D8④＝`move_cue.rs:489-505`・`command.rs:764` の「縮退は無音にしない」・`SkipReason` の型強制） |

### 9.2 R1 は解決済み（§6 の表を更新）

`DeferWindowPos` 一括投入は **enqueue 順を保存し、逐次適用と最終 Z 形が一致する**。実窓の対照テスト `crates/wintf/src/ecs/window/command_batch_tests.rs:633`（並べ替えに敏感な 2 連鎖で両経路 `[2,0,1]` 一致）が既に緑。→ G2 の「グループ内自己参照連鎖の 1 巡一括」は flush 側の前提が成立している。残る Win32 側未知は R2（owner 一組の間への挿入不可＝記憶と一致・設計は挟む指令を出さない形で回避）のみ。

### 9.3 シンセシス（設計判断 §8 の #1〜#7・#11・#12 の決着）

1. **是正発行戦略＝(c) 1 巡 1 グループ・グループ内は自己参照連鎖で一括**（根拠: 9.2。先頭窓は動かさないので実測陳腐化と無縁）。ペア機構との調停は「同巡にペア是正が出たらグループは見送り」（`Added<IssuedPairFix>` 検知）＝1 巡に窓を動かす系統は 1 つ。
2. **語彙の住処＝案 C 確定・ただし既存 `zorder_pair*` 5 ファイルは 1 行も編集しない**（`pub(crate)` 共有で足りることを再実測で確認）。`decide_pair_fix` の署名変更も不要（グループが自前でスコープ内隣接を内包するため）。要件 9.5 が構造で成立。
3. **調停方式＝スコープブロック正規化**: 数値モードは `[Balloon, Char]` 展開＝明示モードの特例という一般化。反転（2.4）も非隣接（R6）も「先出現位置へ隣接ブロックとして寄せ、調整を記録」の 1 規則で処理。
4. **consumer_ledger＝キーを（名前＋選別子）へ拡張**し `("set","zorder")`／`("reset","zorder")` を登記（11.3 の将来余地を型で保持）。
5. **`\![reset,他]`＝debug スキップ**（宛名規律 D8④・他人宛）。
6. **descript 適用点＝main.rs 起動シーム（spawn 後・最初の FrameFinalize 前）・失敗は warn**。
7. **発火経路＝pending 1 ビットに統一**（sink 送出・drain 書込・shown エッジ・外部由来 WINDOWPOS 変化の 4 供給者が同じ pending を立てる）＋維持系が pending 中は毎巡 `ZORDER` 旗（相順序の拘束は追加しない＝1 巡遅延を許容し 7.4 の旗で補償）。**1.3 の検知は flags 解析をやめ「外部由来変化→再検証」に単純化**（ドリフト⑤で新規実装と判明したため・同値ガードが空振りを 0 本で吸収）。
11. **同値ガード必須**（相対順成立中は指令 0 本）＋ verify 連続失敗 3 回で warn を出して pending を降ろす頭打ち（8.3 の「黙って諦めない」を記録で満たす）。
12. **actor 非参照は doc＋決定論テストで固定**（自己選別表に actor 変化不変のケースを含める）。

Build vs Adopt: 新規外部依存なし・既存資産（measure_*・flush・mpsc drain 型・diag 規律・log-capture-kit）を全面採用。Simplification: `decide_pair_fix` 改変案・WINDOWPOS flags 解析案・sylphya 語彙先行登録案を棄却（いずれも要らないことが再実測で確定）。

---

## 10. 差し戻しの根拠（2026-08-29）— R2 の答えが正典実装にあった

> 本節は実装完走・完成検証 NO-GO の後に追記した。**§6 の R2 と §9.2 の「R2 は推論で閉じた」という記述は消していない。**
> 実測せずに閉じたこと自体がこの spec の最大の教訓であり、その物証を残すため。

### 10.1 何が起きたか（要約）

全 22 タスクを実装し機械検査は全緑（workspace 6,120 passed）だったが、**実機で要件 1.1／1.2／2.1 が成立しなかった**。
症状＝窓 4 枚の連鎖が 1 ミリも動かない。`SetWindowPos` は毎巡 `ok=true`、4 枚とも解決済み（`missing=0`・`scan_complete=true`）、
それでも重なりは起動時のまま。窓 2〜3 枚の指定は成立する。詳細は `real-machine-signoff.md` §4。

数値モードは 1 スコープが必ず「バルーン窓・キャラ窓」の 2 枚へ展開されるため、2 スコープ指定は必ず窓 4 枚になる。
**したがって正典の主用法 `\![set,zorder,0,1]` と `seriko.zorder,0,1` が全滅する。**

### 10.2 SSP の窓の親子構造（開発者提供・2026-08-29）

正典実装 SSP のプロセス内の窓木（窓一覧ツールの表示を転記）。ゴースト 2 体（Emily＋Teddy）が立っている状態:

```
ssp.exe : 8848
└ Thread : 2264
  ├ 0x00000000002A07B0  "Emily/Phase4.5"                        Afx:400000:3:10003:6:0
  └ 0x00000000003E0AE6  "Emily/Phase4.5 "the Slapstick Beauty"" TMasterForm
    ├ 0x0000000000290AA2  "Emily"                               Tmainform
    │ ├ 0x00000000000B0722  "SSPInputBoxBG"                     SSPInputBox
    │ └ 0x0000000000350BF4  "Emily/Balloon"                     Tmessageform
    │   ├ 0x0000000000440D86  "Teddy"                           Tkeroform
    │   │ ├ 0x0000000000280D42  ""                              tooltips_class32
    │   │ └ 0x00000000003C0A64  "Teddy/Balloon"                 Tmessageformu
    │   └ 0x0000000000220C1C  ""                                tooltips_class32
    └ 0x00000000001603BC  ""                                    tooltips_class32
```

**中核は 1 本の鎖である**（tooltips と入力箱は枝葉）:

```
TMasterForm ← Tmainform("Emily") ← Tmessageform("Emily/Balloon") ← Tkeroform("Teddy") ← Tmessageformu("Teddy/Balloon")
   最も奥                                                                                        最も手前
```

**この入れ子は WS_CHILD の親子ではなく owner（所有者）の鎖である。** 根拠＝WS_CHILD の子窓は親のクライアント領域へ
切り取られ親相対座標になるが、SSP のバルーンは画面上を独立に動く最上位窓である。また `T*` 接頭辞は Delphi の
`TForm` 系であり、Delphi は `PopupParent` プロパティで owner を設定する。**ただし転記元ツールが owner 入れ子と
parent 入れ子のどちらを描くかは未確認**であり、実装前に `GetWindow(GW_OWNER)` で 1 度確かめること。

### 10.3 これが R2 の答えである

§6 の R2 は「**別スコープの窓を owner 一組（キャラ＋バルーン）の間へ挿せるか**……要件 1.2／1.3／2.4 の
**実現可能性そのもの**。案 A の G6 保証が**グループの敵にもなる**」と問うていた。§9.2 はこれを実測なしに
「挟む指令を出さない形で回避」と閉じた。

**正典の答えは「挿さない」ではなく「そもそも SetWindowPos で並べない」だった。**

owner は「所有される窓は必ず所有者より手前」を OS が強制する。これを**一直線の鎖**にすると、分岐が無いので
兄弟が存在せず、**深いほど手前という全順序が構造的に決まる**。SSP は z 順を毎巡直しているのではなく、
**z 順を owner の鎖として書いている**。あとは OS が維持する——観測も、差分是正も、活性化との追いかけっこも要らない。

> ⚠ 2026-08-29 の対話中に「owner は半順序しか与えない（兄弟の順序が未定）」と述べたのは**誤り**である。
> それは星形（1 owner に複数がぶら下がる）の話で、**鎖形なら全順序になる**。

### 10.4 areka の現行構造との差

areka の owner はスコープごとに独立した対であり、**島が 2 つある**:

```
s0 ← b0        s1 ← b1        ← 島どうしを繋ぐ鎖が無い
```

島の内側の順序は OS が保証する（`design.md:59` の「スコープ内隣接は OS が構造保証する」）。
ところが**島どうしの順序を決める構造が存在しない**ため、そこだけ `SetWindowPos` の連鎖で埋めようとした。
島を跨いで窓を差し込むたびに OS の不変条件が押し戻す——**構造で決まっていないものを、構造と喧嘩しながら手で押さえていた。**

観測との整合: 3 枚 `b0,s0,s1` が成立し 4 枚 `b0,s0,b1,s1` が失敗したのは、3 枚が 2 つ目の島の
バルーンを駒にしていない（島を 1 つしか崩さない）ため。**ただし「`s1` を動かすと `b1` が引きずられて
前段を壊す」という機構の説明は仮説であり未実測**（`real-machine-signoff.md` §4.3 の候補 A／B は切れていない）。

### 10.5 【訂正】要件 9.5 は「5 ファイルの凍結」ではない

要件 9.5 の文面は「**既存の観測記録（重なり関連の記録名、および処理の相を示す記録の語彙）を、
隣接する仕様が読み続けられる形で保つ**」であり、ファイルの編集禁止ではない。
**既存ペア機構 5 ファイルの凍結は、9.5 を構造で保証するために設計が選んだ手段（案 C）である**
（§9.3 項目 2・`design.md:208`・task 6.1）。

したがって **owner の鎖化は要件 9.5 と両立しうる**——記録の語彙さえ保てばよい。
ただし鎖化はペア機構の owner 確立部を必ず触るので、**案 C そのものは撤回になる**。

### 10.6 要件のうち見直しが要る項（実装非依存の項は据え置き）

| 項 | 現行の文面 | 鎖化した場合 |
|---|---|---|
| **7.4** | 表示に変化が無い間の処理が省略され得る状態でも「**是正が適用されるまで処理の実行を促す**」 | **成立不能**。促すべき毎巡の処理が存在しなくなる |
| **9.2** | 「**是正の指令と、その直後に実測した重なり**とを同一の記録行に含める」 | **成立不能**。指令→次巡検証という往復が無くなる |
| 8.2 | 是正が「**実行環境側の理由で失敗**した場合」記録し異常終了させない | 文面は generic で生き残るが、失敗の種類が「鎖の張り替え失敗」へ変わるので見直し推奨 |
| 9.1 | 「どの窓を、どの窓の**すぐ手前へ移した**か」 | 意図は生きるが「移す」→「鎖を張り替える」の言い換えが要る |

**据え置いてよい項（実現方法に依存しない）**: 要件 1（1.1〜1.7）・2・3・4・5・6・7.1／7.2／7.3／7.5・
8.1／8.3／8.4・9.3／9.4／9.5・10・11・12・13。**63 本中およそ 59 本が逐語で生きる。**

**鎖化は要件を強くする**: 1.3（利用者操作からの復帰）と 7.5（背面回りでの保持）は、現行では「毎巡直す」で
近似している約束が、OS の不変条件として**構造的に成立**する。

### 10.7 実装の生死（次の設計への引き継ぎ）

| 生き残る（areka・「何を望むか」を決める側） | 退役する（wintf・「どう強制するか」の側） |
|---|---|
| トークン解釈と拒否 4 分岐（`placement/zorder_group_ledger.rs`） | 観測と是正要否の純判断（`wintf/.../zorder_group.rs`） |
| スコープ内隣接への正規化 | 連鎖発行とペア機構との調停（`zorder_group_maintain.rs`） |
| 台帳の状態遷移（追加・再指定拒否・解除・descript 基底） | 次巡検証・グループごとの頭打ち |
| タグの自己選別と指令送出（`emo2_boot/zorder_cue.rs`） | 是正が適用されるまでの起床促し |
| 担当登記の粒度拡張（`consumer_ledger.rs`・名前＋選別子） | 実窓での連鎖適用テスト |
| shell 設定の読み取りと基底適用（`frame/zorder_descript.rs`） | 追随トリガ 2 点（`window_pos.rs`・`balloon_visibility_phase.rs`） |
| 窓への射影（存在する窓だけを並べる） | 維持系の結線（`spawn.rs` のチェーン末尾） |
| 記録語彙のうち `applied`／`rejected` | 記録語彙のうち `fix`／`skip`／`verify-failed` |

### 10.8 そのまま再利用できる資産（**破棄しないこと**）

- **実機サインオフ一式**＝`signoff-procedure.md`（手順・判定語・検体の作り方・較正 13 通り）／
  `real-machine-signoff.md`（受入記録・§1.1 に全件突合表）／`signoff-scan.ps1`（機械判定・終了コード 0/1/2/3）。
  **新しい設計をそのまま測れる。** 判定語のうち `fix`／`verify-failed` は語彙が変われば差し替えが要る。
- **`tasks.md` の申し送り台帳 60 項目超**——道具の罠 10 件を含む。次の実装が同じ穴に落ちないための地図。
- **分岐網羅の対応表**（`placement/zorder_group_branch_coverage_tests.rs`）——解釈側の 10 分岐は生き残るので大半が有効。
- **先送り語彙の名簿**（両側 `PRODUCTION_FILES`）と**先送りプロパティの固定**（`zorder_property_deferral_tests.rs`）——設計に依存しない。
- **COMPAT §8 の 12 行**——裁量 9 件の大半は解釈側の裁定なので生き残る。実機の現況の 1 行だけ是正後に書き換える。

### 10.9 次の設計フェーズが実測すべきこと（R2 の再オープン）

1. 転記元ツールの入れ子が owner か parent か（`GetWindow(GW_OWNER)` で確認）
2. **実行時に owner を付け替えられるか**（`SetWindowLongPtr(GWLP_HWNDPARENT)`）。Microsoft は正式には推奨していないが SSP は現にやっている
3. 付け替えの副作用——最小化・破棄の連動、タスクバーへの出方、活性化のまとまり方
4. 鎖の張り替え手順（N 窓の鎖を組み替えるとき、どの順で付け替えれば途中状態が壊れないか）
5. グループ解除時に鎖を元へ戻す手順（既定＝非強制へ戻す・要件 6）
6. **窓の出現・破棄で鎖が切れたときの繋ぎ直し**（要件 7.1／7.2）

### 10.10 やってはいけないこと（実測で潰した選択肢）

- **活性化を全部拾って毎巡並べ直す**——引き金は既に `WM_WINDOWPOSCHANGED` の非エコー全部を拾っており、
  実機で 24 回やり直して 24 回失敗している。**引き金の数ではなく指令の中身の問題。**
- **`HWND_TOP` の絶対指定**——`Ok(())`・`GetLastError()==0` のまま黙って断られる（task 7.1 の実測・32 回繰り返しても回復しない）。
  相対指定なら 9/9 成功。
- **owner を星形に増やして順序を表現する**——1 owner に複数がぶら下がると兄弟の順序が未定になり全順序にならない。**鎖であること**が要件。

---

## 11. ギャップ分析（要件改訂第 2 版・2026-08-29）

> 対象: 改訂第 2 版の要件（要件 14＝所有の鎖による構造維持）と、v1 実装が着地済みの現行コードベースとの差分。
> §10.7 の生死表を、実ファイルの再実測で裏取りしたうえで実装戦略の選択肢へ落とす。

### 11.1 現状資産の実測（Requirement-to-Asset Map）

**owner 原始命令は既に在り、実行時付け替えは本番で現用・実証済み**:

| 資産 | 場所 | 状態 |
|---|---|---|
| `set_window_owner`（`SetWindowLongPtrW(GWLP_HWNDPARENT)`） | `crates/wintf/src/api.rs:141` | `pub(crate)`・実窓ラウンドトリップ檻あり（`api.rs:586`） |
| `clear_window_owner` | `crates/wintf/src/api.rs:152` | 同上 |
| ペア確立＝**窓が出揃った後に実行時で** owner を張る | `zorder_pair_establish.rs:169`（`set_window_owner(balloon, char)`） | 本番現用。「実行時 owner 設定は動く」の実証（ただし**初期確立時**であり、表示中の窓の**張り替え**は未実証＝Unknown ②） |
| ペア切離＝破棄経路で owner を外してから壊す | `zorder_pair_maintain.rs:286` | 本番現用。鎖のスプライスの雛形 |
| 生成経路は owner を使わない設計 | `window_factory.rs:149-154`（`CreateWindowExW` の hWndParent を意図的に不使用） | 鎖も「生成後に張る」パターンで整合 |

**要件別の対応**（Missing＝新造・Unknown＝要実測・Constraint＝制約）:

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| 1〜5・8.1・8.3・8.4（解釈・台帳・descript） | `placement/zorder_group_ledger.rs`（`parse_zorder_tokens`・`ZOrderGroupLedger`＝追加/拒否/解除/descript 基底/版数）・`emo2_boot/zorder_cue.rs`・`frame/zorder_descript.rs`・`zorder_drain.rs`・consumer_ledger 拡張 | **ほぼ無傷で生存**（§10.7 どおり。実現方式に依存しない） |
| 14.1／14.4（鎖の構成） | owner 原始命令＋ペア edge | **Missing: 鎖合成モジュール**（グループ→鎖 edge 列の導出と適用） |
| 14.2／14.3／7.4（反復是正の禁止） | — | v1 維持系の**退役**で満たす（下記 11.4） |
| 4／6（解除・既定復帰） | `clear_window_owner`・ペアは無傷 | **Missing: 横断 edge の撤去**＝撤去だけで島 2 つ＝既定状態が復元される（後述 11.2） |
| 7.1／7.2／14.5（出現・破棄での組み替え） | ペア切離の雛形 | **Missing: スプライス**（Unknown ④: 途中状態が壊れない張り替え順） |
| 8.2／11.5（失敗時・副作用封じ） | — | **Unknown ③**: 横断 edge の副作用（最小化・破棄の連動が鎖を伝うか） |
| 9.1／9.2（診断記録） | `zorder_group_diag.rs` の語彙のうち `applied`／`rejected` は生存 | **言い換え**: `fix`／`skip`／`verify-failed` は退役、鎖の組み替え語彙を新設。`signoff-scan.ps1` の判定語も差し替え（§10.8） |
| 9.3／9.4（実測・サインオフ） | 可視隣判定の実装・サインオフ一式・`GetWindow(GW_HWNDNEXT)` 歩行器（テスト 3 箇所） | 生存・流用 |
| 10（決定論檻） | 分岐網羅表（解釈側 10 分岐は有効）・実窓 owner 檻 5 本（establish/detach/survivor/multipair/always_on_top） | 鎖導出の純関数檻を**追加**。v1 の是正判断檻は退役 |
| 13（先送りシーム） | `zorder_property_deferral_tests.rs`・両側 `PRODUCTION_FILES` 名簿 | 生存（名簿はファイル増減に追随） |

### 11.2 構造上の発見——ペア edge は鎖の部分列である

正規化済みのグループ順（要件 1.2／2.4／6.3＝各スコープでバルーンがキャラの直上）では、鎖は必ず

```
（手前）bN ← sN ← bM ← sM（奥）     ＝ 例: グループ「1,0」→ b1 ← s1 ← b0 ← s0
         └ペアedge┘ └横断edge┘ └ペアedge┘
```

の形になる。**スコープ内 edge（b が s に所有される）は既存ペア機構がまさに今張っているものと同一**であり、
鎖に必要な新規 edge は**横断 edge（後続スコープのキャラ窓を、手前スコープのバルーン窓が…ではなく、
手前側の s を奥側の b に所有させる 1 種類）だけ**である。しかも横断 edge の書込先はキャラ窓＝**現在 owner を
持たない窓**なので、ペア機構の書込と衝突しない。

帰結:
- **鎖の構成＝横断 edge の追加のみ**・**解除＝横断 edge の撤去のみ**（撤去すれば島 2 つ＝既定状態が構造的に復元・要件 6）
- §10.5 は「鎖化はペア機構の owner 確立部を必ず触る」としたが、**再実測の結果これは過剰だった**——正規化済みの鎖では
  ペア 5 ファイルは無編集で済む可能性が高い（要調整は破棄経路の連携のみ。ペア切離が走る前に鎖のスプライスを済ませる順序）

### 11.3 要件間の緊張 1 件（設計ディスカッションで裁定を要する）

**要件 2.5「明示モードのグループに属さない窓を動かさない」と要件 14.1 の両立**——バルーンだけの部分グループ
（例 `b0,b1`）を鎖で固定するには、b0 の既存 owner が s0 である以上、鎖が s0 を経由せざるを得ず、グループ外の
s0 の相対順が拘束される。選択肢: (a) スコープ相棒を暗黙のブロック要素として畳み込み、調整を記録する
（2.4 の調停・12.2 の裁量と同型）／(b) スコープをまたぐ部分グループを拒否する（正典の例示は常にペア込みであり
沈黙領域）。**いずれも裁量登記が必要**。要件の再改訂までは不要（2.5 は 2.4 型の調停として読める）が、
設計フェーズ冒頭で裁定し 12.2 の登記対象へ加えること。

### 11.4 退役対象の棚卸（前進コミットで実施・履歴の巻き戻しはしない）

**実装側**: `wintf/ecs/window/zorder_group.rs`（観測と是正要否の純判断）・`zorder_group_maintain.rs`（連鎖発行と
ペア調停）・`zorder_group_diag.rs` の一部語彙・引き金 3 点（`window_proc/window_pos.rs`・`world/tick_wake.rs`・
areka `balloon_visibility_phase.rs`）・結線 2 点（`frame/wiring.rs`・`placement/spawn.rs` 末尾）。
**檻側**: `zorder_group_decision/maintain/order/verify/wake_tests`・`window_pos_zorder_group_tests`・
`spawn_zorder_group_wiring_tests`・`balloon_visibility_phase_zorder_group_tests`（歩行器と Z_SETUP 手法は
鎖檻の雛形として流用価値あり）。
**制約**: 語彙退役は要件 9.5 の範囲で行う（`applied`／`rejected` は生存・隣接仕様が読む語彙は保全）。
両側 `PRODUCTION_FILES` 名簿と分岐網羅表はファイル増減へ追随させる（名簿倒れ防止の檻が既に赤で教える）。

### 11.5 実装アプローチの選択肢

**Option A（推奨）: ペア edge 温存＋横断 edge モジュール新設（ハイブリッド）**
- 新設: 鎖合成の純関数（グループ列＋窓の在庫→横断 edge 列）＝areka 側 or wintf 側の 1 モジュール、
  適用系（`set_window_owner`／`clear_window_owner` 呼出・イベント応答・14.5）、スプライス（出現・破棄）
- 生存: 解釈側全部・ペア 5 ファイル（ほぼ無編集）・診断とサインオフの骨格
- ✅ 変更面積が最小・実証済みコードを壊さない・解除＝撤去だけで既定復帰が構造的
- ❌ ペア破棄経路との順序調整が 1 点残る・部分グループの裁定（11.3）が要る
- 規模 **M**・リスク **Medium**（Unknown ②③が残る間は。実測で潰せば Low 寄り）

**Option B: 鎖モジュールへ全 edge を一元化（ペア機構も退役）**
- ✅ owner の書き手が 1 箇所になり調停消滅
- ❌ 実証済みペア機構と檻 5 本を壊す・要件 9.5 の語彙保全が難しくなる・変更面積が跳ねる
- 規模 **L**・リスク **Medium-High**。Option A で調停が実際に複雑化した場合の避難先

**Option C: スコープ単位は鎖・部分グループのみ v1 是正を残す**
- ❌ **不可**。NO-GO の根因（反復是正）を持ち帰り、要件 14.2 に正面から違反する。列挙のみ

### 11.6 設計フェーズへ持ち込む Research Needed（§10.9 を実測で更新）

1. SSP 窓木の入れ子が owner であることの確認（`GetWindow(GW_OWNER)`）——§10.9①・未着手
2. **表示中の窓の owner 張り替えが z 順へ即時反映されるか**——ペア確立は「出揃った直後」のみ実証済み。
   反映が活性化待ちなら、張り替え直後の 1 回だけの後押し（イベント応答・14.5 適合）で足りるかを実測
3. **横断 edge の副作用**——最小化・非表示・破棄の連動が鎖を伝搬するか（11.5／7.2 のリスク）。
   破棄はペア切離の雛形（先に外してから壊す）で封じられる見込み・最小化は要実測
4. スプライスの安全な順序（N 窓の組み替えで途中状態が壊れないこと・8.2）
5. 活性化のまとまり方——鎖の 1 窓をクリックすると鎖全体が他アプリより手前へ出るか（グループ有効中は
   ピン留めの帰結として許容見込みだが、実測して 11.5 の「クリックへの反応を変えない」との線引きを確定）
6. 既存 always_on_top 檻との交差（11.4 の対象外領域を鎖が壊さないこと）
7. 要件 1.3 の是正経路——構造維持下では条件がほぼ発火しない。外部要因（他プロセスによる owner 書換等）で
   鎖が壊された場合の検知と繋ぎ直しを持つか、持つ場合の引き金（イベント応答に限る・14.2 適合）を設計で裁定

### 11.7 勧告

- **Option A を推奨**。設計は §11.6 の 2・3 を**最初に**実測する檻（表示中の実窓 2〜4 枚で張り替え→
  `GetWindow(GW_HWNDNEXT)` 歩行器で読み戻し・最小化/破棄の連動プローブ）から書き始めること
- 11.3 の裁定（部分グループ）を設計ディスカッションの議題に載せること
- 退役は新設計の確定後に前進コミットで行い、語彙は 9.5 の檻の緑を保ったまま差し替えること
