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
沈黙領域）。**裁定済み（2026-08-29 要件ディスカッション）＝(a) 畳み込み**。要件 2.5 の射程を「グループに属するスコープ以外」へ
明確化し、要件 2.6（相棒窓の暗黙の畳み込みと記録）を新設・12.2 の裁量一覧へ登記した。根拠: バルーン直上の
既存不変条件により初版方式でも相棒窓は同じ位置に拘束されており、見える結果が変わらない。

### 11.4 退役対象の棚卸（前進コミットで実施・履歴の巻き戻しはしない）

**実装側**: `wintf/ecs/window/zorder_group.rs`（観測と是正要否の純判断）・`zorder_group_maintain.rs`（連鎖発行と
ペア調停）・`zorder_group_diag.rs` の一部語彙・引き金 3 点（`window_proc/window_pos.rs`・`world/tick_wake.rs`・
areka `balloon_visibility_phase.rs`）・結線 2 点（`frame/wiring.rs`・`placement/spawn.rs` 末尾）。
**檻側**: `zorder_group_decision/maintain/order/verify/wake_tests`・`window_pos_zorder_group_tests`・
`spawn_zorder_group_wiring_tests`・`balloon_visibility_phase_zorder_group_tests`（歩行器と Z_SETUP 手法は
鎖檻の雛形として流用価値あり）。
> **訂正（2026-08-30・task 3.2 着地時）**: `spawn_zorder_group_wiring_tests` は**退役しない**。
> `design.md` の Modified Files が「改名して流用」と裁定し、実際に `spawn_zorder_chain_wiring_tests.rs` へ
> 改名済みで、本番の処理列の並び順を字面で主張する**現役の檻**である。**5.1 は削除しないこと。**
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

---

## 12. 設計前実測（2026-08-29）

> §11.6 の Research Needed を、**推論ではなく実測**で閉じた記録。初版が実測なしに R2 を
> 閉じて実機 NO-GO になった轍を踏まないための必須手順（要件 14 の前置き）。
>
> 検体＝`crates/wintf/src/api_owner_chain_probe_tests.rs`（`crates/wintf/src/api.rs:625-627` で登記）。
> 本番のゴースト窓と同じ `WS_POPUP` ＋ `WS_EX_TOOLWINDOW`（`crates/areka/src/placement/spawn.rs:572-576`）で
> 0x0 の窓を作り、`ShowWindow(SW_SHOWNOACTIVATE)` で**実際に表示状態**にしてから測った——
> 問いが「**既に表示中の**窓の張り替え」だからである。重なりは `GetTopWindow` →
> `GW_HWNDNEXT` 走査（`zorder_group_order_tests.rs:126` と同形）で読み、隣接ではなく順序で見る。
>
> 走行: `cargo test -p wintf --lib owner_chain_probe`（PowerShell）。
> 検体（9 本）は**恒久の檻として残す**——測った値を assert で固定してあり、Windows 側の性質が
> 変わればここが赤で教える。
>
> **安定性＝cargo 3 プロセス同時 × 14 周 ＝ 42 走行で `937 passed / 0 failed`**（申し送り 7.1 の再現 regime。
> `cargo test -p wintf --lib` の全体走行で測った）。ここへ至るまでに**檻自身の非決定を 3 度潰した**——
> その 3 件はいずれも「こちらが保証していないものを檻に書いていた」形であり、うち 1 件は
> **本番の設計判断を変えた**（§12.2 の後押しの形）。詳細は §12.9。

### 12.1 実測 1: 表示中の窓の owner 張り替え（§11.6-2 の答え）

始点は宣言の逆順（手前から `3,2,1,0`）。鎖は `0 ← 1 ← 2 ← 3`（`0` が最も手前・`3` が根）。

| 段 | 実測した並び（手前から） | 意味 |
|---|---|---|
| 助走（逆順） | `[3,2,1,0]` | 始点が揃っている |
| owner を 4 枚へ張った直後（後押しなし） | `[3,2,1,0]` | **張り替えだけでは 1 ミリも動かない** |
| Z を伴う後押しを **1 回** | `[0,1,2,3]` | **1 回で鎖全体が宣言順へ収まる** |
| さらに最背面の窓を `HWND_TOP` へ（＝利用者の活性化に相当する攪乱） | `[0,1,2,3]` | **OS が鎖の順を保つ**（要件 14.3・1.3） |
| 鎖を外す | `[0,1,2,3]` | **外しても並べ替えは起きない**（束縛が消えるだけ・要件 6） |

**答え**: 即時反映は**されない**。ただし**イベント応答としての 1 回の後押し**で足り、
以後は OS が維持する（要件 14.2「繰り返しの観測と是正をしない」と両立）。

### 12.2 実測 6・7・9: 後押しの形（どれを採るか）

| 後押しの形 | 鎖は収まるか | 備考 |
|---|---|---|
| `SWP_NOZORDER`（触るだけ） | ❌ **収まらない** | Z を伴う指令でなければ OS は再整列しない |
| 根を `HWND_BOTTOM` へ | ✅ | 絶対帯指定。グループの絶対位置を最背面へ移す |
| 先頭／途中の窓を `HWND_TOP` へ | ✅ | 絶対帯指定。グループを最前面へ持ち上げる |
| 根を「いま自分が居る位置」へ差し直す（`GW_HWNDPREV` の直後） | ✅ | ⚠ **挿入位置に他プロセスの窓を渡しうる**——読み取りと書き込みの間にその窓が消えると `SetWindowPos` が黙って失敗し鎖が収まらない。全体走行の並走で実際に再現した |
| ~~鎖の先頭を 2 番目の直後へ差し直す~~（実測 9） | ⚠ **撤回（§13.2）** | **この ✅ は汚染された測定である**——受け皿が無く不可視の IME 窓が鎖の先頭に所有されていたため、先頭が「所有する窓」になっていた。受け皿を敷くと **0/24**（1 枚も動かない）。正しい形は §13.3「鎖の**根**を錨の直後へ」 |
| **鎖の根を錨（1 つ手前の窓）の直後へ差し直す**（§13 実測 10） | ✅ | **採用形**。参照はどちらも自分の窓。所有側を動かすので鎖が並び直る。位置・寸法は変えない |

> ⚠ **2026-08-30 撤回（§13.2）**: ここは当初「**採用する後押しは実測 9 の形**（鎖の先頭を 2 番目の
> 直後へ差し直す）」と書いていた。その測定は受け皿を敷いておらず、**不可視の IME 窓が鎖の先頭に
> 所有されていた**——先頭が「所有する窓」になっていたために効いて見えただけである。受け皿を敷いて
> 測り直すと同じ形は **24 通り中 0 通り**しか収まらない。**採用する後押しは §13.3 の形**
> （鎖の**根**を錨の直後へ差し直す・錨の直後が既に根なら挿入位置を先頭へ切り替える）である。
> 要件 11.1（位置・寸法を変えない）と「外部の窓の生死に依存しない」はそちらでも満たされる。
> 逐語値も測り直した: 助走 `[3,2,1,0,4]` → **撤回済みの形では `[3,2,1,0,4]`（1 枚も動かない）**、
> 採用形なら `[3,0,1,2,4]` ＝鎖が `0,1,2` へ収まり**部外者どうしの相対順（`3` が `4` より手前）は
> 変わらない**（`owner_chain_probe_nudge_referencing_only_our_own_windows` が両方を対で固定）。

> ⚠ **初稿の誤り（実測が訂正した）**: 当初は「その場への差し直しならグループ外の窓が 1 つも動かない」と
> 書いていたが、**これは強すぎる主張だった**。§12.5 のとおり鎖は 1 つの塊として動き、後押しの際に
> 鎖の外の窓を**追い越すことがある**（周囲の窓の状況に依り、並走走行で両方の結果が出た）。
> 要件が縛るのは ⑴ 鎖の中の相対順（要件 1.1／1.2）と ⑵ **鎖の外どうし**の相対順（要件 6.1／6.2）であって、
> その 2 群の間ではない——正典も「グループとそれ以外」の前後関係を規定していない（要件 3.6／6.1）。

### 12.3 実測 2・3: 副作用（§11.6-3 の答え）

鎖 `a ← b ← c`（`a` が最も手前・`c` が根）。

| 操作 | 実測 | 判定 |
|---|---|---|
| 根 `c` を `SW_MINIMIZE` | `a`・`b` が**不可視になる**（隠れるのであって最小化はされない・`IsIconic(a)=false`） | ⚠ **連動する** |
| 根 `c` を `SW_RESTORE` | `a`・`b` が戻る | 可逆 |
| 中間 `b` を `SW_MINIMIZE` | `a` だけ不可視・`c` は無傷 | 連動は**下流（手前側）へだけ**伝う |
| 根 `c` を `SW_HIDE` | `a`・`b` は**可視のまま** | ✅ 連動しない |
| 根 `c` を `DestroyWindow` | `a`・`b` も**破棄される** | ⚠ 連動する |
| 中間 `e` を `DestroyWindow` | 下流 `d` も破棄・上流 `f` は残る | 下流へだけ伝う |
| **先に鎖から外してから**破棄 | 残り 2 枚は**生存** | ✅ **封じられる**（要件 7.2） |

**破棄の連動は完全に封じられる**——既存ペア機構の切離しの雛形（`zorder_pair_maintain.rs:286`）
と同じ「先に外してから壊す」で足りる。

**最小化の連動は封じられない**（鎖である以上、OS の性質として付いてくる）。ただし射程は限定される:
- ゴースト窓は `WS_POPUP` ＋ `WS_EX_TOOLWINDOW` ＋最小化ボックス無しで、**areka はどの経路でも
  `SW_MINIMIZE` を出さない**（要件 11.4 が窓状態指定を射程外とし、先送り語彙の檻
  `PRODUCTION_FILES` が `minimize`／`iconic` の混入を赤で止めている）。
- **同型の連動は既に本番で成立している**——ペア edge（バルーンがキャラ窓に所有される）により、
  キャラ窓を最小化すればそのバルーンは既に隠れる。横断 edge はこの既存構造の射程を
  スコープ間へ広げるものであって、新しい種類の副作用を持ち込むわけではない。
- 要件 11.5 は「グループの指定・解除が窓へ及ぼす**利用者に見える変化**」を縛る。最小化の経路が
  存在しない以上、利用者に見える変化は生じない。**ただし性質そのものは変わる**ので、
  裁量として COMPAT §8 へ登記する（設計 DD-7）。

**タスクバーへの出方**は変わりようがない: 本番のゴースト窓は `WS_EX_TOOLWINDOW` を持ち、
道具窓は owner の有無にかかわらずタスクバーに出ない。

### 12.4 実測 4: スプライス（§11.6-4 の答え）

鎖 `a ← b ← c` の `b` と `c` の間へ、後から現れた窓 `x` を差す手順:

1. `clear_window_owner(b)`（edge を **1 本だけ**切る）→ 実測: 重なりは**崩れない**（`[3,0,1,2]` のまま）
2. `set_window_owner(b, x)` → `set_window_owner(x, c)`
3. 後押し 1 回 → 実測 `[0,1,3,2]` ＝ `a, b, x, c` ＝**差し込んだ鎖の順**

抜去も対称（`b`・`x` の owner を外し、`b` を `c` へ繋ぎ直して後押し 1 回）→ `x` は鎖の外へ戻り、
残りは宣言順を保つ。**途中状態が壊れない**（要件 8.2）ことが実測で確定した。

### 12.5 鎖のまとまり（§11.6-5 の答え）

**鎖は 1 つの塊として動く。** 部外者が最前面にいる状態で鎖のどの窓を `HWND_TOP` へ持ち上げても
鎖全体が部外者より手前へ出る（`[3,0,1,2]` → `[0,1,2,3]`）。さらに、**採用する後押し（実測 9）でも
鎖が鎖の外の窓を追い越すことがある**——追い越すか否かは周囲の窓の状況に依り、並走走行で両方の結果が出た。

これはピン留めの帰結であり、グループ有効中の期待挙動そのものである。要件との関係:

- 要件 6.1／6.2 が縛るのは「**どのグループにも属していない窓どうし**の相対順」であり、実測 9 で
  この不変（前の部外者が後の部外者より手前のまま）が保たれることを確認した
- グループと非グループの間の前後関係は正典も要件も規定していない（要件 3.6／6.1）
- したがって**この挙動は許容される**。ただし「後押しが鎖の外の窓を 1 つも動かさない」という
  **より強い主張はできない**（この点で初稿を訂正した＝§12.2 の註）

この訂正は檻の非決定として現れた（§12.9 の 3 件目）。

### 12.6 実測 5: `clear_window_owner` の落とし穴（新発見）

`clear_window_owner`＝`SetWindowLongPtrW(GWLP_HWNDPARENT, 0)` は、**owner を持たない窓に当てると
`Err(ERROR_INVALID_WINDOW_HANDLE / 0x80070578)` を返す**（owner を持つ窓に当てれば `Ok`）。
二度目の切離しも同じ失敗になる。

- 原因は `set_window_long_ptr`（`api.rs:25-41`）が「戻り値 0 かつスレッドのエラーが非 `S_OK`」を
  失敗と読む形にあり、`SetWindowLongPtrW` 自身が新しい値 0 を窓ハンドルとして検証して
  エラーを立てるため。既存の往復檻（`api.rs:586`）は **owner を持つ窓**しか外していないので
  この経路を通っていなかった。
- **設計への拘束**: 撤去は「**自分が書いた横断 edge だけ**」を対象とし、
  掃除のための一括 `clear_window_owner` を出してはならない。既存ペア edge（バルーン側）にも
  当ててはならない。実装は外す前に `GetWindow(GW_OWNER)` で現状を読む（実測 5 の
  `unlink_all` が採った形）。

### 12.7 §11.6 の一覧に対する決着

| # | 問い | 決着 |
|---|---|---|
| 1 | SSP 窓木の入れ子が owner か parent か | **実測せず・不要になった**。areka 側で owner の鎖が期待どおり働くことを直接実測したので、SSP の内部形の確認は設計の前提ではない（正典の裏取りとしては残件） |
| 2 | 表示中の張り替えの即時反映 | §12.1 — 即時反映なし・**イベント応答の後押し 1 回で足りる** |
| 3 | 横断 edge の副作用 | §12.3 — 破棄は封じられる・最小化は封じられないが到達経路が無い・タスクバーは不変 |
| 4 | スプライスの安全な順序 | §12.4 — 切る 1 本 → 張る 2 本 → 後押し 1 回。途中状態は壊れない |
| 5 | 活性化のまとまり方 | §12.5 — 鎖は塊として動く。鎖の外の窓を追い越しうるが、**鎖の外どうしの相対順**は保たれる（要件 6.1／6.2 が縛るのはそちら） |
| 6 | 既存 always_on_top 檻との交差 | `cargo test -p wintf --lib` **937 passed / 0 failed**（`zorder_pair_maintain_always_on_top_tests` を含む）。鎖は `WS_EX_TOPMOST` を一切書かないので帯へ引き込まれる経路が無い。**既存の檻は 1 本も不安定化していない**（並走 42 走行で確認） |
| 7 | 要件 1.3 の是正経路 | §12.1 の攪乱行で**構造的に保たれる**ことを実測。よって 1.3 に「毎巡の是正」は要らない。外部要因で鎖が壊された場合の検知は**持たない**（要件 14.2 が反復観測を禁じるため）——鎖が壊れるのは自プロセス以外が `GWLP_HWNDPARENT` を書いた場合だけであり、その仮説に対する観測系を置くと NO-GO の根因へ逆戻りする |

### 12.8 シンセシス（設計へ落ちた形）

- **採用は Option A**（§11.5・ペア edge 温存＋横断 edge のみ新設）。§12.4 が「ペア edge を
  1 本も触らずに鎖が組める」ことを実測で裏づけたため、§11.2 の予測どおりペア 5 ファイルは無編集。
- ~~**後押しは「鎖の先頭を 2 番目の直後へ差し直す」1 形のみ**（§12.2 実測 9）~~ ⚠ **撤回**
  ⚠ **§13.2 で撤回。** 正しい形は §13.3——**鎖の根を錨の直後へ**（自分の窓 2 枚だけを参照するので
  外部の窓が消えても黙って失敗しない、という利点はそのまま引き継ぐ）。
- **撤去は自分が書いた edge に限る**（§12.6）。
- **破棄の前に必ず外す**（§12.3）。既存の切離し経路より**先に**走る必要がある。
- 一般化して得たものは無い（鎖合成は本 spec 固有）。単純化して落としたもの＝v1 の
  観測・是正・次巡検証・起床促しの 4 機構すべて（§11.4 の退役表）。

### 12.9 檻自身の非決定を 3 度潰した記録（申し送り）

実測の値そのものは単独走行で何度も同じだったが、**檻を全体走行の並走 regime へ載せると 3 度赤くなった**。
3 件とも原因は同じ形——**こちらが保証していないものを檻に書いていた**——であり、うち 1 件は本番の設計判断を変えた。

| # | 症状 | 真因 | 是正 |
|---|---|---|---|
| 1 | 助走の並びが揃わない（18 走行中 3 回） | 助走に `HWND_TOP`／`HWND_BOTTOM` の**絶対帯指定**を使っていた。デスクトップ全体の状態に依存する | 助走は**自分の窓どうしの相対指定**（`arrange_z`）だけで組む。絶対帯指定を残すのは、それ自体が測定対象の 1 本のみ |
| 2 | 後押しが効かず鎖が収まらない（全体走行でのみ・36 走行中 4〜8 回） | 後押しの挿入位置に `GW_HWNDPREV`＝**他プロセスの窓**を渡していた。読み取りと書き込みの間にその窓が消えると `SetWindowPos` が黙って失敗する | 後押しの挿入位置を**自分の窓 2 枚だけ**を参照する形へ変更（この点は現在も有効）。⚠ ただし当時採った「**鎖の先頭を 2 番目の直後へ**」という**動かす窓の選び方は §13.2 で撤回**され、本番の DD-3 は「**鎖の根を錨の直後へ**」（§13.3）になっている |
| 3 | 鎖の外の窓の位置が期待と違う（36 走行中 4 回） | 「後押しはグループ外の窓を 1 つも動かさない」という**要件より強い主張**を檻に書いていた。実際には鎖は塊として動き、鎖の外の窓を追い越しうる | 檻の主張を ⑴ 鎖の中の相対順 ⑵ 鎖の外どうしの相対順 の 2 つへ絞る。§12.2／§12.5 を訂正 |

**教訓**: 実窓の檻は「単独走行で安定」では足りない（申し送り 7.1 の再確認）。加えて——
**檻が赤くなったとき、まず疑うのは実装ではなく「檻が何を主張しているか」である**。3 件のうち 2 件は
檻の主張の誤りで、1 件は本物の設計上の穴だった。どちらも**赤を根拠に是正できた**。


## 13. 後押しの形の再実測（2026-08-30・task 2.3）

> §12.2 で採った後押し（**鎖の先頭を 2 番目の直後へ差し直す**・⚠ 本節で**撤回**）が、4.3 の実窓検証で
> **完全に空振りする配置**を持つことが判った。本節はその原因を実測で突き止め、
> 採り直した形とその根拠を記録する。**§12.2 の表は本節が上書きする。**
>
> 検体＝`crates/wintf/src/api_owner_chain_nudge_probe_tests.rs`
> （`crates/wintf/src/api.rs` の末尾で登記）。窓の作り・測り方・決定論の規律は §12 と同じ。
> 走行: `cargo test -p wintf --lib owner_chain_probe`（PowerShell）。

### 13.1 実測 10: 4 枚の始点 24 通り × 後押しの形 6 種

鎖は `0 ← 1 ← 2 ← 3`（`0` が最も手前・`3` が根）。始点は 4 枚の並びの**全順列 24 通り**を
`arrange_z`（自分の窓どうしの相対指定）で組み、鎖を張ってから後押しを 1 回だけ出す。

**受け皿を先に 1 枚作る**——スレッド既定の不可視 IME 窓（`class="IME"`）は
「そのスレッドで最初に作られた窓」に所有される。手当てをしないと**檻が最初に作った窓＝鎖の
先頭**が IME 窓の所有者になってしまい、本番には無い性質（先頭も「所有する窓」である）が
測定へ入り込む。よって鎖の窓より先に受け皿の窓を 1 枚作り、IME 窓をそちらへ付ける。

| 後押しの形 | 宣言順へ着いた回数 |
|---|---|
| **先頭を 2 番目の直後へ**（§12.2 で採った形） | **0 / 24** |
| 2 番目を先頭の直後へ | 24 / 24 |
| **根を錨（1 つ手前の窓）の直後へ** | **24 / 24** |
| **根を先頭の直後へ** | **24 / 24** |
| 上 2 つの 2 択（採る形・§13.3） | **24 / 24** |
| 後押しを出さない（対照） | 1 / 24（＝始点が既に宣言順だった 1 通りのみ） |

24 通りのうち **6 通り**で「先頭が 2 番目の生の直後」（初版の形が現在位置と同じ位置を要求
する配置）が成立し、同じく **6 通り**で「根が錨の生の直後」が成立した。どちらの引き金も
実際に立っていることは檻が自己検査している。

### 13.2 なぜ先頭を動かしても並ばないのか

Windows が鎖全体を並べ直すのは、**所有する窓が動いたとき**である——「所有される窓は所有者
より手前」という不変条件を保つため、所有側を動かすと被所有側が引き連れられ、それが鎖の奥
から手前へ伝わる。鎖の中で他の窓を所有しているのは根以下の各窓であり、**先頭だけは誰も
所有していない**。よって先頭への指令はその 1 枚を動かすだけで終わる。しかも
⚠ 撤回済みの形「先頭を 2 番目の**直後**へ」は、Z の並びで言えば「先頭を 2 番目の**奥**へ」であり、望む
関係とは逆である——実際、始点が既に宣言順だった巡では**この後押しが正しい並びを壊した**
（`[0,1,2,3] → [1,0,2,3]`）。

§12.2 実測 9 と 4.1〜4.3 の実窓の檻が緑だったのは、**IME 窓がたまたま鎖の先頭に所有されて
いた**からである（先頭も「所有する窓」になっていた）。本番にその保証は無い——本番は各
スコープでバルーンを先に・キャラ窓を後に作るので、鎖の先頭が IME 窓の所有者になるとは
限らない。

> ⚠ **§12.2 の訂正**: 「鎖の先頭を 2 番目の直後へ差し直す」を採用としていた行は**誤り**で
> ある。実測 9 の逐語値（`[3,2,1,0,4]` → 鎖が `0,1,2` へ収まる）自体は再現するが、それは
> 檻が最初に作った窓が先頭だったことに依存している。§12.9 の教訓（「こちらが保証していない
> ものを檻に書いていた」）と同じ形が、今度は**本番の設計**の側で起きた。

### 13.3 採る形（2 択）と、その premise-independence

**動かすのは鎖の根**（`members` の末尾）。挿入位置は次の 2 択で、**書く直前に 1 度だけ**
錨（根の 1 つ手前の窓）の生の 1 つ奥を読んで選ぶ。

- 錨の直後が根**でない** → 挿入位置は**錨**（望む関係そのものを主張する形）
- 錨の直後が**既に根** → 挿入位置は**先頭**（`members[0]`）

2 つの要求が同時に「現在位置と同じ」になることはない——根の生の 1 つ手前は高々 1 枚であり、
鎖が 3 枚以上なら錨と先頭は別の窓だからである。よって**どちらか一方は必ず本物の位置変更に
なる**。これは「Windows は隣接なら Z 変更を省略する」という**未文書の相関**に賭ける形では
なく、「要求位置が現在位置と違えば Z は動く」という `SetWindowPos` の基本の性質だけに乗る
形である（tasks.md 2.3 の「⚠ 前提の限界」への答え）。

窓が 2 枚のときは錨と先頭が同じ窓になり 2 択は 1 つに畳まれる。そのとき空振りするのは
「根が先頭の直後に居る」＝**既に望む並びである**場合だけなので、収めるものが無い。

> 実測では、根が既に錨の直後に居る 6 通りでも「根を錨の直後へ」がそのまま収まった
> （24/24）。つまり**この配置で Windows は省略していない**。それでも 2 択を残すのは、
> 「省略しない」ことが保証された挙動ではないからである——**採る形の正しさを未文書の性質へ
> 依存させない**というのが 2 択の目的であって、いま省略が起きることが理由ではない。

### 13.4 現況を 1 度読むことと要件 14.2 の関係

要件 14.2 が禁じたのは**周期的な観測と是正**である。ここで読むのは、書く直前に 1 度だけ、
出す指令の形を選ぶためであり、読んだ値は控えもしないし次の巡へも持ち越さない。既存の撤去
経路（`detach_one` が外す前に `GWLP_HWNDPARENT` を読む・§12.6）と同じ規律である。

### 13.5 隣接ハザード（計画が HWND より先に公開されうる経路）

`zorder_drain.rs` の `resolve_member` は在庫（`GhostWindows`）と entity の実在だけを見て
**`WindowHandle` を要求しない**。一方 `placement/spawn.rs` の註のとおり `WindowHandle` は
wintf の窓生成が HWND を得た後に付く。よって**計画は HWND 生成より前に公開されうる**。

適用系は印を `execute_ops` より前に降ろすので、その巡は 1 本も書かないまま印だけが消える。
計画の内容は後から変わらないので再公開もされず、**印は二度と立たない＝鎖が永久に書かれない**。

**是正**: 1 本も書けず、かつハンドル未取得で見送った付与が在る巡は、**印を戻す**
（`zorder_chain_apply.rs`）。重なりは 1 度も読まないので観測ではなく、「まだ適用していない
計画を持ち越す」だけである。持ち越しは「1 本でも書けたら終わる」ので、鎖の窓が 1 枚でも
ハンドルを得た時点で必ず解ける。食い違い（`Diverged`）だけの巡は**持ち越さない**——待っても
変わらないものを毎巡試すのは、要件 14.2 が退役させた反復是正そのものだからである。

### 13.6 檻を「偶然の隣人」から降ろした（4.1〜4.3 の実窓の檻）

実窓の檻 4 ファイルは、窓を作る唯一の入口で**受け皿の窓を先に 1 枚**作るようにした
（`ensure_ime_anchor`）。受け皿は壊さない——Windows はスレッド終了時にそのスレッドの窓を
すべて破棄するので、テスト 1 本につき 0x0 の窓が 1 枚残り、終われば消える。

加えて `zorder_chain_order_tests.rs` の **4 本**（既存 3 本＋下記の新設 1 本）は、助走のあとに
**先頭が同じスレッドの窓を 1 つも所有していないこと**を自己検査する（`head_owns_nothing`）。
易しい配置で測っていないことの証跡である。**生の隣接（先頭が 2 番目の生の直後に居ること）では
測らない**——2 つの窓の間に何も挟まらないことはこちらが保証できるものではなく、実際に 3 プロセス
同時走行で稀な赤を出した（§13.7 の 4 件目）。さらに、もう一方の空振りの引き金（根が錨の直後）を
作る始点 `[1,0,2,3]` の 1 本を新設した。

同じ受け皿は **§12 の実測 7・9 にも敷いた**——その 2 本こそが撤回された結論を記録していた当の檻で
あり、受け皿無しでは「先頭を動かす形でも収まる」と主張したままだったからである（§13.2）。実測 9 は
**撤回そのものを記録する形**へ書き替えてある: 同じ始点から、撤回済みの形では 1 枚も動かないこと
（`[3,2,1,0,4]` のまま）と、採用形なら収まること（`[3,0,1,2,4]`）を 1 本の中で対にして測る。

**変異の効き**: 後押しを初版の形（先頭を 2 番目の直後へ）へ戻すと、実窓の檻**9 本が 9 本とも
赤**になる。2 択の切り替えだけを落とすと決定論の檻 3 本が赤になる。受け皿を外すと
`zorder_chain_order_tests.rs` の 4 本が「先頭が窓を所有している」で赤になる。

### 13.7 檻自身の非決定を潰した記録（§12.9 の続き・**4.4 で全件決着**）

| # | 症状 | 真因 | 是正 |
|---|---|---|---|
| 4 | 引き金の自己検査（先頭が 2 番目の**生の直後**に居ること）が 3 プロセス同時走行で稀に赤（120 走行中 1 件） | **こちらが保証していないものを檻に書いていた**（§12.9 と同じ形の 4 件目）——2 つの窓の間に何も挟まらないことは制御できない。他プロセスの窓がいつでも割り込みうる | 自己検査を**所有関係**へ移した——「鎖の先頭が同じスレッドの窓を 1 つも所有していない」。これは受け皿によってこちらが作る性質なので決定論である |
| 5 | §13.1 の掃き出しを足した直後から、**他の**実窓の檻（4.1／4.2 と既存のペア機構）が 3 プロセス同時走行で稀に赤（75 走行中 3 本） | 掃き出しが 24 通り × 6 形で**窓を 576 枚生成・破棄**していた。その churn が同じプロセスの他スレッドの実窓の檻を揺らす | 掃き出しは**窓 4 枚を使い回す**（始点は毎回 `arrange_z` が組み直し、所有関係も張り直すので各回は独立）。生成は 4 枚のみ。**再走行で 105 走行 0 件**・所要も 1.20s → 0.23s |
| 6 | `a_settled_chain_holds_its_order_when_the_deepest_window_is_raised_to_the_front` の自己検査 `control_before == Some(0)`（`zorder_chain_order_tests.rs:687`）が、**全体走行**の 3 プロセス同時 × 120 走行で 1 件赤。同テスト**単独**の 3 プロセス × 120 走行では 0 件 | ⚠ **未確定。** 有力なのは 4 件目と同じ形——「生成したての検体窓は鎖より手前に居るはず」は**こちらが保証していない性質**である（初版 4.1 が置いた自己検査で、assert とその入力は HEAD と byte 同一）。ただし**2.3 が足した §13.1 の掃き出しが新しい外乱である可能性を排除できていない**——窓の生成は 576→4 枚へ減らしたが、**`SetWindowPos` の回数は 24 通り × 6 形 ≒ 864 回のまま減っていない**。5 件目は同型の外乱が他の檻を赤にすることを実証済みであり、「単独走行では 0 件」という所見はむしろ外乱起因を支持する | ✅ **決着した（task 4.4 の実測）。2.3 の掃き出しは外乱ではない**——真因は 7 件目の道具の欠陥 2 種だった（下記） |
| 7 | 6 件目の中身は**檻の道具の欠陥 2 種**だった——⑴ Z を読む走査が**空を返す**、⑵ 絶対帯指定の持ち上げが**空振りする** | ⑴ 走査は `GetTopWindow` から `GW_HWNDNEXT` でデスクトップ全体の**生きた**連結リストを 1 歩ずつ辿る。歩いている最中に手前の他プロセスの窓が破棄されると `GetWindow` が `Err` を返し、走査はそこで**打ち切られる**。自分の窓へ辿り着く前に切れれば結果は空になる。⑵ `HWND_TOP` は帯の中の**絶対位置**の指定で、結果はデスクトップ全体の状態に依る（§13.6 の 1 件目と同じ性質）。同一プロセスの他の検査が自分の窓を前面窓にしている間、`Ok(())` を返しながら窓を 1 つも動かさない——兄弟の `window_pos_zorder_group_tests.rs` が同じ事象を独立に実測済み（3 プロセス × 120 走行で 21 走行＝17.5%） | ⑴ 走査を**全部拾えるまで有界回数（8 回）やり直す**（`relative_z_order`）。⑵ 攪乱の持ち上げを**届いたことを観測するまで有界回数（8 回・巡ごとに 2ms 譲る）出し直す**（`raise_to_front_until`）。どちらも**道具の側**を直しただけで、主張は 1 つも弱めていない |

**✅ 6 件目の決着（task 4.4 の実測・2026-08-30）**

引き継ぎが指定した測定をそのまま行った。走らせ方は 3 本とも同じ——PowerShell から
`cargo test -p wintf --lib` の**既に建てた実行体を直に** 3 プロセス同時起動し、各プロセスが
120 周する（cargo の起動待ちが無い分、`cargo test` を 120 回叩くより窓の生成・破棄が濃い regime）。
HEAD 側は**使い捨ての別 worktree**（`git worktree add --detach` で `35387f00`・`vendors/pasta` は
現ツリーから複写・`CARGO_TARGET_DIR` も別）で建てた。作業ツリーには一切触れていない。

| # | 走らせたもの | 走行 | 赤 | テスト実行数 | 赤の中身 |
|---|---|---|---|---|---|
| 1 | 現行（2.3 込み・是正前） | 360 | **5** | 366,840 | 種 A 2・種 B 3 |
| 2 | HEAD（2.3 抜き＝`35387f00`）① | 360 | **5** | 361,080 | 種 A 2・種 B 3 |
| 3 | 是正後①（鎖の檻と probe を直した時点） | 360 | **0** | 366,840 | — |
| 4 | 是正後②（書式適用後の再走行） | 360 | **1** | 366,840 | 種 A 1＝**走査の別の写し**（`zorder_group_order_tests`）。残り 4 本の写しも同じ形へ直した |
| 5 | 是正後③（最終・全部の写しを直した後） | 360 | **1** | 366,840 | 既存のペア機構の檻 1（下記・**HEAD でも赤くなる**） |
| 6 | HEAD ② | 360 | **8** | 361,080 | 種 A 1・種 B 6・既存のペア機構の檻 1（`HWND_NOTOPMOST` が `Ok` を返しながら帯から出さない） |

**HEAD 合計 720 走行で赤 13 本／是正後 720 走行（#5 と #3・写しを全部直した後の regime）で赤 1 本**、
その 1 本も本 spec の檻ではない。**鎖の実窓の檻（4.1〜4.3 が置いた 9 本）は是正後 1,080 走行で赤 0 本**である。

→ **レートは同率（5/360 対 5/360）。6 件目は 2.3 の掃き出しとは無関係の既存の競合である。**
引き継ぎが用意した 2 分岐のうち「HEAD でも同程度なら自己検査の書き換えで閉じる」側で閉じた。
**「掃き出しの Z 指令 864 回が外乱」という仮説は測定で否定された**——2.3 は無罪である。

赤 10 本（現行 5・HEAD 5）の内訳は 2 種にきれいに分かれ、**どちらも「こちらが保証していない
ものを檻に書いていた」形**（§12.9 と同じ・通算 5 件目と 6 件目）だった。

| 種 | 現象 | 現行 | HEAD | 落ちた檻 |
|---|---|---|---|---|
| A | `z_shape` が **`[]`** を返す（`left: []` 対 `right: [0, 1, 2, 3]`）。自分の窓は 4 枚とも生きているのに 1 枚も拾えていない | 2 | 2 | `zorder_chain_order_outsider_tests`・`api_owner_chain_probe_tests`（現行）／`zorder_chain_order_tests` 2 本（HEAD） |
| B | 持ち上げが**空振り**する。`SetWindowPos(hwnd, HWND_TOP, …)` が `Ok` を返しながら並びを 1 つも変えない（攪乱の前後で同一） | 3 | 3 | `zorder_chain_order_tests`（`control_after`）・`zorder_chain_order_lifecycle_tests`（`released_scope_moved`）・`zorder_chain_order_outsider_tests`（`after_raise`） |

**是正（道具を直した・主張は 1 つも弱めていない）**

- 種 A: `relative_z_order` を「集合の窓を**全部拾えるまで**有界回数やり直す」形へ。破棄済みの窓を
  含む集合ではいちばん多く拾えた回を返す＝**回数を使い切ったときの意味は従来と同じ**。
  ⚠ **最初は赤を出した 2 本だけに入れて足りなかった**——同じ走査の写しは 5 本あり、直していない
  写し（`zorder_group_order_tests.rs`）が次の走行で同じ形の赤を出した（上表 #4）。結局
  `zorder_chain_order_tests.rs`・`api_owner_chain_probe_tests.rs`・`zorder_group_order_tests.rs`・
  `command_batch_tests.rs`・`window_pos_zorder_group_tests.rs`・
  `areka` の `zorder_chain_tail_order_tests.rs` の **6 本すべて**へ入れた。本番の走査を真似ている
  `RealWindowProbe::scan_in_front` だけは、本番の性質を写す側なので触っていない
- 種 B: 攪乱の持ち上げを `raise_to_front_until`（`zorder_chain_order_tests.rs`）へ集約し、3 本の
  実窓の檻がこれを使う。出し直すのは**同じ持ち上げ 1 本だけ**である——検体窓を下げて条件を作りに
  いく逃げは打たない（それをやると「持ち上げの 1 行を消しても緑」に戻る）
- 検体窓の始点も成り行きに任せず、同じ仕組みで**手前へ据えてから**始める。`control_before` は
  「生成したてだから鎖より手前に居るはず」という**こちらが保証していない性質**に乗っていた
  （6 件目が最初に赤を出した箇所）。⚠ **引き継ぎが禁じた手とは別物である**——禁じられたのは
  「**鎖**を塊ごと背面へ回して始点を固める」形（登る距離を深くして 3〜4/120 へ悪化させた）で、
  ここで動かすのは**検体窓 1 枚だけ**であり鎖は 1 枚も動かさない
- **空虚化していないことの確認（変異注入）**: `raise_to_front_until` から `SetWindowPos` の 1 行を
  抜くと、`zorder_chain_order` 群 9 本のうち**当該 3 本がそろって赤**になる（`6 passed; 3 failed`）。
  有界回数の再試行は「刺激が届いたことの自己検査」を素通りさせない

**同じ走行で既存の檻が不安定化していないこと**

より厳しい regime——3 プロセス同時 × 100 周・**1 周が wintf と areka の 2 本**（wintf 300 走行
305,700 テスト実行＋areka 300 走行 421,200 テスト実行）——も、是正後と HEAD の両方で走らせた。

| 走らせたもの | wintf 走行／赤 | areka 走行／赤 | 赤の中身 |
|---|---|---|---|
| 是正後（3 × 100） | 300 / **1** | 300 / **2** | `tick_bridge` の vblank 500ms 期限 1 本・`emo2_boot::spine` の boot 応答の有界待ち 2 本 |
| HEAD（3 × 100） | 300 / **0** | 300 / **3** | `emo2_boot::spine` の**同じ 2 本**（`spine_boot_smoke_tests.rs:46` × 2・`spine_talk_close_tests.rs:306`） |
| 是正後・最終（3 × 120） | 360 / **1** | 360 / **3** | 同上（vblank 1・spine 3）。**重なり順の檻は 1 本も落ちていない** |

**これらの赤はすべて壁時計期限の飢餓であり、重なり順の檻は 1 本も落ちていない。** `spine` の 2 本は
HEAD でも同じ場所・同じ文言で出るので**既存**である（本 spec が `spine.rs` へ加えた変更は使われない
受け口 1 行だけ）。`tick_bridge` の 1 本は DWM 通知の 500ms 期限である。いずれも本 spec の担当外だが、
**「担当外」で終わらせず、引き受け先の実在を確かめたうえで下記 §13.8 へ登記した。**

**残った 1 本＝既存のペア機構の檻（本 spec の担当外・HEAD でも赤くなる）**

是正後の最終走行 360 本のうち 1 本で `zorder_pair_maintain_always_on_top_tests.rs:767`
（`the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`）が赤くなった
——`measure_window_below(balloon)` がキャラ窓ではない**別の可視窓**を返した。HEAD の 2 回目でも
同じファイルの別のテスト（`:411`）が赤くなっており（`SetWindowPos(HWND_NOTOPMOST)` が `Ok` を
返しながら帯から出さない＝**種 B と同じ形**）。**この檻は表明もコードも `main` とバイト同一の既存の檻である。
ただし、赤くなるレートが本 spec の前後で変わっていないことは測れていない**——下記の隔離測定と §13.8 のとおり、
**どちらの向きにも決着していない**。⚠ §13.7 の 5 件目を「以前から赤かった」根拠として引いてはならない——
あれは**本 spec の掃き出しが既存のペア機構の檻を赤にした**という逆向きの記録である。

真因の見立ては 4 件目・7 件目と同じ形——**owner 一組の間に他プロセスの可視窓が入らないことは
こちらが保証していない**（同一プロセスからは差し込めないことを実測済みだが、他プロセスは別）。
これは既存のペア機構の檻であり、要件 1.2 の「直上」を測っている主張なので、弱めるか作り直すかは
本 task の裁量の外である。**ただし「既存だから触らない」で終わらせてはいけない**——下記の隔離測定と
§13.8 の登記まで行う。


**隔離測定（`main` ＝本 spec の檻が 1 本も無いツリー）**

「既存の赤」だという判定を比較で立てるには、`35387f00` では足りない——**そこには本 spec の実窓の檻
4.1〜4.3 が既に入っている**からである。よって `main`（`9ac40875`）を使い捨ての worktree
（`git worktree add --detach` ＋ 別の `CARGO_TARGET_DIR`・作業ツリーには破壊的 git コマンドを一切
使わない）へ出して測った。`main` には鎖の檻が 1 本も無く（`zorder_chain*` 0 本・
`api_owner_chain_probe_tests` 無し）、一方でペア機構の檻は**在って走っている**ことを
`--list` で確認済み（`the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`
も `pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares` も列に載る）。

⚠ **道具の較正を先に行った。** 使い回していた走行スクリプトが**別物へ差し替わっている**のを
発見したため（`-Cwd` を持たず・失敗出力を保存せず・出力の書式が違う）、その出力は測定として
採用せず、新しいスクリプトを書き直して**既知赤 6/6・既知緑 0/6・実行数の逐語一致
（6 走行 × 850 = 5,100）・作業ディレクトリの復唱**まで確かめてから本測定を回した
（[subagent-tooling-can-be-wrong-calibrate-it] の 2 度目）。

走行数を**両ツリーで揃えて**測った（同じ道具・同じ日・同じ 3 プロセス同時 regime）。

| ツリー | 走行 | 赤 | テスト実行数 | 赤の中身 |
|---|---|---|---|---|
| `main`（本 spec の檻 0 本）① | 420 | **0** | 357,000 | — |
| `main` ②（増力） | 1,800 | **0** | 1,530,000 | — |
| **`main` 合計** | **2,220** | **0** | **1,887,000** | — |
| 本ブランチ（是正後）① | 420 | 2 | 427,980 | **両方とも `tick_bridge` の vblank 500ms 期限** |
| 本ブランチ（是正後）②（増力） | 1,800 | **0** | 1,834,200 | — |
| **本ブランチ合計** | **2,220** | **2** | **2,262,180** | **2 本とも `tick_bridge`。重なり順の檻は 0 本** |

**結論: 走行数を揃えた 2,220 対 2,220 で、重なり順の檻の赤は両ツリーとも 0 本。**
ペア機構の檻は `main` でも本ブランチでも 1 度も落ちておらず、**隔離測定は「本 spec の檻が
ペア機構の檻を赤にしている」という仮説を支持しなかった**。本ブランチに残る赤 2 本は
`tick_bridge` の壁時計期限だけである（§13.8 ②）。

**この数字が言えること／言えないこと（正直に）**

ペア機構の檻の赤は、**是正前**のツリーで 2 件 / 2,640 走行 ≒ 1/1,320（0.076%）だった
（是正前ブランチ 1 件 / 1,920・`35387f00` 1 件 / 720）。是正後は**本ブランチ 0 / 2,220・
`main` 0 / 2,220**である。

- 同じレートなら 2,220 走行の期待値は **1.7 件**、0 件を引く確率は **≒19%**。
  よって **0 件どうしの一致は「両方とも同率で稀」とも「両方とも消えた」とも読める**——
  **どちらか一方に決めるにはまだ足りない**。
- 言えるのは「**走行数を揃えた隔離測定で差が出なかった**」ことであり、
  「本 spec の檻が外乱である」という FINDING の懸念は**この測定では再現しなかった**。
- 完全な決着には両ツリーにさらに数千走行が要る。**本 task はここまでを実測として置き、
  ペア機構の檻の扱いは §13.8 の登記のとおり開発者の裁定へ上げる。**

**それでも打てる手は打った（外乱を減らす側）**——検体窓の据え付けを [`ensure_front_until`] へ
分け、**既に手前に居る通常巡では Z 書込を 1 つも出さない**形にした。刺激の側
（[`raise_to_front_until`]）はそのままなので空虚化しない（変異注入を再実施し、3 本そろって赤に
なることを確認済み）。結果、本 task が定常で足す Z 書込は**置き換え前の 1 発形と同じ 0 増**であり、
増えるのは Win32 が持ち上げを断った巡（実測 3/360 ≒ 0.8%）に限られる。
**教訓（追加）**: 実窓の檻は「自分が緑であること」だけでなく「**同じプロセスの他の檻を赤に
しないこと**」まで含めて安定でなければならない。窓の大量生成は他の檻から見れば外乱である。
**そして「窓を減らした」は「外乱を減らした」と同義ではない**——残る Z 指令の回数まで数えること。

**教訓（task 4.4 で足した 2 つ）**

1. **有力な仮説を測らずに閉じるとこの spec は死ぬ。** 6 件目の第一容疑者は 2.3 の掃き出しで、
   状況証拠（「単独走行では 0 件」「5 件目は同型の外乱で他の檻を赤にした」）も揃っていた。
   **HEAD で測ったら同率で、容疑は完全に晴れた。** もし測らずに掃き出しを削っていれば、
   直っていないものを直したことにして、本当の欠陥 2 種を残したまま 5 へ進んでいた。
2. **道具そのものが赤の出どころでありうる。** 種 A も種 B も、測っている性質ではなく**測り方**の
   欠陥だった。実窓を読む・実窓を動かす API は、成功を返しながら何もしないことがあり
   （`HWND_TOP`）、読み取りは他プロセスの都合で途中で切れる（`GW_HWNDNEXT` の走査）。
   **どちらも「Ok が返った」「値が返った」だけでは届いた証拠にならない**——届いたことを
   観測してから次へ進むこと。
### 13.8 引き受け先が実在しない未担当項目（**`/kiro-complete` で開発者裁定へ上げる**）

task 4.4 の走行が拾った赤のうち、**本 spec の担当外**でありながら**引き受けられる稼働中の spec が
実在しない**ものを、黙って先送りにせずここへ登記する（[deferral-requires-verified-owner]）。
**新規 brief は起票していない**——起票の可否そのものが開発者の裁定事項だからである。

**引き受け先の実在検証（2026-08-30 実施）**

| 候補 | 判定 |
|---|---|
| `completed/areka-P0-ghost-window-zorder`（ペア機構の正本） | **消化不能**——`completed/` に在る |
| `completed/areka-P0-test-cage-determinism`（檻の非決定の正本） | **消化不能**——`completed/` に在る |
| `areka-P0-zorder-property`（稼働中・唯一の隣接） | **引き受けない**——brief Scope に **「Out: 窓の是正機構そのもの」**と明記 |
| `areka-P0-tick-gate-adoption`（稼働中） | **引き受けない**——In は門の裁定（`tick_gate.rs`／`tick_gate_config.rs` 内で閉じる）。`tick_bridge` の檻は範囲外 |
| `areka-P0-present-write-coherence`（稼働中） | **引き受けない**——Out に「フレーム駆動の CPU 負荷（`draw-load-parity` の担当）」と明記。その `draw-load-parity` は `completed/` |
| `areka-P0-emo2-conformance-e2e`（稼働中） | **引き受けない**——brief が「**e2e は検出の場であって修正の担当ではない**」と明記 |

**① 既存のペア機構の実窓の檻（3 プロセス同時 regime で稀に赤）**

- `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:767`
  `the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`
  ——`measure_window_below(balloon)` がキャラ窓ではない**別の可視窓**を返す。
  診断: **owner 一組の間に他プロセスの可視窓が入らないことはこちらが保証していない**
  （同一プロセスからは差し込めないことは同ファイルが実測済みだが、他プロセスは別）。
- `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:411`
  `pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares`
  ——`SetWindowPos(control, HWND_NOTOPMOST)` が `Ok` を返しながら帯から出さない（**§13.7 の種 B と同じ形**）。
- レート: 是正前ブランチ 1,920 走行中 1 本・`35387f00` 720 走行中 1 本・
  **是正後ブランチ 2,220 走行中 0 本・`main` 2,220 走行中 0 本**。
  ⚠ **どちらの向きにも決着していない**——走行数を揃えた隔離測定では**両ツリーとも 0 本**であり
  （期待値 1.7 に対し 0 を引く確率 ≒19%）、**既存であることの証拠にも、本 spec 由来であることの
  証拠にもならない**（計算は §13.7 の隔離測定）。決着には両ツリーにさらに数千走行が要る。

**② 壁時計期限の飢餓（3 プロセス regime でのみ出る・重なり順とは無関係）**

- `crates/wintf/src/runtime/tick_bridge.rs:355` `vblank_notifies_listener_then_joins_on_drop`
  ——「500ms 以内に vblank 通知」の期限切れ。本ブランチ 420 走行中 2 本・`main` 420 走行中 0 本。
  ⚠ ただし `main` の suite は 850 本、本ブランチは 1,019 本で**負荷そのものが違う**ので、
  この差を本 spec のせいと読んではいけない。
- `crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:46`／`spine_talk_close_tests.rs:306`
  ——scripted ghost の boot 応答 5 本が有界内に発火しない。**`35387f00` でも同じ場所・同じ文言で出る**
  （3 × 100 で 3 本／3 × 120 で 3 本）ので本 spec 由来ではない。

**この regime を要求しているのは task 4.4 だけ**である（通常の `cargo test` 1 プロセスでは 1 本も出ない）。
よって「直ちに直すべき欠陥」なのか「この regime を要求する側が引き取るべき測定条件」なのかを
**開発者の裁定に上げる**。
