# ギャップ分析（audit）: audit-placement-core

> 2026-11-14 `/kiro-validate-gap`。この spec では、ギャップ分析の段で audit をかける（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。指摘は通し番号 1〜80 で、以後の段はこの番号で指す。この文書は分析と選択肢を並べるだけで、直す対象は要件の討議で確定する。

## 0. 要約

- 受け持つ 114 ファイル（コード 27 ファイル・10,413 行、テスト 86 ファイル・27,424 行、`Cargo.toml`）を全部読んだ。**指摘は 80 件**: 必ず直す 4・直したほうがよい 36・できれば 40。
- 必ず直すのは 4 件。
  - 1: `alignmenttodesktop` の `top`／`left`／`right` が下端に置かれ、同じ結果の `anchor` と食い違う。
  - 2: 面の windowposition の置き直しが、DPI の丸めの 1px の差で取りこぼされる。
  - 3: 測りの読み手が、動く絵の見出しの大きさを確かめないまま 1 枚目を読む。
  - 4: 手本 `window-placement` が、どのスコープのバルーンにも scope 0 の絵を貼る。
- 申し送り（要件 2）は 6 件の指摘（5〜9・41）にまとめた。
  - ⑵: 属性の理由の文の古さは、`#[allow]` の属性そのものが効いていないことと一緒に扱う（6・7）。
  - ⑶: 外に使い手の無い公開を挙げた（8）。
  - ⑷: 規約が名指さない参照と輪を測った（9）。
  - 環境変数の件: `measure_bake_count_tests.rs` も赤になると分かった（5）。
  - 古い名前: 13 ファイル・27 行（41）。
- 開発者の裁定が要る候補は 7 件（1・7・17・30 と、並走の約束に触れる直し方のうち 8・47・73）。並走の約束（要件 8）に触れる指摘は 14 件。
- 規模は 26〜30 タスクと見込む。20 を越えるので、要件 10 の 2 本目の議題が要る。

---

## 1. 測り直しと作法（要件 1.1〜1.4）

### 1.1 取り込んだ main と範囲の大きさ

- 作業のブランチの HEAD は `9cdb4d56`。これは main `30a343eb` の上に spec の初期化のコミットを 1 つ載せたもので、main は取り込み済み（`git merge-base --is-ancestor origin/main HEAD` で確かめた）。
- 範囲の大きさは要件の段の値と同じ。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 27 | 10,413 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 24,237 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 9 | 3,187 |
| `Cargo.toml` | 1 | 46 |
| 計 | 114 | 37,883 |

- 1,000 行の近く: `measure_tests.rs` 999・`persist_restore_tests.rs` 939・`persist.rs` 928・`lib.rs` 918（どれも変わっていない）。

### 1.2 ponytail の版

- 今入っているのは 5.3.3（`~/.claude/plugins/installed_plugins.json` の `ponytail@ponytail`・user scope）。起票のときは 5.3.2。
- `skills/ponytail-audit/SKILL.md` は 5.3.2 と 5.3.3 で同じ中身（`diff` で差が 0）。物差しは変わっていない。

### 1.3 作法

- ponytail-audit の 6 つの物差し（バグ・危ない所・負荷・テストの抜け・遅い所・要らないコード）で見た。どの指摘にも、具体的な場合を付けた。「使われていない」と書く前に、ツリー全体を grep した。
- `shortcut:`・`ponytail:` の注釈は、書かれた限界を今の負荷が越えていなければ指摘にしない。範囲の中には、この注釈は 0 件だった。
- 外した作法は 2 つ。
  - 「最大 20 件」を外し、全件を出した（要件 1.3）。
  - 「読まなかった所」を外し、114 ファイルを全部、全行読んだ（要件 1.4）。範囲を 6 つに分けて読ませ、それぞれから全ファイルを全行読んだ旨と行数の一覧を受け取った。計 37,883 行で上の表と合う。
- 想定した負荷は次のとおり。
  - 利用者 1 人のデスクトップに、ゴーストと窓が数個。
  - ECS の相が毎フレーム（60fps 前後）回る。
  - 他人のゴーストの descript・台本・絵は信用できない入力。
  - 保存のファイルは、壊れていたり手で直されていたりしうる。
- 組の決め方は ponytail と同じ。
  - 必ず直す: バグ・セキュリティ・データを失う・想定の負荷で壊れる。
  - 直したほうがよい: テストの無い危ないコード・本当に遅い所・写し・仕事が混ざった関数・要らないコード。
  - できれば: 小さな速さ・短く書ける形。

### 1.4 読んだ分け方（6 つ・全部読んだ）

| 組 | 主なファイル | 行 |
|---|---|---|
| A | `Cargo.toml`・`lib.rs`・`error.rs`・`resolver*`・`windowposition*`・`placement_*`・`script_alignment*` | 6,651 |
| B | `persist*`・`balloon_offset_*`・`reset_pos*`・`reset_balloon_pos.rs`・`face_windowposition*`・`source*` | 6,597 |
| C | `measure*`・`descript_window_keys*`・`config*`・`sample_test_support*`・`test_support.rs`・`test_crash_report.rs`・`examples/window-placement.rs`・本体の `placement_tests/measure_*`・`mod.rs` | 5,936 |
| D | `spawn*`・`diag*`・`chain_finalize*`・本体の `placement_tests/spawn_*` | 6,303 |
| E | `zorder_*`・本体の `placement_tests/zorder_*` | 6,490 |
| F | `balloon_limit*`・`balloon_lock*`・`balloon_syncscale*`・`dpi_sync*`・`reseed*`・`chain_realign*`・`sticky_group_ledger*`・本体の `placement_tests/reset_balloon_pos_tests.rs` | 5,906 |

---

## 2. 指摘の一覧（通し番号・3 つの組を通す）

各指摘の印は次のとおり。

- 〔約束〕: 直すと要件 8 の約束（公開の形は足すだけ・相手の持ち物の名前と形・wintf の型と綴り・依存と `Cargo.lock`）に触れる。響く相手も書く（要件 1.7）。
- 〔裁定〕: 要件 5 の 3 種類に当たる、またはその候補。
- 〔申し送り〕: 要件 2 の件と同じ所を指す（要件 1.6。同じ直しを 2 回数えない）。
- 規模: S は 1 タスク以内、M は 1〜2 タスク。

### 2.1 必ず直す（1〜4）

#### 1. `alignmenttodesktop` の `top`／`left`／`right` が下端に置かれ、同じ結果の `anchor` と食い違う
- 場所: `crates/areka-placement/src/resolver.rs:L278-L318`（Seam の warn・P1〜P4 の幾何）・`L392`（`anchor`）
- 種類: バグ
- 何をする所か: `resolve_placement` は、窓の最初の位置と吸着の辺（`anchor`）を一緒に返す。
- 問題: `Alignment::Seam("top"|"left"|"right")` のときも、幾何はいつも下端（`bottom_y`）と右からの連なり（P2）で解く。一方で `anchor` は `Top`／`Left`／`Right` になる。
  - 例: シェルの descript に `seriko.alignmenttodesktop,top` を持つゴーストで並べ直し（`reset_pos.rs:L107-L118`）をする。`move_window_to` は下端の位置をそのまま書き、既定の位置も下端になる。その後のドラッグや大きさの変化で `project_anchor` が上端へ飛ばす。記憶から戻す所（`persist.rs:L230`）も上端へ直す。結果として、同じ「標準の位置」が経路によって違う。
  - あわせて、正典どおりの値にも呼ぶたびに「未使用値（bottom として配置）」の warn が出る（`L279-L285`）。reseed はスコープの数だけ呼ぶので、警告の数はスコープの数の 2 乗になる。
- 直し方: P4 の前に `Anchor::from_alignment` で分け、`Top` は `y = wa.top`、`Left` は `x = wa.left`、`Right` は `x = wa.right − w` にする。Seam の warn を消す。スコープ 1 以降が同じ辺に並ぶときの形は決めが要る。
- 直さないと: 上・左・右に吸着するシェルを並べ直すと違う辺に置かれ、記憶とも食い違う。
- 〔裁定〕3 種類目（窓の出る位置・ukadoc の正典）。
  - ukadoc https://ukatech.shillest.net/ukadoc-spec/descript_ghost.html#seriko.alignmenttodesktop の原文は「全体のサーフェスのデフォルト※表示位置情報。」で、既定値は `bottom`。
  - さらに、持ち主の spec が実在する。`.kiro/specs/window-snap/brief.md` の「追記（2026-10-23）件 1」が、起動の位置と作業領域の変化の分を受け持つ。並べ直しの経路と warn の数は、その件に書かれていない新しい事実。
- 〔約束〕公開の名前と形は変わらない。出す値が変わるので、`crates/areka` の reseed・reset・spawn 系のテストの期待が動く。
- 規模: M

#### 2. 面の windowposition の置き直しが、DPI の丸めの 1px の差で取りこぼされる
- 場所: `crates/areka-placement/src/face_windowposition.rs:L62-L64`
- 種類: バグ・テストの抜け
- 何をする所か: 面が変わったとき、利用者が動かしていないバルーンだけを新しい面の標準の位置へ置き直す。動かしたかどうかは、今の `BalloonFollow.offset` と、標準を今の DPI で解き直した値が同じかで決める。
- 問題: 左辺は「和を 1 回丸めた値」、右辺は「部品ごとに丸めてから足した値」なので、1〜2px ずれることがある。
  - 例: 作者の DPI 96、`sakura.balloon.offsetx,129`、バルーン幅 400、`balloon.alignment,left` で、96 のモニタで起動したとする。窓が 144 のモニタへ入ると、追随の値は −271×1.5 を丸めた −407、解き直しは −600＋194＝−406 になる。一致しないので「動かしてある」と判定され、置き直されない。
  - 主モニタと違う拡大率のモニタで起動しただけでも起きる。今の 3 本のテストは、どれも DPI 96。
- 直し方: 2 つ案がある。
  - 小: 軸ごとに 2px 以内なら一致とみなし、`shortcut:` の注釈を付ける。
  - 筋: 「動かした印」を持たせて判定する（M。follow の持ち物に触れる）。
  - どちらでも、96→144 を挟むテストを 1 本足す。
- 直さないと: 拡大率の違うモニタが混ざる机で、面を変えてもバルーンが前の面の位置に残る。
- 〔約束〕小の案は触れない。筋の案は `follow/drag_follow.rs` ほか、`audit-placement-follow` の持ち物に触れる。
- 〔裁定〕当たらない（今の要件の意図へ戻す直し）。
- 規模: S（筋の案は M）

#### 3. 測りの読み手が、動く絵の見出しの大きさを確かめないまま 1 枚目を読む
- 場所: `crates/areka-placement/src/measure_first_frame.rs:L27-L36`
  - 比べた所: `crates/areka-emo-atlas/src/animated.rs:L46-L52`（資産の道は、確保の前に `pixel_len` で一辺の上限を確かめる）
- 種類: 危ない所
- 何をする所か: `FirstFrameDecoder::decode` は、動く絵の見出しがあれば 1 枚目だけを読む（失敗したら全体を読む `decode` へ戻る）。
- 問題: 資産の道は一辺 16384 を越える見出しを読まずに断るが、測りの道はその確かめを持たない。
  - 例: 他人のシェルに見出しが 60000×60000 の APNG がある。本番では測りが資産より先に走るので、守りに届く前に約 14GB の確保を試みるおそれがある。
- 直し方: 2 か所の案がある。
  - `areka-emo-atlas` の `image_arm::read_first_frame` の先頭に `pixel_len(info.width, info.height)?;` を 1 行足す（共有の所で 1 度だけ直す）。
  - 配置の側で直すなら、`pixel_len`／`MAX_SIDE` を公開する。
  - どちらでも、16385×1 の見出しが `Err` になるテストを 1 本足す。
- 直さないと: 他人のゴーストのファイル 1 つで、起動のときに落ちうる。
- 〔約束〕依存は足さない。直す所が範囲の外の `areka-emo-atlas` にある。`audit-emo-atlas` は完了しているので、申し送っても消化されない。この spec で直すか、どこへ回すかの決めが要る（要件 7.4）。
- 〔裁定〕当たらない。
- 規模: S
- 調べが要る: `image` 0.25 の `PngDecoder::new`・`WebPDecoder::new` が、見出しどおりに確保するのか、既定の `Limits`（max_alloc）で断るのか。重さがこれで変わる。

#### 4. 手本 `window-placement` が、どのスコープのバルーンにも scope 0 の絵を貼る
- 場所: `crates/areka-placement/examples/window-placement.rs:L298`（説明は `L10-L11`）
- 種類: バグ
- 問題: `build_balloon_target(&balloon_dir, &decoder, 0)` の第 3 引数はスコープ。測りはスコープごとの絵で窓の大きさを決めるので、scope 1 の窓（`balloonk0.png` 288×203 で測った寸）へ `balloons0.png`（400×224）を貼ることになる。
- 直し方: 第 3 引数を `scope as u32` にし、説明を直す。
- 直さないと: 手本で scope 1 のバルーンがはみ出すか切れ、目で見て確かめるときに誤る。
- 〔約束〕触れない。〔裁定〕当たらない。
- 規模: S

### 2.2 直したほうがよい（5〜40）

#### 5. 動く絵の上限の環境変数で、テスト 3 本が赤になる〔申し送り＝`audit-emo-atlas` 件 1・要件 2.7・2.8〕
- 場所:
  - `crates/areka-placement/src/measure_first_frame_tests.rs:L227-L231`・`L247-L259`
  - `crates/areka/src/placement_tests/measure_bake_log_tests.rs:L214-L262`（`L226-L228`・`L247-L261`）
  - `crates/areka/src/placement_tests/measure_bake_count_tests.rs:L204-L261`（`L114-L119`・`L230`・`L255-L259`）
- 種類: テストの抜け（環境で結果が変わる）
- 問題: `bake`／`bake_quiet` は `AnimationLimits::from_env()` を使う。これは `OnceLock` で、プロセスで 1 度だけ本物の環境を読む。
  - `AREKA_ANIMATED_IMAGE_MAX_FRAMES=1` か `_MAX_PIXELS`・`_MAX_TOTAL_PIXELS` に 7 以下を入れて走らせると、次のように赤になる。
    - `measure_first_frame_tests` の `Way::RawBake`（`L231`）: 「全透明」の 1 行が増え、`L258` が赤。
    - `measure_bake_log_tests`: `fade.png` の「全透明」が増え、`broken.png` の「動く絵として読めなかった」が消えて、`L247-L261` が赤。
    - `measure_bake_count_tests`（要件 2.8 が名指さない、新しく分かった分）: 資産の道で `decode_frames` が 1 から 0 へ、`decode_first_frame` が 0 から 1 へ変わり、`L230`・`L255-L259` が赤。
  - 同じファイルの `first_frame_measure_matches_full_frame_measure`（`L329-L357`）は緑のままだが、1 枚目どうしを比べるだけの空振りになる。
- 直し方:
  - 配置の crate の側は、`bake_with_limits(&sets, &dec, PackConfig::default(), AnimationLimits::default())`（公開済み）へ替えれば直る。
  - 本体の 2 本は、資産の道（`build_shell_assets` → `areka-emo-present` の `load_shell_target` → `bake`）に上限を渡す公開の口が無い。テストの側だけでは固定できない。
- 〔約束〕本体の 2 本を直すには、`areka-emo-present`（範囲の外）か本体の資産の道に、上限を渡す引数を足す必要がある。足すだけなら要件 8.2 には触れないが、範囲の外の crate に触れる（要件 7）。
- 〔裁定〕当たらない。
- 規模: M

#### 6. 理由の文が古くなった `#[allow]` と説明〔申し送り ⑵・要件 2.1〕
- 場所（どれも今の行で確かめた）:
  - 属性: `lib.rs:L69-L71`（`#[allow(unused_imports)]`・`pub use windowposition::scale_signed`）・`lib.rs:L123`（`MONITOR_SNAPSHOT_CONTEXT`）・`lib.rs:L789`（`prepare_ghost_windows`）・`lib.rs:L838`（`prepare_ghost_windows_with_work_area`）
  - `diag.rs:L53-L56` の前置き（「areka は bin crate ゆえ `pub` でも dead_code 免除されない」）
  - 属性でない説明: `descript_window_keys.rs:L7`・`measure_first_frame.rs:L11`・`spawn.rs:L33-L35`・`spawn.rs:L535-L537`・`crates/areka/src/placement_tests/spawn_assembly_tests.rs:L183-L185`
- 種類: 要らないコード（今の事実に合わない文と、効いていない属性）
- 問題:
  - 配置は lib の crate になった。lib の根から届く `pub` の項目には、`dead_code` も `unused_imports` も鳴らない。
  - examples は `#[path]` で src を取り込んでいない（`grep '#\[path'` の当たりはテストのモジュールの宣言だけ。手本は `areka_placement::…` を使う）。
  - そのため上の 4 つの属性は何もしていない。説明 5 か所の「`crate::` を使わない理由」も、もう事実でない。
  - 実例として、追随の持ち物の `follow/offset_space.rs:L66` は `use crate::scale_signed;` と書いて組めている。
- 直し方: 4 つの属性を理由の文ごと消す。説明の 5 か所は、理由の部分を消すか、今の事実（ほかの crate に依存しない・I/O を持たない、など）に直す。`spawn.rs:L193-L196` の「隣接 markers の allow」の注記（68 番）も一緒に直す。
- 〔約束〕触れない（属性と説明だけ。`scale_signed` の綴りと形は変えない）。〔裁定〕当たらない。
- 規模: S
- 調べが要る: 属性を外して `cargo check -p areka-placement --all-targets` を通し、警告が増えないことを実装で確かめる。

#### 7. 「古くなっていない」とされた `#[allow(dead_code)]` も、どれも効いていない〔申し送り ⑵・要件 2.2〕
- 場所:
  - `descript_window_keys.rs:L188`（`#[cfg_attr(not(test), allow(dead_code))]`・`BalloonSyncScale::is_synced`）
  - `resolver.rs:L418`（`virtual_desktop_union`）
  - `source.rs:L131`（`load_descript_source`・19 番と同じ）
  - `zorder_group_ledger.rs:L668`（`version`）
  - `diag.rs:L167`（`PlacementRoute::SpawnInitial`）・`diag.rs:L248`（`PlacementRoute::ALL`）
- 種類: 要らないコード
- 問題:
  - 理由に書かれた事実は、今も正しい。本番の呼び手は無い。`is_synced` の持ち主は実在する `shell-balloon-scale-setting`、`version` を読むのはテストだけ。
  - しかし、どれも lib の根から届く `pub` の項目（`pub mod` の中の `pub fn`・`pub enum` の欄・`pub const`）なので、属性は何もしていない。
- 直し方: 属性だけを消し、理由の doc は先送りの印として残す。`zorder_group_ledger.rs:L664-L667` の「許可を与える」の文、`diag.rs:L53-L56`（6 番）も合わせて直す。
- 〔裁定〕2 種類目の候補。`diag.rs:L167`（D13 帰結⑷「語彙のみ保持・将来の配線先として予約」）・`descript_window_keys.rs:L188`（要件 6.6 の持つだけ）・`resolver.rs:L418`（DD8 の後送り）は、わざと置いた先送りの印。属性を消しても、項目と理由の doc は残る。それでも「印を消す」に当たるかを聞く。ほかの 3 つ（`L248`・`L668`・`source.rs:L131`）は先送りの印ではない。
- 〔約束〕触れない。
- 規模: S

#### 8. 外から使うために開けた項目のうち、使い手の無い公開と、テストだけの公開〔申し送り ⑶・要件 2.3〕
- ツリー全体（`crates/` の、`crates/areka-placement/` の外）を名前で grep した結果。
  - ⑶ に挙がった 13 のモジュールには、どれも外に使い手がいる。いちばん少ないのは `balloon_syncscale`・`face_windowposition`・`zorder_chain_compose` の 1 ファイルずつ。
  - 根の `scale_signed`（`runtime/move_cue.rs` ほか 3 ファイル）・`load_scope_windowpositions`・`primary_monitor` と、⑶ の項目の大半は、本体の本番のコードが使う。
- 外に使い手が無い、またはテストだけのもの:
  - 使い手 0: `test_support::EMILY_SHAPED_GHOST_DESCRIPT`・`EMILY_SHAPED_SHELL_DESCRIPT`（`test_support.rs:L112-L117`。使うのは crate の中の `placement_char_clamp_tests.rs` と `emily_shaped_scopes` だけ）・`impl Default for AuthorDpi`（45 番）。
  - 外ではテストだけが使う本番の項目: `dpi_sync::evaluate`・`DpiSyncDecision`・`DpiSyncOutcome` の欄（`crates/areka/src/runtime/frame_dpi_sync_hold_tests.rs:L29`・`frame_transition_branch_tests.rs:L66`・`frame_work_area_resnap_hold_tests.rs:L48`）・`DPI_SYNC_HOLD_MAX_FRAMES`（本体のテストと、追随の持ち物の `placement_tests/transition_judge_verdict*`）・`balloon_lock::HoldReason`・`BalloonLockLedger::reason`（`ghost_session_restart_tests.rs:L273-L286`・`runtime/frame/switch_finish_tests.rs:L739-L765`）・`BALLOON_LIMIT_CLAMP_TAG`（`main_restore_seam_tests.rs`）・`PREPARE_GHOST_WINDOWS_CONTEXT`・`prepare_ghost_windows_with_work_area`（42 番）・`source::load_descript_source`（19 番）。
  - `cfg(any(test, feature = "test-support"))` の支えは、どれも使い手がいる。`spawn::test_support` の `titles`・`ghost_window_entities`・`two_scope_placements`、`reseed::test_support` の 3 つ、`measure::test_support` の 2 つ、`persist::project_restore`・`GhostTitles::from_scope_titles`。
- 直し方の候補:
  - 使い手 0 のものは `pub(crate)` にするか消す。
  - テストだけのものは、`#[cfg(any(test, feature = "test-support"))]` の内側へ移す（本体は dev-dependency でだけ `test-support` を付ける＝`crates/areka/Cargo.toml:L97`）か、今のまま残す。
  - `evaluate` は crate の中の本番が使うので、gate を掛けるなら公開の段だけを分ける形になる。
- 〔約束〕要件 8.2 に触れる。公開の形を狭める直しなので、C20 のあいだは開発者が認めた例外が要る。`DPI_SYNC_HOLD_MAX_FRAMES` は追随の持ち物（`transition_judge_verdict`）も使うので、要件 8.1 にも触れる。
- 〔裁定〕当たらない（約束の議題として扱う）。
- 規模: S

#### 9. `lib.rs` の頭の依存の向きの規約が、実際の参照の半分も名指さない。輪もある〔申し送り ⑷・要件 2.4・2.5〕
- 場所: `lib.rs:L8-L15`
- 実際の参照（本番のコードの `use (super|crate)::`。`#[cfg(test)]` から後と、説明の行は除いた）:
  - 名指す 6 つは、規約の向きを守っている。`config`→なし、`resolver`→`config`、`measure`→`resolver`・`error`・`descript_window_keys`、`spawn`→`config`・`diag`・`follow`・`resolver`・`chain_finalize`・`source`、`diag`→なし。
  - 規約の外の参照:
    - `measure`→`descript_window_keys`・`error`、`spawn`→`chain_finalize`・`source`（分けたときの記述と同じ）。
    - `descript_window_keys`→`config`。`descript_window_keys.rs:L13` は `use super::config::…` なのに、`L7` は「`crate::` を参照しない」と書く。
  - 輪:
    - `chain_finalize`↔`chain_realign`（`chain_finalize.rs:L72` の `#[component(on_insert = super::chain_realign::…)]`）
    - `chain_realign`→`spawn`→`chain_finalize`→`chain_realign`
    - `follow` との両向き: `balloon_limit`・`dpi_sync`・`sticky_group_ledger`・`spawn`。`follow` の側は 34 ファイルが、このファイルの側のモジュールを引く。
  - 根を引く子: `reset_pos`→根の `primary_monitor`。`follow/offset_space.rs:L66`→`crate::scale_signed`（追随の持ち物）。
  - 名指さない 21 のモジュールは、互いに広く引き合う。例: `reset_pos`→`chain_finalize`・`chain_realign`・`config`・`follow`・`persist`・`reseed`・`resolver`・`source`・`spawn`。
- 種類: 要らないコード（規約と実物の食い違い）
- 直し方の候補（どちらかは設計で決める・要件 2.4）:
  - 案 a: 規約の文を今の参照に合わせて書き直す。名指す範囲を全部の段に広げるか、「6 つだけの約束」と明記して輪を列挙する。
  - 案 b: 参照を直して輪を断つ。例えば `chain_finalize` の `on_insert` の hook を `chain_realign` の側の登録へ移す。
  - 根の `scale_signed` の形は変えずに済む見込み（要件 2.5 の申し合わせは、変えるときだけ要る）。
- 〔約束〕案 b で `follow` との輪に手を入れると、要件 8.1 に触れる（相手の持ち物の `use` が変わる）。
- 〔裁定〕当たらない。
- 規模: S（案 a）〜M（案 b）

#### 10. テストの一時フォルダの固定の名前が、ワークツリーの外（`%TEMP%`）にある
- 場所: `crates/areka-placement/src/placement_monitor_tests.rs:L250-L252`
- 種類: 危ない所（決まり「実機の根と一時フォルダはワークツリーの `target` の下」に反する）
- 問題: `std::env::temp_dir().join("areka_placement_prepare_snapshot_log")` は名前が固定。そのフォルダがどこかで作られると、検査の前提（root が無い）が崩れる。
- 直し方: 隣の `placement_prepare_tests.rs:L240-L241` と同じく `TempDir::new()` を使う。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 11. `measure_tests.rs` の `TempDir` と `with_com_initialized` が、共有の支えの写しで、`%TEMP%` の下に作る
- 場所: `crates/areka-placement/src/measure_tests.rs:L14-L25`・`L38-L68`
- 種類: 要らないコード・危ない所（10 番と同じ決まりに反する）
- 直し方: `crate::shared_test_support::TempDir`（`placement_shared_test_support.rs:L45-L55`・`target\test-roots\` の下）と `crate::test_support::with_com_initialized` を使う。約 35 行減り、1,000 行の上限にも余裕ができる。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 12. 「シェルの軸の換算 → windowposition の合流」の順の決まりが 2 か所に写してある
- 場所: `lib.rs:L424-L435`（`prepare_stages_for_shell`）・`reseed.rs:L182-L194`（`resolve_standard_scopes`）
- 種類: 要らないコード（一緒に変えるべき写し）
- 問題: 順を入れ替えると調整量が 2 回換算される。片方だけ直すと、起動とシェルの切り替えでバルーンの位置が黙って割れる。
- 直し方: 2 行を順に呼ぶ crate の中の関数を 1 つ置き、両方から呼ぶ。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 13. 主モニタの作業領域を `RECT` から `RectPx` へ写すコードが 3 か所にあり、DPI の写し方も 2 通り
- 場所: `lib.rs:L879-L893`（`work_area_of`）・`reset_pos.rs:L66-L77`・範囲の外の `crates/areka/src/runtime/move_cue_place.rs:L86-L95`。DPI は `lib.rs:L451`（`as u16` で切り詰め）と `reset_pos.rs:L76`（飽和）。
- 種類: 要らないコード
- 直し方: `primary_work_area(monitors) -> Option<RectPx>` を 1 つ置き、範囲の中の 2 か所から使う。DPI は飽和の形に揃える。
- 〔約束〕公開の関数を足すだけなら、要件 8.2 の範囲の中。`move_cue_place.rs` まで直すなら、範囲の外（本体の `runtime`）に触れる（直さなくても成り立つ）。
- 〔裁定〕当たらない。規模: S

#### 14. resolver のテストの補助が、同じ中身で 2 本ずつある
- 場所: `resolver_balloon_keyword_tests.rs:L22-L29`・`L50-L57` と `resolver_resolve_test_support.rs:L14-L21`・`L38-L45`（中身が同じ）。`PlacementConfig { .. }` の手書きは、ほかに `placement_windowposition_vocab_tests.rs:L35-L44`。
- 種類: 要らないコード
- 直し方: keyword_tests の 2 本を消し、`resolve_test_support` のものを使う。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 15. resolver の同じことを確かめるテストが重なり、作りの上で必ず成り立つ式も確かめている
- 場所:
  - `resolver_resolve_contract_tests.rs:L24-L45`（`anchor_propagates_five_values_from_resolved_alignment`。`resolver_from_alignment_tests.rs:L12-L43` の繰り返し）
  - `resolver_balloon_keyword_tests.rs:L388-L419`（T-K6。`resolver_resolve_balloon_tests.rs:L34-L130` の T-R8 と同じ表）
  - `resolver_balloon_keyword_tests.rs:L625-L653`（T-K8。`balloon_offset ≡ balloon_pos − char_pos` は `resolver.rs:L367-L370` が引き算で作るので、必ず成り立つ）
- 種類: 要らないコード（重なるテスト。規約「檻は判断の分岐に、配線には入れない」）
- 直し方: 上の 3 本を消す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 16. 記憶の値が壊れているとき・片方の軸だけのとき、`warn!` を出さずに既定の位置へ戻る
- 場所: `persist.rs:L48-L55`（`parse_px` の doc）・`L527-L584`（`merge_scope`）
- 種類: 危ない所（記録の無い失敗の道・要件 9.2）
- 問題: doc は「呼び手が warn を出して既定へ戻る」と書くが、呼び手のどこも warn を出さない。
  - 例: 手で直した `sylphya.toml` が `[window.0] x = "12.5"` だと、黙って既定の位置に立つ。並べ終えた時点の保存がその既定の位置で上書きし、壊れていたことはどこにも残らない。
- 直し方: `merge_scope` で、4 つの鍵について「鍵はあるが読めない」と「片方の軸だけ」のときに `warn!(target: "areka::persist::restore", …)` を 1 行ずつ出す。`persist_restore_tests.rs:L697` に、warn が出ることの確かめを足す。
- 〔約束〕触れない（記録の出所は `areka_placement::diag` ではない）。〔裁定〕当たらない。規模: S

#### 17. バルーン位置初期化が、キーワードで決まる標準のずらしを生の px で記憶へ焼き付ける
- 場所: `reset_balloon_pos.rs:L98-L99`
- 種類: 危ない所
- 問題: 次の起動では、`merge_scope` が保存した値を採る。そのため、キーワード（`windowposition.x,top`／`bottom`＝中央揃え）から標準を導き直す材料を落とす（`persist.rs:L570-L576`）。
  - 例: `windowposition.x,top` のゴーストで、surface1000（幅 382）を出しているときに初期化する。次の起動が surface0（幅 434）なら、中央から 26px ずれて出る。
  - 書かないだけにすると、古いドラッグの値が残ってもっと悪くなる（`.kiro/specs/completed/areka-P0-window-link-directives/research.md:L11`）。
- 直し方の候補:
  - 案 1: `areka-sylphya` に鍵を消す口を足し、初期化では鍵を消す（M。範囲の外）。
  - 案 2: キーワードの腕ではこのずれを許すと裁定し、記録だけを残す。
- 〔裁定〕1 種類目（`window-link-directives` の裁定 8「次の起動も標準の位置に出る」の形）と 3 種類目（保存して次に起動したときの位置）。
  - ukadoc https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.x の原文は「center(SSPのみ top でも同じ)でシェルの中央上に固定。」。
- 〔約束〕案 1 は範囲の外の `areka-sylphya` の公開の形を足す。
- 規模: M

#### 18. バルーンを標準の部品で据え直す塊が、3 か所にほぼ同じ形である
- 場所: `face_windowposition.rs:L66-L86`・`reset_balloon_pos.rs:L78-L97`・`reseed.rs:L79-L99`
- 種類: 要らないコード（`BalloonFollow::new` を入れる → `BalloonKeywordBase` を入れるか外す → `follow_balloon(.., Restore)` の 3 手が写し）
- 直し方: `reseed.rs` に crate の中の関数を 1 つ置き、3 か所から呼ぶ。
- 〔約束〕触れない（`follow_balloon` の綴りと形は変えない）。〔裁定〕当たらない。規模: S

#### 19. テストだけが使う `load_descript_source` が本番の公開にあり、`#[allow(dead_code)]` も効いていない
- 場所: `source.rs:L131-L134`
- 種類: 要らないコード（7・8 番の 1 項目。数は 1 回だけ数える）
- 直し方: 属性を `#[cfg(any(test, feature = "test-support"))]` に替える（`persist::project_restore` と同じ形）。外の使い手は `crates/areka/src/runtime/assets_tests.rs:L21`・`L695`（テストだけ）。
- 〔約束〕要件 8.2 に触れる（本番のビルドから公開が消える）。〔裁定〕当たらない。規模: S

#### 20. persist の、同じことを重ねて確かめるテスト
- 場所:
  - (a) `balloon_offset_persist_roundtrip_tests.rs:L459-L501`（`persist_restore_offset_base_tests.rs:L59-L126` と同じ 3 つの腕）
  - (b) `balloon_offset_supply_tests.rs:L366-L378`（`L333-L353` と同じ値）
  - (c) `balloon_offset_entries` の生の値の確かめ 3 本（`persist_entries_tests.rs:L95-L117`・`persist_restore_tests.rs:L29-L61`・`roundtrip:L262-L274`）
  - (d) `persist_restore_tests.rs:L65-L90`（`L95-L131` にほぼ含まれる）
- 種類: 要らないコード
- 直し方: (a)(b) と (c) のうち 2 本を消す。(d) は balloon_pos の行を移してから消す。roundtrip の裁定（2026-08-14・08-27）を確かめる本体は残す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 21. `measure_tests.rs` で、同じ emo2 の測りを繰り返すだけのテストが 2 本ある
- 場所: `measure_tests.rs:L129-L162`・`L330-L379`・`L118-L125`（どれも `L114-L117` の厳密な値から導ける。COM と WIC で焼き直す重いテスト）
- 種類: 要らないコード
- 直し方: 消す（約 90 行減る）。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 22. `config_limit_tests.rs` の 5 本が、何も確かめていない
- 場所: `config_limit_tests.rs:L23-L29`・`L56-L60`・`L62-L75`・`L107-L113`・`L119-L148`・`L150-L173`
  - derive の確かめ、テスト自身が書いた欄の読み返し、テストの中の写し `scope_config_for`。
  - 本物の未収載の枝は、`resolver_balloon_keyword_tests.rs:L479`・`resolver_resolve_tests.rs:L64` が確かめ済み。
  - 同じ形の空振りが `config.rs:L627-L643` にもある。
- 種類: 要らないコード
- 直し方: 5 本と `scope_config_for` を消し、2 本を残す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 23. 設定の読みの「読めない値」の枝に、確かめるテストが無い
- 場所: `config.rs:L239-L251`（`balloon.alignment` の未知の値 → Left＋`warn!`）・`L287-L301`（`parse_i32` の `warn!`）・説明の `L518`
- 種類: テストの抜け
- 問題: `center` などを入れるテストが、ツリーに 0 本。枝を `Auto` に変えても、warn を消しても赤にならない。`L518` の「呼び出し側 warn」は今の実装と合わない。
- 直し方: 1 行の入力と、`log_capture_kit::capture` で 2 つの warn が 1 件ずつ出ることを確かめるテストを足す。`L518` を直す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 24. `shell_kv_with_same_key_is_not_used` が何も確かめていない
- 場所: `descript_window_keys_tests.rs:L100-L118`（`shell_kv` を作るが、どこにも渡さない。中身は `L16-L28` と同じ）
- 種類: 要らないコード
- 直し方: 消す。〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 25. 公開の面へ届くかを確かめるテストが、組める以上のことを確かめていない
- 場所: `spawn_zorder_pair_export_tests.rs:L1-L116`（全体）・`spawn_zorder_pair_wiring_tests.rs:L133-L153`
- 種類: 要らないコード
- 問題:
  - 届くかどうかは、`spawn.rs:L91-L95` の import が組めることで確かめ済み。中身は bevy の insert・get・remove だけ。
  - 2 本目は wintf の `Default` を確かめているが、本番は `wire_zorder_pair` が値を明示して入れる。
  - `wire_inserts_plan_a_strategy_resource` は `wire_logs_the_strategy_it_actually_inserted` に含まれる。
- 直し方: export_tests の 1 ファイルと、`spawn.rs` の宣言を消す。`wire_inserts_plan_a_strategy_resource` も消す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 26. クリック透過の登録の口が無いとき、記録を残さずに登録を捨てる
- 場所: `spawn.rs:L780-L791`（`L784-L786`）
- 種類: 危ない所（要件 9.2）
- 問題: `handle` が `None` だと `return` するだけ。`Added<WindowHandle>` はその巡で消費されるので、その窓は二度と登録されない。今は wintf の `runtime/mod.rs:L335` が先に挿すので起きない。
- 直し方: 窓があれば `warn!` を出してから戻す。warn が出ることを確かめる 1 行を、既存のテストに足す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 27. `PlacementRoute::ALL` の入れ忘れを捕まえる、と書かれたテストが実際には無い
- 場所: `diag.rs:L241-L263`・`diag_tests.rs:L72-L103`（同じ形の見送りの理由の一覧が `chain_finalize_tests.rs:L435-L453`）
- 種類: テストの抜け
- 問題: テストは `ALL` と同じ 13 個を手で並べ直しているだけ。14 個目を足して `ALL` に入れ忘れても組めてしまい、`ALL` を回すテストが全部その経路を飛ばす。`is_system_reanchor` の区分を誤っても赤にならない。
- 直し方:
  - 小: doc の偽りの主張を外す。
  - 筋: 列挙と `ALL` を 1 つの `macro_rules!` で同時に定義する（依存は不要）。
- 〔約束〕名前と形は変えない（`follow/window_move.rs`・`follow/visibility.rs` は読むだけ）。〔裁定〕当たらない。
- 規模: S〜M

#### 28. 連鎖のテストが 2 ファイルで同じ入力を 2 回確かめ、テストだけの口が本番のファイルにある
- 場所: `chain_finalize.rs:L129-L143`（`#[cfg(test)] pub fn finalize_chain`）・`chain_finalize_tests.rs:L115-L331`・`chain_finalize_resolve_tests.rs:L64-L180`
  - 重なる組の例: `non_positive_width_is_skipped_and_keeps_its_place` と `non_positive_width_is_kept_and_is_next_base`（入力も期待も同じ）。ほかに 6 組。
- 種類: 要らないコード
- 直し方: chain_finalize_tests は 4 本（実機の再現・欠陥の式の否定・2 度目は何もしない・停滞の診断）を残し、重なる 7 本と `finalize_chain` を消す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 29. 「その幅で式が立つか」の決まりが 2 か所に別々に書いてある
- 場所: `chain_finalize.rs:L381-L391`（`resolve_chain` の判定）・`L414-L427`（`mark_non_positive_widths`）
- 種類: 要らないコード
- 問題: 片方だけ緩めると、動かした窓に「non-positive-width」と記録してしまう。
- 直し方: crate の中の `join_usable` を 1 つ置き、両方から呼ぶ。
- 〔約束〕触れない（呼び手 `chain_realign.rs:L319`・`crates/areka/src/runtime/frame/drain_resnap.rs:L507` の形は変わらない）。〔裁定〕当たらない。規模: S

#### 30. `t_zpd50` は台帳をどこにも渡さず、何も確かめていない
- 場所: `zorder_property_deferral_tests.rs:L553-L612`・`zorder_group_ledger.rs:L721-L722`
- 種類: 要らないコード
- 問題: 台帳はテストの中の変数で、`SylphyaReader` にも `SylphyaCore::apply` にも渡らない。本番がどう変わっても赤にならない。本当に守るべきことは `t_zpd30` が押さえている。
- 直し方: `t_zpd50` と import を消し、置き場の理由の文（`ledger.rs:L721-L722`）を直す。
- 〔裁定〕2 種類目の候補（先送りの檻＝要件 13 のテストの一部を消す）。
- 〔約束〕触れない。規模: S

#### 31. もう直った本番の欠陥を「未解決」と書いた説明が残っている
- 場所: `zorder_chain_tail_order_tests.rs:L5-L7`・`L48-L74`
- 種類: 要らないコード（古い説明）
- 問題: wintf の `nudge_command`（`crates/wintf/src/ecs/window/zorder_chain.rs:L451-L464`）はもう直っているのに、「本番の欠陥・task 2.3 の担当」と書いてある。ファイルの数と行の番号も古い。
- 直し方: 1〜2 行に縮め、数と名前を今のファイルに合わせる。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 32. 実際の窓を使うテスト 2 本の Win32 の道具がほぼ写しで、もう中身がずれている
- 場所: `zorder_chain_tail_order_tests.rs:L161-L349`・`zorder_chain_inversion_order_tests.rs:L60-L140`（tail の側だけに走査の上限 `steps > 100_000` と `is_invalid` の確かめがある）
- 種類: 要らないコード
- 直し方: `#[cfg(test)]` の兄弟の支えへまとめる。
- 〔約束〕触れない（wintf の側の写しは範囲の外）。〔裁定〕当たらない。規模: S

#### 33. Z 順の解釈のテスト 2 ファイルに、同じ入力・同じ期待のテストが重なっている
- 場所: `zorder_group_ledger_tests.rs` と `zorder_group_ledger_written_order_tests.rs`
  - t_zwo06 と t_zgp19・t_zgp22、t_zwo08 と t_zgp25、t_zwo10 と t_zgp1、t_zwo12 と t_zgp3、t_zgp26 と t_zgp18、t_zgp27 の頭の 3 つの assert。
- 種類: 要らないコード
- 直し方: 分岐の対応表が名指さないものだけを消す（t_zwo08・t_zwo12・t_zgp26・t_zgp19 の前半・t_zgp27 の頭の 3 つ）。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 34. 分岐の対応表の検査 2 本の本体がほぼ写し（もう中身もずれている）
- 場所: `crates/areka/src/placement_tests/zorder_group_branch_coverage_tests.rs:L388-L477`・`zorder_inversion_branch_coverage_tests.rs:L388-L453`（inversion の側にだけ、分岐の名前の重なりの検査がある）
- 種類: 要らないコード
- 直し方: `check_table(branches, files, coverage)` を 1 つ置き、両方の表から呼ぶ。
- 〔約束〕触れない（範囲の中の本体のテスト）。〔裁定〕当たらない。規模: S

#### 35. シェルの切り替えで作業領域が引けないとき、記録を残さずにキャラ窓の矩形で代える
- 場所: `reseed.rs:L205-L207`・`L227-L234`
- 種類: 危ない所（要件 9.2）
- 問題: `MonitorSnapshot` が無いか空だと、`unwrap_or(running.char_rect)` で代え、warn を出さない。`balloon.alignment,none` のバルーンは反対の側に付くことがある。起動の関門（`balloon_limit.rs:L259-L268`）は、同じ場面で warn を出している。
- 直し方: `unwrap_or_else` の中で `warn!` を出す。テストを 1 本足す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

#### 36. シェルの切り替えでも、窓の無いスコープの記録が `when="seed"` と出る
- 場所: `sticky_group_ledger.rs:L262-L284`（`L275`）
- 種類: バグ（記録の取り違え）・テストの抜け
- 直し方: `apply_descript` に `when` を足し、切り替えの側では `"replace"` を渡す。テストを 1 本足す。
- 〔約束〕触れない（crate の中の関数）。〔裁定〕当たらない。規模: S

#### 37. 収める式 `clamp_axis` が 3 か所に写してあり、テストは 4 つ目の写しと比べている
- 場所: `balloon_limit.rs:L83-L92`・`resolver.rs:L408-L410`・`persist.rs:L160-L162`（手で並べ直した所: `resolver.rs:L317-L318`・`persist.rs:L259-L268`）・`balloon_limit_tests.rs:L57-L63`・`L356-L382`
- 種類: 要らないコード（一緒に変えるべき写し）
- 問題: `t_bl6` はテストの中の写し `clamp_axis_reference` と比べているので、resolver の式を変えても緑のまま。
- 直し方: resolver の `clamp_axis` を crate の中に開き、写しを消す。`t_bl6` と参照の写しを消す。
- 〔約束〕触れない（`clamp_rect_to_work_area` の名前と形は変えない。追随の持ち物の `work_area_clamp.rs`・`move_shift.rs` が使う）。〔裁定〕当たらない。規模: S

#### 38. 帰属のモニタの拡大率を引く式が 2 つあり、記録を切っていても毎回計算する
- 場所: `dpi_sync.rs:L184-L189`・`L264-L273`
- 種類: 要らないコード・小さな遅さ（窓を書く点ごとに毎フレーム `dpi_for_point` を回すが、記録を切っていると使わない）
- 直し方: `table_dpi_of` を 1 つ置く。`defers_window_write` で記録の値を組むのは `transition_diag::is_enabled()` が真のときだけにする。
- 〔約束〕触れない（公開の型と形は同じ）。〔裁定〕当たらない。規模: S

#### 39. 「`DPI` を持たない窓は進む」という doc と、実際の振る舞いが逆
- 場所: `dpi_sync.rs:L176-L199`（`L182`・`L184`）
- 種類: バグ（約束と中身の食い違い）・テストの抜け
- 問題: `map_or(0, ..)` で窓の拡大率を 0 とし、`Hold` を返す。その結果、30 フレーム見送った後で警告が出る。今の呼び手（`crates/areka/src/runtime/frame/dpi.rs:L336-L381`）は DPI のある窓しか渡さないので、今は起きない。
  - 読み手の報告は「外の crate は `evaluate` を使っていない」としていたが、誤り。本体のテスト 3 本が使う（8 番）。形は変えないので、直しには響かない。
- 直し方: DPI が無ければ Proceed にする。テストを 1 本足す。
- 〔約束〕触れない（形は同じ。本体のテスト 3 本は DPI のある窓だけを使う）。〔裁定〕当たらない。規模: S

#### 40. 帳簿が無いときの扱いが、同じモジュールの中で黙る所と記録する所に割れている
- 場所: `balloon_lock.rs:L55-L61`（`L58` の `get_resource_or_insert_with`）と `L65-L69`（`warn!(event = "balloon_lock_ledger_missing")`）
- 種類: 危ない所（要件 9.2）
- 直し方: `replace_balloon_lock_descript` でも同じ warn を出してから帳簿を据える。テストを 1 本足す。
- 〔約束〕触れない。〔裁定〕当たらない。規模: S

### 2.3 できれば（41〜80）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 41 | 説明に古い名前 `emo2_boot`・`Emo2Wiring` が 13 ファイル・27 行残る〔申し送り・要件 2.9〕 | 3 節の表 | 今の名前（`runtime` ほか）へ直し、`runtime-crate-split` の brief へ数を申し送る | — |
| 42 | テストだけが使う入口 `prepare_ghost_windows_with_work_area` が本番に入っている。`prepare_stages` は 1 行の取り次ぎ | `lib.rs:L826-L846`・`L373-L379` | 前者に `cfg(any(test, feature="test-support"))` を付ける（外の使い手は本体のテスト 5 か所）。後者は消す | 〔約束〕8.2 |
| 43 | `apply_scope_windowpositions` の `present` の絞り込みは何も絞っていない | `lib.rs:L589-L595` | `load_scope_windowpositions` を直接呼ぶ | — |
| 44 | `apply_windowposition` が、手元にある `ScopeConfig` をもう一度引く。本番では届かない腕とそのテストがある | `windowposition.rs:L202-L225`・`L433-L439` | 引数を `&mut ScopeConfig` にし、腕とテストを消す | — |
| 45 | `impl Default for AuthorDpi` に使い手が無い | `lib.rs:L202-L206` | 消す | 〔約束〕8.2（外の使い手 0） |
| 46 | 「綴りを固定するテストはまだ無い」と書いてあるが、ある（`balloon_offset_supply_tests.rs:L310`） | `lib.rs:L459-L461` | その 1 文を消す | — |
| 47 | `async-io` の版の指定を 3 つの crate で手で揃えている | `Cargo.toml:L46`・`crates/areka/Cargo.toml:L47`・`crates/wintf/Cargo.toml:L36` | ワークスペースの表へ移す | 〔約束〕根の `Cargo.toml`（C20 は `wintf-winmsg-executor-import` が触る）と wintf の `Cargo.toml`（`audit-wintf-window` の持ち物の近く）。`Cargo.lock` は変わらない見込み |
| 48 | `balloon.alignment,none` の「ちょうど中央」の腕を確かめていない（`<` を `<=` に変えても落ちない） | `resolver.rs:L348-L354` | 境のテストを 1 本足す | — |
| 49 | 純粋な関数を 2 回呼んで比べるだけのテスト 2 本 | `persist_entries_tests.rs:L31-L36`・`persist_restore_tests.rs:L770-L786` | 消す | — |
| 50 | バルーン位置初期化が、スコープの数だけ記憶へ書きに行く（そのたびに fsync・rename が走る） | `reset_balloon_pos.rs:L99` | 束ねて 1 回だけ `persist_entries` を呼ぶ | — |
| 51 | `load_restored_state` の `default_encoding` に、どの呼び手も `Ansi` を渡す | `persist.rs:L303-L306` | 引数を消す（呼び手は `crates/areka/src/main.rs:L785`・`runtime/switch_assets.rs:L239`・テスト） | 〔約束〕8.2（公開の関数の形） |
| 52 | 記憶へ書く時機を「2 つだけ」とする doc が古い（今は 6 か所から書く） | `persist.rs:L9-L15`・`L633-L639` | 一覧に合わせて直す | — |
| 53 | descript を読んで KV にする 1 行が 2 か所にある | `source.rs:L158-L159`・`L187-L188` | `read_kv` を 1 つ置く | — |
| 54 | `emo2_utf8_fixture_decodes_to_the_same_text_as_before` は parser だけを確かめ、説明も古い | `measure_tests.rs:L945-L949`・`L980-L999` | 消して較正の当たり（`measure_template_tests.rs:L186-L215`）を移すか、説明だけを直す | — |
| 55 | 「`balloon: `」の接頭辞を 2 か所で別々に作る | `measure.rs:L114-L119`・`L447-L450` | `SizeKind::Balloon.reason_prefix()` を使う | — |
| 56 | scope を u32 へ変える形が同じファイルの中で 2 通り（`as` は黙って切り詰める） | `descript_window_keys.rs:L127-L130`・`L204-L211` | `u32::try_from` に揃える | — |
| 57 | `with_com_initialized` が、初期化に失敗したときも `CoUninitialize` を呼ぶ | `test_support.rs:L18-L26` | 成功したときだけ呼ぶ（公開の関数。形は同じ） | — |
| 58 | `apply_scaling` は 1 行で書ける | `measure.rs:L303-L312` | `map().collect()` | — |
| 59 | 古い説明（行の番号・行数・文の中身） | `measure_tests.rs:L665`・`measure_template_tests.rs:L25` | 今の値に直す | — |
| 60 | `sample_test_support_tests.rs` の 2 本は配線を確かめるだけ | `sample_test_support_tests.rs:L10-L28` | 消す | — |
| 61 | 手本の「面が無いとき」の代わりの面が、測りと違う | `examples/window-placement.rs:L280-L284`・`measure.rs:L245` | `builtin_default_surface(scope)` に揃える | — |
| 62 | `moved_default_pos` は 1 行の薄い包み。呼び手は直後に y を上書きする | `chain_finalize.rs:L174-L183`・`chain_finalize_tests.rs:L337-L346` | 関数とテストを消し、呼び手で書く | 〔約束〕8.2（公開の関数を消す。呼び手は `crates/areka/src/runtime/frame/drain_resnap.rs:L16`・`L588`） |
| 63 | `external_position_authority()` は定数を返すだけ | `spawn.rs:L629-L645` | 値そのものにする | — |
| 64 | 実際の窓を作るテストの道具が、2 ファイルに丸ごと写してある | `spawn_zorder_pair_wiring_tests.rs:L50-L98`・`spawn_zorder_chain_wiring_tests.rs:L55-L97`・`L275-L282` | `spawn_test_support.rs` へ移し、`code_only` は既存のものを使う | — |
| 65 | 先送りの語彙を走査する 2 つの片割れが、近い写し | `crates/areka-placement/src/spawn_zorder_pair_deferred_tests.rs:L83-L131`・`L248-L315`・`crates/areka/src/placement_tests/spawn_zorder_pair_deferred_tests.rs:L43-L91`・`L143-L205` | 共有の `test_support::zorder_pair_deferred` に関数を足す | 〔約束〕`test_support` の公開を足す（8.2 の範囲の中・本体の側も範囲の中） |
| 66 | `t_i3` が `Anchored` を重ねて確かめている | `spawn_assembly_tests.rs:L571-L581`・`L592-L595` | 3 つの assert を消す | — |
| 67 | 値が無いことを表す `"-"` を、wintf と別々に持つ | `diag.rs:L141-L145`（wintf の `transition_diag.rs:L60` の `MISSING`） | wintf の定数を使う | 〔約束〕wintf の定数を読むだけ（8.3 の型と綴りは変えない） |
| 68 | 数や名前が古くなった説明 | `diag.rs:L35`・`L151`・`L244`・`diag_tests.rs:L45`・`L367`・`chain_finalize_tests.rs:L202`・`spawn.rs:L193-L196` | 今の物に合わせる（`spawn.rs` の分は 6 番と一緒に） | — |
| 69 | 消したテストの経緯を書いた注記 | `spawn_clickthrough_tests.rs:L9-L21`・`spawn_follow_pipeline_tests.rs:L255-L258` | 消す | — |
| 70 | `spawn_ghost_windows` が Resource の写しを戻り値でも返す（写しに書いても効かない） | `spawn.rs:L441-L447`・`L614-L616` | doc に明記する、または戻り値を変える | 〔約束〕戻り値を変えるなら 8.2（`crates/areka/src/ghost_session.rs:L293`・examples・テストの支え多数）。doc だけなら触れない |
| 71 | 停滞の診断を時間でなく回数で数える。tick の門を開けると警告が大きく遅れる | `chain_finalize.rs:L446-L452`・`L591-L601` | doc に条件を足す。時間で数えるのは門を既定で開ける段で | 〔約束〕時間にするなら 8.2（`note_chain_deferral` の呼び手 `chain_realign.rs:L412`・`drain_resnap.rs:L717`） |
| 72 | 補いの段と再指定の判定が、要素の数の 2 乗で回る | `zorder_group_ledger.rs:L372-L445`・`L676-L690` | `seen` の HashSet と位置の HashMap を使う | 調べが要る（台本の引数の長さの上限） |
| 73 | 毎フレーム、窓の無い宣言の要素の数だけ文字列を作る | `zorder_chain_compose.rs:L144` | `ChainPlan.absent` を値で持ち、文字列にするのは記録のときだけにする | 〔約束〕8.3（wintf の型 `ChainPlan` が変わる。本体の `runtime/frame/zorder_drain.rs` にも響く） |
| 74 | 合成のテストの中の重なり 2 つ | `zorder_chain_compose_tests.rs:L743-L746`・`L787-L811` | t_zcc17 と、t_zcc18 の呼び出しを消す | — |
| 75 | 状態のテストの説明が、実際の入力と合わない | `zorder_group_ledger_state_tests.rs:L595-L600`・`L651-L675` | 説明と入力を直す | — |
| 76 | テストの小さな道具 `code_only`・`token` の写し | `zorder_group_ledger_state_tests.rs:L115-L122`・`zorder_chain_compose_written_order_tests.rs:L246-L251` | 既存の `code_only`・本番の `element_text` を使う | — |
| 77 | `b`・`s`・`ent`・`edge`・`tag_group` の道具が 6 ファイルに写してある | `zorder_group_ledger_tests.rs:L40-L53` ほか 5 ファイル | `#[cfg(test)]` の兄弟の支え 1 つにまとめる | — |
| 78 | 記憶の重ね合わせが空のときの記録が、解決が空のときと同じ語 | `reseed.rs:L67-L75`・`L213-L219` | `reason = "restore_empty"` に分ける | — |
| 79 | `t_bl7` は中身をなぞるだけ | `balloon_limit_tests.rs:L388-L410` | 消す | — |
| 80 | テストの小道具が、兄弟のファイルどうしで何度も写してある（`contained` は片方だけ `saturating_add` で、もうずれている） | `balloon_limit_tests.rs:L26-L33`・`L69-L74`・`balloon_limit_gate_tests.rs:L428-L466`・`balloon_limit_flip_tests.rs:L69-L82`・`L212-L217`・`balloon_lock_drag_tests.rs:L51-L66`・`L264-L266`・`balloon_lock_tests.rs:L32-L34`・`reset_balloon_pos_tests.rs:L42-L50` | 範囲の中は 1 つにまとめる | 〔約束〕`SharedFakeIo` まで（16 ファイル）なら、範囲の外の `areka-sylphya` の公開に impl を足す |

41〜80 の〔裁定〕は、どれも「当たらない」。規模は 41（M・13 ファイル）・72（S〜M）・73（M）のほかは、どれも S。

---

## 3. 申し送りの件の結果（要件 2）

### 3.1 ⑴ wintf との重なり（要件 2.6）

- `diag.rs:L371-L383` の `MonitorRecord` と、wintf の `ecs/window/transition_diag.rs:L403` の `MonitorRecord` は、名前が同じだけで中身が違う。前者は起動のときのモニタの写し、後者はモニタの変化の記録。混ぜて使う所も無い（grep で確かめた）。
- `resolver.rs:L47` の `RectPx` も、audit では指摘にならなかった。
- 要件 2.6 の「audit が指摘したら」の条件には当たらない。直すものは無い。

### 3.2 ⑵ `#[allow]` と説明（要件 2.1・2.2）→ 6・7・19・68 番

- 古い理由の属性 4 つ（`lib.rs:L71`・`L123`・`L789`・`L838`）は、どれも効いていない。属性ごと消せる見込み。
- 「古くなっていない」とされた 5 つ（`descript_window_keys.rs:L188`・`lib.rs:L838`・`resolver.rs:L418`・`source.rs:L131`・`zorder_group_ledger.rs:L668`）と、`diag.rs` の 2 つ（`L167`・`L248`）。
  - 理由の事実は今も正しい。
  - ただし `pub` の項目なので、属性は効いていない（7 番）。
  - 先送りの印の 3 つ（`diag.rs:L167`・`descript_window_keys.rs:L188`・`resolver.rs:L418`）は裁定の候補。

### 3.3 ⑶ 外から使うために開けた項目（要件 2.3）→ 8・19・42・45 番

- ⑶ に挙がった項目には、どれも外に使い手がいる。
- 使い手の無い公開は `EMILY_SHAPED_GHOST_DESCRIPT`・`EMILY_SHAPED_SHELL_DESCRIPT`・`impl Default for AuthorDpi` の 3 つ。
- 外ではテストだけが使う公開は 8 番の表のとおり。

### 3.4 ⑷ 依存の向きの規約（要件 2.4・2.5）→ 9 番

- 名指す 6 つは向きを守っている。
- 食い違いは、名指さない 21 のモジュール、輪 3 種（`chain_finalize`↔`chain_realign`・`chain_realign`→`spawn`→`chain_finalize`・`follow` との両向き）、`descript_window_keys.rs:L7` の説明の嘘。
- 根の `scale_signed`（`windowposition.rs:L133`・`lib.rs:L72` で出し直す）は形を変えずに済む見込み。

### 3.5 `audit-emo-atlas` 件 1（要件 2.7・2.8）→ 5 番

- `measure_first_frame_tests.rs` の直しは公開済みの `bake_with_limits` で済む。
- `measure_bake_log_tests.rs` に加えて `measure_bake_count_tests.rs` も、環境変数で赤になる。本体の 2 本を直すには、範囲の外の資産の道に上限を渡す口が要る。

### 3.6 古い名前（要件 2.9）→ 41 番

| ファイル（`crates/areka-placement/` の下） | 行 |
|---|---|
| `src/lib.rs`（L69・L105・L133・L179・L185・L228） | 6 |
| `src/chain_finalize.rs`（L29・L34・L63・L448） | 4 |
| `src/spawn_zorder_pair_deferred_tests.rs`（L12・L42・L43） | 3 |
| `src/chain_realign.rs`（L139・L146） | 2 |
| `src/config.rs`（L103・L104） | 2 |
| `src/measure_template_tests.rs`（L11・L20） | 2 |
| `src/placement_prepare_tests.rs`（L616・L683） | 2 |
| `src/balloon_offset_persist_roundtrip_tests.rs`（L198） | 1 |
| `src/dpi_sync.rs`（L102） | 1 |
| `src/measure.rs`（L387） | 1 |
| `src/sample_test_support.rs`（L3） | 1 |
| `src/spawn.rs`（L360） | 1 |
| `src/zorder_chain_compose.rs`（L78） | 1 |
| 計 13 ファイル | 27 |

- 本体の `placement_tests/` の 9 ファイルには 0 行。
- `lib.rs:L69` は 6 番の属性と一緒に消える。
- `runtime-crate-split` の brief の数との突き合わせ:
  - brief の「本体の外の 43 ファイル・76 行」は、今の HEAD で数え直しても同じ（`rg -n 'emo2_boot|Emo2Wiring' crates -g '!crates/areka/**'`）。
  - 「`areka-placement` の 4 行は棚卸㊲で直した」も事実と合う。PR#454（`ea3e809b`）が `diag.rs`・`persist.rs`・`persist_io_wiring_tests.rs` の 4 行を直している。
  - この spec が 27 行を直せば、`runtime-crate-split` に残るのは 30 ファイル・49 行になる。`areka-placement` の中で残るのは、追随の持ち物の 9 ファイル・20 行で、こちらは `audit-placement-follow` の持ち場。

---

## 4. 要件と今あるものの突き合わせ

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1 洗い出し | 本書の 80 件 | 無し（全部読んだ・件数の上限なし） |
| 2.1〜2.2 `#[allow]` | 6・7 番 | 属性の消し方と、先送りの印の扱い（7 番は裁定の候補） |
| 2.3 公開の見直し | 8 番 | 狭める直しは要件 8.2 と当たる（Constraint） |
| 2.4〜2.5 依存の規約 | 9 番 | 文を直すか参照を直すかは、設計で決める（Unknown） |
| 2.7〜2.8 環境変数 | 5 番 | 本体の 2 本は、範囲の外に上限を渡す口が無い（Missing） |
| 2.9 古い名前 | 41 番 | 数は確定。申し送りの文が要る |
| 4・6 直しと確かめ | — | バグと危ない所 25 件（1〜5・10・11・16・17・23・26・27・35〜40・48・56・57・70・71）に、壊れたら落ちるテストを置く |
| 6.7 決定論 | 5・10・11 番 | 環境変数と `%TEMP%` に頼るテストを直す |
| 7 範囲の外 | 1・3・5・13・17・47・51・62・65・70・71・73・80 | 3（`areka-emo-atlas`）・5（`areka-emo-present`／本体の資産）・17（`areka-sylphya`）・80（`areka-sylphya`）は範囲の外の crate |
| 8 並走の約束 | 5 節の表 | 14 件に響く |
| 9 決まり | 10・11（一時フォルダ）・16・26・35・40（記録の無い失敗） | 決まりの違反そのものを直す指摘 |
| 10 大きさ | 6 節 | 20 を越える見込み |

## 5. 並走の約束（要件 8）に触れる指摘

| 番号 | 触れる約束 | 響く相手 |
|---|---|---|
| 2（筋の案） | 8.1 | `audit-placement-follow`（`follow/drag_follow.rs` ほか） |
| 3 | 7（範囲の外の crate） | `areka-emo-atlas`（完了した `audit-emo-atlas` の持ち物・申し送り先が無い） |
| 5 | 7（範囲の外の crate） | `areka-emo-present`、または本体の `runtime` の資産の道 |
| 8 | 8.2（公開を狭める）・8.1（`DPI_SYNC_HOLD_MAX_FRAMES`） | 本体のテスト・`audit-placement-follow` |
| 9（案 b） | 8.1 | `audit-placement-follow`（`follow*` の `use`） |
| 13（呼び手まで） | 7 | 本体の `runtime/move_cue_place.rs` |
| 17（案 1） | 7（範囲の外の crate） | `areka-sylphya` |
| 19・42 | 8.2（cfg で本番から公開を消す） | 本体のテスト |
| 45 | 8.2（公開の trait の実装を消す・外の使い手 0） | 無し |
| 47 | 根の `Cargo.toml` と wintf の `Cargo.toml` に触る | `wintf-winmsg-executor-import`（根の `Cargo.toml`）・`audit-wintf-window` |
| 51・62 | 8.2（公開の関数の形を変える・消す） | 本体の `main.rs`・`runtime/switch_assets.rs`・`runtime/frame/drain_resnap.rs` |
| 70・71（形を変える案） | 8.2 | 本体の `ghost_session.rs`・`drain_resnap.rs`・テストの支え |
| 73 | 8.3（wintf の型 `ChainPlan`） | `audit-wintf-window`・本体の `runtime/frame/zorder_drain.rs` |
| 80（`SharedFakeIo` まで） | 7（範囲の外の crate） | `areka-sylphya` |

- `DESPAWNED_SKIP_TAG`（`diag.rs:L139`・wintf の `lifecycle.rs:L34`）と、記録の出所 `areka_placement::diag`（wintf の `window_pos_tests.rs:L14`）を変える指摘は 0 件。
- 依存を足す指摘も 0 件。47 番は `Cargo.lock` を変えない見込み。

## 6. 直し方の選択肢・規模・危うさ

### 6.1 選択肢

- **案 A（今のファイルの中で直す）**: 指摘の大半（S の 70 件あまり）は、今あるファイルの中で消す・まとめる・1 行を足す直し。新しいファイルは要らない。
- **案 B（新しいテストの支えを置く）**: 写しをまとめる指摘には、`#[cfg(test)]` の兄弟の支えを 1〜3 つ新しく置く案がある（32・64・77・80 の写しと、3 か所の写しの本番の関数＝12・13・18・37）。1,000 行の番人に掛からないよう、新しいファイルにする。
- **案 C（混ぜる・推す形）**: 本番の直しは案 A、テストの写しは案 B にする。並走の約束に触れる 14 件は、約束に触れない形（doc だけ・足すだけ）を先に採り、残りを裁定に回す。
  - よい点: C20 のうちに着地できる。
  - 弱い点: 約束に触れる指摘の一部（8・51・62・73）が、後の spec へ回る。

### 6.2 規模と危うさ

- 規模: L（1〜2 週）。80 件のうち M は 7 件（1・2 の筋の案・5・9 の案 b・17・41・73）で、残りは S。ファイルのまとまりで束ねると 22〜26 タスク。これに、かけ直しの audit（要件 6）と全体テストで 2〜4 タスクが足される。
- 危うさ: 中。
  - 指摘のほとんどは消す・まとめる直しで、今の形の延長にある。
  - 1（窓の出る位置）・17（次の起動の位置）は外から見た振る舞いを変える。3・5・17 は範囲の外の crate に及ぶ。

### 6.3 20 タスクを越える件（要件 10）

- 越える見込み。2 本目の切り方の候補を挙げる（決めは要件の討議）。
  - 切り方 1（組で）: 1 本目＝必ず直す＋直したほうがよい（1〜40）、2 本目＝できれば（41〜80）。
    - よい点: 1 本目に危ない直しが集まる。
    - 弱い点: 同じファイルを 2 本が続けて触る（例: `lib.rs` は 6・12・13・42〜46）。
  - 切り方 2（ファイルのまとまりで）: 1 本目＝組 A・B・F（`lib`・`resolver`・`persist`・`balloon_*`・`dpi_sync`・`reseed` ほか）、2 本目＝組 C・D・E（`measure`・`config`・`spawn`・`diag`・`chain_finalize`・`zorder`）。
    - よい点: 触るファイルが分かれ、並べて走れる。
    - 弱い点: 必ず直すが 2 本に割れる（1・2 と 3・4）。
- どちらでも、申し送りの件（5〜9・41）は 1 本目に置く案が素直。要件 2 は 1 本目の受け持ちになる。

## 7. 調べが要ること（設計の段へ）

1. 3 番: `image` 0.25 の `PngDecoder::new`・`WebPDecoder::new` は、見出しどおりに確保するのか、既定の `Limits` で断るのか（指摘の重さが変わる）。
2. 6・7 番: `pub` の項目の属性を外しても、`cargo check -p areka-placement --all-targets` で警告が増えないこと（lib の根から届く `pub` には lint が鳴らない、という読みの確かめ）。
3. 1 番: 起動の経路で、最初の `resize_window_to` が必ず走って上端へ直るか（並べ直しの経路は、コードで確かめ済み）。
4. 2 番: 96→144 の手計算の値を、実際のテストで確かめる。
5. 71 番: tick の門の heartbeat の間隔。
6. 72 番: 台本の引数の分け方に、長さの上限があるか（1 万トークンが実際に届くか）。
7. 要件 4.4: 直す所が `clippy-199-lints` の指摘と重なるかを、実装の段で clippy を回して確かめる（`clippy-199-lints` の brief は旧 `crates/areka/src/placement/*` を名指していて、今の crate の中の行は挙げていない）。

## 8. 設計の決め（要件の討議の材料）

1. 1 番を、この spec で直すか、`window-snap` の件 1 へ回すか（並べ直しの経路と warn の数を、その brief へ書き足す）。直すなら、スコープ 1 以降が同じ辺に並ぶときの形。
2. 2 番を、小の案（2px 以内なら一致）にするか、筋の案（動かした印・`audit-placement-follow` と申し合わせる）にするか。
3. 3 番を、範囲の外の `areka-emo-atlas` で 1 行直すか、配置の側で直すか（公開の形が足される）。
4. 5 番の本体の 2 本を、範囲の外の資産の道に上限の口を足して直すか、ほかの形にするか。
5. 7 番: 先送りの印に付いた、効いていない属性 3 つを消すか。
6. 8・19・42 番: テストだけの公開を `test-support` の内側へ移すか、C20 のあいだは足すだけの約束を守って残すか。
7. 9 番: 依存の規約を、文で直すか（案 a）、参照で直すか（案 b）。
8. 17 番: 案 1（`areka-sylphya` に鍵を消す口）か、案 2（ずれを許す裁定）か。
9. 30 番: 先送りの檻の空振りのテストを消すか。
10. 47・51・62・70・71・73 番: C20 のうちに相手と申し合わせて直すか、持ち主の spec へ回すか（要件 8.6）。
11. 2 本目の spec の切り方（6.3）。
12. 写しをまとめる先（新しいテストの支えのファイル名と置き場）。

要件の討議（2026-11-14）で設計へ回した事項（上の 1〜12 のうち、討議の議題になったもの〔1・3・4・5・6・8・9・10・11〕の「直すか・どこで直すか」は requirements.md の「確定した指摘の一覧」と「要件の討議の裁定」が正本。ここに残るのは直し方だけ）:

13. 2 番の直し方は、`audit-placement-follow` の持ち物に触れない形から選ぶ。比べる両辺を同じ丸め方で作る（解き直す側も「和を 1 回丸める」にそろえる）形を第一に確かめ、どうしても follow の持ち物に触れるなら、そのときに議題へ戻す。
14. 9 番は案 a（規約の文を今の参照に合わせる）を既定にし、案 b で輪を断つのは `follow*` の `use` に響かないものだけにする。
15. 1 番のスコープ 1 以降が同じ辺（上・左・右）に並ぶときの形（P2 の連なりの向き）。
16. 5 番で上限を渡す口の形（`areka-emo-present` の `load_shell_target` の兄弟として足すか・本体の資産の道のどこから渡すか）。足すだけにし、今の関数の形は変えない。
17. 13 番の新しい関数の置き場と、本体の `runtime/move_cue_place.rs` の呼び手の直し方。
18. 27 番の小の案（doc の主張を外す）と筋の案（`macro_rules!` で列挙と `ALL` を同時に定義）のどちらにするか。
19. 70・71 番は doc を直す形（戻り値と数え方は変えない・C20 の約束 ⑵ に触れない）。
（2 本目の spec は切らない＝2026-11-14 開発者「議題７、１本でやらない？」。80 件をこの spec の設計で扱う。）
20. 7 節の調べ（`image` の `Limits`・属性を外したときの `cargo check`・起動の経路・96→144 の数値・heartbeat の間隔・台本の引数の長さ・`clippy-199-lints` との重なり）。

## 9. 開発者の裁定が要るもの（要件 5 の 3 種類）

| 番号 | 種類 | 中身 |
|---|---|---|
| 1 | 3（窓の出る位置・ukadoc の `seriko.alignmenttodesktop`）。あわせて持ち主の spec（`window-snap` 件 1）がある | 直すか、回すか |
| 7 | 2（先送りの印: `diag.rs:L167`・`descript_window_keys.rs:L188`・`resolver.rs:L418`） | 効いていない属性だけを消すか |
| 17 | 1（`window-link-directives` の裁定 8）と 3（次の起動のバルーンの位置・ukadoc の `windowposition.x`） | 案 1 か案 2 か |
| 30 | 2（要件 13 の先送りの檻の一部） | 消すか |

ほかの 76 件は 3 種類のどれにも当たらない。要件 5.3 に従い、議題にせずに直す（並走の約束に触れる 14 件は、要件 8.6 の議題として別に扱う）。

## 10. 次の段

- 要件の討議（`/kiro-requirements-discussion audit-placement-core`）で、80 件を直す対象として requirements.md に落とす。そこで 8 節・9 節の決めと 5 節の約束の扱い、6.3 の切り方を確定する。
- その後、`/kiro-design audit-placement-core` へ進む。

---

## 11. 設計の段の調べ（2026-11-14 `/kiro-spec-design -y`）

発見の型は軽い（今ある crate の中の直し・新しい依存なし）。使った決まりは `kiro-spec-design` の `design-discovery-light`・`design-synthesis`・`design-review-gate`。外の調べ（web）は要らなかった（`image` はレジストリのソースで足りた）。

### 11.1 7 節の調べの答え

| 7 節 | 答え | 出所 |
|---|---|---|
| 1 `image` の `Limits` | `PngDecoder::new` は `Self::with_limits(r, Limits::no_limits())`＝見出しの寸法どおりに確保する。`Limits::default()` の `max_alloc` 512 MiB は `new` では使われない。WebP も確保の上限をかけない（`audit-emo-atlas` の設計の決め 2・`image-webp` 0.2.4 の `memory_limit` の既定は `usize::MAX`）。指摘 3 は重さのとおり「必ず直す」 | `C:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\image-0.25.10\src\codecs\png.rs:L42-L55`・`src\io\limits.rs:L50-L66` |
| 2 属性を外したときの警告 | 9 つの属性の項目は、どれも `pub mod`（`lib.rs:L23-L63`）の中の `pub` の項目か、根の `pub use`（`L72`）で、lib の根から届く。`dead_code`・`unused_imports` は鳴らない読み。実装で `cargo check -p areka-placement --all-targets`（機能あり・なし）を 1 回ずつ回して確かめる | `lib.rs` のモジュールの宣言 |
| 3 起動の経路の `resize_window_to` | 指摘 1 で最初の位置そのものを辺で解くので、問わなくてよい | — |
| 4 96→144 の数値 | 設計で U4 のテストに固定する（手計算: 追随 −407・解き直し −406） | 2 節の 2 番 |
| 5 heartbeat の間隔 | 指摘 71 は doc に条件を書く形（議題 7）なので値は要らない | — |
| 6 台本の引数の長さ | 指摘 72 は `HashSet`／`HashMap` で長さに依らない形にするので要らない | — |
| 7 `clippy-199-lints` との重なり | 実装の最後（U28）に clippy を回して確かめ、同 brief へ書く | — |

### 11.2 指摘 2 の「同じ丸め」の形を確かめた結果

- 追随の値: `BalloonFollow` の基準（`OffsetBase`＝起動のときの標準のずらしと採寸の DPI）を、`rescale_follow_offset`（`follow/offset_space.rs:L208-L246`）で今の DPI へ 1 回で換算した値。基準は追随で変わらない（`apply_rescaled`）。変わるのはドラッグとキーワードの再導出（`reestablish`・`follow/drag_follow.rs:L581`・`follow/keyword_base.rs:L152`）だけ。
- 解き直しの値: `resolve_standard_scopes`（`reseed.rs:L144-L224`）が、今の窓の寸法と今の DPI で部品ごとに換算して足した値。
- 解き直す側を「基準の DPI で解いて 1 回で換算」にそろえるには、基準の DPI のときの窓寸が要るが、世界に残っていない（窓寸は今の DPI の `WindowPos` だけ・面を変えると寸法も変わる）。「動かした印」を配置の側の部品で持つ形も、キーワードの再導出（追随の持ち物）が基準を焼き直すので、印を付け直すには追随の持ち物に触れる。
- よって、follow に触れない形は「差を許す」だけになる。差の見積もり（`n` 個の部品・比 `r`・基準の拡大率 `k0`）:
  - `k0 = 1`（作者の DPI＝主モニタの DPI）のとき、Side の腕は |差| ≤ 2、キーワードの腕は |差| ≤ 3（`r < 3`）。
  - `k0 ≠ 1` で拡大率が上がる移り（例: 作者 96・主モニタ 144 で起動して 288 へ）は、基準の丸めの差が `r` 倍されるので 3 を越えうる。
- 設計の決め: 軸ごとに 3px を許し、限界を `shortcut:` の注釈に書く（design の U4）。議題へは戻さない（follow の持ち物に触れないため・8 節 13 の条件）。

### 11.3 公開を狭める形（議題 5）の確かめ

- `pub(crate)` の項目を `pub use` で開き直すのは Rust では組めない（E0364）。テストだけに開く項目のうち crate の本番も使うもの（`evaluate`・`DpiSyncDecision`・`DpiSyncOutcome`・`DPI_SYNC_HOLD_MAX_FRAMES`・`HoldReason`・`BALLOON_LIMIT_CLAMP_TAG`・`PREPARE_GHOST_WINDOWS_CONTEXT`）は、同じファイルの私有の子モジュールに `pub` で置き、機能 `test-support` の有無で `pub use`／`pub(crate) use` を切り替える形にした。道と名前は変わらないので、追随の持ち物（`pt/transition_judge_verdict.rs:L57`・`follow.rs:L53`）は直さなくてよい。
- `BalloonLockLedger::reason` は crate の中の本番（`balloon_lock.rs:L125`）も使うメソッドなので、子モジュールの関数 `hold_reason` にする。外の使い手は本体のテスト 2 本だけ。
- ワークスペースに `unreachable_pub` などの lint の設定は無い（根と配置の `Cargo.toml` に `[lints]` が無い）。

### 11.4 指摘 5・13・17・36・73 の口と呼び手

- 5: 資産の道で焼くのは `areka-emo-present` の `shell_target.rs:L621-L622`（`bake`／`bake_quiet`）と `balloon.rs:L806-L807`。測りの 2 本が動く絵を登録するのはシェルの側だけ（`measure_bake_log_tests.rs:L169-L180`）なので、上限の口はシェルの側だけに足す。本体は `areka-emo-atlas` に直に依存している（`crates/areka/Cargo.toml:L58`）。`build_shell_assets` の呼び手は本番 2 か所（`assets.rs:L252`・`switch_assets.rs:L219`）とテスト。実装で、測りの数のテスト（`measure_bake_count_tests.rs`）の動く絵の検体ではバルーンの面 2 枚（`balloons0.png`・`balloons1.png`）も動く絵だと分かったので、バルーンの側の上限の口（`balloon.rs` の `build_balloon_target_from_faces_with_limits` と本体の `build_balloon_assets_with_limits`）も足した。
- 13: 3 か所とも、主モニタを引いた後の `RECT`→`RectPx` の写しが同じ。並べ直しは DPI も要るので、`Option<RectPx>` を返す形でなく、モニタ 1 つを写す `work_area_px` にした。
- 17: sylphya の書き込みは、投函 `SylphyaMsg::PersistPut` → `SylphyaCore::apply` の効果 `SetDottedGlobal`・`PersistSave` → アクターの実行（`actor.rs:L715`・`L731`）→ `save_scope`（`persist/mod.rs:L301-L417`・錠・既存の読み・`apply_entry`・`to_toml_string_over(base, doc)`・commit）。`to_toml_string_over` は `doc` を `base` に重ねるだけなので、消すには `base` の表からも消す要がある。`LastRunning` の「空文字は無し」の約束は、バルーンのずらしでは壊れた値と見分けがつかないので取らない。
- 36: `apply_descript`（`sticky_group_ledger.rs:L262`）は既に `action`（`descript_seed`／`descript_replace`）を受けるが、`L275` が `"seed"` を固定で渡す。呼び手は crate の中の 2 か所だけ。
- 73: `ChainPlan.absent` の読み手は本体の `runtime/frame/zorder_drain.rs:L124`・`L258-L277` とテスト。`GroupElement` は配置の型なので wintf に置けない。値は `(u32, char, u32)` にする。

## 12. 設計の総合（`design-synthesis`）

- **まとめられる形**: 記録の無い縮退（16・26・35・40）は、どれも「今の縮退の手前に `warn!` を 1 行」で同じ形。写し（11・14・32・37・64・65・76・77・80）は、どれも今ある支え（`test_support.rs`・`placement_shared_test_support.rs`・`spawn_test_support.rs`）か新しい 2 つの支えへ寄せる。公開を狭める 6 件（8・19・42・45・51・62）は、表の 3 種類の置き方で 1 つに決めた。
- **作るか使うか**: 新しい依存は使わない（`visibility` のような crate は足さない）。`image` の上限は確保の前の確かめを自前の `pixel_len` で持つ（`image` の `Limits` は `new` で効かない）。
- **削ったもの**: 指摘 27 の `macro_rules!`（テストの網羅の `match` で足りる）・指摘 13 の `primary_work_area`（`work_area_px` で足りる）・指摘 2 の「動かした印」（追随の持ち物に触れる）。指摘 5 のバルーンの側の上限の口は、設計では削ったが、測りの数のテストの動く絵の検体でバルーンの面 2 枚（`balloons0.png`・`balloons1.png`）も動く絵だったので、実装で足した（11.4）。

## 13. 設計の検証の材料（`design.md` の決めの要点）

- 単位は 28（U1〜U28・`design.md` の一覧）。指摘 80（U26）はほかの直しの後の最後の実装。
- 外から見た振る舞いが変わるのは 1・2・17 の 3 件。
- 範囲の外に触るのは、裁定で認めた所（3・5・13・17・47・51・62・67・73 と、8・19・42 の本体のテスト）だけ。
## 14. 設計の討議（2026-11-14）

- 議題 1（指摘 2）: 丸めの差を 3px まで許す形は、作者の DPI と主モニタの DPI が違う構成と拡大率が 3 倍以上の移りで取りこぼしが残り、3px 以内の利用者の移動を取り違える（設計の検証の大きな問題 2）。開発者の承認で、値の比べをやめて印 `BalloonUserMoved` を状態で持つ形にした（付ける＝ドラッグの据え直しと記憶からの戻し・外す＝標準への据え直し）。追随の持ち物の `follow/drag_follow.rs` に 1 行を足すのは、相手の着地の後に申し合わせて行う。
- 議題 2（指摘 4・61）: 手本の中の `assert_eq!` だけでは全体テストで走らない（設計の検証の大きな問題 3）。選ぶ所を純粋な関数にし、手本の中の `#[test]` と `[[example]] test = true` で全体テストに入れる形にした。

## 15. 追随の側から受けた一覧（2026-11-14 `audit-placement-follow` の実装の段）

`audit-placement-follow` が直さずに残した、この spec の持ち物の件（相手の要件 6.4・正本は相手の `research.md`「核へ伝える一覧（実装の段）」）。どれも確定した 80 件の直しと同じ所か、その隣にある。実装の頭（タスク 1）で、相手の着地の後の `main` で行を grep し直してから、下の表のタスクで直す。かけ直し（タスク 8.1）で新しい指摘として 2 度数えない。

| 件 | 場所 | 受け持つタスク |
|---|---|---|
| `fake_handle`・`position_of`・`px` の写し | `reseed_test_support.rs`・`resolver_test_support.rs` ほか | 2.2（指摘 14 の隣） |
| `BottomSnapPolicy`（消えた名前）を指す説明 | `resolver.rs:L438` | 2.1（resolver の doc） |
| 同上 | `persist.rs:L227`・`L238`・`persist_restore_tests.rs:L295` | 5.1（persist の説明） |
| `dpi(v)` の写し | `balloon_offset_persist_roundtrip_tests.rs` | 5.1 |
| `debug_assert!` に依る `should_panic`（`cargo test --release` で赤） | `dpi_sync_tests.rs` の `a_write_reaching_a_waiting_window_trips_the_single_writer` | 4.3（相手と同じ `#[cfg_attr(not(debug_assertions), ignore = "debug_assert! に依る")]` を付ける） |
| `emo2_boot`・`wire_emo2_boot` の古い名前（13 ファイル） | `lib.rs` ほか | 4.2（指摘 41・数え直しに入る） |
| `diag_tests.rs` の古い行の参照・`judge-perf.py::parse_fields` をファイル名で指す 1 行・`is_system_reanchor` の「D9 の 6 経路」（今は 7 経路） | `diag_tests.rs`・`diag.rs` | 6.5（指摘 27・68 の隣。判定器は名を書かずに「tools/perf/ の Python の道具の `parse_fields`」と指す＝相手と `audit-tools-perf-judge` の申し合わせに揃える） |
| `code_only` の写し | `spawn_zorder_chain_wiring_tests.rs`・`zorder_group_ledger_state_tests.rs`・`test_support.rs` の `zorder_pair_deferred` | 6.3・6.6（指摘 64・76。`measure_template::code_only` は別物で残す） |
| `SharedFakeIo` の写し | `persist_finalize_save_tests.rs`・`persist_io_wiring_tests.rs`・`reset_pos_tests.rs`・`balloon_lock_drag_tests.rs` | 7.2（指摘 80 と同じ置き場 `test_support.rs` へ。sylphya の `SharedFakeIo` には足さない＝議題 6。追随の `SylphyaRig` は follow の子なので使わない） |
| `assert_no_event`・`contained`・`point(x, y)` の写し | `balloon_limit_gate_tests.rs`・`balloon_limit_tests.rs`・`balloon_limit_flip_tests.rs`・`placement_windowposition_vocab_tests.rs` | 7.2（指摘 80） |

- 相手が `follow` の `VisibilityVerdict`・`guard_visibility` を `pub(crate)` にした。この spec の範囲で名で呼ぶ所は 0（grep で確かめた。範囲の外の `frame_balloon_offset_follow_tests.rs:L199` の doc が名を書くだけ）。相手への知らせは要らない。
- `completed/` の古い記録に残る消えたテストの名は、当時の記録なので直さない（相手と同じ判断）。

## 16. 先の段の 4 本の着地の後の確かめ（2026-11-14・`main` 71594918 を取り込んだ後）

`#458`・`#459`・`#460`（`audit-placement-follow`）・`#461`（`audit-wintf-window`）の差分（`7e0373f4..71594918`）を、この spec が名指す所と突き合わせた。要件・設計・タスクは変えない。

- 80 件のうち先の段が直したものは 0 件。範囲の中で動いたのは、`#461` の 11 番（`WindowHandle.instance` を消す）で 15 ファイルの構築の式とその import だけ（各 1〜7 行・`dpi_sync_tests.rs`・`face_windowposition_tests.rs`・`spawn_zorder_*_wiring_tests.rs`・`zorder_chain_*_order_tests.rs`・`pt/reset_balloon_pos_tests.rs` ほか）。指摘の中身には触れていない。設計の行番号のうち、`spawn_zorder_pair_wiring_tests.rs`・`spawn_zorder_chain_wiring_tests.rs`（窓を作る道具・指摘 64）と `zorder_chain_tail_order_tests.rs`（指摘 31）は数行ずれた。タスク 1 の grep で読み替える。
- 範囲の外の触る所（本体の `move_cue_place.rs`・`assets.rs`・`switch_assets.rs`・`frame/drain_resnap.rs`・`frame/zorder_drain.rs`・`main.rs`、`shell_target.rs`、`areka-emo-atlas` の `decode`、`areka-sylphya`）は、どの着地も触っていない。
- 設計の前提はそのまま成り立つ。
  - 指摘 2: 追随の側の `reestablish_balloon_follow(world: &mut World, balloon: Entity, balloon_pos: Point)` が `follow/drag_follow.rs:L557` にあり、`on_balloon_drag`（`L543`）と離したとき（`L793`）から呼ばれる。印を付ける 1 行はこの関数の中に置く。`face_windowposition.rs:L64` の値の比べは残っている。
  - 指摘 47: 3 つの `Cargo.toml` の `async-io` の行は同じ版のまま（`#458` は触っていない）。
  - 指摘 67: wintf の `transition_diag::MISSING`（`pub const`・`"-"`）がある。配置の `diag.rs:L145` の私有の `UNKNOWN` は残っている。
  - 指摘 73: `ChainPlan.absent` は `Vec<(u32, String)>`、`log_chain_absent(group_id, element: &str)` のまま。`#461` は同じファイルの doc と `dirty` の持ち越しの説明だけを直した。
  - 指摘 7（`[[example]]`）: `crates/areka-placement/Cargo.toml` に `[[example]]` の節は無く、追随の側も触っていない。
  - 15 節の件: `BottomSnapPolicy` を指す 4 行（`persist.rs:L227`・`L238`・`persist_restore_tests.rs:L295`・`resolver.rs:L438`）は残っている。
- 重なる相手のうち、まだ着地していないのは `audit-wintf-input`（指摘 80・タスク 7.2）だけ。指摘 80 を最後にする順は変えない。

## 17. 実装の頭（タスク 1・2026-11-14）

取り込んだ `main`（71594918）の上で、設計が名指す行と 15 節の表の行を grep し直した。ずれは `#461` が窓の構築の式から `instance` の行を消した所の後ろだけ。設計の文は直さず、ここの読み替えで直す。

### 17.1 読み替えた行

| 指摘・件 | 設計の行 | 今の行 |
|---|---|---|
| 64 | `spawn_zorder_pair_wiring_tests.rs:L50-L98`（窓を作る道具） | `L50-L93`（`create_test_window` の doc〜`spawn_window`） |
| 25 | 同 `L133-L153`・`wire_inserts_plan_a_strategy_resource` | `L128-L148` |
| 64 | `spawn_zorder_chain_wiring_tests.rs:L55-L97`（窓を作る道具） | `L55-L92` |
| 64 | 同 `L275-L282`（`code_only`） | `L270-L277` |
| 31 | `zorder_chain_tail_order_tests.rs:L5-L7`・`L48-L74` | 同じ（ずれは `L83` より後ろだけ） |
| 80 | `balloon_limit_flip_tests.rs:L212-L217`（`fake_handle`） | `L212-L216` |
| 80 | `balloon_lock_drag_tests.rs:L51-L66`（`SharedFakeIo`・`fake_handle`） | `L51-L65` |
| 80 | 同 `L264-L266`（`scopes`） | `L263-L265` |
| 80 | `balloon_limit_flip_tests.rs:L69-L82`・本体の `pt/reset_balloon_pos_tests.rs:L42-L50` | 同じ |
| U11 | 追随の `pt/transition_judge_verdict.rs:L57`（`DPI_SYNC_HOLD_MAX_FRAMES` を読む所） | `L60`（道は `areka_placement::dpi_sync::DPI_SYNC_HOLD_MAX_FRAMES` のまま） |
| U11 | `follow.rs:L53`（`BALLOON_LIMIT_CLAMP_TAG`） | `L52` |
| 9（タスク 4.1） | `lib.rs` の規約の文に挙げる名（`follow` との両向きは `balloon_limit`・`dpi_sync`・`sticky_group_ledger`・`spawn`、根を引く子は `reset_pos`・`follow/offset_space`） | 今の参照を grep し直して足した: 両向きに `persist`（`persist.rs:L40`↔`follow.rs:L56`）・`transition_diag`（`transition_diag.rs:L61`↔`follow.rs:L148`）、根を引く子に `reseed`→`scale_then_merge`・`ScopeWindowPositions`（`reseed.rs:L39`・タスク 2.3 で入った） |
| U28 | 追随の `follow/work_area_clamp.rs:L18` の古い文（「examples は `#[path]` で…」） | 追随の側が直した（今の `L15-L20` に `examples`・`#[path]` の語は無い）。完了の申し送りに書く件は 1 つ減る |
| U13（タスク 5.1） | `persist_entries` の呼び手は 6 か所 | 5 か所（`drag_follow.rs` に 2・`sticky_follow.rs`・`persist.rs` の `persist_unremembered_char_positions`・`reset_balloon_pos.rs`）。文書には今の 5 か所を挙げる |

15 節の表の行は、どれも今もある（タスク 2.1・2.2・4.2・4.3・5.1・6.3・6.5・6.6・7.2 で直す）。

- `fake_handle`・`position_of`・`px`: `reseed_test_support.rs:L97`・`L131`、`resolver_test_support.rs:L11`（`px`）、`balloon_limit_tests.rs:L26`・`balloon_limit_gate_tests.rs:L428`（`px`）。
- `BottomSnapPolicy`: `resolver.rs:L438`・`persist.rs:L227`・`L238`・`persist_restore_tests.rs:L295`。
- `dpi(v)`: `balloon_offset_persist_roundtrip_tests.rs:L147`。
- `should_panic`: `dpi_sync_tests.rs:L328-L329`（`cfg_attr` はまだ無い）。
- `emo2_boot`・`wire_emo2_boot`: 配置の 13 ファイル（本体の `runtime/mod.rs`・`runtime/spine.rs` にもある）。
- `diag_tests.rs:L107`（`judge-perf.py::parse_fields`）・`diag.rs:L295`・`L308`（「D9 の 6 経路」）。
- `code_only`: `spawn_zorder_chain_wiring_tests.rs:L271`・`zorder_group_ledger_state_tests.rs:L116`・`test_support.rs:L284`（`zorder_pair_deferred` の中・6.3・6.6 で直す）。`L149` は `measure_template::code_only` で残す。
- `SharedFakeIo`: `persist_finalize_save_tests.rs:L56`・`persist_io_wiring_tests.rs:L136`・`reset_pos_tests.rs:L63`・`balloon_lock_drag_tests.rs:L51`。
- `assert_no_event`・`contained`・`point(x, y)`: `balloon_limit_gate_tests.rs:L52`・`L96`・`L438`、`balloon_limit_tests.rs:L53`・`L69`、`balloon_limit_flip_tests.rs:L42`、`placement_windowposition_vocab_tests.rs:L62`。

### 17.2 テスト

取り込んだ `main` の上で `cargo test -p areka-placement --features test-support`・`cargo test -p areka`・`cargo fmt --all -- --check` を回した。直した所は 0。

- 配置の crate: 緑（lib 1011 件が通り、1 件は ignore）。
- 本体: 3183 件のうち 1 件が赤（`runtime::spine::third_scope_talk_tests::spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits`・`spine_third_scope_talk_tests.rs:L154` の「会話が止まった」の待ちの期限切れ）。この 1 件だけを回し直すと緑（3.24 秒）。待ちの期限に依るテストで、全部を並べて回したときの込み具合で揺れたと見る。この spec の範囲（配置）の外で、先の段の 4 本が触ったのも `spine.rs` の import の 1 行（`HINSTANCE` を外す）だけなので、直さない。
- `cargo fmt --all -- --check`: 緑。

### 17.3 知らせる相手と名前（要件 8.7）

重なる相手のうち、まだ着地していないのは `audit-wintf-input` だけ（今は `brief.md` だけで、下の名前はどれも書いていない）。この相手はまだセッションが動いていないので、知らせは送っていない。この記録を知らせの代わりとし、タスク 4.4・4.5・7.1 で下の名前を変える前に、もう一度名前を確かめる（その時までに相手のセッションが動き出していれば、そこへ伝える）。着地した 4 本（`#458`〜`#461`）へは送らない。

- 4.4（U11・公開を狭める）
  - `test_support::EMILY_SHAPED_GHOST_DESCRIPT`・`EMILY_SHAPED_SHELL_DESCRIPT` を `pub(crate)` にする。`impl Default for AuthorDpi` を消す。
  - `source::load_descript_source`・`prepare_ghost_windows_with_work_area` を test-support のときだけにする。
  - `dpi_sync::{evaluate, DpiSyncDecision, DpiSyncOutcome, DPI_SYNC_HOLD_MAX_FRAMES}`・`balloon_lock::HoldReason`・`balloon_limit::BALLOON_LIMIT_CLAMP_TAG`・根の `PREPARE_GHOST_WINDOWS_CONTEXT` を test-support のときだけ外へ開く（道と名前は同じ）。
  - `BalloonLockLedger::reason` を関数 `balloon_lock::hold_reason(ledger, scope)` に替える。
- 4.5（U12）
  - `areka_placement::work_area_px(monitor)` を足す（crate の中に `square_dpi` も足す）。
  - `persist::load_restored_state` の引数 `default_encoding` を消す。
  - `chain_finalize::moved_default_pos` を消す。
- 7.1（U25）
  - wintf の `ChainPlan.absent` を `Vec<(u32, char, u32)>` に、`log_chain_absent(group_id, kind, scope)` にする（行の綴りは同じ）。本体の `ZOrderAbsentReports.reported`・`report_absent_elements` も同じ値の型にする。
  - 根の `Cargo.toml` の `[workspace.dependencies]` に `async-io` を足し、`areka-placement`・`areka`・`wintf` の `Cargo.toml` を `async-io = { workspace = true }` にする（版は同じ・`Cargo.lock` は変わらない）。
  - 配置の `diag.rs` の私有の `UNKNOWN` を wintf の `transition_diag::MISSING` に替える（私有なので外の名前は変わらない）。

### 17.4 タスク 2.2 で寄せなかった写し

- 2.2: `reseed_test_support.rs` の `fake_handle`・`position_of` は寄せない。対になる写しが追随の持ち物のテストだけの `follow_test_support.rs`（`follow.rs` が `#[cfg(test)]` の `test_support` として載せる）にあり、触ると追随の持ち場に入る（要件 8.1）。完了の申し送り（タスク 8.2）で `audit-placement-follow` へ渡す。`balloon_limit_flip_tests.rs`・`balloon_lock_drag_tests.rs` の `fake_handle` の写しはタスク 7.2 で直す。

## 18. かけ直しの記録（タスク 8.1・要件 6.1〜6.4）

### 18.1 1 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `1df5e100a`）。
- 範囲は 1.1 節と同じ数え方で測り直した。受け持つ範囲に、この spec が足した 3 ファイル（`test_support_com_tests.rs`・`zorder_test_support.rs`・本体の `placement_tests/zorder_branch_coverage_test_support.rs`）が入り、消した 2 ファイル（`sample_test_support_tests.rs`・`spawn_zorder_pair_export_tests.rs`）が抜けた。範囲の外にこの spec が足した `areka-sylphya` の `persist/persist_remove_tests.rs` も読んだ。範囲の外で直した所（本体・`areka-emo-*`・sylphya の本番・wintf）は、設計 U27 のとおり除いた。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 27 | 10,542 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 23,152 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,163 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 225 |
| 計 | 116 | 37,133 |

- ponytail は 5.3.3（`ponytail-audit` の `SKILL.md` は 1.2 節と同じ版）。
- 作法は 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 1.4 節と同じ 6 つで、足したファイルは名前の近い組に入れた（`test_support_com_tests.rs`→C・`zorder_*`→E・`persist_remove_tests.rs`→B）。6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取り、計 37,133 行で上の表と合う。
- 実装の段で拾っておいた候補（`tasks.md` の Implementation Notes の 2.1・4.4・5.2・6.7・6.8）と、`audit-wintf-input` からの申し送り（`brief.md` の 2026-11-15 の節）も、同じ一覧に入れた。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,707 | 0 | 2 | 23 | 25 |
| B | 20 | 6,871 | 0 | 2 | 26 | 28 |
| C | 20 | 5,917 | 1 | 3 | 14 | 18 |
| D | 17 | 5,737 | 0 | 0 | 19 | 19 |
| E | 15 | 6,020 | 0 | 0 | 13 | 13 |
| F | 20 | 5,881 | 0 | 2 | 21 | 23 |
| `audit-wintf-input` の申し送り | — | — | 0 | 1 | 0 | 1 |
| 計 | 116 | 37,133 | 1 | 10 | 116 | **127** |

- 127 件のうち 166〜207 の 42 件（できれば）は、1 回目のレビューの差し戻しで指摘に数え直したもの（名前の無い古い spec の task 番号。下の一覧の末尾）。

#### 指摘の一覧（81〜207）

扱いの印: 直した＝この回で直した。裁定＝直さずに裁定を仰ぐ（18.1 の「裁定が要るもの」）。申し送り＝直しが追随の持ち物かほかの spec の持ち物に要る（タスク 8.2 が書く）。残す＝裁定で直さないと決めた。回す＝裁定でこの spec の外へ回した。番号は 80 の続き。165 は A の組で後から番号を付けた。166〜207 は 1 回目のレビューの差し戻しで足した（ファイルごとに 1 件）。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 81 | 必ず直す | C | シェルが `balloon.offsetx`／`offsety` の片方だけを書くと両方を捨て、記録も残さない | `config.rs` の `resolve_scope` | 直した（裁定: 推奨案で承認。書かれた軸を使い、書かれない軸は 0。`balloon_offset_axes_are_independent`。直す前は横だけの入力で `None` が返って赤。古い決まりを書いた `lib.rs`・`balloon_offset_supply_tests.rs` の説明と、台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 4 項目・`briefing.md`・`briefing-assets.md` の行も直した。台帳の 4 項目は、綴りの後ろに `l`／`r` を付けた形を読まない点が残るので未対応のまま） |
| 82 | 直したほうがよい | A | 辺ごとに連ねる resolver の解き方（指摘 1 の直し）に、壊れたら落ちるテストが無い（辺の混ざる入力が無い） | `resolver.rs` の `prev_left_y`／`prev_right_y`・`side_y` | 直した（`t_r5_mixed_edges_chain_per_edge`。`prev` を動かす・2 つを 1 つにする、の 2 通りの壊し方で赤。クランプの前の y から連ねる壊し方は、上へ連ねる限り結果が同じで、どの入力でも割れない） |
| 83 | 直したほうがよい | A | `assert_no_event` が共有の支えと追随の `follow_test_support.rs` に写してある | `placement_shared_test_support.rs`・`follow_test_support.rs` | 申し送り（`audit-placement-follow`。追随の側の写しを消して共有の側を使う） |
| 84 | 直したほうがよい | B | 「記憶からの戻しが画面の内への補正を焼き付けない」テストが、補正を通していない | `balloon_offset_persist_roundtrip_tests.rs` | 直した（戻しの後に `apply_balloon_limit` を通す。補正が `balloon_offset` へ書く壊し方で赤） |
| 85 | 直したほうがよい | B | sylphya の鍵を消す道 `toml_path` が表と欄の名を写しで持ち、10 の腕のうち 2 つしか確かめていない | sylphya `persist/mod.rs` の `toml_path` | 直した（全部の `PersistKey` を回すテスト。名を 1 つ綴り違えると赤） |
| 86 | 直したほうがよい | C | `PlacementConfig.shell_dpi_raw` は読み手の無い公開の欄 | `config.rs` | 直した（裁定: 推奨案で承認。欄と転記の 1 行と T-C5 の 2 つの確かめを消した。ツリー全体の grep で使い手は 0） |
| 87 | 直したほうがよい | C | 手本が測りの「最初の面」の決まりを写し、手本のテストは写しを確かめている | `examples/window-placement.rs`・`measure.rs` | 直した（`DefaultSurfaces::initial_surface` を足して両方が呼ぶ） |
| 88 | 直したほうがよい | C | `test_crash_report.rs` は wintf の同名のファイルの写し（差は説明の 2 行） | `test_crash_report.rs` | 残す（裁定: 推奨案で承認。理由は下の裁定の表） |
| 89 | 直したほうがよい | F | バルーン位置初期化で外したスコープの記憶のずらしが残ることを、どのテストも確かめていない | 本体の `placement_tests/reset_balloon_pos_tests.rs` | 直した（外すスコープに保存を置いてから確かめる。全部のスコープを消す壊し方で赤） |
| 90 | 直したほうがよい | F | `missing_material_only_warns` の「書かない」の確かめが、決して落ちない | 同上 | 直した（保存を置いてから「消さない」を確かめる） |
| 91 | できれば | A | `lib.rs` の頭の依存の向きの文が、この spec の直しで古くなった（`diag` が wintf の `MISSING` を、`reset_pos` が `square_dpi`・`work_area_px` を引く） | `lib.rs` の頭 | 直した。追随の `follow/move_shift.rs` が根の `scale_signed` でなく `windowposition::scale_signed` を直に引く所は申し送り（`audit-placement-follow`） |
| 92 | できれば | A | 古い名前と主張（`primary_work_area`・`mod.rs`・失敗が必ず終了になる・`load_descript_source` を道として名指す・`test_support::capture` ほか 11 か所） | `lib.rs`・`error.rs`・`windowposition.rs`・`placement_*_tests.rs`・`resolver_balloon_keyword_tests.rs` | 直した |
| 93 | できれば | A | `# 失敗` の段落が、読み込みと重ね合わせを分けた後で重ね合わせの側に残る | `lib.rs` の `merge_scope_windowpositions` | 直した（`load_scope_windowpositions` へ移す） |
| 94 | できれば | A | 指摘 42 の後半が残っていた: `prepare_stages` は 1 行の取り次ぎ | `lib.rs` | 直した（消して呼び手 3 か所を `prepare_stages_for_shell(…, None)` に） |
| 95 | できれば | A | `DEFAULT_AUTHOR_DPI` が同じ crate に 2 つ | `lib.rs`・`source.rs` | 直した（`source` の 1 つにする） |
| 96 | できれば | A | 何も新しく確かめないテスト 4 本 | `windowposition_vocab_tests.rs`・`resolver_balloon_keyword_tests.rs` | 直した（消した。確かめは残るテストにある） |
| 97 | できれば | A | `PlacementConfig` のテストの組み立てが 4 か所に写してある | `resolver_resolve_test_support.rs` ほか 3 ファイル | 直した（`shared_test_support::cfg_of` 1 つ） |
| 98 | できれば | A | emo2 の検体の寸法の定数が 4 つのテストのファイルに写してある | `placement_prepare_tests.rs` ほか 3 ファイル | 直した（`sample_test_support.rs` の 1 組） |
| 99 | できれば | B | `char_pos_entries` の doc が別の関数の上にある | `persist.rs` | 直した |
| 100 | できれば | B | 「2 つを満たす」と書いて 3 つを並べる | `persist.rs` | 直した |
| 101 | できれば | B | 消えた基準の逆変換を書いた説明が 2 か所に残る | `persist.rs` | 直した |
| 102 | できれば | B | `merge_scope` がずらしを手で読み、同じ読みの `saved_axis` を使わない | `persist.rs` | 直した（鍵の作り方を受ける `saved_axis` を両方で使う） |
| 103 | できれば | B | `GhostWindows` が無いとき、隣りの印を黙って足さない | `persist.rs` の `push_adjacency_marks` | 直した（`debug!` を 1 行・テスト付き） |
| 104 | できれば | B | テストの補助の `Option` の引数が、どの呼び手も `Some` | `balloon_offset_persist_roundtrip_tests.rs` | 直した |
| 105 | できれば | B | 面の選びごとに標準の材料を 2 回まるごと写し、1 回しか呼ばない閉包に要らない引数がある | `face_windowposition.rs` | 直した（借りて比べ、写しは 1 回） |
| 106 | できれば | B | 面の windowposition の 2 つの飛ばしの道にテストが無い | `face_windowposition.rs` | 直した（`missing_material_only_warns`） |
| 107 | できれば | B | 本番の読みの失敗の決まりが、テストだけの包みの doc にしか無い | `source.rs` | 直した（`load_descript_source_for_shell` へ移す） |
| 108 | できれば | B | `source_tests.rs` の古い説明（記録の受け方・行の番号・経緯） | `source_tests.rs` | 直した |
| 109 | できれば | B | 値だけを確かめるテスト 5 本が、値も確かめる記録のテストと重なる | `source_tests.rs` | 直した（消して、足りない入力を記録のテストへ移す） |
| 110 | できれば | B | `TempPath` が消すのに手で消す行・一時フォルダの包みと最小のゴーストの書き手の写し | `source_tests.rs`・`persist_io_wiring_tests.rs`・`source_shell_tests.rs` | 直した |
| 111 | できれば | B | 値だけが違う近い写しのテスト 2 組と、重なる確かめ | `persist_entries_tests.rs` | 直した |
| 112 | できれば | B | 落ちようの無いテスト（純粋な関数を 2 回呼んで比べる） | `persist_restore_group_tests.rs` | 直した（消した） |
| 113 | できれば | B | 2 つのファイルで同じ組み立ての写し | `persist_restore_group_tests.rs`・`persist_finalize_save_tests.rs` | 直した（`old_memory_rig` 1 つ） |
| 114 | できれば | B | 記憶の試験台・窓の手の付け方・スコープ 0 の標準の探し方の写し | `reset_pos_tests.rs`・`persist_finalize_save_tests.rs`・`face_windowposition_tests.rs` | 直した |
| 115 | できれば | B | 記憶からの戻しのテストの重なり 3 本 | `persist_restore_tests.rs` | 直した（消して、足りない確かめを残る方へ移す） |
| 116 | できれば | B | 手で直した記憶の `."00"` のようなスコープの綴りは、読むときはスコープ 0 なのに、鍵を消す道が消さない | sylphya `persist/mod.rs` の `remove_scope_keys` | 直した（消す道と保存の道の両方。保存は、同じスコープとして読まれるほかの綴りの同じ鍵を消してから書く。`saving_also_drops_hand_written_spellings_of_the_same_key`。直す前は `"00"` の古い値が `"0"` と並んで読み戻されて赤） |
| 117 | できれば | B | 済んだ仕事を先のこととして書く文と、消えた `prepare_stages` を名指す説明 | `balloon_offset_supply_tests.rs` | 直した（あわせて組み立てを `cfg_of` に） |
| 118 | できれば | C | `impl Default for MeasureScaling` に本当の使い手が無い（指摘 45 と同じ形） | `measure.rs` | 直した（裁定: 推奨案で承認。impl と `measure_tests.rs` の確かめ 1 つを消した。ツリー全体の grep で使い手は 0） |
| 119 | できれば | C | 本文の走査の見落としの番が、`'\"'` と `br"…"`／`cr"…"` を通す | `test_support.rs` | 直した（較正の行を足した） |
| 120 | できれば | C | 古い説明（`main.rs` が読む・`#[cfg(test)]` 限定・読取点・依存の一覧・1,000 行に近い・数え違い・`emo2-boot` ほか） | `config.rs`・`measure*.rs`・`test_support.rs` | 直した |
| 121 | できれば | C | 「未結線」「後続タスク」と、今は済んだことを先のこととして書く | `config_limit_tests.rs`・`config.rs` | 直した |
| 122 | できれば | C | 手本がスコープを `as u32` で変える（黙って切り詰める） | `examples/window-placement.rs` | 直した（`u32::try_from`・合わないスコープは `error!` で飛ばす） |
| 123 | できれば | C | 読めないゴーストの `balloon.dontmove` を、スコープの数だけ同じ警告で記録する | `descript_window_keys.rs` | 直した（1 度だけ読んで 1 行。警告は、シェルのスコープごとの値が決めるとき（ゴーストの値を使わないとき）にも出るようになり、`scope` の欄は番号でなく文字の `all` になる） |
| 124 | できれば | C | 組み込みの値を言い直すだけの定数 `KERO_SURFACE_ID` | `measure_template_tests.rs` | 直した |
| 125 | できれば | C | 手本の煙の門が、読めない値で黙って切れる | `examples/window-placement.rs` | 直した（`warn!`） |
| 126 | できれば | D | 綴りの違う古い名前 `emo2-boot` が残る（指摘 41 の grep が拾わなかった） | `spawn.rs` | 直した（`measure.rs` の分は 120） |
| 127 | できれば | D | 配線の経緯の注記・書き手の置き場・呼び手の一覧が古い | `diag.rs` | 直した |
| 128 | できれば | D | `DESPAWNED_SKIP_TAG` の使う所の数と出所の一覧が違う | `diag.rs` | 直した（数と一覧を書かない。綴りは同じ） |
| 129 | できれば | D | クリック透過の doc が「no-op で安全」のまま（指摘 26 の直しで警告になった） | `spawn.rs` | 直した |
| 130 | できれば | D | 古い理由の文（`crate::` が使える・`crate::` を持たない） | 本体の `placement_tests/spawn_assembly_tests.rs` | 直した（依存の向きを理由に書く） |
| 131 | できれば | D | 共有の検体は記憶から戻した置き方と同じ印が付くのに、doc は resolver の出力と書く | `spawn_test_support.rs` | 直した（doc に 1 行） |
| 132 | できれば | D | `remembered_decision` の同じ入力を 2 本で確かめる | `chain_finalize_resolve_tests.rs` | 直した（消した） |
| 133 | できれば | D | クリック透過の古い no-op のテストが、新しい警告のテストに含まれる | `spawn_clickthrough_tests.rs` | 直した（消した） |
| 134 | できれば | D | 記録の行のテストの重なり（全文の一致の後の欄の繰り返し 3 つ・テスト 3 本） | `diag_tests.rs` | 直した（消した） |
| 135 | できれば | D | `ChainDeferReason::scope()` の腕の多くが確かめられていない | `chain_finalize_tests.rs` | 直した（表にスコープの有無を足す。腕を 1 つ動かすと赤） |
| 136 | できれば | D | ファイルの頭が 2 つの系と書き、テストは 4 つを確かめる | `spawn_zorder_pair_wiring_tests.rs` | 直した |
| 137 | できれば | E | `ChainPlan.absent` を組にした後の古い説明（`element_text` の使い先・属性の経緯・t_zcc18） | `zorder_chain_compose.rs`・`zorder_chain_compose_tests.rs` | 直した |
| 138 | できれば | E | 消したテストの摂動 ⑵ をまだ説明する | `zorder_chain_compose_tests.rs` | 直した |
| 139 | できれば | E | 失敗の文に 9 つの空白（行の続きの `\` が抜けた） | `zorder_chain_compose_tests.rs` | 直した |
| 140 | できれば | E | t_zgp27 の名前が、指摘 33 で中身を消した後の本体と合わない | `zorder_group_ledger_tests.rs` | 直した（名を変えた） |
| 141 | できれば | E | 「本番と同じ順」と書き、本番の 3 つの系のうち 2 つしか回さない | `zorder_chain_tail_order_tests.rs` | 直した（doc） |
| 142 | できれば | E | 分岐の表が wintf を外す理由（ほかの crate のファイルは読めない）が誤り | 本体の `placement_tests/zorder_group_branch_coverage_tests.rs` | 直した |
| 143 | できれば | E | 足した支えが `code_only` の写しを持つ | 本体の `placement_tests/zorder_branch_coverage_test_support.rs` | 直した（共有の `zorder_pair_deferred::code_only` を使う） |
| 144 | できれば | E | `place_after` が要るより広く開いている | `zorder_test_support.rs` | 直した（私有に） |
| 145 | できれば | F | `HoldSite::PatternMove` を足した後も「4 点」と書く | `dpi_sync.rs` | 直した |
| 146 | できれば | F | 経路の語の数が 12（今は 13） | `dpi_sync_tests.rs` | 直した（数を書かない） |
| 147 | できれば | F | 消えた `test_support::capture` を名指す頭の説明 | `balloon_limit_gate_tests.rs`・`balloon_lock_tests.rs`・`sticky_group_ledger_world_tests.rs` | 直した |
| 148 | できれば | F | 収める式を共有にした後、t_bl4 が `clamp_axis` を自分と比べている | `balloon_limit_tests.rs`・`balloon_limit.rs` | 直した（期待値を数で書く。式の上下を入れ替えると赤） |
| 149 | できれば | F | 兄弟のテストの小道具の写し（`DPIS`・`work_area`・`snapshot`・挟みの印の綴り） | `balloon_limit_*_tests.rs` | 直した（あわせて組み立てを `cfg_of` に） |
| 150 | できれば | F | 記録を `event` で絞る補助が 3 つのファイルに写してある | `balloon_lock_tests.rs`・`balloon_lock_drag_tests.rs`・`sticky_group_ledger_world_tests.rs` | 直した（`shared_test_support::events_with`。本体の側の 1 つは crate の私有の支えが見えないので残す） |
| 151 | できれば | F | 名前の言うこと（クリックでなくドラッグの終わりとして届く）を確かめないテスト | `balloon_lock_drag_tests.rs` | 直した（消した） |
| 152 | できれば | F | `nothing_is_recorded_while_unarmed` の前半が武装を呼ばず、門を消しても緑 | `chain_realign_script_moved_tests.rs` | 直した（武装を呼ぶ。門を消すと赤） |
| 153 | できれば | F | バルーンの窓はあるが寸法が無いとき、記録が「窓が無い」と言う。モジュールの doc の誤り | `reseed.rs` | 直した（`balloon_size_unknown` を分ける） |
| 154 | できれば | F | 武装のあいだ、同じ窓の台本の移動を毎回足し、止まると限りなく伸びる | `chain_realign.rs` | 直した（同じ窓は 1 度だけ） |
| 155 | できれば | F | `CHAR1`・`boot_placements` が要るより広く開いている | `reseed_test_support.rs` | 直した（私有に） |
| 156 | できれば | F | キーワードのバルーンの descript の文が 2 つの crate に写してある | `reseed_tests.rs`・本体の `placement_tests/reset_balloon_pos_tests.rs` | 直した（`test_support::KEYWORD_BALLOON_DESCRIPT`） |
| 157 | できれば | A | 「射影を通しても動かない（べき等）」は、作業領域に収まる寸法のときだけ（タスク 2.1 のレビューの提案） | `resolver.rs` | 直した |
| 158 | できれば | A | `from_alignment` の知らない値の `warn!` にスコープの番号が載らない（同上） | `resolver.rs` | 直した（スコープを知る `resolve_placement` が記録する。台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 8 行の出所も直した） |
| 159 | できれば | A | `lib.rs` の行の番号が古い（タスク 4.4 のレビュー） | `doc/COMPAT_ARCHITECTURE.md`・本体の `examples/collision-probe.rs` | 直した（`COMPAT_ARCHITECTURE.md` は行の番号をやめて関数の名に）。`collision-probe.rs` は申し送り（`audit-areka-app`） |
| 160 | できれば | B | `Effect::PersistRemove` の doc にこの spec のタスク番号（タスク 5.2 のレビュー） | sylphya `actor.rs` | 直した |
| 161 | できれば | E | `check_table` の較正が、`#[test]` の欠け・`#[path]` の欠け・分岐名の重なりを確かめていない（タスク 6.7 のレビュー） | 本体の `zorder_branch_coverage_test_support.rs` | 直した（3 つの形を足した。どれも検査を外すと赤） |
| 162 | できれば | E | tail のテストの頭が wintf の実窓のファイルを 3 つと挙げる（今は 4 つ）（同上） | `zorder_chain_tail_order_tests.rs` | 直した |
| 163 | できれば | F | `restore_empty` の行の文が「配置の解決が空を返した」のまま（タスク 6.8 のレビュー） | `reseed.rs` | 直した |
| 164 | 直したほうがよい | — | ドラッグを始めた窓の位置が取れないとき、`DraggingState.initial_window_pos` に (0,0) が入り、追随の `policy_mapped_position` がそれを基準に窓を動かしうる（`audit-wintf-input` の申し送り件 1） | wintf の `DraggingState`・追随の `follow/drag_follow.rs` | 回す（裁定: 推奨案で承認。C20 の後に `/kiro-discovery` で起票） |
| 165 | できれば | A | この spec の指摘番号がコードの説明に入っている（spec を閉じると意味を失う） | `lib.rs`・`placement_prepare_tests.rs`・`resolver_resolve_*_tests.rs`・`face_windowposition.rs`・`face_windowposition_tests.rs`・`persist.rs`・`persist_io_wiring_tests.rs`・`spawn.rs`・`spawn_clickthrough_tests.rs`・`test_support_com_tests.rs`・本体の `placement_tests/reset_balloon_pos_tests.rs` | 直した（差し戻しで残りの 13 か所も直した。どれも番号を消した: 説明の文が中身をもう言っているので、番号が無くても読める。`persist.rs` の 1 か所は要件の番号が残るので spec の名前を足した。spec の名前の付いた 4 か所（`audit-placement-core 指摘 17` の形・`reset_balloon_pos.rs`・本体の `reset_balloon_pos_tests.rs` の頭・sylphya の `persist/mod.rs`・`persist_remove_tests.rs`）は、ツリーのほかの所（`spec: audit-emo-atlas 指摘 19`）と同じ形なので残した） |
| 166 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（7 行） | `lib.rs` | 直した（番号を消した） |
| 167 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（1 行） | `placement_monitor_tests.rs` | 直した（番号を消した） |
| 168 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（4 行） | `placement_prepare_tests.rs` | 直した（番号を消した） |
| 169 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（1 行） | `placement_shared_test_support.rs` | 直した（番号を消した） |
| 170 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（5 行） | `placement_windowposition_tests.rs` | 直した（番号を消した） |
| 171 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（2 行） | `placement_windowposition_vocab_tests.rs` | 直した（番号を消した） |
| 172 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（1 行） | `resolver.rs` | 直した（番号を消した） |
| 173 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（1 行） | `resolver_resolve_contract_tests.rs` | 直した（番号を消した） |
| 174 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（1 行） | `resolver_resolve_free_tests.rs` | 直した（番号を消した） |
| 175 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（3 行） | `windowposition.rs` | 直した（番号を消した） |
| 176 | できれば | A | 名前の無い古い spec の task 番号が説明に残る（1 行） | `windowposition_vocab_tests.rs` | 直した（番号を消した） |
| 177 | できれば | B | 名前の無い古い spec の task 番号が説明に残る（3 行） | `balloon_offset_persist_roundtrip_tests.rs` | 直した（番号を消した） |
| 178 | できれば | B | 名前の無い古い spec の task 番号が説明に残る（10 行） | `balloon_offset_supply_tests.rs` | 直した（番号を消した） |
| 179 | できれば | B | 名前の無い古い spec の task 番号が説明に残る（2 行） | `persist.rs` | 直した（番号を消した） |
| 180 | できれば | B | 名前の無い古い spec の task 番号が説明に残る（1 行） | `persist_io_wiring_tests.rs` | 直した（番号を消した） |
| 181 | できれば | B | 名前の無い古い spec の task 番号が説明に残る（3 行） | `source.rs` | 直した（番号を消した） |
| 182 | できれば | B | 名前の無い古い spec の task 番号が説明に残る（1 行） | `source_tests.rs` | 直した（番号を消した） |
| 183 | できれば | C | 名前の無い古い spec の task 番号が説明に残る（3 行） | `examples/window-placement.rs` | 直した（番号を消した） |
| 184 | できれば | C | 名前の無い古い spec の task 番号が説明に残る（1 行） | `config.rs` | 直した（番号を消した） |
| 185 | できれば | C | 名前の無い古い spec の task 番号が説明に残る（3 行） | `measure.rs` | 直した（番号を消した） |
| 186 | できれば | C | 名前の無い古い spec の task 番号が説明に残る（2 行） | `measure_template_tests.rs` | 直した（番号を消した） |
| 187 | できれば | C | 名前の無い古い spec の task 番号が説明に残る（3 行） | `measure_tests.rs` | 直した（番号を消した） |
| 188 | できれば | C | 名前の無い古い spec の task 番号が説明に残る（1 行） | `test_support.rs` | 直した（番号を消した） |
| 189 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（4 行） | `diag.rs` | 直した（番号を消した） |
| 190 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（14 行） | `spawn.rs` | 直した（番号を消した） |
| 191 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（2 行） | `spawn_cleanup_tests.rs` | 直した（番号を消した） |
| 192 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（1 行） | `spawn_clickthrough_tests.rs` | 直した（番号を消した） |
| 193 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（2 行） | `spawn_follow_pipeline_tests.rs` | 直した（番号を消した） |
| 194 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（3 行） | `spawn_zorder_chain_wiring_tests.rs` | 直した（番号を消した） |
| 195 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（2 行） | `spawn_zorder_pair_wiring_tests.rs` | 直した（番号を消した） |
| 196 | できれば | D | 名前の無い古い spec の task 番号が説明に残る（8 行） | 本体の `placement_tests/spawn_assembly_tests.rs` | 直した（番号を消した。うち 1 行は spec の名前を正式の綴り（`areka-P0-windowposition-limit`）にして番号を消した） |
| 197 | できれば | E | 名前の無い古い spec の task 番号が説明に残る（1 行） | `zorder_chain_tail_order_tests.rs` | 直した（番号を消した） |
| 198 | できれば | E | 名前の無い古い spec の task 番号が説明に残る（5 行） | `zorder_group_ledger_state_tests.rs` | 直した（番号を消した） |
| 199 | できれば | E | 名前の無い古い spec の task 番号が説明に残る（1 行） | `zorder_property_deferral_tests.rs` | 直した（番号を消した） |
| 200 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（4 行） | `balloon_limit.rs` | 直した（番号を消した） |
| 201 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（2 行） | `balloon_limit_gate_tests.rs` | 直した（番号を消した） |
| 202 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（1 行） | `balloon_lock_drag_tests.rs` | 直した（番号を消した） |
| 203 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（1 行） | `balloon_lock_tests.rs` | 直した（番号を消した） |
| 204 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（3 行） | `dpi_sync.rs` | 直した（番号を消した） |
| 205 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（3 行） | `dpi_sync_tests.rs` | 直した（番号を消した） |
| 206 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（1 行） | `reseed_tests.rs` | 直した（番号を消した） |
| 207 | できれば | F | 名前の無い古い spec の task 番号が説明に残る（1 行） | `sticky_group_ledger_world_tests.rs` | 直した（番号を消した） |

- 申し送りとの対応（要件 1.6）: 2 節の 80 件と同じ所を指す指摘は 0 件（どれも 80 件の直しの後の今のコードの指摘）。15 節の表の件を 2 度数えたものも 0 件（17.4 の `reseed_test_support.rs` の `fake_handle`・`position_of` は、わざと残した物として数えない）。実装の段で拾った候補は 157〜163、`audit-wintf-input` の申し送りは 164 に当たる。
- 166〜207（名前の無い古い spec の task 番号）: 受け持つ範囲の 42 ファイル・119 行。完了した audit の spec が同じ形を指摘にして直している（`completed/audit-emo-compose` の 43・143、`audit-mcp-update-crates` の 51）ので、指摘に数えた。どれも番号を消した（番号の指す spec を 1 行ずつ調べ直して名前を足すより、説明の文が中身をもう言っているので消すほうが確か）。番号が文の主語だった所は、指す物の名前（「下の『供給結線』の群」「上の純関数の側の檻」など）に書き換えた。同じ行か前の行に spec の名前のある 10 行（`areka-P0-balloon-offset-dpi・…・task 4.1` の形）は、どの spec の番号かが読めるので残した。`test_crash_report.rs` の 1 行は 88 の裁定で残した。追随の持ち物の分は申し送り。
- 直しのついでに見つけた、直しで古くなった所も直した: 109 で動いた行を指す `crates/ukadoc-survey/src/evidence/extract_tests.rs` の説明の行の番号（範囲の外・直しが作った古さなので 1 行だけ直した）と、`measure.rs` の「`measure_tests.rs` は 1,000 行の上限に余裕が無い」（今は 846 行・120 に含める）。
- 2 度目の差し戻しで、直しが動かした行の番号も直した（直しが作った古さなので直した）。`HEAD` の時点では正しい行を指していて、この回のコードの直しで行がずれた参照である。この回が触った `.rs`（`git diff --name-only HEAD`）を指す `<ファイル>.rs:<行>` の形を、同じ行の続きの `:<行>` も込みでツリー全体（`.kiro/specs/completed/` と `vendors/` を除く）から拾い、`git show HEAD:` の行と今の行を 1 つずつ見比べた。
  - `doc/ukadoc-coverage/briefing-assets.md`（今の行の番号に）: `source.rs:149`→`150`・`source.rs:155`→`156`・`source.rs:159`／`:162`→`160`／`163`・`config.rs:244`→`242`・`config.rs:292`→`293`・`config.rs:135`〜`:138`・`:209`・`:239`→`134`〜`136`・`207`・`237`（同じ行の `:255`・`:260` は動いていない）。`:138` が指していた `seriko.dpi` の転記は 86 で消えたので、同じ名前を今も同じ表から引く `source.rs:117`（`shell_author_dpi`）を並べた。
  - `doc/COMPAT_ARCHITECTURE.md`（159 と同じく行の番号をやめて名前に）: `config.rs:135`→`config.rs` の `build_placement_config`・`zorder_chain_compose.rs` の続きの `:93`→同じファイルの `SCOPE_BLOCK`。
  - `crates/areka/examples/collision-probe.rs`（今の行の番号に）: `source.rs:153-155`→`154-156`・`source.rs:157-168`→`158-169`・`source_tests.rs:264-268`→`236-240`（109 で動いた）。
- `HEAD` の時点でもう古かった参照は、この回では直さない（2 回目の候補）。
  - `doc/COMPAT_ARCHITECTURE.md`: `resolver.rs:267`（`HEAD` で `resolve_placement` は 268 行・`measure_dpi` は 272 行）・`resolver.rs:383-386`（基準対を刻む所は `HEAD` で 395〜398 行）・`zorder_chain_compose.rs:123`・`:149`・`:156`・`:134`・`:193`（`HEAD` でどれも 4 行後ろ＝127・153・160・138・197）・`diag.rs:274`（`BalloonFollow` の腕は `HEAD` で 270 行）。
  - `doc/ukadoc-coverage/briefing-assets.md`: `source.rs` の続きの `:191`（2 か所。警告は `HEAD` で 196 行）・`source.rs:210`（`load_balloon_author_dpi` は `HEAD` で 213 行）・`source.rs:237`／`:247`（2 つの警告は `HEAD` で 242・252 行）・`measure.rs:399`（2 か所。呼び出しは `HEAD` で 398 行）・`resolver.rs:280`／`:468`（警告は `HEAD` で 479 行の 1 つだけ。158 で `resolve_placement` へ移したので、「前者」「後者」と 2 つを語る文ごと書き直す）。
  - `crates/areka/examples/collision-probe.rs`: `source.rs:208-212`（`HEAD` で 213〜217 行）・`measure.rs:394-405`（393〜404 行）・`measure.rs:433`（432 行）・`lib.rs:575`（`load_scope_windowpositions` は `HEAD` で 585 行）・`lib.rs:373-455`・`lib.rs:400-407` と消えた `prepare_stages` の名（下の `audit-areka-app` への申し送りに入っている）。`crates/areka/examples/collision-probe/setup.rs` の `window-placement.rs:245-247`（`MonitorSnapshot` を入れる所は `HEAD` で 249〜253 行）。
- 拾ったが直さないもの: 日付つきの記録に書かれた当時の行（`.kiro/steering/roadmap-history.md`・`.kiro/specs/clippy-199-lints/brief.md` の 2026-10-03 の実測・この spec の `brief.md`）。置き場の名前も古い（`placement/*`）当時の写しで、今の行を指す参照ではない。

#### 指摘にしなかったもの

- `source.rs` の `read_kv_lenient`・`load_balloon_author_dpi`・`load_descript_source_for_shell` を crate の中だけに狭める、という報告。本体の `runtime/assets.rs`・`switch_assets.rs` ほかが使う（grep で確かめた）ので誤り。
- バルーン位置初期化で解けないスコープの記録が 2 行になる、という報告。doc のとおりのわざとの形（`trigger` を足す行）で、読み手も「直さなくてよい」とした。
- 測りのテストの emo2 の測りを 4 回から 2 回にまとめられる、という小さな速さの案。読み手が数に入れなかった。

#### 裁定が要るもの（要件 5・8.6）

5 件とも推奨案で承認（2026-11-15。audit の spec は議題を推奨案で承認してよい、という開発者の決まりによる）。

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 81 | 要件 5 の 1 種類目（完了した `areka-P0-window-placement` の設計「両方あるときのみ」・`areka-P0-balloon-offset-dpi` の要件 2.6「片軸／未宣言の受理規約は不変」）と 3 種類目（バルーンの出る位置・SSP との互換）。ukadoc https://ukatech.shillest.net/ukadoc-spec/descript_shell.html#sakura.balloon.offsetx 「X座標とY座標はそれぞれ独立に決まる。」 | 片軸だけのとき両方を捨てるか、書かれた軸だけを使うか | **直す**: `config.rs` の `zip` をやめ、書かれた軸を使い、書かれない軸は 0（標準の位置）にする。T-C5 の片軸のテストを裏返し、`lib.rs`・`balloon_offset_supply_tests.rs` の「両方揃ったときのみ」の説明も直す。ukadoc の正典に合わせる直しで、SSP と同じ位置になる | 推奨案で承認。直した |
| 88 | 要件 5 の 1 種類目（完了した `areka-test-binary-stability` の設計「新しいクレートを作らない（依存を足さない）ための写し」・`placement-crate-split` の設計「3 つ目の写し」） | 落ちの報告のテストの支えを wintf と写しで持ち続けるか | **残す**: 依存を足さずに 1 つにする道は `#[path]` でほかの crate の `src` を取り込むことだけで、wintf の中の置き場を動かすとこの crate のテストが組めなくなる。写しの相手は各ファイルの頭の行で追える。裁定で残すなら理由をここに書き、0 件の数から外す（要件 5.4） | 推奨案で承認。残す。理由: 依存を足さずに 1 つにできず、写しの相手は各ファイルの頭の行で追える。0 件の数から外す。写しの中の 483 行の「タスク 2.4」（名前の無い task 番号）も、wintf の同じ行と食い違わせないために残す |
| 86 | 要件 8.6（C20 のあいだ公開の形を変えない＝8.2。欄を消す） | `PlacementConfig.shell_dpi_raw` を C20 のうちに消すか | **C20 のうちに直す**: 範囲の外の使い手は 0（本体にも無い）。テストの 4 つの組み立ては `cfg_of` に寄せたので、欄と `build_placement_config` の 1 行と T-C5 の 2 つの確かめを消すだけ。議題 5 と同じ形 | 推奨案で承認。直した |
| 118 | 要件 8.6（同上。impl を消す） | `impl Default for MeasureScaling` を C20 のうちに消すか | **C20 のうちに直す**: 使い手は `measure_tests.rs` の確かめ 1 つだけ。指摘 45 と同じ形 | 推奨案で承認。直した |
| 164 | 要件 8.6（wintf の型＝8.3 と、追随の持ち物＝8.1。`drag_follow.rs` は今も 1,000 行ちょうどで 1 行も足せない） | `DraggingState` に「位置が分からない」を持たせる直しを、この spec で申し合わせて行うか、回すか | **回す**: 持ち主の spec は今は無い（`audit-wintf-input` と `audit-placement-follow` は完了済み）。C20 の着地の後に `/kiro-discovery` で起票し、`DraggingState` を組み立てる 9 か所（`git grep -n "DraggingState {" -- crates`）と `drag_follow.rs` の分け方をあわせて決める。今の振る舞いは `audit-wintf-input` の前と同じで、窓に `WindowPos`・配置が無いときだけ起きる | 推奨案で承認。回す（完了の申し送りに「C20 の後に `/kiro-discovery` で起票」として書く） |

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ: 83（`follow_test_support.rs` の `assert_no_event` を消し、共有の `shared_test_support::assert_no_event` を使う）・91 の残り（`follow/move_shift.rs` が `windowposition::scale_signed` を直に引く。根の `scale_signed` を使えば規約の文と合う）・追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）に残る、名前の無い古い spec の task 番号（`git grep -nE "(task|タスク) ?[0-9]+"` で 47 ファイル・220 行。spec の名前の付いた行も入った粗い数）と、名前の無い指摘番号（`git grep -nE "指摘 ?[0-9]+"` で 18 ファイル・39 行）。この spec の 165・166〜207 と同じ形で、番号を消すか spec の名前を足す。
- `audit-areka-app` へ: 159 の残り（`crates/areka/examples/collision-probe.rs` の `lib.rs` の行の番号と、消えた `prepare_stages` の名。`HEAD` の時点でもう古かった `source.rs:208-212`・`measure.rs` の行の番号と、`collision-probe/setup.rs` の `window-placement.rs:245-247` も同じ持ち主。この spec の直しが動かした `source.rs`・`source_tests.rs` の行の番号は、2 度目の差し戻しで直した）・81 の残り（`crates/areka/examples/emo-present/balloon.rs` の `read_balloon_offset` は descript を自分で読み、`balloon.offsetx`／`offsety` の片方だけを「仕様外」として捨てる。81 と同じ形に直す）。
- C20 の後に `/kiro-discovery` で起票: 164（`DraggingState` に「位置が分からない」を持たせる。組み立てる 9 か所と `drag_follow.rs` の分け方をあわせて決める。裁定の表の 164 の行）。
- 116 の保存の道の隙は、差し戻しでこの回のうちに直した（申し送りから外した）。

#### テスト（差し戻しの直しの後に回し直した）

- `cargo test -p areka-placement --features test-support`: lib 972 件が通り 1 件 ignore、手本 2 件が通る。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る（説明の 1 行と台帳の 8 行、81 の台帳の 4 項目と briefing の 2 行を直した後）。
- `cargo test -p areka`: 本体の bin 3180 件が通り 6 件が ignore、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo fmt --all -- --check`: 緑。`cargo test -p log-capture-kit --test file_length_guard_test`: 6 件が通る（`persist.rs` 976 行・`lib.rs` 925 行）。
- 2 度目の差し戻しの直し（行の番号と記録だけ・コードの行は動かしていない）の後にも回し直した: `cargo test -p areka-placement --features test-support`（lib 972 件が通り 1 件 ignore・手本 2 件）・`cargo test -p ukadoc-survey`（全部通る）・`cargo test -p areka`（本体の bin 3180 件が通り 6 件が ignore、ほかの的も全部通る）・`cargo fmt --all -- --check`（緑）・行数の門（6 件が通る）。`areka-sylphya` はこの直しで触っていないので回していない。

#### 判定

1 回目の新しい指摘は 127 件（必ず直す 1・直したほうがよい 10・できれば 116）で、0 件ではない。直した 124 件・裁定で残す 1 件（88）・裁定で回す 1 件（164）・申し送りだけのもの 1 件（83）。裁定待ちは 0 件。2 回目をかける（要件 6.2）。

### 18.2 2 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- `main` は 1 回目の後に 1 コミット進んだ（`2f664eb68`・`audit-tools-perf-run`）。触ったのは `tools/perf`・スキル・spec の文書・`roadmap.md` だけで、範囲とは重ならない。取り込みは `246b6d75f`（競合なし）。取り込んだ後の `HEAD..origin/main` は 0 件。
- 範囲は 1.1 節と 18.1 と同じ数え方で測り直した。ファイルの数は 1 回目と同じ 116。行は 1 回目の直しで減った（sylphya の `persist_remove_tests.rs` だけは指摘 116 の直しで 225 行から増えた）。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 27 | 10,537 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,894 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,191 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 357 |
| 計 | 116 | 37,030 |

- ponytail は 5.3.3（1 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,705・B 20／6,823・C 20／5,971・D 17／5,616・E 15／6,020・F 20／5,895。計 37,030 行で上の表と合う）。
- 18.1 の「2 回目の候補」（`HEAD` の時点でもう古かった行の番号）と、1 回目のレビューが候補から漏れていたと見つけた `briefing-assets.md:253` の `config.rs` の続きの番号も、同じ一覧に入れた（293〜295）。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,705 | 0 | 3 | 11 | 14 |
| B | 20 | 6,823 | 0 | 0 | 21 | 21 |
| C | 20 | 5,971 | 0 | 3 | 10 | 13 |
| D | 17 | 5,616 | 0 | 1 | 16 | 17 |
| E | 15 | 6,020 | 0 | 0 | 9 | 9 |
| F | 20 | 5,895 | 0 | 1 | 10 | 11 |
| 18.1 の 2 回目の候補（文書の行の番号） | — | — | 0 | 0 | 3 | 3 |
| 計 | 116 | 37,030 | 0 | 8 | 80 | **88** |

- 2 つ以上の組が同じ所を出したものは 1 件に数えた（208 は A・B・C、217 は A・C、220 は A・C、251 は C・F、218 は A・D の組が出した）。

#### 指摘の一覧（208〜295）

扱いの印は 18.1 と同じ。裁定待ち＝開発者の裁定を仰いでいて、まだ直していない。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 208 | 直したほうがよい | A | 指摘 81 の直しの後も「片軸だけの宣言は `None`」「両軸が揃ったときのみ採用」と書く説明が 2 か所に残る | `lib.rs:517`・`balloon_offset_supply_tests.rs` の頭 | 直した |
| 209 | 直したほうがよい | A | 「準備が保存を読み書きしない」テストが、どこも読まない `ghost.dat` を見ていて決して落ちない（本物の保存は sylphya の `profile` の下） | `placement_prepare_tests.rs`・`lib.rs` の 2 か所の doc | 直した（本物の `sylphya.toml` を置いて `load_restored_state` が読めることを確かめ、準備の後に配置・題名・ファイルの中身が変わらないことを確かめる形に。名は `prepare_never_reads_or_writes_the_saved_state`。準備の中で記憶を戻す呼び出しを入れる壊し方で赤（スコープ 0 の x が 1486 から -117）。`ghost.dat` の文と「後続タスクの merge シーム」も直した。`test_support.rs` の doc の古い名も直した） |
| 210 | 直したほうがよい | A | 右の辺のスコープと下端の連なりのスコープが、初めて起動したとき右下の角でまるごと重なる（指摘 1 の直しの形。`t_r5_mixed_edges_chain_per_edge` がこの重なりを期待として固定している） | `resolver.rs` の `resolve_placement`（P1'・P2） | 残す（3 回目の前に開発者の裁定で今のまま。SSP も重なりを離さない。測りと理由は 18.3 の裁定の表） |
| 211 | できれば | A | 真上の完全一致の後で同じことを言い直す確かめと、引き算で作る `balloon_offset ≡ balloon_pos − char_pos` の確かめ（決して落ちない） | `placement_prepare_tests.rs`・`placement_windowposition_tests.rs`・`placement_monitor_tests.rs`・`resolver_resolve_contract_tests.rs`・`resolver_balloon_keyword_tests.rs`・`resolver_resolve_balloon_tests.rs`・`resolver_resolve_tests.rs` | 直した（消した。理由は残る完全一致の確かめの文へ移した） |
| 212 | できれば | A | 記録の出所の札のテストが 3 つのうち 2 つしか確かめない（`WORK_AREA_SYNC_CONTEXT` を `"monitor_snapshot"` にしても緑） | `placement_monitor_tests.rs` | 直した（3 つの綴りを固定。その壊し方で赤） |
| 213 | できれば | A | 既定と同じ 96/96 の emo2 で確かめるだけのテスト（宣言のあるテストが全部を確かめる） | `placement_prepare_tests.rs` の `prepare_emo2_carries_author_dpi_read_once_for_attach` | 直した（消した。確かめは `prepare_declared_author_dpi_drives_each_axis_k0_and_attach_value` にある） |
| 214 | できれば | A | 同じ入力・同じ早い戻りのテスト 2 本 | `windowposition.rs` のテスト | 直した（消した。行列のテストと `none_preserves_existing` のテストが確かめる） |
| 215 | できれば | A | `AuthorDpi::DEFAULT` の doc が、無い使い方（準備の失敗の代わり）を書く。使い手はテストだけ | `lib.rs` | 直した（doc だけ。公開の形は変えない） |
| 216 | できれば | A | 古い説明（つなぎ目の置き場・`placement/follow/` の道・関数の置き場・「採寸の後で失敗する道は無い」・「以後静的」・「逐語同一」・消したテストを確かめると書く頭・差し替えたテストの経緯） | `lib.rs`・`windowposition.rs`・`resolver.rs`・`resolver_resolve_test_support.rs`・`resolver_balloon_keyword_tests.rs`・`resolver_resolve_contract_tests.rs` | 直した |
| 217 | できれば | A | 名前の無い古い spec の task 番号の複数形「（tasks 6.1）」「（tasks 4.1）」が、1 回目の grep（`task ?[0-9]`）をすり抜けた | `placement_prepare_tests.rs:24`・`measure_tests.rs:26` | 直した（番号を消した） |
| 218 | できれば | A | spec の名の略称 `scg`（`areka-P0-scope-chain-gap`）・`atom`（`areka-P0-dpi-transition-atomicity`）が、どの spec かを読み手に言わない | `chain_finalize.rs`・`chain_finalize_tests.rs`・`placement_prepare_tests.rs`・`resolver.rs`・`resolver_resolve_free_tests.rs`・`resolver_resolve_tests.rs`・`spawn.rs`・`spawn_cleanup_tests.rs` | 直した（範囲の中の 41 か所と 3 か所を正式の名に。範囲の外の分は申し送り） |
| 219 | できれば | A | 記録を `event` で絞る補助の写しが 1 つ残っていた（指摘 150 が見落とした） | `script_alignment_tests.rs` の `events_of` | 直した（`shared_test_support::events_with`） |
| 220 | できれば | A | 拡大率を作る小さな補助 `k(num, den)` が 4 か所に写してある | `placement_prepare_tests.rs`・`windowposition.rs`・`balloon_offset_supply_tests.rs`・`measure_tests.rs` | 直した（`placement_shared_test_support.rs` の 1 つ。追随の写しは申し送り） |
| 221 | できれば | A | `load_scope_windowpositions` の読めない枝の `warn!` に、確かめるテストが無い（シェルの切り替えは測らずに呼ぶので届く） | `lib.rs` の `scope_windowposition` | 直した（`load_scope_windowpositions_warns_and_skips_an_unreadable_balloon`。`warn!` を消す壊し方で赤） |
| 222 | できれば | B | 並べ終えた時点の保存が「1 度だけ」と書くが、シェル位置初期化でも毎回呼ばれる | `persist.rs` の頭と `persist_unremembered_char_positions` | 直した（2 つ目の時点を書いた。記録の文は変えない） |
| 223 | できれば | B | 古い参照 3 つ（名前の無い「4.1 群」・「上の 3 本」・「第 2 面」） | `balloon_offset_supply_tests.rs` | 直した |
| 224 | できれば | B | この spec の指摘番号と、spec の名の無い要件の番号が説明に残る | `persist_restore_tests.rs` | 直した（`spec: audit-placement-core 要件 9.2` の形に） |
| 225 | できれば | B | 消えた関数を名指す説明（指摘 101 と同じ形） | `persist_restore_tests.rs` の頭 | 直した（消した） |
| 226 | できれば | B | 消した引数の経緯の括弧書き | `persist.rs` | 直した（消した） |
| 227 | できれば | B | `apply_restored_placements` の doc が、本体の呼び手を違う名で書く（今は `restore_and_judge`） | `persist.rs` | 直した |
| 228 | できれば | B | シェルの descript の読み失敗が「致命（終える）」と書くが、シェルの切り替えでは切り替えが失敗するだけ | `source.rs` | 直した（呼び手へ `Err` を返す、と書く） |
| 229 | できれば | B | 頭の依存の決まりが「兄弟のモジュールを引かない」と書くが、`error` を引く | `source.rs` | 直した |
| 230 | できれば | B | 1 行の取り次ぎを本体と比べるだけの確かめ（決して落ちない） | `source_shell_tests.rs` | 直した（消して、本来のシェルの表を確かめる形に） |
| 231 | できれば | B | 何も足さないテスト（doc の言う関数も違う） | `persist_entries_tests.rs` の `entries_values_round_trip_through_parse_px` | 直した（消した。確かめは `char_pos_entries_carries_scope_and_negative_values` と `parse_px_*` にある） |
| 232 | できれば | B | 完全一致の直後で決して落ちない確かめ 3 つ | `persist_restore_tests.rs`・`persist_restore_offset_base_tests.rs` | 直した（消した） |
| 233 | できれば | B | 共有の記憶の試験台 `Store` を手で組み直す写しが 2 か所 | `persist_io_wiring_tests.rs`・`balloon_lock_drag_tests.rs` | 直した（`finalize_save_tests::Store` を使う。追随の写しは申し送り） |
| 234 | できれば | B | 「結線が無いとき」のテストが、doc の言う `debug!` を確かめず、`forget_entries` の同じ枝にはテストが無い | `persist_io_wiring_tests.rs` | 直した（`forget_entries_without_wiring_logs_debug_and_does_nothing`。`debug!` を消す壊し方で赤。`persist_entries` の側の `debug!` は `follow_keyword_base_tests` が確かめる） |
| 235 | できれば | B | 窓の帳簿が無いときに標準の材料が変わらないことを確かめていない | `face_windowposition_tests.rs` の `missing_material_only_warns` | 直した（確かめを足した。材料の差し替えを帳簿の確かめより前へ動かす壊し方で赤） |
| 236 | できれば | B | 記憶からの戻しのテストの小道具（`placement`・`bo`・`CSZ`／`BSZ`・`MEASURE_DPI`・5 つのアンカー）の写し | `persist_restore_tests.rs`・`persist_restore_offset_base_tests.rs`・`balloon_offset_persist_roundtrip_tests.rs` | 直した（1 組にまとめて引く） |
| 237 | できれば | B | テストの小道具が要るより広く開いている | `persist_finalize_save_tests.rs` | 直した（私有に） |
| 238 | できれば | B | sylphya のテストの `ghost_roots`／`ghost_file` が 3 つのファイルに写してある | sylphya `persist_remove_tests.rs`・`persist_lock_tests.rs`・`persist_unreadable_tests.rs` | 直した（`persist_unreadable_tests` の 1 組を引く） |
| 239 | できれば | B | 同じ記録の探し文字列が 4 か所にある | `balloon_offset_supply_tests.rs`・`placement_windowposition_tests.rs`・`placement_windowposition_vocab_tests.rs` | 直した（`shared_test_support::WINDOWPOSITION_SUPPLY_NEEDLE`） |
| 240 | できれば | B | 一時フォルダを 3 つ目の作り方で作る | `source_shell_tests.rs` | 直した（`unique_temp_dir`） |
| 241 | できれば | B | spec の名の無い裁定の番号 | `reset_balloon_pos.rs` | 直した（`areka-P0-window-link-directives の裁定 8`） |
| 242 | できれば | B | `parse_px`・`has_saved_char_pos`・`saved_adjacency` が、使い手が無いのに公開 | `persist.rs` | 直した（裁定: 推奨案で承認。私有にした。ツリー全体の grep で、コードの使い手は 0（`main.rs:795` の説明の文だけ）） |
| 243 | 直したほうがよい | C | 焼きの数のテストが、シェルのフォルダの直下の絵しか数えず、emo2 の絵のうち下のフォルダの絵（スコープ 1 の顔を含む）を見ていない | 本体の `placement_tests/measure_bake_count_tests.rs` の `in_dir` | 直した（フォルダからの相対の道で数える。数える絵は 1 枚から 55 枚に。直下だけを数える形に戻す壊し方で赤） |
| 244 | 直したほうがよい | C | 手本の DPI の説明が今の振る舞いと合わない（等倍・k=1.0・「膨れない」）。重なりの式も誤り | `examples/window-placement.rs` | 直した（文書だけ。窓寸は 96 での寸法×モニタの DPI÷96、式は resolver の P2 で確かめた `balloon_x(1) = char_x(0)`） |
| 245 | 直したほうがよい | C | 1 回目の指摘 87 の直しで書き直した分岐（候補がどれも無いときは記録しない）にテストが無い | `measure.rs` の `initial_surface` の呼び手 | 直した（`no_candidate_in_shell_leaves_it_to_the_compose_failure`。`&& exists(id)` を消す壊し方で赤） |
| 246 | できれば | C | `DefaultSurfaces::builtin()` の doc が、手本の入口と書く（呼び手はテストだけ） | `descript_window_keys.rs` | 直した（doc だけ） |
| 247 | できれば | C | 測りの説明が違う関数を名指す（`candidates`・`build_boot_assets`・`load_shell_target`） | `measure.rs`・`measure_template_tests.rs`・`examples/window-placement.rs` | 直した |
| 248 | できれば | C | `balloon_alignment` の doc が `none` を書かない | `config.rs` | 直した |
| 249 | できれば | C | ファイルの頭が、中で確かめる `balloon.dontmove`・`balloon.syncscale` を書かない | `descript_window_keys_tests.rs` | 直した |
| 250 | できれば | C | 共有の寸法を言い直すだけの定数 | `measure_default_surface_tests.rs` | 直した（消した） |
| 251 | できれば | C | 表を組む小さな補助 `kv` が 8 か所に写してある | `config.rs`・`config_limit_tests.rs`・`descript_window_keys_tests.rs`・`measure_default_surface_tests.rs`・`balloon_syncscale_tests.rs`・`reseed_test_support.rs`・`resolver_resolve_contract_tests.rs`・`spawn_follow_pipeline_tests.rs` | 直した（`test_support::kv` の 1 つ。テストの支えへの足し算だけ） |
| 252 | できれば | C | 動く絵の小道具（`image`・`info`・`frames`）が 2 つの crate に写してある | `measure_first_frame_tests.rs`・本体の `placement_tests/measure_bake_log_tests.rs` | 直した（`test_support::animated_image`。繰り返しの数は 1 にそろえた。どのテストもその数に頼らない） |
| 253 | できれば | C | 同じ読めない値を確かめるテストが 2 本ある | `config.rs` の `t_c2_*` | 直した（1 本にまとめ、行を 1 つ足した） |
| 254 | できれば | C | `include_str!` が作業の木のファイルを読むのに「`git HEAD` の実物」と書く | 本体の `placement_tests/measure_template_tests.rs` | 直した |
| 255 | できれば | C | 手本が面の有無を聞くためだけに描画の世界をスコープごとに 1 つ余計に組む | `examples/window-placement.rs` | 直した（1 つを両方に使う） |
| 256 | 直したほうがよい | D | 記録の水準のテストの支えが `log-capture-kit` の `capture_under_filter` の写しで、捕まえが死んでいても気付く確かめ（番兵）を落としている。「何も出ない」を確かめる 2 本が、何も捕まえなくても緑 | `diag_tests.rs` の `emit_all_under_filter` | 直した（dev-dependency の `log-capture-kit` に `env-filter` の機能を足し、`capture_under_filter` を使う。`Cargo.lock` は変わらない。`debug!` を `info!` にする壊し方で 2 本が赤。追随の写しは申し送り。3 回目に、log-capture-kit の見張りを赤にしていたと分かり、18.3 の 380 の裁定で機能を戻し、`diag_tests.rs` は手の写しに番兵を足す形に変えた） |
| 257 | できれば | D | 名前の無い古い spec の task 番号（「3.1（…）」ほか。1 回目に見出しの番号だけを消したので、下の行の番号が要件の番号に見える） | 本体の `spawn_assembly_tests.rs`・`spawn_test_support.rs`・`spawn_follow_pipeline_tests.rs` | 直した（番号を消した。要件の番号は「要件 N.M」に） |
| 258 | できれば | D | 同じテストの中で前の確かめを言い直すだけの確かめ 6 か所 | 本体の `spawn_assembly_tests.rs` | 直した（消した） |
| 259 | できれば | D | 差し替えた古いテストの経緯の説明 | 本体の `spawn_assembly_tests.rs` | 直した（消した） |
| 260 | できれば | D | ほかのテストが同じことを確かめるテスト | `spawn_cleanup_tests.rs` の `t_v1_paired_followup_despawn_is_noop` | 直した（消した。確かめは `t_v1_surviving_scope_state_unchanged_by_cleanup` にある） |
| 261 | できれば | D | 記録の札と宛先の綴りを言い直すだけのテスト 2 本 | `diag_tests.rs` | 直した（消した。札は全文のテストが、宛先は手順の絞り込みで点くテストが確かめる。宛先の後ろに文字を足す改名だけは赤にならなくなったが、絞り込みは頭からの一致で効くので手順はそのまま動く） |
| 262 | できれば | D | 前のテストの言い直しと、`reset()` の直後の決して落ちない確かめ | `chain_finalize_tests.rs` | 直した（消した） |
| 263 | できれば | D | それぞれ確かめてある 2 つの関数をつないで確かめるだけ | `chain_finalize_adjacency_tests.rs` | 直した（消した） |
| 264 | できれば | D | 待ちの列を差し込むことを、列を引くテストが既に確かめる | `spawn_window_band_wiring_tests.rs` | 直した（消した。残るテストの doc に一行） |
| 265 | できれば | D | 1 回目に書いた「モジュールの API はどれも本番から呼ばれる」が誤り（テストだけの物がある）。無い属性の説明 | `diag.rs` | 直した |
| 266 | できれば | D | `Restore` の経路の doc が、作る所を 1 つと書く（実は 3 つの引き金が 1 つの語を使う） | `diag.rs` | 直した（裁定: 推奨案で承認。doc に 3 つの呼び手を書き、語は増やさない） |
| 267 | できれば | D | モニタの写しの記録の doc が 3 つ誤る（呼び手・出る時・文脈の一覧） | `diag.rs` | 直した |
| 268 | できれば | D | 起動の後は読まれないと書くが、DPI の移り変わりで毎回使い直される | `chain_finalize.rs` の `ChainFinalizeStall` | 直した |
| 269 | できれば | D | `ScriptPlaced` の読み手の一覧が 1 つ足りない | `chain_finalize.rs` | 直した（`follow::work_area_clamp::clamp_grown_free_default` を足した） |
| 270 | できれば | D | 済んだ実機の測りを先のこととして書く | `chain_finalize.rs` | 直した |
| 271 | できれば | D | 分ける前のモジュールの道 `placement::` が残る | `chain_finalize.rs`・`diag.rs`・`spawn.rs` | 直した |
| 272 | できれば | D | 語の不在を確かめる 2 つの半分が、ファイルを読む補助を写して持つ | `spawn_zorder_pair_deferred_tests.rs`・本体の同名のファイル | 直した（`test_support::zorder_pair_deferred::read_source`。較正は本物の `spawn.rs`・`window_state_cue.rs` を読んだまま） |
| 273 | できれば | E | 台帳の読み出しの並びを「前後の決まりではない」と書くが、鎖の組み立てはその順で連ねる | `zorder_group_ledger.rs`・`zorder_group_ledger_state_tests.rs` | 直した |
| 274 | できれば | E | 並びの完全一致の後で決して落ちない確かめと、それだけに使う定数 | `zorder_chain_tail_order_tests.rs` | 直した（消した） |
| 275 | できれば | E | 完全一致の直後の言い直し 2 つ | `zorder_chain_compose_tests.rs` | 直した（消した） |
| 276 | できれば | E | 覆された寄せ（スコープのかたまりへの組み替え）をまだ書く | `zorder_group_ledger_tests.rs` | 直した |
| 277 | できれば | E | 名前の無い task 番号（「4.1 で 2 度差し戻された」「初版 1.2 の教訓」） | `zorder_chain_tail_order_tests.rs`・`zorder_group_ledger_tests.rs` | 直した（消した） |
| 278 | できれば | E | 「本 spec」「完了 spec」「本リリース」が、どの spec かを言わない | `zorder_chain_compose.rs` ほか 7 ファイル | 直した（指す物の名か、spec の正式の名に） |
| 279 | できれば | E | 支えの頭の使い手の数が古い | `zorder_test_support.rs` | 直した（数を書かない） |
| 280 | できれば | E | 隣り合う組を全部辺にする補助の写し | `zorder_chain_compose_tests.rs`・`zorder_chain_compose_written_order_tests.rs` | 直した（`zorder_test_support::all_consecutive_edges`） |
| 281 | できれば | E | 分岐の表の検査の決まりを 3 か所に別々の番号で書く | 本体の `zorder_branch_coverage_test_support.rs`・`zorder_group_branch_coverage_tests.rs`・`zorder_inversion_branch_coverage_tests.rs` | 直した（支えの 1 か所だけに） |
| 282 | 直したほうがよい | F | 指摘 154 の直しの後、印を付け直したことが記録されることを確かめるテストが無い（`on_insert` を `on_add` にしても緑） | `chain_realign_script_moved_tests.rs` | 直した（`reinsertion_after_arming_on_a_window_marked_before_is_recorded`。その壊し方で赤。古いテストの名を中身に合わせた） |
| 283 | できれば | F | 説明が `None` と書くが、コードは `BalloonPlace::Fits` で分ける | `balloon_limit.rs`・`balloon_limit_gate_tests.rs` | 直した |
| 284 | できれば | F | 兄弟のファイルも小道具を使うと書くが、使わない | `balloon_limit_tests.rs` | 直した |
| 285 | できれば | F | 名前の無い task 番号 | `balloon_limit_flip_tests.rs`・`balloon_limit_tests.rs` | 直した（消した） |
| 286 | できれば | F | 長さと順を言い直すだけのテスト | `balloon_limit_gate_tests.rs` の `boot_gate_preserves_order_and_length` | 直した（消した。長さは `boot_gate_clamps_display_position_and_keeps_offset_raw`、順は `boot_gate_logs_only_for_scopes_it_actually_moved` が確かめる） |
| 287 | できれば | F | 純粋な関数を 2 回呼んで集合を比べる（決して落ちない） | `sticky_group_ledger_tests.rs` の `parse_order_has_no_meaning` | 直した（消した。確かめは `parse_accepts_two_or_more_distinct_ids` にある） |
| 288 | できれば | F | 共有の `scopes` の写し 2 つ | `sticky_group_ledger_tests.rs`・`sticky_group_ledger_world_tests.rs` | 直した |
| 289 | できれば | F | バルーン位置初期化のテストの試験台が、窓の手の付け方を写して持つ | 本体の `placement_tests/reset_balloon_pos_tests.rs` | 直した（`test_support::fake_handle` を使う。残りは `reseed` の私有の `world_from` と入力（作業領域・題名）が違うので、そのまま） |
| 290 | できれば | F | 同じ決まった準備を 2 回比べる（決して落ちない） | 本体の `placement_tests/reset_balloon_pos_tests.rs` | 直した（消した） |
| 291 | できれば | F | シェルの切り替えなどで、知らない `alignmenttodesktop` の警告が同じスコープで走るスコープの数だけ繰り返す（2 スコープで 4 行） | `reseed.rs` の `resolve_standard_scopes` | 直した（作業領域と DPI の組ごとに 1 度だけ解く。各スコープは自分の要素を取る。RED を先に取った: `t_rs08_unknown_alignment_warns_once_per_scope` が直す前は `["0", "0", "1", "1"]`） |
| 292 | できれば | F | 基準を差し直す時の一覧に、面の windowposition の置き直しが抜けている | `balloon_syncscale.rs` の頭 | 直した |
| 293 | できれば | — | 行の番号が古い（`resolver.rs`・`zorder_chain_compose.rs`・`diag.rs`。E の組が見つけた `zorder_group_ledger.rs` の 4 か所と、E の直しで 1 行ずれた 9 か所も） | `doc/COMPAT_ARCHITECTURE.md` | 直した（159 と同じく、配置の crate を指す行の番号をやめて関数や型の名に。18 か所） |
| 294 | できれば | — | 行の番号が古い（`source.rs`・`measure.rs`・`resolver.rs`・`config.rs` の続きの 4 つ）。2 つの警告を語る「前者」「後者」の文 | `doc/ukadoc-coverage/briefing-assets.md` | 直した（今の行の番号に。警告は `resolve_placement` の 1 つなので文を書き直した） |
| 295 | できれば | — | 行の番号が古い・消えた `prepare_stages` を名指す | 本体の `examples/collision-probe.rs`・`collision-probe/setup.rs` | 直した（今の行の番号と `prepare_stages_for_shell` に。1 回目は `audit-areka-app` へ申し送ったが、ほかの直しの行を確かめる同じ手でここで直すほうが確かで、1 回目のレビューも一部をここで直すのを認めていた） |

- 申し送りとの対応（要件 1.6）: 1〜207 と同じ所を指す指摘は 0 件。2 つの組が同じ所を出したものは 1 件に数えた（上の注）。291 は指摘 1 と近いが、1 の直しは正典の値の繰り返しだけを消したもので、知らない値の繰り返しは残っていた（A の組は 1 に含めて数えなかったが、F の組の見立てを採って新しく数えた）。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を指す `<ファイル>.rs:<行>` の形を、ツリー全体（`.kiro/specs/completed/` と `vendors/` を除く）から拾い、`git show HEAD:` の行と今の行を比べた。今の行を指す参照で古くなったのは、上の 293〜295 で直した所だけ。この spec の `brief.md`・この節より前の記録・`.kiro/steering/roadmap-history.md` の当時の行は、日付つきの記録なので直さない（18.1 と同じ）。

#### 指摘にしなかったもの

- spec の名の無い要件・設計の番号（「要件 N.M」・`design.md`・`tasks.md`・`research.md`）。1 回目は task 番号と指摘番号だけを指摘にした。要件の番号は範囲のほぼ全部のファイルにあり、多くはファイルの頭で spec の名を言っている。1 回目と同じ線を保つ。
- `from_alignment` から警告を移したこと（1 回目の 158）で警告を失った呼び手。確かめた結果、本番の呼び手は `resolve_placement` の 1 か所だけで、失ったものは無い。
- 数える復号器 2 つ（`measure_first_frame_tests.rs` の `Counting` と本体の `CountingDecoder`）を 1 つの型にまとめる案。数える物（呼び出しの合計 3 つと、ファイルごとの 5 つ）も土台の復号器も違い、まとめると公開のテストの支えへ 75 行ほどを足して 38 行ほどを消すことになる。
- `AuthorDpi::DEFAULT`（215）と `DefaultSurfaces::builtin()`（246）を消す、またはテストの支えの奥へ隠す案。公開の形を狭める（要件 8.2）割に得る物が無いので、doc を直すだけにした。
- `PlacementRoute` に面の windowposition とバルーン位置初期化の語を足す案（266）。公開の列挙を増やし、追随の持ち物の網羅の `match` に触り、見え方の門の振る舞いも変える。doc に 3 つの呼び手を書くだけにした（推奨案で承認）。
- `reseed_test_support::world_from` を公開して本体の試験台と 1 つにする案（289）。作業領域と題名の入力が違い、公開のテストの支えを広げるので、一番小さい分だけを直した。
- 組から上がって、確かめて外したもの: `BalloonPlace::how()` の `Fits` の腕（網羅の `match` に要る）・`ClickThroughRegistrar` の実装が 1 つ（テストの偽の境界のためのわざとの継ぎ目）・`compose_chain` の毎フレームの割り当て（窓が数個で軽い）・`allocate_id` の飽和（届かない・書かれた決め）・手で直した記憶に `"0"` と `"00"` が並ぶときの読み（sylphya の読みの前からの振る舞いで、範囲の外）・`BalloonSyncScaleState.0` の読み手（テストの無いビルドで警告が出ないことを確かめた）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 210 | 要件 5 の 3 種類目（利用者に見える位置）。この spec の指摘 1 の直しの形で、1 回目に足した `t_r5_mixed_edges_chain_per_edge` が重なりを期待として固定している | 右（左）の辺のスコープの縦の連なりが下端の `bottom − h` から始まるので、下端の連なりの窓が持つ右下（左下）の角でまるごと重なる。たとえば `kero.seriko.alignmenttodesktop,right` で、さくらが既定の下端のとき | **SSP の振る舞いを MCP で測ってから決める**。重ならないようにするなら、辺の連なりを、同じ角を持つ下端の連なりの窓の上から始め、`t_r5_mixed_edges` の期待を裏返す（RED を先に）。測るまでは今のまま | 開発者の裁定で今のまま（SSP も重なりを離さない）。測りと理由は 18.3 の裁定の表 |
| 242 | 要件 8.6（C20 のあいだ公開の形を変えない＝8.2。関数を私有に） | `parse_px`・`has_saved_char_pos`・`saved_adjacency` を C20 のうちに私有にするか | **C20 のうちに直す**: ツリー全体で、コードの使い手は `persist` のモジュールの中だけ。指摘 86・118 と同じ形 | 推奨案で承認。直した |
| 256 | 要件 8（依存を足さない・`Cargo.lock` に触れない） | dev-dependency の `log-capture-kit` に `env-filter` の機能を足すか | **足す**: 新しい crate は無く、`log-capture-kit` から `tracing-subscriber` への辺は wintf が同じ機能で既に張っている。`Cargo.lock` の差分が 0 であることを確かめた | 推奨案で承認。直した（3 回目に、`log-capture-kit` の見張りを赤にしていたと分かり、18.3 の 380 の裁定で機能を戻した） |
| 266 | 要件 8.6（公開の列挙 `PlacementRoute` を増やす） | `Restore` を 3 つの引き金で分けるか | **doc だけ直す**: 語を足すと追随の持ち物の網羅の `match` に触り、見え方の門の振る舞いも変わる | 推奨案で承認。doc だけ直した |

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（持ち主の spec はもう完了しているので、8.2 が扱いを決める）:
  - `follow_test_support.rs` の `point`／`contained` の写し（`contained` は飽和加算でなく、`test_support.rs` が戒める形）と、`Store` の写し（530〜572 行）。
  - `follow_transition_diag_tests.rs:284`・`follow_window_move_diag_tests.rs:487` の記録の捕まえの手書きの写し（256 と同じ形。`capture_under_filter` を使う）。
  - `follow_move_shift_tests.rs` の `k` の写し（220）・`follow_sticky_follow_drag_tests.rs:79` の `events_of` の写し（219）・`follow/sticky_follow.rs` の略称 `scg`（218）。
- `audit-areka-app` へ:
  - 本体の `src/` に残る略称 `scg`・`atom`（`ghost_session.rs`・`main_restore_seam_tests.rs`・`runtime/frame.rs`・`runtime/frame/drain_resnap.rs`・`runtime/frame_chain_finalize_*_tests.rs`・`runtime/move_cue_place.rs`・`main.rs`・`main_monitor_snapshot_seam_tests.rs` ほか）。`placement_tests/transition_*` の分は追随の側。
  - `runtime/frame_diag_route_tests.rs:19` が、もう無い `areka_placement::test_support::capture_logs` を名指す（`audit-emo-atlas` の #451 で消えた）。
  - 1 回目の 159 の残り（`collision-probe.rs` の行の番号と `prepare_stages`）は、295 でここで直したので申し送りから外す。81 の残り（`examples/emo-present/balloon.rs` の `read_balloon_offset`）は残る。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 965 件が通り 1 件 ignore、手本 2 件が通る。
- `cargo test -p areka`: 本体の bin 3180 件が通り 6 件が ignore、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る。
- `cargo fmt --all -- --check`: 緑。`cargo test -p log-capture-kit --test file_length_guard_test`: 6 件が通る（`persist.rs` 977 行・`lib.rs` 926 行・`persist_restore_tests.rs` 838 行・`measure_tests.rs` 836 行）。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。
- 直しは 6 つの組ごとに分けて同時に進めた。1 度、壊し方を戻すのに `git checkout` で `chain_finalize.rs` を `HEAD` へ戻した事故があったが、その時点でそのファイルに入っていた直しは無く、失った物は無い（後の直しはその後に入れた）。

#### 判定

2 回目の新しい指摘は 88 件（必ず直す 0・直したほうがよい 8・できれば 80）で、0 件ではない。直した 87 件・裁定待ち 1 件（210）。3 回目をかける（要件 6.2）。210 の裁定が出たら、その直し（または残す理由）を 3 回目の前に入れる。210 は 3 回目の前に、開発者の裁定で今のまま残すと決まった（18.3 の裁定の表。コードは変えず、`t_r5_mixed_edges_chain_per_edge` もそのまま）。

### 18.3 3 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `246b6d75f`）。
- 範囲は 1.1 節と 18.1・18.2 と同じ数え方で測り直した。ファイルの数は 2 回目と同じ 116。行は 2 回目の直しで減った。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 27 | 10,511 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,400 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,058 |
| `Cargo.toml` | 1 | 52 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 347 |
| 計 | 116 | 36,368 |

- ponytail は 5.3.3（1 回目・2 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,479・B 20／6,701・C 20／5,969・D 17／5,404・E 15／5,951・F 20／5,864。計 36,368 行で上の表と合う）。読む段では何も変えず、指摘を集めて番号を付けてから、組ごとに直させた。
- 2 回目のレビューが残した候補（`doc/COMPAT_ARCHITECTURE.md` の `spawn.rs:624-625`・`windowposition.rs:85`）も同じ一覧に入れた（344・297）。
- 直しの段で、A の組が `cargo test -p log-capture-kit --test with_default_guard_test` が `HEAD` の時点で赤であることを見つけた（2 回目の 256 の直しが作った赤。2 回目は `log-capture-kit` の行数の門しか回していなかった）。380 として足した。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,479 | 1 | 0 | 12 | 13 |
| B | 20 | 6,701 | 0 | 0 | 16 | 16 |
| C | 20 | 5,969 | 0 | 0 | 18 | 18 |
| D | 17 | 5,404 | 1 | 0 | 15 | 16 |
| E | 15 | 5,951 | 0 | 0 | 10 | 10 |
| F | 20 | 5,864 | 0 | 1 | 10 | 11 |
| 2 回目の直しが作った赤（`log-capture-kit` の見張り） | — | — | 1 | 0 | 0 | 1 |
| 計 | 116 | 36,368 | 3 | 1 | 81 | **85** |

- 2 つの組が同じ所を出したものは 1 件に数えた（325 は C と D の組が出した）。直す組は、触るファイルが重ならないように振り直した（307 は F の組・328 と 301 は A の組・322 と 339・340・342 は C の組・297・323・341・344 は進行役）。

#### 指摘の一覧（296〜380）

扱いの印は 18.1・18.2 と同じ。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 296 | 必ず直す | A | `alignmenttodesktop` の大文字と小文字の扱いが、同じ鍵で割れる。descript の `bottom`／`free` は小文字の完全一致だけで、`top`／`left`／`right` は大小を問わない（`Anchor::from_alignment` が小文字にする）。台本の `\![set,alignmenttodesktop,Free]` も大小を問わない。たとえばシェルの `seriko.alignmenttodesktop,Free` は `Bottom` になり、上下に動かせなくなって「未知のアンカー」の警告が出る。`Bottom` は動きは合うが、解くたびに偽の警告が出る | `config.rs` の `resolve_scope`・`resolver.rs` の `from_alignment` と `Seam` の警告・`script_alignment.rs` の doc・`resolver_from_alignment_tests.rs` の「parsers 側で正規化済み」 | 直した（5 回目に開発者の裁定。18.5） |
| 297 | できれば | A | 行の番号が古い（「小文字の完全一致」の決まりは 85 行でなく `classify_x_vocab` の中） | `doc/COMPAT_ARCHITECTURE.md` の重なりの語の大小文字の行 | 直した（`windowposition.rs::classify_x_vocab` に） |
| 298 | できれば | A | 「本仕様」が、どの spec かを言わない（`windowposition.rs` の中では 2 つの spec を指す） | `lib.rs`・`resolver.rs`・`windowposition.rs`・`placement_windowposition_tests.rs`・`placement_windowposition_vocab_tests.rs`・`windowposition_vocab_tests.rs` | 直した（行ごとに指す spec の正式の名か、指す物の名に） |
| 299 | できれば | A | 「記憶 …」が、リポジトリに無い開発者の記憶の帳面を名指す | `lib.rs`・`placement_prepare_tests.rs` | 直した（決まりをその場の文で書く） |
| 300 | できれば | A | `temp-path-kit` の行の「同 spec」が、すぐ上に足された別の spec（`areka-P0-nar-install`）を指してしまう | `Cargo.toml` | 直した（`spec: areka-P0-test-cage-determinism`。依存は変えない） |
| 301 | できれば | A | 256 の直しで `log-capture-kit` の説明の「有効化するのは wintf のみ」が実際と合わなくなった | `log-capture-kit` の `Cargo.toml`・`src/lib.rs` | 直した（380 で 256 の機能を戻したので、説明は元の文のまま正しくなった。A の組がいったん書き換えた 2 行は元に戻した） |
| 302 | できれば | A | 作者 DPI と重なりの指定を運ぶ道を `main` → `wire_runtime` と書く（今は本体の `ghost_session` が読む） | `lib.rs` の 2 か所 | 直した |
| 303 | できれば | A | `Anchor` の doc が「wintf 非依存で走る」と書く（`resolver.rs` は `wintf::ecs::DPI` を引く） | `resolver.rs` | 直した（型そのものが wintf の型を持たない、と書く） |
| 304 | できれば | A | 表の説明が 4 つの欄を挙げるが、組は 3 つ | `resolver_balloon_keyword_tests.rs` | 直した |
| 305 | できれば | A | 左右の寄与を数で確かめた直後に、左右が等しいことを確かめる（決して落ちない） | `placement_windowposition_tests.rs` | 直した（消して、文を残る確かめへ移した） |
| 306 | できれば | A | 既に確かめてある準備を、COM と WIC でもう 1 回まるごと回すテストと確かめの塊（主モニタの DPI が無いとき・等倍の寸法） | `placement_prepare_tests.rs` の `prepare_emo2_without_primary_dpi_still_measures` と `prepare_emo2_scales_window_sizes_by_k0` の `native` の塊 | 直した（消した。確かめは `build_measure_scaling_degrades_to_96_equivalent_when_primary_dpi_unobtainable`・`prepare_stamps_measure_dpi_onto_balloon_offset_base`・`prepare_emo2_returns_two_scope_placements` にある。準備が 2 回減る） |
| 307 | できれば | A | テストの小道具 `DPIS` と作業領域の矩形が、resolver の支えとバルーンの画面の内のテストに写してある | `resolver_test_support.rs`・`resolver_resolve_test_support.rs`・`balloon_limit_tests.rs`・`balloon_limit_gate_tests.rs` | 直した（`resolver_test_support.rs` の `DPIS`・`offset_work_area` の 1 組。値が同じことを確かめてから寄せた） |
| 308 | できれば | A | 解放の時の関門の持ち主を、テストのファイルの名で書く | `resolver_resolve_balloon_tests.rs` | 直した（`follow::drag_follow::on_balloon_drag_end`） |
| 309 | できれば | B | 完全一致の直後で決して落ちない確かめ 3 つ（232 の残り） | `persist_restore_tests.rs` | 直した（消した） |
| 310 | できれば | B | 引き算で作る `balloon_offset ≡ balloon_pos − char_pos` の確かめ（211 と同じ形・211 の一覧に無かった） | `balloon_offset_supply_tests.rs` | 直した（消した） |
| 311 | できれば | B | 同じファイルに同じ値の定数が 2 つ（`SZ`・`CSZ`） | `persist_restore_tests.rs` | 直した（`CSZ` の 1 つ） |
| 312 | できれば | B | 保存の窓の位置を読む補助が 2 つ | `persist_finalize_save_tests.rs` の `window_pos`・`reset_pos_tests.rs` の `saved` | 直した（`window_pos` の 1 つ） |
| 313 | できれば | B | 10 の欄の `ScopePlacement` を手で書く写しが 4 か所（共有の `placement_sized` がある） | `persist_adjacency_tests.rs`・`persist_finalize_save_tests.rs`・`persist_restore_group_tests.rs`・`reset_pos_tests.rs` | 直した（`placement_sized` に `anchor` だけを足す形） |
| 314 | できれば | B | 同じ合成のゴーストで COM と emo2 の準備を 2 回ずつ回し、2 本目は同じ出力をもう 1 度見るだけのテストが 2 組 | `balloon_offset_supply_tests.rs` | 直した（各組を 1 本に。どちらの確かめも全部残した。準備が 2 回減る） |
| 315 | できれば | B | 「スコープ 0 には前の窓が無い」の決まりを 2 度確かめる | `persist.rs` の `with_adjacency_marks` | 直した（呼び手の側の絞りを消した。`push_adjacency_marks` の確かめが残る） |
| 316 | できれば | B | `load_restored_state` の doc が「起動時」だけと書く（シェルの切り替えの資産づくりでも呼ばれる） | `persist.rs` | 直した |
| 317 | できれば | B | 欄の doc が「読取失敗は致命」と書く（228 の残り） | `source.rs` の `DescriptSource::shell_kv` | 直した |
| 318 | できれば | B | シェル位置初期化の頭が「連鎖の確定と同じ 3 手」と書くが、絵の移動の基準の差し直しと台本の移動の印も付ける | `reset_pos.rs` | 直した |
| 319 | できれば | B | 「本仕様」「本タスク」が、どの spec かを言わない | `balloon_offset_persist_roundtrip_tests.rs`・`balloon_offset_supply_tests.rs`・`persist_restore_offset_base_tests.rs` | 直した |
| 320 | できれば | B | 経緯の説明（2026-08-28 までどの檻も固定していなかった） | `balloon_offset_persist_roundtrip_tests.rs` | 直した（消した） |
| 321 | できれば | B | `unique_temp_dir` の使い手の一覧が 1 つ足りない（240 の直しで増えた） | `source_tests.rs` | 直した |
| 322 | できれば | B | 行の番号が古い（`persist.rs:309` は `warn!` の欄の行。`warn!` は 307 行。1 回目の前から古かった） | `doc/ukadoc-coverage/briefing-assets.md` | 直した（`:307`） |
| 323 | できれば | B | 行の番号が古い（`persist.rs::merge_scope`：564-570） | `doc/COMPAT_ARCHITECTURE.md` | 直した（番号をやめて名前だけに） |
| 324 | できれば | B | 読めないゴーストの descript の `warn!` だけが「保存が無い」と見分ける手がかりなのに、テストが記録を見ない | `persist.rs` の `load_restored_state`・`persist_io_wiring_tests.rs` | 直した（`warn!` が 1 行で `ghost_root` を持つことを確かめる。`warn!` を消す壊し方で赤: 「読めないゴーストの warn が 1 行: [] left: 0 right: 1」） |
| 325 | できれば | C | 手本の説明と画面の案内が「ダブルクリックで終える」と書くが、その手当て（`on_ghost_pressed`）はもう無く、ダブルクリックでは何も起きない | `examples/window-placement.rs` | 直した（終えるのは Ctrl+C と煙の門、と書く） |
| 326 | できれば | C | 手本が絵の作者 DPI に決め打ちの 96 を渡し、測りの読んだ値（`prepared.author_dpi`）を使わない。`seriko.dpi,192` のゴーストでは窓と絵の大きさが 2 倍食い違う | `examples/window-placement.rs` の `attach_target` の呼び出し | 直した（`PendingTarget` に `author_dpi` を足し、キャラはシェルの値・バルーンはバルーンの値を渡す。RED は取れない: 値を使うのは GPU の資源の要る `boot_present_system` だけで、GPU の無い継ぎ目が無い。emo2 はどちらも 96 で振る舞いは変わらない） |
| 327 | できれば | C | 要らない `Vec` を作って回る | `examples/window-placement.rs` | 直した |
| 328 | できれば | C | 手本のテストの説明が、もう無いテスト（代わりの面）を挙げる | `Cargo.toml` の `[[example]]` | 直した（「絵の選び・u32 に収まらないスコープ」） |
| 329 | できれば | C | 測りの失敗が必ず起動の終わりになると書く（ゴーストの切り替えでは切り替えの失敗。228・92 の残り） | `measure.rs` の 3 か所・`measure_tests.rs` | 直した |
| 330 | できれば | C | 「本 spec」「本仕様」が、どの spec かを言わない | `descript_window_keys.rs`・`measure.rs`・`measure_tests.rs` | 直した |
| 331 | できれば | C | 既定の面のテストの数が「3 通り」（245 で 4 通り） | `measure.rs` | 直した |
| 332 | できれば | C | 関数の doc が型の別名の上にある | `measure_tests.rs` | 直した |
| 333 | できれば | C | 経緯の注記 5 つと、自分を呼ぶ関数を名指す堂々めぐりの doc | `measure.rs` | 直した（消した） |
| 334 | できれば | C | 完全一致の直後で決して落ちない `assert_ne!` | `measure_tests.rs` の 3 本 | 直した（消した） |
| 335 | できれば | C | 同じシェルを組み、同じ置き換えを確かめるテストが 2 本 | `measure_tests.rs` の `failed_scope_substitutes_scope0_size` | 直した（消した。確かめは `no_candidate_in_shell_leaves_it_to_the_compose_failure` にある） |
| 336 | できれば | C | 「本番の測りが通る道そのもの」と書くが、1 枚目だけを読む包み（`FirstFrameDecoder`）を通していない | `measure_template_tests.rs` | 直した（本番と同じ包みを通す。期待の値は同じ） |
| 337 | できれば | C | シェルの `balloon.syncscale` で `u32` に収まらないスコープを捨てる枝にテストが無い（`as u32` にするとスコープ 0 の同期が点く） | `descript_window_keys.rs` の `read_balloon_syncscale` | 直した（`shell_syncscale_ignores_scope_beyond_u32`。`as u32` にする壊し方で赤: `shell_scopes: {0}`） |
| 338 | できれば | C | 合成の検体の置き場の説明が誤る | `test_support.rs` | 直した |
| 339 | できれば | C | 行の番号が古い（2 回目の 295 が 1 行長く書いた範囲と、この回の `measure.rs` の直しで動いた行） | 本体の `examples/collision-probe.rs` | 直した（`measure.rs:379-390`・`:417`） |
| 340 | できれば | C | 行の番号が古い（`measure.rs:399`） | `doc/ukadoc-coverage/briefing-assets.md` | 直した（`:384`。同じ呼び出しを指す 227 行の参照もこの回の直しで動いたので直した） |
| 341 | できれば | C | `COMPAT_ARCHITECTURE.md` の配置の crate を指す行の番号が残る（`config.rs:255`・同じ行の `resolver.rs:1-12`） | `doc/COMPAT_ARCHITECTURE.md` | 直した（`config.rs::resolve_scope`・`resolver.rs` のモジュールの doc に） |
| 342 | できれば | C | ゴーストとシェルの descript の読み手の一覧が足りない（既定の位置・スコープの見つけ方・`seriko.defaultsurface`・`balloon.dontmove`・`balloon.syncscale`）のに「それ以外の名前はどこからも引かれない」と書く | `doc/ukadoc-coverage/briefing-assets.md` | 直した（今の行の番号で足した） |
| 343 | 必ず直す | D | バルーンの画面の内の指定（`BalloonLimit`）を書くのは起動の窓づくりだけで、バルーンの切り替え・面の切り替え・シェルの切り替え・バルーン位置初期化は置き直さない。たとえば `windowposition.limit` が既定の 1 のバルーンで起動し、`windowposition.limit,0` のバルーンへ切り替えても、解放と書き込みの関門がバルーンを画面の内へ引き戻し続ける（次の起動まで） | `spawn.rs` の `BalloonLimit`・`reseed.rs` の `reinstall_balloon`・`face_windowposition.rs` | 直した（5 回目に開発者の裁定。18.5） |
| 344 | できれば | D | 行の番号が古い（`spawn.rs:624-625`） | `doc/COMPAT_ARCHITECTURE.md` | 直した（`spawn.rs::window_style`） |
| 345 | できれば | D | 「本 spec の共通規約」 | `diag.rs` | 直した（`areka-P0-dpi-window-vanish`） |
| 346 | できれば | D | 「本フィーチャー」が、どの機能かを言わない | `spawn_zorder_pair_deferred_tests.rs` | 直した（重なり順の 2 つの機構と、その spec の名に。要件の文の引用は引用と分かる形で残した） |
| 347 | できれば | D | 同上 | 本体の `placement_tests/spawn_zorder_pair_deferred_tests.rs` | 直した |
| 348 | できれば | D | 「本タスクの受け入れ条件」 | `spawn_zorder_pair_wiring_tests.rs` | 直した（`areka-P0-ghost-window-zorder 要件 5.6`） |
| 349 | できれば | D | もうどこにも無い名の不在を確かめる（決して落ちない） | `spawn_zorder_chain_wiring_tests.rs` | 直した（消した） |
| 350 | できれば | D | 結線しない対照が空の段を回して印が立ったままを確かめる（決して落ちない）一方、結線の直後に印が立っている前提を確かめていない | `spawn_zorder_chain_wiring_tests.rs` | 直した（前提を確かめる形に。結線が受け口を `default()` で入れ直し、鎖の適用を外す壊し方で赤: 「前提: 結線の後、回す前は受け口の印が立っている」） |
| 351 | できれば | D | 結線しない半分が空の段を回すだけ（決して落ちない） | `spawn_window_band_wiring_tests.rs` | 直した（消した。前提の確かめは残る） |
| 352 | できれば | D | 前の確かめから導ける `assert_ne!` 2 つ（258 の残り） | 本体の `spawn_assembly_tests.rs` | 直した（消した） |
| 353 | できれば | D | 完全一致の後で導ける確かめ 4 つと、それだけが使う補助 | `chain_finalize_tests.rs` | 直した（消した。文は残る確かめの文へ移した） |
| 354 | できれば | D | 「既定値を保つ」と書いて wintf の既定の数 5 を決め打ちで比べる | 本体の `spawn_assembly_tests.rs` | 直した（`DragConfig::default().threshold` と比べる） |
| 355 | できれば | D | バルーンの窓にはマウスの手当てが無い、と書く（本体の入力の層が窓づくりの直後に付ける） | `spawn.rs` の 2 か所・本体の `spawn_assembly_tests.rs` | 直した |
| 356 | できれば | D | `DefaultPlacementBasis` を読むのは並べ直しだけ、と書く（バルーン位置初期化・面の windowposition・`balloon.syncscale` の読み直しも読む） | `spawn.rs` | 直した（読み手を並べた。同じ文の画面の内の指定の半分は 343 の裁定まで残した。5 回目に 343 で直した） |
| 357 | できれば | D | 開発者の記憶の帳面の名を名指す | `spawn_cleanup_tests.rs`・本体の `spawn_assembly_tests.rs` | 直した |
| 358 | できれば | D | ドラッグの知らせと窓の位置を読む補助が、追随の支えの写し | `spawn_follow_pipeline_tests.rs`・追随の `follow_test_support.rs` | 申し送り（追随の支えを `pub(crate)` に開いて使う。開く所が追随の持ち物） |
| 359 | できれば | E | 決して落ちない確かめ 2 つ（1 つは `composed()` が既に確かめる） | `zorder_chain_compose_tests.rs` の `t_zcc16` | 直した（消した） |
| 360 | できれば | E | 届かない枝（`GetTopWindow`／`GetWindow` は空の窓で `Err` を返す） | `zorder_test_support.rs` の `scan_z_once` | 直した（消した） |
| 361 | できれば | E | 「本番の確定段と同じ順」と書くが、帯の適用系を除いている（141 と同じ形） | `zorder_chain_inversion_order_tests.rs` | 直した |
| 362 | できれば | E | 「昇順を本当に確かめるのは 1 本だけ」が古い | `zorder_chain_tail_order_tests.rs` | 直した |
| 363 | できれば | E | 覆された寄せ（かたまりを後ろへ）を前提にした説明が 2 か所に残る（276 の残り） | `zorder_group_ledger_tests.rs` | 直した |
| 364 | できれば | E | 別の spec の要件 1.3 を受け持つと書く（受け持つのは完全形と省略形の同義） | `zorder_group_ledger_written_order_tests.rs` | 直した |
| 365 | できれば | E | 版の数の doc が「射影を組み直すかの判定に使う」と書き、すぐ下の段落と食い違う | `zorder_group_ledger.rs` | 直した |
| 366 | できれば | E | 文の行き先が抜けて読めない | `zorder_group_ledger.rs` のモジュールの doc | 直した |
| 367 | できれば | E | 片づけを共有の `teardown` と同じ形で手で書き直している | `zorder_chain_inversion_order_tests.rs` の `Drop` | 直した（`teardown` を呼ぶ。壊す窓と順は同じ） |
| 368 | できれば | E | ほかの確かめが先に必ず落ちる確かめ 2 つ | `zorder_property_deferral_tests.rs` の `t_zpd10`・`t_zpd11` | 直した（消した） |
| 369 | 直したほうがよい | F | 291 の直し（作業領域と DPI の組ごとに 1 度だけ解く）の DPI の半分に、壊れたら落ちるテストが無い。鍵から DPI を外すと、DPI の移り変わりの途中で窓の DPI が違うスコープが別の DPI で解かれる | `reseed.rs` の `resolve_standard_scopes` | 直した（`t_rs03b_scopes_with_different_window_dpis_are_solved_with_their_own_dpi`。鍵から DPI を外す壊し方で赤: スコープ 1 の基準が `Some(DPI 96)`） |
| 370 | できれば | F | 起動の関門の記録の札 `boot-gate` を、どのテストも綴りで固定していない（実機の確かめの手順はその綴りで grep する） | `balloon_limit_tests.rs` の T-BL9 | 直した（綴りを固定。定数を `boot` にする壊し方で赤） |
| 371 | できれば | F | 引き算で作る決して落ちない確かめ | `balloon_limit_gate_tests.rs` | 直した（消した。文は残る確かめの上へ） |
| 372 | できれば | F | 同じテストの中で補助がもう確かめる欄を繰り返す | `balloon_limit_gate_tests.rs` | 直した（消した） |
| 373 | できれば | F | 保存のずらしの組み立てが、公開の `persist::balloon_offset_entries` の写し | `reseed_tests.rs` | 直した（共有のものを使う） |
| 374 | できれば | F | テストの手元の帳簿を確かめる（借りるだけなので変わらない） | `reseed_tests.rs` の t_rs01 | 直した（消した。資源の側の確かめが残る） |
| 375 | できれば | F | spec の名の無い裁定の番号（241 と同じ形） | `sticky_group_ledger.rs` | 直した（`areka-P0-window-link-directives の裁定 7`） |
| 376 | できれば | F | spec の名の無い設計の討議の議題の番号 | `dpi_sync.rs` | 直した（`areka-P0-dpi-transition-atomicity の設計討議 議題 1`） |
| 377 | できれば | F | 差し替えたテストの経緯の説明 | `dpi_sync_tests.rs` | 直した（今の理由だけを残した） |
| 378 | できれば | F | `PatternMove` を足した後も、見送った書き込みの持ち越しと `PatternMove` の使い手の一覧が古い（145 の残り） | `dpi_sync.rs` | 直した |
| 379 | できれば | F | バルーンの画面の内の式が、キャラの窓の収めでも使われることを頭が書かない | `balloon_limit.rs` | 直した |
| 380 | 必ず直す | — | 2 回目の 256 の直しで `areka-placement` が `log-capture-kit` の `env-filter` の機能を有効にしたため、見張り `with_default_guard_test` の 2 本（`only_one_crate_declares_the_env_filter_feature`・`every_direct_call_exception_still_has_a_real_hit`）が赤。全体テストが赤になる。見張りは、完了した `areka-P0-test-cage-determinism` の「有効にするのは wintf のみ」を守る唯一の担保 | `crates/areka-placement/Cargo.toml`・`diag_tests.rs` | 直した（裁定: 推奨案で承認。機能を戻し、`diag_tests.rs` の `emit_all_under_filter` を手で持つ形に戻したうえで、256 が求めた番兵を足した: 窓の内側で番兵の宛先へ 1 行出し、それが捕まっていなければ panic。RED: 直す前は見張りの 2 本が赤（A の組が `HEAD` で確かめた）。番兵の行を消す壊し方で、記録の水準のテスト 3 本が「番兵が捕まらない」で赤。見張りは直した後に 24 件が通る） |

- 申し送りとの対応（要件 1.6）: 1〜295 と同じ所を指す指摘は 0 件。前の回の直しの残り（同じ形で、前の回の一覧に無かった所）は新しく数え、どの番号の残りかを一言に書いた（305・309・310・317・329・352・361・363・378 ほか）。380 は 256 の直しが作った赤で、256 の裁定をこの回の裁定で置き換えた（下の表）。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を指す `<ファイル>.rs:<行>` の形を、ツリー全体（`.kiro/specs/completed/`・`vendors/`・この spec の記録・`.kiro/steering/roadmap-history.md` を除く）から拾い、`git show HEAD:` の行と今の行を 1 つずつ見比べた。動いたのは 339・340 で直した `measure.rs` の 5 つだけで、どれも今の行を指す。

#### 指摘にしなかったもの

- 頭の `areka-P0-` を落とした spec の名（`windowposition-limit`・`scope-chain-gap`・`window-placement`・`ghost-window-zorder` ほか、範囲に 40 か所ほど）。どれも実在する spec の名の一部で、どの spec かが一意に読める（`scg`・`atom` の 218 とは違う）。1 回目・2 回目も数えなかった線を保つ（A の組が出し、D・F の組は外した。外す側を採った）。
- `.kiro/specs/zorder-chain-residue/brief.md:135` の `spawn.rs:709`・`balloon-flip-side-rules`／`shell-balloon-scale-setting` の brief と `roadmap.md` の古い `placement/…` の道と行。日付つきの起票の写しで、その spec が着手のときに測り直す（18.1 と同じ扱い）。
- 重なり順の台帳のテストのうち、ほかのファイルのテストと同じことを言うもの（`t_zgp2`・`t_zgp4`・`t_zgp5`・`t_zwo10` ほか）。分岐の対応表の錨なので、33 の決まりで残す。
- `PlacementRoute::ALL` と `Display` がテストだけに使われること。公開の形を狭め（要件 8.2）、追随のテストに触る割に得る物が小さい。
- `MeasureScaling::IDENTITY` の使い手がテストだけであること（215・246 と同じ理由）。
- `config.rs` の `bottom`／`free` の大小文字は、C の組が「`windowposition` と同じ小文字の完全一致」として外したが、同じ鍵の `top`／`left`／`right` と台本が大小を問わないので、A の組の見立てを採って 296 に数えた。
- `StickyGroupLedger::reset_to_descript` の戻り値をテストだけが読む・`reseed` の `monitor_snapshot` を 1 回の切り替えで 2 回呼ぶ・届かない `char_size_unknown` の枝・`face_windowposition` のスコープが解けない枝（呼び手は戻り値を見ない）・`persist_unremembered` の届かない飛ばし・本番の `error!` を確かめないテスト（`Err` が返るので失敗は見える）・`build_titles` で `char02` と `char2` の両方があるときの決まった勝ち方（正典が沈黙）。どれも具体的な害の場合が無い。
- 公開の doc から私有の物への rustdoc のリンク・`diag.rs` の `tracing_subscriber::EnvFilter` へのリンク。ツリーに `cargo doc` の門が無く、壊れた振る舞いが無い。
- 組から上がって、確かめて外したもの: 記録の書式の組み立て（`DIAG_TARGET` で debug のときだけ組まれる）・`remove_entry_of` の線形の探し（スコープは数個）・`spawn_follow_pipeline_tests` の `balloon_pos − char_pos`（窓づくりの写しを確かめるので 211 の形ではない）・`with_com_initialized` が `f` の panic で `CoUninitialize` を飛ばすこと（テストの糸がそのまま終わる）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 210 | 要件 5 の 3 種類目（利用者に見える位置） | 2 回目の指摘。右（左）の辺のスコープと下端の連なりのスコープが、右下（左下）の角でまるごと重なる | SSP の振る舞いを MCP で測ってから決める | 開発者の裁定で今のまま（SSP も重なりを離さない）。コードは変えず、`t_r5_mixed_edges_chain_per_edge` もそのまま。測り（2026-11-15・進行役が SSP の MCP で、ゴースト「えも2DEBUG」・幅 2560 のモニタ・右の端が x=0）: 前はさくら（下端）-2224,1248,-1842,1795・けろ（下端）-2563,1395,-2227,1795。`\1\![set,alignmenttodesktop,right]` の後、けろは -336,1395,0,1795（x は右へ寄り、y は下端のまま）。さくらを右下の角へ動かした後、さくら -382,1248,0,1795・けろは変わらず -336,1395,0,1795。SSP は 2 つの窓をまるごと重ねたまま、どちらも動かさない。測った後に両方を元の矩形へ戻した。限り: 測ったのは動いている間の揃え直しの道で、本当の初めての起動ではない（ghost_dev のシェルの descript を書き換え、SSP の保存の位置を消す必要があり、していない） |
| 296 | 要件 5 の 3 種類目（キャラの窓の出る位置）。ukadoc https://ukatech.shillest.net/ukadoc-spec/descript_ghost.html#seriko.alignmenttodesktop は「全体のサーフェスのデフォルト※表示位置情報。」と既定値 `bottom` を書くだけで、大小文字に沈黙する。SSP の扱いは測っていない | descript の `alignmenttodesktop` の大小文字を、どちらの決まりにそろえるか | **SSP の振る舞いを MCP で測ってから決める**。SSP が大小を問わないなら、`config.rs` で値を小文字にしてから `bottom`／`free` と比べ、descript と台本を 1 つの決まりにする（RED を先に: `Free` が `Alignment::Free`・`Bottom` が警告なしの `Alignment::Bottom`）。あわせて `resolver.rs`・`resolver_from_alignment_tests.rs` の「parsers 側で正規化済み」の文を直す。小文字の完全一致にそろえるなら、`top`／`left`／`right` と台本の扱いも変わる。測るまでは今のまま | 裁定待ち（利用者に見える位置が変わるので、推奨案で承認にしない）。5 回目のあいだに開発者の裁定（2026-11-15・推奨どおり）が届き、直した（18.5） |
| 343 | 要件 5 の 1 種類目（完了した `areka-P0-windowposition-limit` の設計 C9／DD4「spawn 焼込み・以後不変」）と 3 種類目（バルーンの見え方）。ukadoc https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.limit 「バルーンを強制的に画面内に維持するかどうか。1で維持、0で維持しない」 | バルーン・面・シェルの切り替えとバルーン位置初期化で、`BalloonLimit` を置き直すか | **直す**: `reinstall_balloon` が `BalloonLimit(placement.balloon_limit)` も入れ直す（呼び手 3 つは `ScopePlacement` を既に持つ）。`apply_face_windowposition` は、利用者が動かしたバルーンを置き直さないときも値だけは入れ直す。`spawn.rs` の「以後不変」の文も直す。RED を先に（面の windowposition で limit を `0` にした後、バルーンが `BalloonLimit(false)` を持つ） | 裁定待ち（完了した spec の設計に反し、利用者に見える振る舞いが変わるので、推奨案で承認にしない）。5 回目のあいだに開発者の裁定（2026-11-15・推奨どおり）が届き、直した（18.5） |
| 380 | 要件 5 の 1 種類目（完了した `areka-P0-test-cage-determinism` の「`env-filter` を有効にするのは wintf のみ」と、それを守る見張り）。2 回目の 256 の裁定（推奨案で承認）はこの決まりを見落としていた | 見張りを広げて 256 を保つか、256 の機能を戻すか | **256 の機能を戻す**: 見張りと完了した spec の決まりはそのまま。256 が求めた「何も捕まえなくても緑」の穴は、`diag_tests.rs` の手の写しに番兵（窓の内側の 1 行と、それが無ければ panic）を足して塞ぐ。依存の表は 2 回目の前と同じ形に戻り、`Cargo.lock` は変わらない。小さく、利用者に見える振る舞いも変わらない | 推奨案で承認。直した。256 の行の「推奨案で承認。直した」は、この裁定で置き換わった |

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（持ち主の spec はもう完了しているので、8.2 が扱いを決める）:
  - 358（`follow_test_support.rs:37`・`:45` の `position_of`・`drag_event` を `pub(crate)` に開き、`spawn_follow_pipeline_tests.rs` の写しを消す）。
  - `follow_drag_tests.rs:144` の開発者の記憶の帳面の名（357 と同じ形）・`follow/move_shift.rs:118` の名前の無い「監査の指摘 1」（18.1 のまとめの申し送りに入る形）。
  - 2 回目の申し送りの直し: `follow_transition_diag_tests.rs`・`follow_window_move_diag_tests.rs` の記録の捕まえの手の写しは、`capture_under_filter` を使うのではなく（`env-filter` は wintf だけ・380）、`diag_tests.rs` と同じく番兵を足す。
- `audit-areka-app` へ:
  - `runtime/frame_transition_branch_tests.rs:527-546` が「4 つの見送り点」と書き、5 つ目の `HoldSite::PatternMove`（`runtime/frame/move_shift.rs:71`）の呼び出しの綴りを固定していない（145・378 と同じ形）。同じ所に名前の無い「task 6.5」。
  - `main.rs:939-940`・`ghost_session.rs:241` が、記憶からの戻しの継ぎ目を `load_restored_state` → `apply_restored_placements` と書く（今は `restore_and_judge` → `apply_balloon_limit`。227 の本体の側）。
  - `runtime/assets_tests.rs:774`（`synth_declared_dpi_ghost` は `test_support.rs` にある）・`:783`（`measure_tests.rs` に 1,000 行の余裕が無い、は誤り。今は 789 行）・`:779`／`:788`（名前の無い「タスク 4.2」）。
  - 開発者の記憶の帳面の名: `input_events/choice_drain.rs:11`・`input_events_tests.rs:255`・`boot_config.rs:504`・`smoke_exit.rs:43`。
  - `runtime/move_cue_apply_move_tests.rs:52` の `attach_fake_handles` が `spawn_follow_pipeline_tests.rs` の写し。
- `audit-test-kits` へ: `crates/log-capture-kit/src/lib.rs:28` の「消費する 19 crate」（`log-capture-kit` を引く `Cargo.toml` は 20）。
- 持ち主の見直しの spec が無いもの（記録だけ）: 開発者の記憶の帳面の名が `areka-kanade/src/actor_tests.rs:410`・`areka-kanade/tests/kanade/resource_query_test.rs:14`（`audit-kanade` は完了済み）・`dola/src/cue/command_tests.rs:191`（dola の見直しの spec は無い）にもある。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 963 件が通り 1 件 ignore、手本 2 件が通る。
- `cargo test -p areka`: 本体の bin 3180 件が通り 6 件が ignore、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る。
- `cargo test -p log-capture-kit --test with_default_guard_test`: 24 件が通る（直す前は 2 件が赤＝380）。`cargo test -p log-capture-kit --test file_length_guard_test`: 6 件が通る（`persist.rs` 976 行・`lib.rs` 926 行・`persist_restore_tests.rs` 832 行・`measure_tests.rs` 789 行）。
- `cargo fmt --all -- --check`: 緑。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。
- 直しは 6 つの組ごとに分けて同時に進めた。壊し方は狙った編集で入れて狙った編集で戻し、`git checkout`・`restore`・`stash`・`reset` は使っていない。

#### 判定

3 回目の新しい指摘は 85 件（必ず直す 3・直したほうがよい 1・できれば 81）で、0 件ではない（210 は数に入れていない）。直した 82 件・裁定待ち 2 件（296・343）・申し送りだけのもの 1 件（358）。4 回目をかける（要件 6.2）。296 と 343 の裁定が出たら、その直し（または残す理由）を 4 回目の前に入れる。2 つの裁定は 5 回目のあいだに届き、5 回目に直した（18.5）。

### 18.4 4 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `246b6d75f`）。
- 範囲は 1.1 節と 18.1〜18.3 と同じ数え方で測り直した。ファイルの数は 3 回目と同じ 116。行は 3 回目の直しで減った。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 27 | 10,513 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,254 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,055 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 347 |
| 計 | 116 | 36,220 |

- ponytail は 5.3.3（1〜3 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,452・B 20／6,660・C 20／5,926・D 17／5,402・E 15／5,912・F 20／5,868。計 36,220 行で上の表と合う）。読む段では何も変えず、指摘を集めて番号を付けてから、触るファイルが重ならないように組ごとに直させた。
- この回は、3 回目までに数えなかった線（各回の「指摘にしなかったもの」）を読み手に渡し、古い文は「今の事実と食い違うもの」だけ、決して落ちない確かめは「どの直しでも赤にならない理由を言えるもの」だけを指摘にした。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,452 | 0 | 0 | 10 | 10 |
| B | 20 | 6,660 | 1 | 0 | 7 | 8 |
| C | 20 | 5,926 | 0 | 0 | 6 | 6 |
| D | 17 | 5,402 | 0 | 0 | 9 | 9 |
| E | 15 | 5,912 | 0 | 0 | 7 | 7 |
| F | 20 | 5,868 | 0 | 1 | 5 | 6 |
| 計 | 116 | 36,220 | 1 | 1 | 44 | **46** |

- 2 つの組が同じ所を出したものは無い。同じ形を別の所に出したもの（383 と 406 の `apply_restored_placements` の名・386 と 408 の関門や読み手の一覧）は、所ごとに数えた（1〜3 回目と同じ）。

#### 指摘の一覧（381〜426）

扱いの印は 18.1〜18.3 と同じ。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 381 | できれば | A | 準備の手順を「読み込み → 設定 → 測り → モニタの列挙 → 解決」と書くが、モニタの列挙と主モニタの選びは読み込みより前（測りが主モニタの DPI を要る）。作業領域の取り出しと 0 台の `Err` だけが測りの後 | `lib.rs` の `prepare_ghost_windows` の doc・`placement_prepare_tests.rs` の `prepare_missing_root_returns_mount_err_to_caller` の doc | 直した |
| 382 | できれば | A | 主モニタの DPI が取れないときの縮退を「いずれにせよ窓は生え、表示は失われない」と書き、`error!` も同じ文を出す。本番で `None` になるのはモニタが 0 台のときだけで、直後の `work_area_of` が `Err(Monitor)` を返し窓は出ない | `lib.rs` の `build_measure_scaling` の doc と `error!` の文 | 直した（「窓が生える」を DPI が 0 のときに限り、0 台なら後で失敗すると書いた。記録の文を字面で読む所が無いことをツリー全体の grep で確かめた） |
| 383 | できれば | A | 本体が呼ぶ記憶からの戻しを `apply_restored_placements` と書く（今は `restore_and_judge`・227 の残り） | `placement_prepare_tests.rs` の `prepare_never_reads_or_writes_the_saved_state` の doc | 直した |
| 384 | できれば | A | `ScopeInput` の寸法を「原寸」と書くが、測りの k₀ 倍の後の寸法（主モニタ 120 DPI で emo2 のスコープ 0 は 543×859） | `resolver.rs` の `char_size`・`balloon_size` の doc | 直した |
| 385 | できれば | A | P5 の `Side` の決まりを左右の 2 つだけと書く（`balloon.alignment,none` の `Auto` が抜ける・248 の残り） | `resolver.rs` の `resolve_placement` の doc と P5 の注記 | 直した |
| 386 | できれば | A | バルーンの画面の内の関門を起動と実行中の 2 つと書く（バルーンだけのドラッグの解放の関門が抜ける） | `resolver.rs` の `ScopePlacement.balloon_pos` の doc・`windowposition.rs` のモジュールの doc | 直した |
| 387 | できれば | A | ファイルの頭が `Side` について確かめるのは 2 つ「だけ」と書くが、T-K6・T-K7 補・T-K9 も確かめる | `resolver_balloon_keyword_tests.rs` | 直した |
| 388 | できれば | A | 合成のゴーストの組み立て（`synth_declared_dpi_ghost`）を 2 つのテストが手で写す | `placement_prepare_tests.rs`・`placement_char_clamp_tests.rs` | 直した（共有の `synth_declared_dpi_ghost` を呼ぶ。`test_support.rs` の写しの相手を名指す doc も直した） |
| 389 | できれば | A | ずらしの軸の表の在りかを `follow::OffsetBase` のモジュールと書くが、`OffsetBase` は `resolver.rs` の型で、表は `follow/offset_space.rs` にある | `lib.rs` | 直した（リンクをやめ、`follow/offset_space.rs` のモジュールの doc と書く） |
| 390 | できれば | A | `Err` を返さずに代わりの値で続ける 2 つの縮退（primary の印を持つモニタが無い・主モニタの DPI が取れない）の記録だけが異常の手がかりなのに、テストが記録を見ない（221・324 と同じ形） | `lib.rs` の `primary_monitor` の `warn!`・`build_measure_scaling` の `error!`・`placement_monitor_tests.rs`・`placement_prepare_tests.rs` | 直した（2 本のテストで記録が 1 行で欄を持つことを確かめる。`warn!`／`error!` を消す壊し方で赤: 「`primary フラグを持つモニタが見つからない` を含むログがちょうど 1 件ではない: [] left: 0 right: 1」ほか） |
| 391 | 必ず直す | B | 記憶の位置を寄せて表示した起動で、古い記憶（位置はあるが隣り合いの印が無い）へ印を書き足さない、と記録に出すのに、隣の窓の位置を書く束の側で印が足される。たとえばスコープ 0 に記憶が無く、スコープ 1 が印の無い古い記憶のとき、スコープ 0 の位置を書く束が寄せた並びから決めた `WindowAdjacent{1}` を足す。次の起動からは印があるので、寄せずに起動した回に正しい印を書き足す道が失われ、寄せた並びで隣り合っていれば、覚えていない並びへ詰められる | `persist.rs` の `persist_unremembered_char_positions`・`persist_entries` | 直した（RED を先に: `clamped_boot_adds_no_mark_to_old_memory_through_the_neighbours_position`。直す前は「寄せた起動で古い記憶へ印が書き足された: {WindowAdjacent { scope: 1 }: "none", …} left: Some("none") right: None」で赤。寄せた起動の古い記憶のスコープを `skip` に移し、`persist_entries` と同じ送り口 `send_entries` がそのスコープの印を束から外す。`skip` を空にする壊し方で同じく赤。スコープ 0 の位置は書かれることも確かめる。`persist.rs` は 988 行） |
| 392 | できれば | B | 記憶からの戻しの射影の決まりを、補軸をいつも作業領域へ収めると書くが、収めるのは再固定した窓がまったく見えないときだけ（一部でも見えれば位置のまま） | `persist.rs` の「射影規則」と `merge_scope` の doc | 直した |
| 393 | できれば | B | 頭と注記が要件 5.6（補正を焼き付けない）を確かめると書くが、どのテストも画面の内の関門を通さず、モニタも空で補正が起きない（84 の残り） | `persist_restore_offset_base_tests.rs` | 直した（5.6 を外し、受け持つテストの名を書いた） |
| 394 | できれば | B | 試験台がシェルのフォルダを「resolve が成功する最小構成」として作るが、`load_restored_state` はもうシェルを解かない。4 本のうち 3 本がシェルを置くので、シェルを解く形に戻されても気付くのは 1 本だけ | `persist_io_wiring_tests.rs` の `plant_minimal_ghost` | 直した（`plant_minimal_ghost` を消し、どのテストもシェルの無いゴーストで回す。シェルが無いことを前提として確かめる 1 本に、シェルを解く形へ戻ったら落ちる番を移した） |
| 395 | できれば | B | 開発者の記憶の帳面の名を名指す（299・357 の残り） | `source.rs` | 直した（決まりをその場の文で書く） |
| 396 | できれば | B | spec の名の無い議題の番号（「research §13 議題 1」・376 の残り） | `reset_pos.rs` | 直した（`areka-P0-move-canon` の名を足した） |
| 397 | できれば | B | 呼び手を台本と右クリックの 2 つと書く（位置の近道の入口が抜ける） | `reset_pos.rs` の頭 | 直した |
| 398 | できれば | B | 既定の位置の材料を置くのは起動の準備とシェルの切り替えだけと書き、「同じシェル・同じ拡大率なら同じ位置」と書くが、面の windowposition もバルーンの面の切り替えで材料を差し替える | `reset_pos.rs`・`reset_balloon_pos.rs` | 直した（書き手を足し、「同じシェル・同じ面・同じ拡大率なら」に） |
| 399 | できれば | C | 手本の煙の門の写し元を `main.rs` の `fn main` と書く（今は `crates/areka/src/smoke_exit.rs`）。本体と「同義」と書くが、読めない値で `warn!` を出すのは手本だけ（125 で足した） | `examples/window-placement.rs` の 5 か所 | 直した |
| 400 | できれば | C | `sticky_window_raw` の doc が起動でだけ台帳へ入ると書く（シェルの切り替えでも差し替える・316 と同じ形） | `config.rs` | 直した |
| 401 | できれば | C | ゴーストの側だけの `balloon.alignment` を拾うことを、既定と同じ `left` で確かめていて、読みをシェルの側だけにしても緑 | `config.rs` の `t_c1_balloon_alignment_two_layer_cascade` | 直した（ゴーストの側だけ `right` で `Right` を期待する。読みをシェルの側だけにする壊し方で赤: 「left: Left right: Right」） |
| 402 | できれば | C | spec の名の無い設計の討議の議題の番号（「設計討議 #1 確定」・376 の残り） | `examples/window-placement.rs` | 直した（`areka-P0-window-placement の設計討議 議題 1`） |
| 403 | できれば | C | 写し元の手本の道 `examples/emo-present.rs` が、この crate の `examples/` には無い | `examples/window-placement.rs` の頭 | 直した（`crates/areka/examples/emo-present.rs`） |
| 404 | できれば | C | 「surface0 だけのシェル」を組む手順が 2 つのテストに写してある（`surfaces.txt` の中身だけが違う） | `measure_default_surface_tests.rs`・`measure_tests.rs` | 直した（`measure_tests.rs` の `surface0_only_shell` の 1 つ。`measure_tests.rs` は 795 行） |
| 405 | できれば | D | `set_default_char_pos` の呼び手を 2 つと書く（`reset_pos::reset_window_positions` が抜ける） | `spawn.rs` | 直した |
| 406 | できれば | D | 起動の戻しを `apply_restored_placements` と書く（227 の残り） | `diag.rs` の `PlacementRoute::Restore` の注記 | 直した（`persist::restore_and_judge`） |
| 407 | できれば | D | `AnchorChange` の書き手を、どの段にも登録されない見本の `anchor_changed_system` と書き、本番でただ 1 つの書き手 `reseed::reanchor_char_windows`（シェルの切り替え）を書かない | `diag.rs` | 直した |
| 408 | できれば | D | `BalloonLimit` の読み手の一覧に、解放の時の補正（`apply_release_limit_correction`）が抜ける | `spawn.rs` | 直した（読み手を足しただけ。343 の「以後不変」の文は裁定まで残した。5 回目に 343 で直した） |
| 409 | できれば | D | 普段の起動が待ちの上限に届かない理由を「残る待ちは GPU の装着と最初の `ShowSurface` だけ」と書くが、起動の呼び手はゴーストの待ち（窓が無い・まだ見せていない）を数えない。数えるのは areka 自身の待ちだけ | `chain_finalize.rs` の頭と `CHAIN_FINALIZE_STALL_FRAMES` の doc | 直した |
| 410 | できれば | D | 走査から外す結線の 1 行を `main.rs` と書く（今は `ghost_session.rs` の `register_systems`） | `spawn_zorder_pair_deferred_tests.rs` | 直した |
| 411 | できれば | D | 偽の窓の手を作る補助が、共有の `test_support::fake_handle` の写し（整数の型だけが違う） | `spawn_test_support.rs` の `fake_window_handle`・`spawn_clickthrough_tests.rs`・`spawn_follow_pipeline_tests.rs` | 直した（共有のものを使う。期待の組は変わらない） |
| 412 | できれば | D | `drag_event` を「follow.rs と同型」と書く（相手は `follow_test_support.rs`） | `spawn_follow_pipeline_tests.rs` | 直した（写しそのものは 358 の申し送り） |
| 413 | できれば | D | emo2 の寸法を決め打ちで書く（98 の残り） | `spawn_follow_pipeline_tests.rs` | 直した（`sample_test_support` の `SCOPE0_W`・`SCOPE0_H`） |
| 414 | できれば | E | 版の数を読むのは状態遷移の檻だけ、全数で押さえる、と書くが、付け替えの檻と本体の descript の段のテストも読み、付け替えでの動きは付け替えの檻だけが確かめる（365 の残り） | `zorder_group_ledger.rs` の `version` の欄と `version()` の doc | 直した |
| 415 | できれば | E | 実窓の檻のうち areka に置くのは「本ファイルだけ」と書くが、兄弟の `zorder_chain_inversion_order_tests.rs` も同じ理由で areka にある | `zorder_chain_tail_order_tests.rs` | 直した |
| 416 | できれば | E | 語の前後の空白は上流で落としてあり、台帳の `trim` は念のためと書くが、descript の道は値の両端しか落とさず、区切りの後の空白（`seriko.zorder,1, 0` の `" 0"`）を落とすのは台帳の `trim` だけ。信じて `trim` を消すと手で直した descript が読めなくなる | `zorder_group_ledger.rs`・`zorder_group_ledger_tests.rs` の t_zgp13 | 直した（文を直し、テストの名を `t_zgp13_spaces_around_tokens_are_ignored` に。名の使い手は定義だけ） |
| 417 | できれば | E | 「本機能」が、どの機能かを言わない（278・346 の残り） | `zorder_property_deferral_tests.rs`・`zorder_group_ledger_tests.rs`・本体の `zorder_inversion_branch_coverage_tests.rs` | 直した |
| 418 | できれば | E | 分岐の表の見出しが、分ける前の道 `placement/…` を書く（今は `crates/areka-placement/src/`）。落ちたときの文にそのまま出る（271 の残り） | 本体の `zorder_group_branch_coverage_tests.rs`・`zorder_inversion_branch_coverage_tests.rs` | 直した（`areka-placement/…` に。表の検査はファイルの名だけを使うので振る舞いは同じ） |
| 419 | できれば | E | 並びの完全一致の後で、同じ 2 つの欄から決まる確かめ（決して落ちない・275 の残り） | `zorder_chain_compose_written_order_tests.rs` の t_zcw01〜03 | 直した（消した。検査の較正は t_zcw05、全数は t_zcw04 にある） |
| 420 | できれば | E | 解釈と台帳のテストの小道具（`accept`・`expect_added`・`expect_accept`・`expect_reject`・`written`・`implied`）が 4 つのファイルに写してある（77 の残り） | `zorder_group_ledger_*_tests.rs` | 直した（`zorder_test_support.rs` の 1 組） |
| 421 | 直したほうがよい | F | 公開の欄 `ChainRealignPending.armed_frame` を「診断が待ちの長さを見るのに読む」と書くが、読み手は 0（武装の `debug!` は手元の値を使う。86 と同じ形） | `chain_realign.rs`・`reset_pos_tests.rs`・本体の `runtime/frame_chain_finalize_restart_tests.rs` | 直した（裁定: 推奨案で承認。欄を消し、テストの組み立て 2 か所から 1 行ずつ消した） |
| 422 | できれば | F | `snapshot_char_rects` の呼び手を拡大率の相の 1 つと書く（隣り合いの印を書く `persist::push_adjacency_marks` も呼び、読めない組は印を書かない） | `chain_realign.rs` | 直した |
| 423 | できれば | F | 「本仕様」が、どの spec かを言わない（すぐ近くで名指すのは別の spec・298・319・330 の残り） | `balloon_limit.rs` | 直した（「ここでは」に） |
| 424 | できれば | F | 説明が `None` と書くが、関門は `BalloonPlace::Fits` で分ける（283 の残り） | `balloon_limit_gate_tests.rs` | 直した |
| 425 | できれば | F | 「設計の決め 12」が、ファイルが名指す spec（`areka-P0-shell-balloon-switch`）には無く、`areka-P0-window-link-directives` の決め | `reseed.rs` | 直した（spec の名を足した） |
| 426 | できれば | F | 窓の位置・ドラッグの終わりの知らせ・窓の矩形を作る補助が、追随の支えの写し（358 の残り） | `balloon_limit_flip_tests.rs`・`balloon_lock_drag_tests.rs`・追随の `follow_test_support.rs` | 申し送り（追随の支えの 4 つを `pub(crate)` に開いて使う。開く所が追随の持ち物） |

- 申し送りとの対応（要件 1.6）: 1〜380 と同じ所を指す指摘は 0 件。前の回の直しの残り（同じ形で、前の回の一覧に無かった所）は新しく数え、どの番号の残りかを一言に書いた（383・385・393・395・396・400・402・406・412・413・414・417・418・419・420・423・424・426）。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を指す `<ファイル>.rs:<行>` の形を、ツリー全体（`.kiro/specs/completed/`・`vendors/`・この spec の記録・`.kiro/steering/roadmap-history.md`・日付つきの brief の写しを除く）から拾い、`git show HEAD:` の行と今の行を 1 つずつ見比べた。動いたのは `lib.rs`（本体の `examples/collision-probe.rs` の 3 行の 5 つ）・`config.rs`（`doc/ukadoc-coverage/briefing-assets.md` の 4 行の 12 個）・`resolver.rs`（同じ文書の 1 つ）・`persist.rs`（同じ文書の 1 つ・B の組が直した）で、どれも今の行に直した（直しが作った古さなので、この回で直した）。

#### 指摘にしなかったもの

- `reseed_test_support.rs:18` の `CHAR0`（emo2 の寸法の決め打ち・98 の形）。`reseed_test_support.rs` は本体のテストのために `test-support` の機能でも組まれ、`sample_test_support` は `cfg(test)` だけなので使えない（`spawn_test_support.rs:22` も同じ）。
- `persist::apply_restored_placements` に本番の呼び手が無いこと。公開で、追随と本体のテストに 26 か所の使い手があり、消すのは公開の形を変える割に得る物が無い（215・246 と同じ線）。383・406 は呼び手の名の誤りだけを直した。
- `balloon.alignment` と `balloon.dontmove` の値の大小文字（`Right` が左と警告・`TRUE` が読めない）。296 と同じ問いなので、別に数えず 296 の裁定で一緒に決める（下の裁定の表）。
- `remembered_decision` が公開で、本番の呼び手が同じモジュールの `boot_pair_decision` だけ。狭めても得る物が無い（215・246 と同じ線）。
- 決め打ちの定数どうしを比べる `assert_ne!`（`balloon_offset_persist_roundtrip_tests.rs`）・表の期待の値を確かめる `contained`（`balloon_limit_*_tests.rs`）。検体の定数や表を書き換えると赤になるので、決して落ちない確かめではない。
- 32 ビットの組み立てで `1usize << 32` が組めないテスト。この crate を 32 ビットで組む所は無い。
- `create_raw_window` が `WS_EX_LAYERED`・`WS_EX_ACCEPTFILES` を持たないのに「本番のゴースト窓と同じ作り」と書くこと。重なり順に効く部分（`WS_POPUP`・`WS_EX_TOOLWINDOW`・活性化しない表示）は同じ。
- 説明の一覧が全部を挙げないだけのもの（`lib.rs:42` の `descript_window_keys` の要約・`placement_shared_test_support.rs` の頭）。偽りを言っていない。
- 組から上がって、確かめて外したもの: `compose_chain` の毎フレームの仕事（2 回目に外した）・`zorder_test_support.rs` の Win32 の補助が wintf の写し（32 の裁定）・`b01` をスコープ 1 と読む（正典が沈黙）・`reset_pos.rs` が主モニタの DPI を読む（バルーンの基準だけに使い、位置は変わらない）・同じ DPI の emo2 の準備が数本ある（どれも別のことを確かめる）・`test_crash_report.rs` の中身（88 の裁定）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 296 | 3 回目から（要件 5 の 3 種類目） | descript の `alignmenttodesktop` の大小文字（18.3 の表）。この回、`balloon.alignment` と `balloon.dontmove` の値も大小文字を区別して読むと分かった（C の組） | 18.3 と同じ（SSP の振る舞いを MCP で測ってから決める）。決めるときは `balloon.alignment`・`balloon.dontmove` も同じ決まりにそろえる | 裁定待ち（この回のあいだに裁定は届かなかった。コードは変えていない）。5 回目に開発者の裁定が届き、3 つの鍵をそろえて直した（18.5） |
| 343 | 3 回目から（要件 5 の 1 種類目・3 種類目） | `BalloonLimit` をバルーン・面・シェルの切り替えとバルーン位置初期化で置き直さない（18.3 の表） | 18.3 と同じ（直す） | 裁定待ち（この回のあいだに裁定は届かなかった。コードは変えていない。408 は読み手を足しただけで「以後不変」の文には触れていない）。5 回目に開発者の裁定が届き、直した（18.5） |
| 391 | 要件 5 に当たるかを確かめた（B の組は 1 種類目と見た） | 寄せて表示した起動で、古い記憶へ隣り合いの印を書き足してしまう | **直す**: 完了した `window-work-area-clamp` の要件 4.6「記憶の位置を寄せて表示した起動であるとき、隣り合いの印の無い古い記憶へ印を書き足さず、書き足さなかったことを記録に残す」に今のコードが反している。コードの doc と記録の行もそう約束している。`dpi-realign-remembered-chain` の要件 6.2（位置と一緒に印を書く）・6.5（古い記憶への書き足し）は、後から 4.6 が寄せた起動について狭めた。完了した要件に合わせる直しなので、要件 5 の 3 種類には当たらない | 進行役が要件 5 に当たらないと判断して直した（下の一覧の 391） |
| 421 | 要件 8.6（公開の資源 `ChainRealignPending` の欄を消す。本体のテストが組み立てる） | 読み手の無い公開の欄 `armed_frame` を C20 のうちに消すか | **C20 のうちに直す**: 読み手は 0（`debug!` は手元の値を使う）。86・118・242 と同じ形。小さく、利用者に見える振る舞いは変わらない | 推奨案で承認（audit の spec は議題を推奨案で承認してよい、という開発者の決まりによる）。直した |

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（持ち主の spec はもう完了しているので、8.2 が扱いを決める）:
  - `follow/window_move.rs:525-528` が `PlacementRoute::AnchorChange` を「本番では未結線・現に到達しない」と書くが、`reseed::reanchor_char_windows`（シェルの切り替えのたび）が書く。`DpiSyncHold` を持つ窓のあいだにシェルを切り替えると、正しい書き込みで見張りが鳴りうる。あわせて `window_move.rs:129`（`resize_window_to` の上流の一覧に `reanchor_char_windows` が無い）・`keyword_base.rs:47`。
  - `follow.rs:105` が `persist::apply_restored_placements` を名指す（本番は `restore_and_judge`・383・406 と同じ形）。同じ所の名前の無い「task 1.4」「task 2.4」。
  - 358 を広げる: `follow_test_support.rs` の `point_of`（298 行）・`position_of`（37 行）・`drag_end_event_at`（108 行）・`window_pos_sized`（118 行）を `pub(crate)` に開き、`balloon_limit_flip_tests.rs`・`balloon_lock_drag_tests.rs` の写し（426）を消す。
- `audit-areka-app` へ:
  - `runtime/frame/drain_resnap.rs:601` が `persist_unremembered_char_positions` の本番の呼び手を「ここ 1 か所だけ」と書く（`reset_pos.rs` も呼ぶ。222 の形）。
  - `boot_config.rs:563` の開発者の記憶の帳面の名（18.3 の 504 行とは別の行）。
  - `smoke_exit.rs:1`・`:8`・`:67` の名前の無い「task 2.3」・`examples/collision-probe/fixture.rs:23` の「main.rs／他 example と同名」（今は `src/smoke_exit.rs:44`。399 と同じ形）。
  - `main_restore_seam_tests.rs:24` が、自分の `plant_minimal_ghost` を「persist.rs の `plant_minimal_ghost` と同型」と書く（前の回から置き場が違っていて、394 で消えた）。同じ所の「resolve が成功する」も、`load_restored_state` がシェルを解かなくなったので古い。
  - 421 の直しで `runtime/frame_chain_finalize_restart_tests.rs:45` の `armed_frame: 7,` を消した（欄を消すと組めなくなるので避けられない 1 行）。
- 持ち主の見直しの spec が無いもの（記録だけ）: `areka-ghost/src/runtime.rs:117-118` の開発者の記憶の帳面の名。`areka-sylphya/src/persist/mod.rs:120` の「4 key 族」（列挙は 10 の腕。走っている `sylphya-sqlite-kvs` が触るファイル）。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 963 件が通り 1 件 ignore、手本 2 件が通る（足した 391・390 の確かめと、消した・まとめたテストの差し引き）。
- `cargo test -p areka`: 本体の bin 3180 件が通り 6 件が ignore、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る（行の番号を直した後にも回し直した）。
- `cargo test -p log-capture-kit --test file_length_guard_test --test with_default_guard_test`: 6 件と 24 件が通る（`persist.rs` 988 行・`lib.rs` 928 行・`persist_restore_tests.rs` 832 行・`measure_tests.rs` 795 行）。
- `cargo fmt --all -- --check`: 緑。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。範囲の外で触ったのは、421 の欄を消すために要る本体の `runtime/frame_chain_finalize_restart_tests.rs` の 1 行と、行の番号を直した `doc/ukadoc-coverage/briefing-assets.md`・本体の `examples/collision-probe.rs` だけ。
- 直しは 6 つの組ごとに分けて同時に進めた（触るファイルは重ならない）。壊し方は狙った編集で入れて狙った編集で戻し、`git checkout`・`restore`・`stash`・`reset` は使っていない。

#### 判定

4 回目の新しい指摘は 46 件（必ず直す 1・直したほうがよい 1・できれば 44）で、0 件ではない（3 回目の 85 件から減った）。直した 45 件・申し送りだけのもの 1 件（426）。新しい裁定待ちは 0 件で、3 回目からの裁定待ち 296・343 は残る（5 回目に裁定が届いて直した・18.5）。5 回目をかける（要件 6.2）。5 回目で 0 件に収まらず、5 回目に「必ず直す」と本番の振る舞いに関わるものが無ければ、開発者に一言知らせ、5 回目の指摘を直したところで打ち切る（タスク 8.1）。この回は 391（本番の振る舞い・必ず直す）があったので、打ち切りの形にはまだ当たらない。

### 18.5 5 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `246b6d75f`）。
- 範囲は 1.1 節と 18.1〜18.4 と同じ数え方で測り直した。ファイルの数は 4 回目と同じ 116。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 27 | 10,543 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,172 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,055 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 347 |
| 計 | 116 | 36,168 |

- ponytail は 5.3.3（1〜4 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,431・B 20／6,677・C 20／5,924・D 17／5,404・E 15／5,864・F 20／5,868。計 36,168 行で上の表と合う）。読む段では何も変えず、指摘を集めて番号を付けてから、触るファイルが重ならないように組ごとに直させた。
- 読み手には 4 回目と同じ線（各回の「指摘にしなかったもの」・古い文は今の事実と食い違うものだけ・決して落ちない確かめはどの直しでも赤にならない理由を言えるものだけ）を渡した。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,431 | 0 | 0 | 7 | 7 |
| B | 20 | 6,677 | 0 | 1 | 6 | 7 |
| C | 20 | 5,924 | 0 | 1 | 5 | 6 |
| D | 17 | 5,404 | 0 | 0 | 4 | 4 |
| E | 15 | 5,864 | 0 | 0 | 3 | 3 |
| F | 20 | 5,868 | 1 | 0 | 1 | 2 |
| 計 | 116 | 36,168 | 1 | 2 | 26 | **29** |

- 433 は E の組の読み手が A の組のファイル（`lib.rs`）に見つけたもので、A に数えた。2 つの組が同じ所を出したものは無い。

#### 指摘の一覧（427〜455）

扱いの印は 18.1〜18.4 と同じ。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 427 | できれば | A | `classify_x_vocab` の doc が、`x_num` を `get_scalar::<i32>("windowposition.x")` の結果と書く（今の parsers は `parse().ok()` で読み、数でない値は `None` で記録も積まない） | `windowposition.rs` | 直した |
| 428 | できれば | A | シェルの切り替えで scope 付きの記録は読めないバルーンの `warn!` だけ、と書くが、面 0 が無い系列では静かな解決が自分の scope 付きの `error!` も出す（221 の残り） | `lib.rs` の `load_scope_windowpositions`・`placement_windowposition_vocab_tests.rs` の節の見出し | 直した（読めないフォルダのときに限ると書いた） |
| 429 | できれば | A | T-R5 の試験台がどのスコープにも `defaulty` を書かないので、上・左・右の辺が `defaulty` を使わないことを誰も確かめていない | `resolver_resolve_tests.rs` の `seam_cfg` | 直した（スコープ 0 に `defaulty` を書く。RED: 上と左の辺で `default_y` を使う壊し方で `t_r5_top_left_right_resolve_on_their_own_edge` が赤「left: PointPx { x: 1544, y: 80 } right: PointPx { x: 1544, y: 40 }」） |
| 430 | できれば | A | 共有の寸法の定数を引いている同じテストが、2 つの寸法だけ数で書く（98・413 の残り） | `placement_prepare_tests.rs` | 直した（`BALLOON0_W/H`・`SCOPE1_W/H`） |
| 431 | できれば | A | 追随の `clamp_x_into` が resolver の `clamp_axis` の写し（37 の残り） | 追随の `follow/visibility.rs` | 申し送り（`audit-placement-follow`。`clamp_axis` を呼ぶ） |
| 432 | できれば | A | `MonitorSnapshot::from_monitors` が根の `work_area_px` の写しで、doc が消えた `mod.rs` の `primary_work_area` を名指す（13・92 の残り） | 追随の `follow/work_area.rs` | 申し送り（`audit-placement-follow`。`crate::work_area_px` を使い、doc を直す） |
| 433 | できれば | A | `zorder_chain_compose` を「Win32／ECS 非依存」と書くが、`bevy_ecs` の `Entity` を使う（モジュールの doc は「Win32 も World も触らない」） | `lib.rs` | 直した（「World も Win32 も触らない純関数」に） |
| 434 | 直したほうがよい | B | 記憶の位置を寄せて表示した起動のあいだに、ドラッグや連れ動きの保存（`persist_entries`）が、印の無い古い記憶へ隣り合いの印を書き足す（寄せた矩形で判じる）。391 は並べ終えた時点の保存だけを塞いだ | `persist.rs` の `persist_entries`・`send_entries` | 裁定待ち（下の裁定の表） |
| 435 | できれば | B | `persist_entries` の呼び手を 4 か所と書くが、並べ終えた時点の保存は 391 の直しの後、内側の `send_entries` を直に呼ぶ（391 の直しが作った古さ） | `persist.rs` のモジュールの doc・`PersistWiring` の doc | 直した（doc だけ。`persist.rs` は 990 行） |
| 436 | できれば | B | 同じ入力・同じ呼び出しのテスト 2 本が確かめを分けて持つ（115 の残り） | `persist_restore_tests.rs` | 直した（1 本にまとめ、確かめは全部残した） |
| 437 | できれば | B | 保存値の腕のテストが、往復のテストの 5 つのアンカーの確かめに含まれる（20 の残り） | `persist_restore_offset_base_tests.rs` | 直した（消した。確かめは `a_saved_offset_survives_the_save_restore_roundtrip_bit_for_bit` にある。2 つのファイルの頭の説明を直した） |
| 438 | できれば | B | 拡大率と順を自分で渡す純粋な足し算のテスト 2 本（各段の単体と、本番の順の準備のテストが確かめる） | `balloon_offset_supply_tests.rs` | 直した（消した。確かめは `prepare_merges_each_supply_in_its_own_axis_and_reaches_the_placement`・`prepare_at_identity_scale_matches_the_pre_spec_merge` にある） |
| 439 | できれば | B | `test_support::fake_handle` の手書きの写し 2 か所と、「ゴーストの窓の全部に偽の手を付ける」補助の写し 2 つ（411 の残り） | `persist_finalize_save_tests.rs`・`reset_pos_tests.rs`・`spawn_follow_pipeline_tests.rs` | 直した（`test_support::give_fake_handles` の 1 つ。本体の `move_cue_apply_move_tests.rs` の写しは申し送り） |
| 440 | できれば | B | 鍵を全部保存して消すテストの鍵の並びが手書きで、鍵の種類が増えても気付かない（85 の残り） | sylphya `persist_remove_tests.rs` | 直した（`_` の無い網羅の `match` で並びを確かめる。RED: 腕を 1 つ消すと「non-exhaustive patterns」で組めない・並びから 1 つ消すと赤） |
| 441 | 直したほうがよい | C | 採寸でスコープ 0 の合成が失敗する枝にテストが無く、テストの頭が「`measure_tests.rs` が確かめる」と書く（245 の残り） | `measure.rs` の `compose_size(…)?`・`measure_default_surface_tests.rs` の頭 | 直した（`scope0_compose_failure_is_measure_error_with_one_error_log`。RED: `?` を `unwrap_or(1×1)` にする壊し方で新しいテストだけが赤「本体側の面が無いシェルで Ok は誤成功」） |
| 442 | できれば | C | 鍵の並びの完全一致の直後で決して落ちない確かめ | `config.rs` の `t_c3_char_n_prefix_does_not_add_scope_n` | 直した（消した） |
| 443 | できれば | C | 完全一致の後で決して落ちない、繰り返しの半分（採寸のバルーン） | 本体の `measure_bake_count_tests.rs` | 直した（資産の側だけを回す） |
| 444 | できれば | C | 組み込みの既定の面で emo2 を測り直すだけのテスト（既定の面・寸法・記録の無さはほかのテストが確かめる） | `measure_default_surface_tests.rs` の `unwritten_default_surface_keeps_current_sizes` | 直した（消した。「4 通り」を「3 通り」に） |
| 445 | できれば | C | 包まない読み手の測りを「全コマを焼く今の測り」と書く（本番は 1 枚目だけを読む包みを通す） | 本体の `measure_bake_count_tests.rs` の 3 か所 | 直した |
| 446 | できれば | C | 同じ 2 つの入力を回すテスト 2 本が確かめを分けて持つ（253 の残り） | `measure_tests.rs` | 直した（`Err` の確かめを記録のテストへ移し、境界だけのテストを `apply_scaling_identity_keeps_i32_max` に） |
| 447 | できれば | D | `MoveCue` の doc が台本の `\![move]` だけを書く（起動の連鎖の確定・シェル位置初期化・台本の揃え直し・連れ動きも同じ語で書く。266 の残り） | `diag.rs` | 直した（`Restore` と同じ形で呼び手を挙げ、語だけでは見分けられないと書いた。`ChainRealign` の doc の同じ誤りも） |
| 448 | できれば | D | 連鎖の確定の規則を「resolver の P2 式そのもの」と書くが、記憶の印が右の組は `x(n−1) + w(n−1)` | `chain_finalize.rs` の頭 | 直した（2 つの式と `resolve_chain` を書いた） |
| 449 | できれば | D | 同じ入力を直前の確かめが通す（132 の残り） | `chain_finalize_resolve_tests.rs` | 直した（消した） |
| 450 | できれば | D | 語の確かめ 2 つが、ほかのテストの確かめの言い直し | `chain_finalize_resolve_tests.rs` の `pair_decision_words` | 直した（`JoinRight` の 1 行だけ残した） |
| 451 | できれば | E | 説明の並びが、覆された寄せ（かたまりへの組み替え）の形のまま（276・363 の残り） | `zorder_group_ledger_state_tests.rs` の t_zgl6 | 直した |
| 452 | できれば | E | 台帳の順と組の番号の順が食い違う入力（シェルの切り替えで基底の番号が大きくなる）を、鎖の組み立てのテストが持たない | `zorder_chain_compose_tests.rs` の t_zcc08 | 直した（基底を据え直した台帳の場合を足した。RED: 組を番号で並べ替える壊し方で t_zcc08 だけが赤。規則の doc も「台帳の順」に） |
| 453 | できれば | E | 書けない属性の書き込みは「効果を 1 つも生まない」と頭が書くが、テストは `NotSettable` の 1 つを期待する | `zorder_property_deferral_tests.rs` の頭 | 直した |
| 454 | 必ず直す | F | バルーンを切り替えても、標準の位置の材料のバルーンの作者の DPI が前のバルーンのまま。新しいバルーンの `windowposition` を前の拡大率で解き、値が前と同じなら解き直しもしない（バルーン位置初期化も同じ材料を使う）。たとえば `dpi,192` のバルーンから `dpi` の無いバルーンへ替えると、`windowposition.x,40` のずらしが 20 になる | 本体の `runtime/frame/switch.rs` の `finish_balloon`・`runtime/skin_switch.rs` の `SwapFinish::Balloon` | 直した（後始末へ新しいバルーンの作者の DPI を運び、材料の値と違えば入れ替えて面の `windowposition` を忘れ、面の書き手に全スコープを解き直させる。RED: 「(材料の作者の DPI, scope 0 の追随) 前の windowposition が同じ値 left: (192, … offset: PointPx { x: -223, y: 0 } …) right: (96, … offset: PointPx { x: 504, y: 40 } …)」。テストは `switch_finish_face_tests.rs` の `balloon_switch_resolves_the_face_windowposition_with_the_new_author_dpi`。`reset_pos.rs` の `DefaultPlacementBasis` の doc にも 1 行） |
| 455 | できれば | F | テストの doc が「隣り合いの印は残す」と書くが、印を保存していない | 本体の `reset_balloon_pos_tests.rs` | 直した（印を 1 つ保存して確かめる。RED: 初期化で印も消す壊し方で「ずらしの鍵だけが消え、ほかの表は残る」が赤） |

- 申し送りとの対応（要件 1.6）: 1〜426 と同じ所を指す指摘は 0 件。前の回の直しの残りは新しく数え、どの番号の残りかを一言に書いた（428・430・431・432・436・437・439・440・441・446・447・449・451）。435 は 391 の直しが作った古さ。
- 454 は要件 5 に当たるかを確かめた: 完了した `areka-P0-balloon-offset-dpi` の要件 1.4（`windowposition` 由来の量はバルーンの軸＝そのバルーンの作者の DPI で換算する）に今のコードが反しているので、完了した要件に合わせる直しで、要件 5 の 3 種類には当たらない（391 と同じ判断）。進行役が直した。
- 4 回目の「指摘にしなかったもの」の「`persist::apply_restored_placements` に本番の呼び手が無い」は誤り（B の組が見つけた）。`reseed::apply_shell_descript` がシェルの切り替えのたびに呼ぶ。狭めない、という結論は変わらない。

#### 開発者の裁定の 296・343 の直し

3 回目からの裁定待ち 296・343 に、5 回目のあいだに開発者の裁定が届いた（2026-11-15・チャットで「両方とも推奨どおりで」）。どちらも推奨案のとおりに直した。

- 296: descript の `seriko.alignmenttodesktop`（別綴の `alignmentondesktop` も）・`balloon.alignment`・`balloon.dontmove` の値を、読む所で 1 度だけ小文字にしてから比べる（`config.rs` の `resolve_scope`・`descript_window_keys.rs` の `read_flag`）。未知の語の警告は、作者が書いたままの綴りを載せる。RED を先に取った: `t_c1_balloon_alignment_ignores_case`（「Right left: Left right: Right」）・`balloon_dontmove_words_ignore_case`（「TRUE left: {} right: {0}」）・`descript_alignment_words_ignore_case`（`BOTTOM` で偽の「未知アンカー」の警告）。`resolver.rs`・`resolver_from_alignment_tests.rs` の「parsers 側で正規化済み」の文と `script_alignment.rs` の doc を直し、台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 14 行（`alignmenttodesktop` の 8 行・`balloon.alignment` の 2 行（抜けていた `none` も足した）・`balloon.dontmove` の 4 行）に大文字小文字の決まりを足した。裁定の外の `balloon.syncscale`（`raw.trim() == "true"`）・`windowposition.x` の語（`Center`）・`seriko.zorder` の語は変えていない。
- 343: `reseed::reinstall_balloon` が、置き直す `ScopePlacement` の `balloon_limit` で `BalloonLimit` を入れ直す（窓の書き込みの関門が読むので、置き直す前に）。呼び手はシェルの切り替え・面の `windowposition` の置き直し（バルーンの切り替えもここを通る）・バルーン位置初期化の 3 つ。利用者が動かしたバルーンは、面の置き直しで位置を変えずに値だけを入れ直す。`spawn.rs` の「spawn 時に焼き込んだあとは不変」の文と、`resolver.rs` の `balloon_limit` の欄の doc を直した。RED を先に取った: `t_rs09_balloon_limit_is_reinstalled_from_the_new_placement`・`face_limit_reaches_the_balloon_window_moved_or_not`・`balloon_reset_reinstalls_the_limit_from_the_basis` の 3 本が「left: Some(BalloonLimit(true)) right: Some(BalloonLimit(false))」で赤。動かしたバルーンの腕は、値を `true` にする壊し方で赤。動かしたバルーンへ limit 1 を入れ直しても、その場では画面の内へ引き戻さない（次の書き込みで関門が効く）。

#### 指摘にしなかったもの

- `PlacementRoute::ALL` に新しい語を足し忘れても気付かないこと（タスク 6.5 が網羅の `match` で決めた形）。
- `spawn.rs` の `BalloonFollow` が `balloon_offset_base` を写すのに doc が `balloon_offset` と書くこと（どの作り手でも 2 つは同じ値）。
- `runtime/frame/drain_resnap.rs` の `DpiSyncHeld` の腕に届かないこと（範囲の外・害が無い。申し送りに記録だけ）。
- 寄せた起動のあいだの隣の窓の書き込みが、印のある記憶や新しい記憶の印を、寄せた矩形で判じて書き直すこと（完了した `window-work-area-clamp` の要件 4.6 は印の無い古い記憶だけを言い、`dpi-realign-remembered-chain` の要件 6.2 は位置の書き込みのたびに印を書くと言う）。
- 面の `windowposition` が変わるたびに全スコープを解き直し、知らない `alignmenttodesktop` の警告がスコープの数だけ出ること（記述が変わるたびの 1 度で、毎フレームではない。291 で決めた形）。
- `check_table` が `#[test]` より上の `#[ignore]` を見落とすこと（リポジトリは 49 か所とも `#[test]` の後に書く）。
- 組から上がって、確かめて外したもの: 一緒に動く組の大きさに上限が無い（設計の決め 11）・`seriko.sticky-window` の末尾の読点で組ごと捨てる（わざとの形でテストがある）・DPI の関門の時間切れの後の入り直し・`capture` の番兵の無い 0 件の確かめ（`log_capture_kit::capture` は較正済みで全段を捕まえる）・`measure_tests.rs` の `parse` の比べ（54 の較正の当たり）・同じ `scaled_extent` との比べ（丸めを変えると赤になりうる）・`FirstFrameDecoder` が 1 枚目の失敗を黙って全コマの読みへ落とす（測りは静かな読みと決めた形）・`config.rs` の `cascade2` で読めないシェルの数がゴーストの値を隠す（正典が沈黙）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 296 | 3 回目から（要件 5 の 3 種類目） | descript の `alignmenttodesktop`・`balloon.alignment`・`balloon.dontmove` の大小文字（18.3・18.4 の表） | 大小を問わない形にそろえる | 開発者の裁定（2026-11-15・推奨どおり）。直した（上の節） |
| 343 | 3 回目から（要件 5 の 1 種類目・3 種類目） | `BalloonLimit` を切り替えとバルーン位置初期化で置き直さない（18.3 の表） | 直す | 開発者の裁定（2026-11-15・推奨どおり）。直した（上の節） |
| 434 | 要件 5 の 1 種類目（完了した `window-work-area-clamp` の設計は、寄せた起動で印を書き足さない所を並べ終えた時点の保存（`persist_unremembered_char_positions`）と書き、ドラッグや連れ動きの保存は名指さない）と 3 種類目（次の起動の窓の並び） | 寄せて表示した起動のあいだのドラッグ・連れ動きの保存で、印の無い古い記憶へ隣り合いの印を書き足すのを止めるか。たとえばモニタ B が無くて寄せた起動で、スコープ 1 が B の x=5000 の印の無い古い記憶のとき、スコープ 0 を寄せた窓の隣へドラッグすると `WindowAdjacent{1}` が書かれ、B が戻った次の起動でスコープ 1 が B からスコープ 0 の隣へ引き寄せられる（隣でない所へドラッグしたときは、寄せずに起動した回に書く印と同じ `none` になる） | **直す**: 要件 4.6 の「寄せて表示した起動では古い記憶へ印を書き足さず、寄せずに起動した回に表示の矩形から書き足す」の趣旨は、寄せた矩形から古い記憶の印を作らないこと。送らない窓の組を、`send_entries` が World（`RestoredPositionsClamped`・既定の位置・記憶の印・同じ束の位置）から作り、`persist_entries` の全部の呼び手に効かせる。RED を先に（寄せた起動でドラッグの保存をしても印が書かれない）。`persist.rs` は 990 行なので、足すなら兄弟のファイルへ分けてから | 裁定待ち（完了した spec の設計の範囲を広げ、利用者に見える次の起動の並びが変わるので、推奨案で承認にしない。コードは変えていない）。6 回目の前に開発者の裁定（2026-11-15・推奨どおり）が届き、直した（18.6） |

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（持ち主の spec はもう完了しているので、8.2 が扱いを決める）:
  - 431（`follow/visibility.rs` の `clamp_x_into` を消し、`resolver::clamp_axis(x, wa.left, wa.right.saturating_sub(w))` を呼ぶ）。
  - 432（`follow/work_area.rs` の `MonitorSnapshot::from_monitors` で `crate::work_area_px` を使い、doc の `mod.rs` の `primary_work_area` を根の `work_area_px` に。あわせて `lib.rs` の「根を引く子」の文に 1 行）。
- `audit-areka-app` へ:
  - `runtime/move_cue_apply_move_tests.rs:52` の `attach_fake_handles` は、439 で足した `areka_placement::test_support::give_fake_handles` の写し。
  - `runtime/frame/drain_resnap.rs:477` の `DpiSyncHeld` の腕は、`collect_chain_states` がその値を返さないので届かない（害は無い）。
- 454 の直しのために本体の `runtime/skin_switch.rs`・`runtime/frame/switch.rs`・`switch_finish_tests.rs`（下ごしらえの 1 行・999 行）・`switch_finish_face_tests.rs` を直した（範囲の外。`audit-emo2-boot-switch` の受け持つ所に近いが、走っている spec は無い）。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 965 件が通り 1 件 ignore、手本 2 件が通る。
- `cargo test -p areka`: 本体の bin 3181 件が通り 6 件が ignore（足した 454 のテストの 1 件の分だけ増えた）、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る（台帳の 14 行と briefing の行の番号を直した後）。
- `cargo test -p log-capture-kit --test file_length_guard_test --test with_default_guard_test`: 6 件と 24 件が通る（`persist.rs` 990 行・`lib.rs` 929 行・`persist_restore_tests.rs` 814 行・`measure_tests.rs` 815 行・本体の `switch_finish_tests.rs` 999 行）。
- `cargo fmt --all -- --check`: 緑。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。範囲の外で触ったのは、454 の本体の 4 ファイル・行の番号を直した `doc/ukadoc-coverage/briefing-assets.md`（`persist.rs:309`・`config.rs` の 7 つ）と本体の `examples/collision-probe.rs`（`lib.rs:579`）・296 の台帳 `doc/ukadoc-coverage/ledger/assets.toml`。
- 直しは 6 つの組と、裁定の 2 つ（296・343）を受けた 2 つの組に分けて同時に進めた（触るファイルは重ならない）。壊し方は狙った編集で入れて狙った編集で戻し、`git checkout`・`restore`・`stash`・`reset` は使っていない。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を指す `<ファイル>.rs:<行>` の形を、ツリー全体（`.kiro/specs/`・`vendors/`・`.kiro/steering/roadmap-history.md` を除く）から拾い、`git show HEAD:` の行と今の行を見比べた。動いたのは上の `briefing-assets.md`・`collision-probe.rs` の分だけで、どれも今の行に直した。

#### 判定

5 回目の新しい指摘は 29 件（必ず直す 1・直したほうがよい 2・できれば 26）で、0 件ではない（4 回目の 46 件から減った）。直した 26 件・申し送りだけのもの 2 件（431・432）・裁定待ち 1 件（434）。あわせて、3 回目からの裁定待ち 296・343 は開発者の裁定を受けて直した。

5 回目には「必ず直す」が 1 件（454）あり、本番の振る舞いを変える指摘も 2 件（454・434）ある。454 は直したが、434 は裁定待ちで本番の振る舞いに関わるまま残る。したがって、タスク 8.1 の「5 回目に『必ず直す』と本番の振る舞いに関わるものが無ければ打ち切る」の形には当たらない（5 回目の中に 454 があり、直した後も 434 が残る）。434 の裁定（直すか残すか）が出て、その直しを入れたところで、打ち切るか 6 回目をかけるかを開発者に仰ぐ。

### 18.6 6 回目（2026-11-15）

#### 開発者の裁定の 434 と大小文字の直し

5 回目の後、開発者の裁定が届いた（2026-11-15・チャットで「両方とも推奨どおりで」）。かけ直しの前に、どれも推奨案のとおりに直した。RED を先に取った。

- 434: 記憶の位置を寄せて表示した起動のあいだは、ドラッグや連れ動きの保存でも、印の無い古い記憶へ隣り合いの印を書き足さない（391 は並べ終えた時点の保存だけを塞いでいた）。
  - 足す前に `persist.rs`（990 行）を分けた。送り口と保存の側（`PersistWiring`・`persist_entries`・`forget_entries`・隣り合いの印・`persist_unremembered_char_positions`）を兄弟のファイル `persist_save.rs`（子のモジュール `persist::save`）へ移し、`persist::` の公開の道は再公開で保った。`persist.rs` は 692 行。
  - 決める所は共有の送り口に置いた。追随の持ち物（`follow/drag_follow.rs`・`follow/sticky_follow.rs`）は 1 行も触らず、その呼び出しが通る `persist_entries` が決める。`clamped_old_memory` が World から飛ばす組を作る（`RestoredPositionsClamped` が真で、スコープ 1 以上・既定の位置が無い・覚えた印が無い・同じ束がそのスコープの位置を書かない）。印を外したら `info!` を 1 行残す。
  - 位置を書くスコープの印は寄せた起動でも書く（その記憶はもう古くない）。寄せずに起動した回は今までどおり書く。
  - RED: `clamped_boot_drag_save_adds_no_mark_to_old_memory`（「寄せた起動=true … left: true right: false」）と `clamped_boot_keeps_the_mark_of_the_scope_whose_position_is_written`（「隣の古い記憶へは書かない … left: Some("none") right: None」）が赤。
  - 知られた限りを `shortcut:` で書いた。古い記憶かは起動の時点の台帳で決めるので、この起動でドラッグして印を書いたスコープも、後で隣の窓を動かしたときは印を書き直さない。`GhostWindows` を `&World` から書き換えられないので、困る例が出たら、書いたスコープを台帳から外す形にする。
  - 文書: `doc/COMPAT_ARCHITECTURE.md` の記憶からの組の戻しの行（並べ終えた時点の保存でもドラッグや連れ動きの保存でも、と書き、定義点を `persist_save.rs` の `clamped_old_memory` にした）・`persist.rs` のモジュールの doc・`RestoredPositionsClamped` の doc。完了した spec の文書（`.kiro/specs/completed/window-work-area-clamp/`）は履歴なので触っていない。
- 大小文字（296 と同じ形）: 読む所で 1 度だけ小文字にしてから比べる。
  - `balloon.syncscale`: `descript_window_keys.rs` の `read_syncscale`（`eq_ignore_ascii_case`）。RED: `syncscale_values_other_than_true_mean_unspecified` の `True`（「left: false right: true」）。台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `balloon.syncscale` の 4 行に「大文字小文字は区別しない」を足した。
  - `windowposition.x` の語: `windowposition.rs` の `classify_x_vocab`。RED: `classify_x_vocab_ignores_case`（「left: Invalid right: Keyword(CenterTop)」）。綴り違いの警告のテストは `centre` に替えた。`doc/COMPAT_ARCHITECTURE.md` の「`windowposition.x` のキーワードの大小文字」の行を直した。
  - `seriko.zorder` の語: `zorder_group_ledger.rs` の `interpret_token`。タグと descript は同じこの解釈器を通る（完了した `areka-P0-scope-zorder-pinning` の要件 5.2）ので、タグ `\![set,zorder,…]` も同じ決まりになる。拒否の値は作者が書いたままの綴りを載せる。RED: `t_zgp10_vocabulary_ignores_case`（「受理されるべき指定が拒否された: tokens=["Balloon0", "s1"]」）。分岐の対応表（本体の `zorder_group_branch_coverage_tests.rs`）で t_zgp10 を「解釈失敗」から「明示モード」へ移した。本体の `zorder_descript_tests.rs`・`zorder_drain_tests.rs` の拒否の例は、今も読めない `Surface0x`・`Balloon0x` に替えた（範囲の外・避けられない）。`doc/COMPAT_ARCHITECTURE.md` の「重なり指定の語の大小文字」の行を直した。
- 5 回目のレビューの提案（指摘として直した・6 回目の数には入れない）: `finish_balloon` が作者の DPI を入れ替えるとき、前と後の DPI を `info!(event = "balloon_author_dpi_swapped", from, to)` で残す（本体の `runtime/frame/switch.rs`）。テストは `switch_finish_face_tests.rs` の `balloon_switch_resolves_the_face_windowposition_with_the_new_author_dpi` に足した（RED: 記録の名を壊すと「left: [] right: [(Some("192"), Some("96"))]」）。`switch_finish_tests.rs`（999 行）には何も足していない。

#### 取り込みと範囲の大きさ

- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `246b6d75f`）。
- 範囲は 1.1 節と 18.1〜18.5 と同じ数え方で、上の裁定の直しを入れた後に測り直した。434 で分けた `persist_save.rs` が足され、ファイルは 117。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 28 | 10,654 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,327 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,057 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 373 |
| 計 | 117 | 36,462 |

- ponytail は 5.3.3（1〜5 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、`persist_save.rs` は B に入れた。6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,499・B 21／6,777・C 20／5,983・D 17／5,403・E 15／5,899・F 20／5,901。計 36,462 行で上の表と合う）。読む段では何も変えず、指摘を集めて番号を付けてから直した。
- 読み手には 5 回目と同じ線（各回の「指摘にしなかったもの」・古い文は今の事実と食い違うものだけ・決して落ちない確かめはどの直しでも赤にならない理由を言えるものだけ）と、この回の裁定の直しの中身を渡した。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,499 | 0 | 0 | 4 | 4 |
| B | 21 | 6,777 | 0 | 0 | 4 | 4 |
| C | 20 | 5,983 | 0 | 0 | 1 | 1 |
| D | 17 | 5,403 | 0 | 0 | 2 | 2 |
| E | 15 | 5,899 | 0 | 0 | 5 | 5 |
| F | 20 | 5,901 | 1 | 0 | 3 | 4 |
| 計 | 117 | 36,462 | 1 | 0 | 19 | **20** |

- 459 は A の組の読み手が文書（`doc/COMPAT_ARCHITECTURE.md`）に見つけたもので、A に数えた。475 は A の組の読み手が F の組のファイル（`dpi_sync_tests.rs`）に見つけたもので、F に数えた（433 と同じ扱い）。463 は B の組の読み手が文書に見つけたもので、B に数えた。2 つの組が同じ所を出したものは無い。

#### 指摘の一覧（456〜475）

扱いの印は 18.1〜18.5 と同じ。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 456 | できれば | A | T-R5 の試験台の doc が「上・左・右の辺は `defaultx`・`defaulty` のどちらも使わない」と書くが、上の辺の x は下端と同じ連なりで `defaultx` を使う（429 で書いた文） | `resolver_resolve_tests.rs` の `seam_cfg` | 直した（左右の辺はどちらも使わず、上の辺は `defaulty` を使わない、に） |
| 457 | できれば | A | `classify_x_vocab` の doc が「完全一致のみ」と書いた直後に「大文字小文字は問わない」と続き、述語の抜けた文もある（この回の裁定の直しが作った古さ） | `windowposition.rs` | 直した（「1 度だけ小文字にした値が … と完全一致するときだけ」の 1 文に） |
| 458 | できれば | A | 偽の窓の手を付ける手書きが、共有の `test_support::give_fake_handles` の写し（439 の残り） | `script_alignment_tests.rs` | 直した（`give_fake_handles` の 1 行。`HWND`・`WindowHandle` の import を消した） |
| 459 | できれば | A | 「6 値の語の大小文字」の行が、根拠に「本表の既存の先例（`windowposition.x` のキーワード・重なり指定の語）にも揃う」を挙げるが、その 2 行はこの回の裁定で大小を問わない形になった | `doc/COMPAT_ARCHITECTURE.md` の 6 値の語の行 | 直した（2 つの先例は裁定で改め、6 値の語は改めていない、と書いた） |
| 460 | できれば | B | ドラッグや連れ動きの保存で印を外したときの `info!` を、どのテストも見ていない（消しても緑） | `persist_save.rs`・`persist_finalize_save_tests.rs` | 直した（`clamped_boot_drag_save_adds_no_mark_to_old_memory` で、寄せた起動では 1 行・寄せない起動では 0 行を確かめる） |
| 461 | できれば | B | 1 つの決めが 2 行になる。寄せた起動の並べ終えた時点の保存で、その関数の行と、送り口が隣の位置から足した印を外したときの行が両方出る。送り口の行の `scopes` は飛ばす候補の全部で、実際に外したスコープではない | `persist_save.rs` | 直した（送り口の本体 `send_entries` が実際に外したスコープだけを載せ、並べ終えた時点の保存はもう記録した組を渡して重ねない。`persist_entries` の形は同じ。RED: 「left: 2 right: 1」（行の数）・「left: [Some("{1, 2}")] right: [Some("{1}")]」（`scopes`）） |
| 462 | できれば | B | テストの doc に、spec の名前の無い指摘番号「開発者の裁定 434」「434 の飛ばし方」（165 の残り） | `persist_finalize_save_tests.rs` | 直した（番号を消した） |
| 463 | できれば | B | 434 で `persist.rs` を分けたので、`load_restored_state` の `warn!(` を指す行の番号が 1 つずれた（322 と同じ所。この回の直しが作った古さ） | `doc/ukadoc-coverage/briefing-assets.md` | 直した（`persist.rs:308`） |
| 464 | できれば | C | `read_balloon_syncscale` の doc が「`true` と完全に一致するときだけ有効」のまま（この回の裁定の直しが作った古さ） | `descript_window_keys.rs` | 直した |
| 465 | できれば | D | 経路の語のテストの理由が、tools/perf の `parse_fields` が値を空白で割ると書くが、今の `parse_fields`（`tools/perf/perf_lines.py`）は次の名前の直前まで（空白を含む）を値にする | `diag_tests.rs` | 直した（手順書の grep が `route=<語>` を 1 語として当てる、を理由にした） |
| 466 | できれば | D | 「設計の決め 12」に spec の名前が無く、このファイルが名指す spec のどれにも無い（425 の残り） | `spawn.rs` のモジュールの doc | 直した（`areka-P0-window-link-directives の設計の決め 12`） |
| 467 | できれば | E | t_zgp10 は受理を確かめるテストになったのに、「拒否④」の節にあり、頭の「拒否分岐の檻はすべて `expect_reject` を通す」と食い違う（この回の裁定の直しが作った古さ） | `zorder_group_ledger_tests.rs` | 直した（「入力の前処理」の節へ移した。中身は同じ） |
| 468 | できれば | E | 実窓の檻が、本体の指令消化の相の `publish_chain_plan` の規則を手で写す（本番の規則を変えても緑のまま「同じ規則」と言い続ける） | `zorder_chain_inversion_order_tests.rs`・本体の `runtime/frame/zorder_drain.rs` | 直した（`publish_chain_plan` を `zorder_chain_compose` の公開の関数へ移し、本体の相と檻の両方が呼ぶ。公開は足すだけ（要件 8.2）。モジュールの doc の「World を触らない」を、鎖の導出に限ると直した） |
| 469 | できれば | E | モジュールの doc の規則 ⑴⑵⑶ と、本体の注釈の ⑴⑵⑶ が別の規則を指す | `zorder_chain_compose.rs` | 直した（本体の注釈を doc にそろえ、繋ぎの段の数字を外した） |
| 470 | できれば | E | t_zpd30 の doc が「書込の効果を 1 つも生まない」と書くが、確かめは `NotSettable` の 1 つを求める（453 の残り） | `zorder_property_deferral_tests.rs` | 直した |
| 471 | できれば | E | `set_effects` の doc が先送りしたプロパティ専用と書くが、名前は引数で、対照の名前にも使う | `zorder_property_deferral_tests.rs` | 直した |
| 472 | 必ず直す | F | シェルを切り替えると、バルーンの `windowposition` を今出している面でなく面 0 の値で解き直し、`BalloonLimit` も面 0 の値で入れ直し、標準の位置の材料も面 0 の値に替わる。たとえば面 2 の設定ファイルだけが `windowposition.x,200`・`windowposition.limit,0` を持つバルーンで `\b2` を出してシェルを替えると、ずらしの +200 が消え、画面の内へ引き戻される。面 2 のままのあいだは直らない（面の書き手は面の番号が変わったときだけ動く） | 本体の `runtime/frame/switch.rs` の `finish_shell`（切り替えの読み込み `runtime/switch_assets.rs` の `build_shell` は面 0 だけを読む）・`reseed.rs` の `BalloonPlacementInputs` の doc | 直した（後始末で、読み込みの値を今出している面の値（`balloon_definitions[scope].for_face(face)`・面の書き手と同じ取り方）へ替えてから解き、同じ値を材料へ入れる。RED: `switch_finish_face_tests.rs` の `shell_switch_resolves_with_the_current_face_windowposition` が「left: (… offset: PointPx { x: 464, y: 40 } …, BalloonLimit(true), {}) right: (… offset: PointPx { x: 504, y: 40 } …, BalloonLimit(false), {0: …, 1: …})」で赤） |
| 473 | できれば | F | 偽の窓の手を付ける手書きの繰り返しが `give_fake_handles` の写し（439 の残り） | 本体の `reset_balloon_pos_tests.rs` | 直した |
| 474 | できれば | F | `world_from` の手を付ける繰り返しと私有の `fake_handle` が、共有の `give_fake_handles`・`fake_handle` の写し（439・411 の残り） | `reseed_test_support.rs` | 直した（共有のものを使い、私有の `fake_handle` を消した。17.4 は「対の写しが追随の側にある」を理由に残したが、その後に共有の `test_support` に同じ関数ができたので、その理由は当てはまらない。`position_of` は共有の側に無いので残す） |
| 475 | できれば | F | 偽の窓の手を手で書く（`test_support::fake_handle` の写し・439 の残り） | `dpi_sync_tests.rs` | 直した |

- 申し送りとの対応（要件 1.6）: 1〜455 と同じ所を指す指摘は 0 件（463 は 322 と同じ行を指すが、この回の分割で番号がまたずれたもの）。前の回の直しの残りは新しく数え、どの番号の残りかを一言に書いた（458・462・466・470・473・474・475）。457・459・464・467・463 はこの回の裁定の直しが作った古さ。
- 472 は要件 5 に当たるかを確かめた: 完了した `balloon-scroll-arrows` の要件 1.7（「`\b[ID]` などでバルーンの面が替わった … すべての鍵 … にその面の値を使う」）に、シェルの切り替えの後の今のコードが反しているので、完了した要件に合わせる直しで、要件 5 の 3 種類には当たらない（454・391 と同じ判断）。進行役が直した。
- 468 は公開の関数を 1 つ足す（要件 8.2 が許す足し算だけ）。本体の `runtime/frame/zorder_drain.rs` は範囲の外で、移した関数を呼ぶ形に替えただけ。

#### 指摘にしなかったもの

- `test_crash_report.rs` の「件 3」「タスク 2.4」「設計の議題 2」（88 の裁定の写しの中身）。
- `zorder_group_ledger_rebase_tests.rs:2` の「task 4.1」（同じ括弧に `stay-on-top-directives` がある）。
- 重なり順の指定の大きさに上限が無いこと（要件 3.7 の決めた形・2 回目の線）。`interpret_token` がトークンごとに小文字の文字列を作ること・`classify_x_vocab` が呼ぶたびに作ること（毎フレームの道ではない）。全角の英字を小文字にしないこと（前から拒否）。
- 本体の `runtime/zorder_cue_tests.rs` の t_zcs3 の `Surface0`（その層はトークンを解釈せず、doc は今も正しい）。
- 434 の `shortcut:` の限り（寄せた起動で動かしたスコープの印を、後の隣の移動で書き直さない）。書いた限りで、寄せた起動のあいだに両隣を続けて動かすときだけ届く。
- 寄せた起動で、動かした窓自身の印を寄せた隣の矩形で判じること（5 回目の線・要件 6.2 は位置の書き込みのたびに印を書く）。
- `persist_save.rs` の doc の [`apply_restored_placements`] が子のモジュールからは解けない rustdoc のリンクであること（3 回目の線・`cargo doc` の門が無い）。`chain_realign.rs` の「`persist::push_adjacency_marks`」（リンクでなく名前で、grep で見つかる）。
- `balloon_syncscale.rs` の基準を入れ直す時の一覧にバルーンの切り替えが無いこと（「これだけ」とは言っていない）。
- 組から上がって、確かめて外したもの: `measure_tests.rs` のテスト名の「interim」・`measure.rs` の `atlas().clone()`（中身は `Arc`）・`config.rs` の `parse_i32` が空白を除かない（kv の層が除く）・`boot_pair_decision` が組の窓にも `JoinLeft`（設計の表どおり）・`resolve_chain` がスコープの番号の抜けを隣どうしに連ねる（本番に抜けを作る呼び手が無い）・`dpi_sync.rs` の「後から 4 点目として加えた」（当時の経緯）・`sticky_group_ledger.rs` の「設計の決め 1・11・12」（同じモジュールが正しい spec を名指す）・`apply_dpi_phase_gate` の時間切れの後のめぐり（めぐらない）・`script_alignment` の「design 議題 2」（ファイルの頭が spec を名指し、実在する）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 434 | 5 回目から（要件 5 の 1 種類目・3 種類目） | 寄せて表示した起動のあいだのドラッグ・連れ動きの保存で、印の無い古い記憶へ隣り合いの印を書き足すのを止めるか（18.5 の表） | 直す | 開発者の裁定（2026-11-15・推奨どおり）。直した（上の節） |
| — | 裁定の外の大小文字（18.5 の 296 の行の末尾） | `balloon.syncscale`・`windowposition.x` の語・`seriko.zorder` の語を 296 と同じ形にそろえるか | そろえる | 開発者の裁定（2026-11-15・推奨どおり）。直した（上の節） |

この回の新しい裁定待ちは 0 件。

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（持ち主の spec はもう完了しているので、8.2 が扱いを決める）: `follow/sticky_follow.rs:311` の組のドラッグの終わりは、相手ごとに `persist_entries` を 1 回ずつ呼ぶ（1 回のドラッグの終わりで記憶へ N+1 回書く）。1 つの束にまとめれば 1 回で済む（50 と同じ形）。
- `tools/perf` の持ち主へ（記録だけ）: `tools/perf/judge-perf.py:27` が `parse_fields` を「空白区切り」と書き、同じ道具の `perf_lines.py:59` と食い違う（465 と同じ種類の古い文）。
- 範囲の外で触った所: 472 と 5 回目のレビューの提案のために本体の `runtime/frame/switch.rs`・`switch_finish_face_tests.rs`、468 のために本体の `runtime/frame/zorder_drain.rs`、大小文字の直しのために本体の `runtime/frame/zorder_descript_tests.rs`・`zorder_drain_tests.rs`・`placement_tests/zorder_group_branch_coverage_tests.rs`、文書の `doc/COMPAT_ARCHITECTURE.md`・`doc/ukadoc-coverage/ledger/assets.toml`・`doc/ukadoc-coverage/briefing-assets.md`。走っている spec と重なる所は無い。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 967 件が通り 1 件 ignore、手本 2 件が通る（足した 434 の 2 本と、直しで足した確かめの差し引き）。
- `cargo test -p areka`: 本体の bin 3182 件が通り 6 件が ignore（472 と裁定の直しの記録の確かめの分だけ増えた）、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る（台帳の 4 行と briefing の行の番号を直した後）。
- `cargo test -p log-capture-kit --test file_length_guard_test --test with_default_guard_test`: 6 件と 24 件が通る（`persist.rs` 693 行・`persist_save.rs` 369 行・`lib.rs` 929 行・`persist_finalize_save_tests.rs` 686 行・本体の `switch_finish_tests.rs` 999 行（触っていない）・`switch.rs` 703 行）。
- `cargo fmt --all -- --check`: 緑。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。
- 直しは、読む段の後に進行役が組ごとに順に入れた（下請けは読むだけ）。壊し方は狙った編集で入れて狙った編集で戻し、`git checkout`・`restore`・`stash`・`reset` は使っていない。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を指す `<ファイル>.rs:<行>` の形を、ツリー全体（`.kiro/specs/`・`vendors/`・`.kiro/steering/roadmap-history.md` を除く）から拾い、`git show HEAD:` の行と今の行を見比べた。動いたのは `briefing-assets.md` の `persist.rs:309` だけで、463 で今の行に直した。

#### 判定

6 回目の新しい指摘は 20 件（必ず直す 1・直したほうがよい 0・できれば 19）で、0 件ではない（5 回目の 29 件から減った）。20 件とも直した（申し送りだけのもの・裁定待ちは 0 件）。あわせて、5 回目からの裁定待ち 434 と、裁定の外だった 3 つの鍵の大小文字は、開発者の裁定を受けて直した。

6 回目には「必ず直す」が 1 件（472・本番の振る舞い）あったが、直した。直した後に残る「必ず直す」と本番の振る舞いに関わる指摘は 0 件で、裁定待ちも 0 件。要件 6.3 とタスク 8.1 の打ち切りの形（5 回目で 0 件に収まらず、その回に「必ず直す」と本番の振る舞いに関わるものが無い）は 5 回目にも 6 回目にも当たらない（どちらの回にも 1 件ずつあった）。6 回目の指摘を直した今、残る指摘は 0 件なので、7 回目をかけるか、ここで打ち切るかを開発者に仰ぐ。

### 18.7 7 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- 6 回目の後、開発者が 7 回目を認めた（2026-11-15）。打ち切りの決まりつき: 7 回目に「必ず直す」と本番の振る舞いを変える指摘が無ければ、7 回目を最後にしてタスク 8.1 を終える。
- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `246b6d75f`）。
- 範囲は 1.1 節と 18.1〜18.6 と同じ数え方で、直す前に測り直した。ファイルは 6 回目と同じ 117（434 で分けた `persist_save.rs` は B に入る）。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 28 | 10,707 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,308 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,051 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 373 |
| 計 | 117 | 36,490 |

- ponytail は 5.3.3（1〜6 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,491・B 21／6,806・C 20／5,983・D 17／5,402・E 15／5,929・F 20／5,879。計 36,490 行で上の表と合う）。読み手は読むだけで何も変えず、進行役が指摘を集めて番号を付けてから、組ごとに順に直した。
- 読み手には 6 回目と同じ線（各回の「指摘にしなかったもの」・古い文は今の事実と食い違うものだけ・決して落ちない確かめはどの直しでも赤にならない理由を言えるものだけ）と、6 回目の直しの中身（`a42f20539`）を渡した。6 回目のレビューの任意の提案 2 つ（本体の `finish_shell` の黙った面 0 への落ち方・472 のテストが面 0 だけの束を使うこと）も候補に入れた。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,491 | 0 | 0 | 5 | 5 |
| B | 21 | 6,806 | 0 | 0 | 2 | 2 |
| C | 20 | 5,983 | 0 | 0 | 2 | 2 |
| D | 17 | 5,402 | 1 | 0 | 1 | 2 |
| E | 15 | 5,929 | 0 | 0 | 1 | 1 |
| F | 20 | 5,879 | 0 | 1 | 5 | 6 |
| 計 | 117 | 36,490 | 1 | 1 | 16 | **18** |

- 476 は A と E の 2 つの組の読み手が同じ行を出したもので、A に 1 件だけ数えた。478 は F の組の読み手が A の組のファイル（`lib.rs`）に見つけたもので、A に数えた。491 は F の組の読み手が見つけ、`persist.rs`（B）の 1 か所も含めて F に数えた。492・493 は 6 回目のレビューの提案で、本体のファイル（範囲の外）なので、触る仕組み（シェルの切り替えの解き直し）の近い F に数えた。

#### 指摘の一覧（476〜493）

扱いの印は 18.1〜18.6 と同じ。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 476 | できれば | A | `zorder_chain_compose` を「World も Win32 も触らない純関数」と書くが、468 で `publish_chain_plan(world: &mut World, …)` が同じモジュールに入った（433 で書いた文・468 の残り） | `lib.rs` の `pub mod zorder_chain_compose` の doc | 直した（鎖を組む純関数と、World に触る `publish_chain_plan` の 2 つを書いた） |
| 477 | できれば | A | `monitor_records` の doc が「`diag` は `World`・wintf の型を知らない」と書くが、`diag.rs` は wintf の `ZOrderPairStrategy` と `transition_diag::MISSING` を引く（`lib.rs` の頭と `diag.rs` の doc は例外として挙げている） | `lib.rs` の `monitor_records` | 直した（「wintf の `Monitor` を知らない」に） |
| 478 | できれば | A | `scale_then_merge` の doc が呼び手を「起動の準備とシェルの切り替えの入れ直しの 2 か所」と書くが、2 つ目の `reseed::resolve_standard_scopes` は面の `windowposition` の置き直しとバルーン位置初期化も通る（順を変える人が 3 つのうち 2 つの道を見落とす） | `lib.rs` の `scale_then_merge` | 直した |
| 479 | できれば | A | 吸着の 5 つの語（`top`・`bottom`・`left`・`right`・`free`）の読みが、descript の `Anchor::from_alignment` と台本の `parse_alignment_word` に写してある（296 で大小文字の扱いが食い違った所。語や決まりを足すとき 2 か所を直す） | `resolver.rs`・`script_alignment.rs` | 直した（`Anchor::from_word` の 1 つにし、両方が呼ぶ。知らない値の警告も「`Seam` が下端に落ちた」でなく「5 つの語のどれでもない」で判じる。`config.rs` は `Seam` へ `bottom`・`free` を入れないので結果は同じ。`config.rs` の `bottom`・`free` の振り分けは、`config` が依存の根で `resolver` を引けないので残した） |
| 480 | できれば | A | テストの補助 `one`（欄 `event` がちょうど 1 件と確かめて返す）が 2 つのファイルに同じ中身で写してある（150 の残り） | `script_alignment_tests.rs`・`sticky_group_ledger_world_tests.rs` | 直した（`shared_test_support::one_event_with` の 1 つ） |
| 481 | できれば | B | 434 で分けた後、`PersistWiring` の doc の「モジュールの doc に並べる」が `persist_save.rs` 自身の doc を指し（並べてあるのは `persist.rs`）、`persist.rs` の doc は「`PersistWiring` も本モジュールにある」のまま（435 の残り・434 の分割が作った古さ） | `persist_save.rs` の `PersistWiring`・`persist.rs` のモジュールの doc | 直した |
| 482 | できれば | B | 並べ終えた時点の保存の doc が「記憶から戻したスコープは書き換えない」「送る鍵は位置と触れる組の印」と書き、`persist.rs` の doc は「記憶に位置が無いキャラだけ」と書くが、印の無い古い記憶へは隣り合いの印を足す（dpi-realign-remembered-chain 6.5・`old_memory_gets_only_the_adjacency_mark_added` が確かめる） | `persist_save.rs` の `persist_unremembered_char_positions`・`persist.rs` のモジュールの doc | 直した |
| 483 | できれば | C | テストの doc が「本体の `areka_parsers::shell::parse` の呼び出し」と書く（このリポジトリで「本体」は本体の crate。呼び出しはこのファイルの本文にある） | `measure_tests.rs` の `emo2_utf8_fixture_decodes_to_the_same_text_as_before` | 直した（「本文の」に） |
| 484 | できれば | C | 手本の装着待ちの絵が `Option` で持たれ、`take()` と届かない `continue` は、借りたまま回す形のためだけにある | `examples/window-placement.rs` の `PendingTarget`・`boot_present_system` | 直した（`std::mem::take` で回し、絵の組を直に持つ。装着済みの門 `attached` は、待ちが初めから空のときに毎フレームの出し入れを止めるので残した） |
| 485 | 必ず直す | D | 起動時の連鎖の確定が、記憶に位置の無いスコープを全部「前の窓の x − 自分の幅」（`JoinLeft`）で解き直し、resolver の規則と食い違う。窓の幅が変わらなくても動く。⑴ `kero.defaultx,36` の 36 の隙間が最初の確定で消える（完了した `areka-P0-scope-chain-gap` の要件 2.8「`defaultx` の意味を変えない」）。⑵ `seriko.alignmenttodesktop,right`（`left` も）のスコープが辺から本体の隣へ動く（指摘 1 の直しを最初の確定で戻す。`Anchored(Right)` は残るので、次に大きさが変わると辺へ跳ぶ）。⑶ `free` で x を書いたスコープが本体の隣へ動く（P3）。⑷ 辺のスコープの次のスコープの基準が resolver と違う（resolver は辺のスコープで横の連なりを進めない） | 本体の `runtime/frame/drain_resnap.rs` の `finalize_chain_once_with`・`chain_finalize.rs` の `boot_pair_decision`・`resolve_chain` | 開発者の裁定で直した（2026-11-15「推奨どおりで」。下の「485 の直し」） |
| 486 | できれば | D | テストの窓を壊す道具が 3 つある（`spawn_test_support.rs` の `destroy_test_window`・`spawn_zorder_chain_wiring_tests.rs` の `destroy`・共有の `zorder_test_support::teardown`）。teardown の直しが写しに届かない（64 の残り） | `spawn_test_support.rs`・`spawn_zorder_chain_wiring_tests.rs`・`spawn_zorder_pair_wiring_tests.rs` | 直した（`teardown` の 1 つ。teardown の doc どおり所有される側から渡す。窓を作る `create_test_window` は非表示の窓を作るので残した） |
| 487 | できれば | E | 拒否に作者が書いたままの綴りを載せることを、配置の crate のテストが確かめない（6 回目の大小文字の裁定で t_zgp10 が受理のテストになった後、t_zgp9・t_zgp13 の値はどれも小文字にしても同じ。確かめるのは本体の 2 つのテストだけ） | `zorder_group_ledger_tests.rs` の t_zgp9 | 直した（`Balloon0x` の行を足した。RED: 拒否に小文字にした値を載せる壊し方で「left: UnparsableToken { token: "balloon0x" } right: UnparsableToken { token: "Balloon0x" }」） |
| 488 | 直したほうがよい | F | シェルの切り替えで、記憶から戻したバルーンに利用者が動かした印を付ける所（`reinstall_balloon` → `mark_by_base`）を、どのテストも確かめない。印をいつも外す壊し方で crate のテストが全部緑のまま。印が無いと、切り替えの後に面の `windowposition` が変わったとき、記憶から戻したバルーンが標準の位置へ引き戻される | `reseed.rs` の `reinstall_balloon`・`reseed_tests.rs` の t_rs02 | 直した（t_rs02 で、記憶のある scope 0 に印があり、記憶の無い scope 1 に無いことを確かめる。RED: 印をいつも外す壊し方で「記憶から戻した scope 0 に動かした印が無い」（crate の 967 本のうちこの 1 本だけが赤）） |
| 489 | できれば | F | reseed のモジュールの doc が、結果から「`Anchored`・`BalloonFollow`・`BalloonKeywordBase` だけを取り」と書くが、343 の直しで `BalloonLimit` も入れ直し、利用者が動かした印も付け外しする（343 の残り） | `reseed.rs` の頭 | 直した |
| 490 | できれば | F | `StickyGroupLedger::reject_overlap` の絞り `considered` が、2 つの呼び手のどちらでもいつも真（`replace_descript` は直前に台本の組だけを残す） | `sticky_group_ledger.rs` | 直した（引数を消した。私有の関数で、答えは同じ） |
| 491 | できれば | F | 窓の矩形を位置と大きさから手で組む式が 3 か所にあり、追随の `follow/visibility.rs` の `rect_at` の写し（`rect_at` は追随のモジュールの中に閉じている） | `balloon_limit.rs`・`reseed.rs`・`persist.rs` | 申し送り（`audit-placement-follow`。`follow.rs` の `rect_at` の再公開を `pub(crate)` にし、3 か所が使う。追随の持ち物に触るので、この spec では直さない） |
| 492 | できれば | F | 本体の `finish_shell` が、面の番号はあるが面の定義の束が無いスコープで、記録を残さずに読み込みの面 0 の `windowposition` のまま解く（6 回目のレビューの提案 ⑴） | 本体の `runtime/frame/switch.rs` の `finish_shell` | 直した（`debug!` を 1 行。面の書き手 `select_balloon_face` は束の無いスコープの面を記録しないので、ふだんは起きない） |
| 493 | できれば | F | 472 のテストが面 0 だけの束（`ScopeBalloonDefinitions::single`）を使うので、面 2 は面 0 の定義に落ち、`finish_shell` が `for_face(face)` でなく面 0 の定義を引いても緑のまま（6 回目のレビューの提案 ⑵） | 本体の `runtime/frame/switch_finish_face_tests.rs` の `shell_switch_resolves_with_the_current_face_windowposition` | 直した（本番と同じ `load_scope_balloon_definitions` で、面 0 が `windowposition.x,0`・面 2 が自分の設定ファイルで `windowposition.x,40`・`windowposition.limit,0` の束を組み、読み込みの値を面 0 の値にした。RED: 面 0 の定義を引く壊し方で「left: BalloonLimit(true) right: BalloonLimit(false)」） |

- 申し送りとの対応（要件 1.6）: 1〜475 と同じ所を指す指摘は 0 件。前の回の直しの残りは新しく数え、どの番号の残りかを一言に書いた（476・480・481・486・487・489）。481 は 434 の分割が作った古さ。492・493 は 6 回目のレビューの提案で、6 回目の数に入れなかったものをこの回で数えた。
- 485 は確かめてから載せた: 一時のテストで、resolver の出力をそのまま今の位置と既定の位置として `boot_pair_decision`・`resolve_chain` へ渡す（本体の `collect_chain_states` が渡すのと同じ形・spawn が resolver の出力を既定の位置に置く）と、3 つの場合のどれでも動かす指示が出た（「defaultx36: resolver x1=1164 but boot finalize moves [ChainMove { scope: 1, new_x: 1200 }]」・「right: resolver x1=1600 …」・「free_x: resolver x1=100 …」。どれも `new_x: 1200`＝本体の隣）。一時のテストは元のファイルの写しから戻して消した。本体の起動時の確定のテスト（`frame_chain_finalize*_tests.rs`）には、`defaultx`・`alignmenttodesktop`・左右の辺を使うものが無い。

#### 指摘にしなかったもの

- `publish_chain_plan` に配置の crate の単体のテストが無いこと（本体の `zorder_drain_projection_tests` の t_zdp06・`zorder_descript_tests.rs`・実窓の檻が 3 つの腕を通す）。
- `zorder_chain_tail_order_tests.rs` が `ZOrderChainPlan` を手で入れること（新しい World へ 1 度入れるだけで、比べの規則を写していない。468 の形ではない）。
- `AdjacencyMark::parse` が大小文字を区別すること（areka 自身の保存の語。6 回目の大小文字の裁定は descript と台本の語だけ）。
- `ScopeChainState` と `ChainSlot` が 1 つの欄だけ違うこと（どちらも公開で本体が使う。まとめると公開の形が変わる・要件 8.2）。
- `balloon_offset_persist_roundtrip_tests.rs` の `observe()` が本体の `balloon_offset_follow` の腕の分け方を写すこと（本番を変えると本体の World のテストが赤になる。まとめるには追随と本体のファイルが要る）。
- 帳簿が無いときの `replace_balloon_lock_descript`（空の帳簿を据える）と `replace_sticky_descript`（警告して何もしない）の扱いの違い（どちらの帳簿も spawn で据えるので本番では届かず、どちらも doc とテストがある）。
- シェルの切り替えの記憶の写しが背景の読み込みの時点のもので、その後に終わったドラッグを含まないこと（ミリ秒の窓で、具体の失敗を示せない）。
- `KEYWORD_BALLOON_DESCRIPT` が `use_self_alpha,1` を落とすこと（2 つの呼び手が頼らない）。
- 本体の `spawn_assembly_tests.rs` の 12 本のうち `input_events` を要るのが 1 本だけなこと（`placement-crate-split` がファイルごと移した決め）。
- `load_scope_windowpositions` の doc がシェルの切り替えでの今の面の上書き（472）を書かないこと（足りないだけで誤りではない。面の分からないスコープは今も面 0）。
- 寄せた起動での組のドラッグが相手ごとに `persist_entries` を呼ぶこと（6 回目の申し送り。寄せた起動でも答えは順に依らない）。
- 組から上がって、確かめて外したもの: `resolver_resolve_test_support.rs` の再公開（307 で選んだ形）・`keyword_balloon_pos` の 2 つ目の使い手の名指し（間接だが正しい）・定数と定数の `assert_within_1px`（4 回目の線）・`build_measure_scaling` の作者の DPI 0 の `error!`（`parse_author_dpi` が 0 を返さないので届かない）・`virtual_desktop_union` の本番の呼び手の無さ（DD8 の先送りの印）・`Cargo.toml` の `tracing-subscriber` の説明が手本の使い道を書かないこと（足りないだけ）・`test_support.rs` の面 10 の寸法の文（検体を測って正しい）・`measure.rs` の段の説明が `measure_baked` の 2 段下の繰り返しを `measure_native_scope_sizes` に帰すこと（段の説明として正しい）・手本の `smoke_exit_ms` が本体の写しであること（手本は本体の crate を引けない）・`descript_window_keys_tests.rs` の `1usize << 32`（32 ビットの組み立ては無い）・`zorder_chain_tail_order_tests.rs` の見出しの改行（`cargo doc` の門が無い）・`ent()` の 2^31 以上のスコープ（テストの小さな値だけ）・`dpi_sync` の 5 つの `HoldSite` と呼び所（grep で合う）・`reseed_tests.rs` の「設計の決め 12」（親の `reseed.rs` が spec を名指す・6 回目の線）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 485 | 要件 5 の 3 種類目（起動のときの窓の出る位置が変わる・SSP との互換。210 の SSP の測りは、`alignmenttodesktop,right` のスコープを右の辺に置く）と要件 8.6（直し方によっては公開の `ChainSlot`／`PairDecision` の形が変わる＝要件 8.2）。直しそのものは、完了した `areka-P0-scope-chain-gap` の要件 2.8（`defaultx` の意味を変えない）・P3（Free の x）と、この spec の指摘 1（4 つの辺で解く）に合わせるもので、1 種類目には当たらない | 起動時の連鎖の確定で、記憶に位置の無いスコープを resolver と同じ規則で解き直すか。たとえば `kero.defaultx,36` のゴーストを記憶なしで起こすと、今は最初の確定で kero が 36px 右へ動き本体に付く。`kero.seriko.alignmenttodesktop,right` なら、右の辺から本体の隣へ動く | **直す**: 記憶に位置の無いスコープの決めを、そのスコープの揃え方と `defaultx` から作る。下端・上端と x の無い Free は「前の窓の解いた後の x − 自分の幅 − `defaultx`」、左右の辺のスコープは動かさず横の連なりの基準も進めない（resolver と同じ）、x を書いた Free は動かさず今の x を基準にする。揃え方と `defaultx` は `DefaultPlacementBasis` から `build_placement_config` で引く（`reset_pos::reset_window_positions` と同じ取り方）。公開の形は足すだけにする（たとえば `PairDecision` へ腕を足すなら、範囲の外で網羅して match する所が無いことを先に grep する）。RED を先に（`defaultx`・右の辺・x を書いた Free の 3 つを本体の `frame_chain_finalize_tests` に）。本体の `drain_resnap.rs`（範囲の外）と `chain_finalize.rs`・その doc の式の文を直す | 開発者の裁定で直した（2026-11-15「推奨どおりで」。推奨案のとおり。下の「485 の直し」） |

この回の新しい裁定待ちは 1 件（485）で、開発者の裁定（推奨どおり）で直した。

#### 485 の直し（開発者の裁定・2026-11-15）

- 決めの置き場: `chain_finalize.rs` に `boot_default_decision(anchor, default_x)` を足した。記憶に位置の無い、既定のままのスコープ（`boot_pair_decision` が `JoinLeft` を返したもの）の決めを、resolver（`resolve_placement` の P1'・P2・P3）と同じ揃え方の規則で作る。下端・上端・x の無い Free は `JoinLeft`（`defaultx` が 0 以外なら新しい腕 `PairDecision::JoinLeftOffset { offset }`＝「前の窓の解いた後の x − 自分の幅 − `defaultx`」）、x を書いた Free は `Keep(FreePosition)`、左右の辺は `Keep(DesktopEdge)`。`defaultx` は DPI で換算しない生の値（resolver と同じ）で、上端も下端と同じ式（resolver の `Anchor::Top => (bottom_x, top)`）。
- 呼び手: 本体の `drain_resnap.rs` の `finalize_chain_once_with` が、揃え方と `defaultx` を `DefaultPlacementBasis` から `build_placement_config` で引き（`reset_pos::reset_window_positions` と同じ取り方）、既定のままのスコープの `JoinLeft` を `boot_default_decision` で改める。左右の辺のスコープは解きの列（`slots`・`states`・`targets`）から外して `debug!` を 1 行残す＝動かさず、次のスコープの基準にもしない（resolver が横の連なりを進めないのと同じ）。次が記憶から戻したスコープのときだけは、覚えた印がこの窓との隣り合いなので `Keep(DesktopEdge)` のまま列に残す。材料（`DefaultPlacementBasis`）が無いときは `warn!` を 1 行出し、下端・`defaultx` 0（今までの解き）で解く。
- 解き: `resolve_chain` と `join_usable`・`as_str` に `JoinLeftOffset` を足した（語は `left-offset`、`KeepReason` の語は `desktop-edge`・`free-position`）。作業領域へ寄せる追随の `place_chain_in_work_area` は 2 つのスロットを `resolve_chain` に渡して隣の x を得るので、式は写していない。追随の `opposite`（並べ替えの反対の側）は `keep => keep` の残りの腕で `JoinLeftOffset` をそのまま返す（並べ替えは同じ位置になり、入らなければ端へ寄せる＝resolver の P4 のクランプと同じ向き）。追随の持ち物は 1 行も触っていない。
- 公開の形は足しただけ（`PairDecision::JoinLeftOffset`・`KeepReason::DesktopEdge`・`KeepReason::FreePosition`・`boot_default_decision`）。足す前にツリー全体で `PairDecision`・`KeepReason` を網羅して match する所を grep した: `chain_finalize.rs`（直した）・`follow/work_area_clamp.rs` の 2 つ（どちらも残りの腕があり、そのまま組める）・テストの語の表（`keep_reason_fixed_words`・`pair_decision_words` に足した）。
- RED（直す前のコードで、本体の新しい `frame_chain_finalize_align_tests.rs` の 5 本がどれも赤）: ⑴ `defaultx` 36 の隙間「left: 2168 right: 2132」・⑵ 右の辺「left: 2168 right: 2602」・⑶ x を書いた Free「left: 2168 right: 100」・⑷ 辺のスコープの次の基準「left: 2135 right: 2602」・⑸ 幅が本当に変わったときの直し「left: 2113 right: 2077」（2168・2113 は本体の隣＝隙間 0 に付けた値）。窓は本番と同じく resolver の出力で生やし、材料も起動の準備と同じ形で置く。
- 配置の crate のテスト: `chain_finalize_resolve_tests.rs` に `join_left_offset_keeps_the_defaultx_gap`（式と非正の幅）と `boot_default_decision_follows_the_resolver_rules`（5 つの揃え方と `defaultx` の有無）を足した。
- 今のテストで直したもの: 本体の `frame_chain_finalize_tests.rs` の `finalize_within_the_bounded_wait_emits_no_diagnostic` だけ。このテストは「正常な起動では WARN が 0 行」を確かめるが、使う `resnap_world` が並べ直しの材料を置かないので、材料の無いときの新しい `warn!` が 1 行出て赤になった。正常な起動は材料を置くので、このテストにだけ材料（`frame_test_support.rs` に足した `placement_basis`）を置いた。`resnap_world` そのものには置かない（材料が無いことを前提にする檻が `frame_drain_text_tests.rs`・`frame_move_order_tests.rs` にある）。今までの起動時の確定のテストは全部下端・`defaultx` 無しなので、決めも位置も変わらない（壊れた振る舞いを固定していたテストは無かった）。
- 範囲の外で触った所: 本体の `runtime/frame/drain_resnap.rs`・`frame.rs`（テストの module を 1 つ）・`frame_test_support.rs`（`placement_basis`）・`frame_chain_finalize_tests.rs`（上の 1 本）・新しい `frame_chain_finalize_align_tests.rs`。`drain_resnap.rs` の行が動いたので、そこを行の番号で指す本体の `zorder_drain.rs` の「`drain_resnap.rs:79-87`」（直す前から古かった）を関数の名前（`run_move_drain_with`）に替えた。

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（持ち主の spec はもう完了しているので、8.2 が扱いを決める）: 491（`follow.rs` の `rect_at` の再公開を `pub(crate)` にし、`balloon_limit.rs`・`reseed.rs`・`persist.rs` の手で組む矩形の式を `rect_at(pos, size)` に替える）。
- 範囲の外で触った所: 492・493 のために本体の `runtime/frame/switch.rs`・`switch_finish_face_tests.rs`（`switch_finish_tests.rs`（999 行）には何も足していない）、行の番号を直した本体の `examples/collision-probe.rs`（`lib.rs:581`・`lib.rs:382-456`・`lib.rs:395-406`）と `doc/ukadoc-coverage/briefing-assets.md`（`persist.rs:310`・`resolver.rs:293`）。走っている spec と重なる所は無い。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 967 件が通り 1 件 ignore、手本 2 件が通る（テストの数は 6 回目と同じ。この回は今のテストに確かめを足した）。
- `cargo test -p areka`: 本体の bin 3182 件が通り 6 件が ignore、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る（briefing の行の番号を直した後）。
- `cargo test -p log-capture-kit --test file_length_guard_test --test with_default_guard_test`: 6 件と 24 件が通る（`lib.rs` 931 行・`persist.rs` 695 行・`persist_save.rs` 372 行・`resolver.rs` 542 行・本体の `switch.rs` 710 行・`switch_finish_face_tests.rs` 234 行・`switch_finish_tests.rs` 999 行（触っていない））。
- `cargo fmt --all -- --check`: 緑。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。
- 壊し方は狙った編集で入れて狙った編集で戻し（485 の確かめの一時のテストは、元のファイルの写しから戻した）、`git checkout`・`restore`・`stash`・`reset` は使っていない。読み手の下請けは読むだけで、直しは進行役が順に入れた。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を指す `<ファイル>.rs:<行>` の形を、ツリー全体（`.kiro/specs/`・`vendors/`・`.kiro/steering/roadmap-history.md` を除く）から拾い、`git show HEAD:` の行と今の行を見比べた。動いたのは上の `collision-probe.rs` の 3 つと `briefing-assets.md` の 2 つだけで、どれも今の行に直した。

- 485 の直しの後（2026-11-15）: `cargo test -p areka-placement --features test-support` の lib 969 件が通り 1 件 ignore（足した 2 件）・手本 2 件、`cargo test -p areka` の本体の bin 3187 件が通り 6 件が ignore（足した 5 件）・ほかのテストの的も全部通る（知られた揺れは出なかった）、`cargo test -p areka-sylphya` 221 件、`cargo test -p ukadoc-survey` 全部、1,000 行の番人 6 件と 24 件（`chain_finalize.rs` 628 行・`chain_finalize_resolve_tests.rs` 381 行・本体の `drain_resnap.rs` 780 行・`frame_test_support.rs` 931 行・`frame_chain_finalize_align_tests.rs` 182 行）、`cargo fmt --all -- --check` が緑。`git diff --stat Cargo.lock` と `vendors/` の差分は空、追随の持ち物は 1 行も触っていない。

#### 判定

7 回目の新しい指摘は 18 件（必ず直す 1・直したほうがよい 1・できれば 16）で、0 件ではない（6 回目の 20 件から減った）。直した 16 件・申し送りだけのもの 1 件（491）・裁定待ち 1 件（485）。

7 回目には「必ず直す」が 1 件（485）あり、これは本番の振る舞い（記憶の無い起動の窓の並び）を変える指摘で、直さずに裁定待ちとして残る。ほかの 17 件は本番の振る舞いを変えない（492 は記録 1 行を足すだけで、ふだんは届かない）。したがって、開発者の決めた打ち切りの形（7 回目に「必ず直す」と本番の振る舞いを変える指摘が無ければ 7 回目で終える）には当たらず、タスク 8.1 はまだ終わらない。485 の裁定（直すか、直し方を変えるか、ほかへ回すか）と、その直しを入れた後に 8 回目をかけるかを開発者に仰ぐ。

485 は開発者の裁定（2026-11-15「推奨どおりで」）で推奨案のとおりに直した（上の「485 の直し」）。これで 7 回目の 18 件は、直した 17 件と申し送りだけのもの 1 件（491）になり、裁定待ちは 0 件。485 の直しは本番の振る舞い（記憶の無い起動の窓の並び）を変えたので、打ち切りの形にはまだ当たらない。この直しを入れた後に 8 回目をかけるかを開発者に仰ぐ。

### 18.8 8 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- 485 の直しの後、開発者が 8 回目を認めた（2026-11-15）。打ち切りの決まりつき: 8 回目に「必ず直す」と本番の振る舞いを変える指摘が無ければ、8 回目を最後にしてタスク 8.1 を終える。
- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `246b6d75f`）。
- 範囲は 1.1 節と 18.1〜18.7 と同じ数え方で、直す前に測り直した。ファイルは 7 回目と同じ 117。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 28 | 10,761 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,379 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,051 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`） | 1 | 373 |
| 計 | 117 | 36,615 |

- ponytail は 5.3.3（1〜7 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,509・B 21／6,811・C 20／5,981・D 17／5,500・E 15／5,931・F 20／5,883。計 36,615 行で上の表と合う）。読み手は読むだけで何も変えず、進行役が指摘を集めて番号を付けてから、触るファイルが重ならないように順に直した。
- 読み手には 7 回目と同じ線（各回の「指摘にしなかったもの」・古い文は今の事実と食い違うものだけ・決して落ちない確かめはどの直しでも赤にならない理由を言えるものだけ）と、7 回目の直しの中身（`f594e7e4a`・485 の直しの `20223ddd3`）を渡した。485 のレビューが残した 2 つ（⑴ スコープ 0 が左右の辺のとき、次の既定のスコープが鎖の先頭になり、出ている幅が準備の幅と違っても直されない・⑵ 覚えた辺のスコープが、その後の覚えていないスコープの基準になる）も候補に入れた。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,509 | 0 | 0 | 2 | 2 |
| B | 21 | 6,811 | 0 | 0 | 0 | 0 |
| C | 20 | 5,981 | 0 | 0 | 1 | 1 |
| D | 17 | 5,500 | 1 | 1 | 3 | 5 |
| E | 15 | 5,931 | 0 | 0 | 0 | 0 |
| F | 20 | 5,883 | 1 | 0 | 0 | 1 |
| 計 | 117 | 36,615 | 2 | 1 | 6 | **9** |

- 494 は B・D・F の 3 つの組の読み手が同じ仕組みを出したもので、本番で困る所（拡大率の変化の後の詰め直し）が F の `chain_realign.rs` にあるので F に 1 件だけ数えた。495 は D の組の読み手と 485 のレビューの残り ⑵ が同じ所を指すもので、D に 1 件だけ数えた。496〜498 の場所の一部（本体の `drain_resnap.rs`・`frame_chain_finalize_*_tests.rs`）は範囲の外で、485 の直しが足した所なので D に数えた。

#### 指摘の一覧（494〜502）

扱いの印は 18.1〜18.7 と同じ。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 494 | 必ず直す | F | 485 の直しの規則（`defaultx` の隙間を残す・左右の辺のスコープを横の連なりの基準にしない）を、隣り合いの印の保存と拡大率の変化の後の詰め直しが知らない。どちらも今も組 (n−1, n) を 2px の幅で判じる。⑴ `kero.defaultx,36` のゴーストを記憶なしで起こすと、起動の確定は 36 の隙間を残し、並べ終えた時点の保存は隙間 36 を「離れている」と判じて印 `none` を書く。100% から 200% へ移ると、2 つの窓が下端の中央を保って広がり、隙間は 36 − 191 − 168 = −323（emo2 の幅）。詰め直しは変わる直前の隙間 36 を `apart` と判じて動かさないので、2 つの窓が 323px 重なったまま残る。次の起動からは印 `none` なので、本体の幅が保存のときと違っても kero は付いてこない。485 の前は起動の確定が隙間を 0 にしたので、印は `left` で、詰め直しも付け直していた。⑵ `kero.seriko.alignmenttodesktop,right` でスコープ 2 が下端のとき、起動の確定はスコープ 2 を本体の隣に置くが、印と詰め直しは組 (1, 2) を判じて `none`／`apart` にする。拡大率が上がると本体とスコープ 2 が重なったまま残る | `chain_realign.rs` の `realign_chain_once_with`・`persist_save.rs` の `push_adjacency_marks`・`chain_finalize.rs`（規則の置き場）・本体の `runtime/frame/drain_resnap.rs` の `finalize_chain_once_with` | 開発者の裁定で直した（2026-11-15「推奨どおりで」。下の「494・495 の直し」） |
| 495 | 必ず直す | D | 起動の確定が、左右の辺のスコープを鎖から外すかを、窓の `Anchored` でなく起動の決めで判じる。決めが `Keep(DesktopEdge)` になるのは記憶に位置が無く既定のままのスコープだけなので、⑴ 辺のスコープを覚えていて次のスコープを覚えていないとき（次のスコープが前の回に台本で置かれて書かれなかった・スコープの増えたシェルへ替えた）、⑵ 起動の台本 `\![set,alignmenttodesktop,left]` が最初の確定より前に kero を辺へ動かしたとき（決めは `ExplicitlyPlaced`）、辺のスコープが次のスコープの基準のまま残る。⑴ では次のスコープが辺の窓の隣へ動き（右の辺なら本体と重なる位置）、並べ終えた時点の保存がその位置を書く。⑵ では左の辺の隣に入らず右へ反転して、本体から離れる（485 のレビューの残り ⑵） | 本体の `runtime/frame/drain_resnap.rs` の `finalize_chain_once_with` の辺のスコープを外す所・`chain_finalize.rs` の `boot_default_decision` | 開発者の裁定で直した（2026-11-15「推奨どおりで」。494 と同じ規則・下の「494・495 の直し」） |
| 496 | 直したほうがよい | D | 起動の確定の「辺のスコープの次が記憶から戻したスコープなら、辺のスコープを鎖に残す」枝にテストが無い。条件を消しても全部のテストが緑（記憶のテストは `DefaultPlacementBasis` を置かず下端に落ち、揃え方のテストには記憶が無い） | 本体の `runtime/frame/drain_resnap.rs` の `finalize_chain_once_with`・`frame_chain_finalize_align_tests.rs` | 直した（`boot_finalize_keeps_a_side_edge_scope_as_the_base_of_a_remembered_next_scope`。条件を消す壊し方で「left: 2146 right: 2302」の赤。今の振る舞いを固定するテストで、494・495 の裁定で規則が変われば期待も変わる） |
| 497 | できれば | D | `PairDecision`・`ChainSlot.decision`・`resolve_chain` の doc が「組 (n−1, n)」「本体から昇順に解き」「先頭（本体）」と書くが、485 の直しの後は呼び手が左右の辺のスコープを鎖から外すので、組は前のスロットとの組で、先頭は本体とは限らない（同じファイルの 358 行はもう外すと書く・485 の残り） | `chain_finalize.rs` | 直した（「前のスロットとの組」「先頭のスロット」に） |
| 498 | できれば | D | spec の名の無い指摘番号「指摘 485」が、doc・コメント・記録の文に入っている（165 の残り。記録の 2 つは利用者の記録に出る） | `chain_finalize.rs`・`chain_finalize_resolve_tests.rs`・本体の `drain_resnap.rs`（`warn!`・`debug!` の文を含む）・`frame_chain_finalize_tests.rs` | 直した（番号を消した。文が中身をもう言っている。記録の文を字面で読む所が無いことをツリー全体の grep で確かめた。spec の名の付いた `audit-placement-core 指摘 485` の 3 か所は残した） |
| 499 | できれば | D | 486 で私有の `destroy` を消した後、見出しだけで中身の無い節が残る | `spawn_zorder_chain_wiring_tests.rs` | 直した（見出しの 4 行を消した。道具の在りかは `use` の行が言う） |
| 500 | できれば | A | `Anchor::from_alignment` の doc が `Seam` の `top`／`left`／`right` のほかは `Bottom` と書くが、479 の後は `from_word` を通るので `Seam("free")` は `Free` になる（479 の残り） | `resolver.rs` | 直した（5 つの語はそれぞれのアンカー、ほかは `Bottom`、に） |
| 501 | できれば | A | 10 の欄の `ScopePlacement` を手で書く写しが残る（共有の `placement_sized` と違うのは `anchor` だけ・313 の残り） | `script_alignment_tests.rs` | 直した（`placement_sized` に `anchor` だけを足す形） |
| 502 | できれば | C | 同じ descript の読めない値の `warn!`（`defaultx,abc` の数値化の失敗・知らない `balloon.alignment`）が、1 回の起動で 2 行出る（準備と、485 で足した起動の確定がそれぞれ `build_placement_config` を呼ぶ）。シェルの切り替えでも 2 行出る（`reseed` と本体の `switch.rs` の後始末・前からの形） | `config.rs` の `warn!`・本体の `drain_resnap.rs`・`switch.rs` の `build_placement_config` の呼び出し | 開発者の裁定で今のまま（2026-11-15「推奨どおりで」。下の裁定の表） |

- 申し送りとの対応（要件 1.6）: 1〜493 と同じ所を指す指摘は 0 件。前の回の直しの残りは新しく数え、どの番号の残りかを一言に書いた（494・495・497＝485、498＝165、499＝486、500＝479、501＝313）。
- 494・495 は要件 5 に当たるかを確かめた: 494 は完了した `dpi-realign-remembered-chain` の決めの表（変わる直前に「離れていた → 動かさない」・印は組 (n−1, n) の隣り合い）を変える直しで、485 の前はその表のままで困らなかった（起動の確定がいつも隙間を 0 にしたので）。どちらも利用者に見える窓の並び（拡大率の変化の後・次の起動）が変わる。要件 5 の 1 種類目と 3 種類目に当たるので、進行役は直さずに裁定を仰ぐ。

#### 指摘にしなかったもの

- 485 のレビューの残り ⑴（スコープ 0 が左右の辺のとき、次の既定のスコープが鎖の先頭になり、出ている幅が準備の幅と違っても右の辺へ直されない）。鎖の先頭を動かさないのは `areka-P0-scope-chain-gap` の決め（`resolve_chain` は先頭を読まない）で、スコープ 0 が下端の先頭のときも同じく、置き直しが下端の中央を保つので右の辺から幅の差の半分だけ離れる。スコープ 1 が先頭のときも同じ扱いで、新しい振る舞いではない（485 の前は、辺のスコープ 0 の隣へ付けていた＝resolver と違う形）。
- `send_entries` の `logged` を集合でなく真偽にする案（B の組）。今の呼び手では、並べ終えた時点の保存の `unmarked` と送り口の飛ばす組がいつも同じだが、2 つの決め方がずれたとき、集合の形なら記録が欠けない。減るのは 2 行ほど。
- 組から上がって、確かめて外したもの: `opposite(JoinLeftOffset)` が反転しない（7 回目の 485 の直しの記録に書いた決め・追随の持ち物）・`defaultx` を DPI で換算しない（resolver と同じ）・起動の確定の中の `build_placement_config`（窓の一式ごとに 1 度）・材料が無いときの `warn!`（本番は窓と同じ命令で材料を置く）・最初の確定より前のシェルの切り替え（材料と揃え方が両方入れ替わる）・`boot_chain` の「同じ順」（足りないだけ）・`pair_decision_words` の doc（足りないだけ）・`resolve_placement` が `Seam` の値を 2 度小文字にする（解くたびの 1 度・毎フレームではない）・`resolver_resolve_balloon_tests::scope_cfg_balloon`（`..default` で 1 ファイルの中）・`placement_monitor_tests::make_monitor`（`bounds` と作業領域の取り違えを捕まえるためにわざと違う）・`restored_windows` が知らない印で警告しない（記録が 1 行増えるだけ）・`persist_remove_tests` の `KINDS`（440 で受け入れた限り）・`check_table` が見出しと取り込むファイルの食い違いを捕まえない（今の表は全部合う）・t_zcc16 の不変条件の確かめ（359 の後に残した較正）・`descript` の鍵の大文字小文字（6 回目の裁定は値だけ・SSP の差を示せない）・`runtime/assets.rs` の `initial_surface_id`（範囲の外・同じ面に落ちる）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 494・495 | 要件 5 の 1 種類目（完了した `dpi-realign-remembered-chain` の決めの表＝変わる直前に「離れていた → 動かさない」、印は組 (n−1, n) の隣り合い）と 3 種類目（拡大率の変化の後と次の起動の窓の並び）。直しは、完了した `areka-P0-scope-chain-gap` の要件 2.8（`defaultx` の意味を変えない）と、この spec の指摘 1・485（resolver と同じ揃え方の規則）に合わせるもの | 485 の規則を、隣り合いの印の保存と拡大率の変化の後の詰め直しにも当てるか。たとえば `kero.defaultx,36` のゴーストで 100% から 200% へ移ると、今は 2 つの窓が 323px 重なったまま残る | **この spec で直す**（485 の直しが作った残りで、ほかへ回すとその間 `main` に残る）。`chain_finalize.rs` に 1 つの規則を置き、3 か所（起動の確定・隣り合いの印・詰め直し）が呼ぶ。⑴ 鎖の前の窓は「自分より下の番号で、窓の `Anchored` が左右の辺でない一番近いスコープ」（`Anchored` は配置の唯一の正本・495 の 2 つの場合も塞がり、`drain_resnap.rs` の「次が記憶なら残す」の枝は消える）。⑵ 組の「隣り合う」は、隙間が 0 か、そのスコープの `defaultx`（`DefaultPlacementBasis` から `build_placement_config` で引く）の ±2px。`defaultx` の隙間の組は印 `left` を書き、起動と詰め直しは `JoinLeftOffset { defaultx }` で解く（覚えた印 `left` も、`defaultx` が 0 でなければその隙間で付ける）。規模は M（本体の `drain_resnap.rs`・`chain_realign.rs`・`persist_save.rs`・`chain_finalize.rs` と、本体の起動・詰め直し・保存のテスト）。保存の形（印の語）は変えない。RED を先に（⑴ `defaultx` 36 で拡大率を上げても隙間 36・⑵ 辺のスコープの次のスコープの印が `left`・495 の ⑴⑵）。決めてほしいこと: 覚えた印 `left` を「その組の `defaultx` の隙間で付ける」と読み替えてよいか（利用者がドラッグで隙間 0 に付けた窓も、`defaultx` が 0 でなければ次の起動で `defaultx` だけ離れる）。読み替えないなら、印の語を 1 つ足す（保存の形が変わる＝要件 8 の相手に sylphya の読みが入る） | 開発者の裁定で直した（2026-11-15「推奨どおりで」。印 `left` は読み替える＝印の語は足さない。下の「494・495 の直し」） |
| 502 | 要件 8.6（直すなら公開の `DefaultPlacementBasis` に組んだ設定を持たせる＝欄を足すと本体の 2 つの組み立てが組めなくなる・要件 8.2）。利用者に見えない（記録だけ） | 同じ読めない値の `warn!` が、起動とシェルの切り替えで 2 行ずつ出るのを止めるか | **残す**: 記録が 1 行増えるだけで、どちらも 1 回の出来事に 1 度（毎フレームではない・5 回目の線）。止めるには公開の形を変えるか、`config.rs` の鍵の名を本体へ写す（同じ決まりが 2 か所になる）。494 の直しで起動の確定の読み方が変わるので、そのときに見直す | 開発者の裁定で今のまま（2026-11-15「推奨どおりで」。494 の直しで呼ぶ所が増えた分は下の「494・495 の直し」） |

この回の新しい裁定待ちは 3 件（494・495・502）で、3 件とも開発者の裁定（2026-11-15「推奨どおりで」）で決まった。494・495 は推奨案のとおりに直し（下の「494・495 の直し」）、502 は今のまま。

#### 494・495 の直し（開発者の裁定・2026-11-15）

- 規則の置き場: `chain_finalize.rs` に「鎖の規則」の節を置き、3 か所（本体の `drain_resnap.rs` の `finalize_chain_once_with`・`persist_save.rs` の `with_adjacency_marks`／`push_adjacency_marks`・`chain_realign.rs` の `realign_chain_once_with`）が同じ関数で組を作る。足した公開の関数は `joins_chain`（左右の辺は鎖の外）・`chain_gap`（下端・上端の `defaultx`。Free と辺は 0）・`join_left`（隙間 0 なら `JoinLeft`、ほかは `JoinLeftOffset`）・`judge_chain_adjacency`（`judge_adjacency` に、左の組の `defaultx` ±2px を足す。右は隙間 0 ±2px だけ）・`adjacency_decision`（隣り合いから決め。左の組は隙間がほぼ 0 なら `JoinLeft`、`defaultx` の隙間なら `JoinLeftOffset`）。`judge_adjacency` は `judge_chain_adjacency(prev, cur, 0)` になり、判定は変わらない。`boot_default_decision` は `join_left(chain_gap(..))` を通る（決めは変わらない）。公開の型（`PairDecision`・`KeepReason`・`Adjacency`・`AdjacencyMark`）の形は変えていない（腕を足していない）ので、網羅して match する所に追随は要らない。
- ⑴ 鎖の前の窓: 起動と詰め直しは左右の辺のスコープを解きの列から外し、前のスロットが鎖の前の窓になる（`debug!` を 1 行）。保存は「n より下の番号で鎖に入る一番近いスコープ」と組む。辺のスコープ自身の印は `none`（鎖の外）。位置を書いたスコープ k の印に加えて、k の次の鎖のスコープの印を書く（k+1 が辺なら k+2）。`drain_resnap.rs` の「次が記憶なら辺のスコープを鎖に残す」の枝は消した。
- ⑵ 隣り合い: 上の `judge_chain_adjacency`。`defaultx` は DPI で換算しない生の値（resolver と同じ）。x を書いた Free の `defaultx` は位置なので隙間にしない（`chain_gap` が 0）。詰め直しは、変わる直前の隙間が 0 の組は今までどおり `JoinLeft`、`defaultx` の隙間の組は `JoinLeftOffset { defaultx }`（記録の語 `left-offset`）で解く。
- ⑶ 覚えた印 `left`: 起動は `remembered_decision` の `JoinLeft` を `join_left(chain_gap(..))` に改める（`defaultx` が 0 でなければその隙間で付く）。印の語と保存の形は変えていない。
- 決め (a) 辺かを決める揃え方: 印を書く保存は、次の起動でも効く descript の揃え方（`DefaultPlacementBasis` から `build_placement_config`）で決める。台本の吸着（`\![set,alignmenttodesktop]`）はその回かぎりで記憶に書かないので、台本の揃え方で組を作ると、次の起動の並べ直しが読む組と違う窓との組を書くことになる。窓を今動かす起動の並べ直しと詰め直しは、窓の今の揃え方（`Anchored`・無ければ descript の値）で決める（推奨案の「窓の `Anchored`」。495 ⑵ の、台本が最初の確定より前に辺へ動かした窓を外すにはこれが要る）。限界: 台本で辺へ動かした窓がある回の保存は、その窓を飛ばさずに印を決める（たとえば本体の左に付いた scope 2 は、左の辺へ動かした kero との組として `none` を書く）。次の起動で kero が既定の位置へ戻ると、scope 2 は覚えた位置のまま動かず、kero と重なりうる。まれなので受け入れた。
- 決め (b) 今までに書いた印の読み替え: 辺のスコープの次のスコープ n の印は、この直しの前は組 (n−1, n)＝辺のスコープとの組として書かれていた。今は辺のスコープを飛ばした前の窓との組として読む。保存の形が同じなので古い印と新しい印は見分けられず、まれなので受け入れた。起動の並べ直しが、記憶から戻したスコープの覚えた印（`left`・`right`）を番号の続かない前の窓との組で読むときは、`debug!` を 1 行残す（「覚えた印を、左右の辺のスコープを飛ばした鎖の前の窓との組として読む」）。今までに書いた `left` は、`defaultx` が 0 でなければ次の起動でその隙間で付く（裁定の「読み替えてよいか」の答え）。485 の後に書いた `defaultx` の隙間の組の印は `none` なので、そのまま動かない（次にその組の位置を書いたときに `left` に変わる）。
- 502 との関係: 保存（位置を書く束ごと）と詰め直し（拡大率の変化ごと）も `build_placement_config` を呼ぶようになったので、descript の読めない値の `warn!` はその出来事ごとにも 1 行出る（どれも 1 回の出来事に 1 度・毎フレームではない）。502 の裁定（今のまま）の線に入る。並べ終えた時点の保存は、古い記憶へ印を書き足すときだけ組む。
- 材料（`DefaultPlacementBasis`）が無いとき: 保存と詰め直しは全スコープを下端・`defaultx` 0 として今までどおり判じる（詰め直しは `debug!` を 1 行。起動の確定は 485 の `warn!` のまま）。
- 「隙間なく」の字面: ツリー全体（`.kiro/specs/completed/` を除く）の grep で当たったのは、本体の `frame_chain_finalize_persist_tests.rs` の前提の文と `reset_pos_tests.rs` の手で解いた位置の説明だけで、どちらも `defaultx` の無い場合の正しい文なので変えていない。印の意味を述べる文（コードの doc と `doc/COMPAT_ARCHITECTURE.md`）は下のとおり改めた。
- RED（直す前のコードで、足した 8 本がどれも赤）: 配置の crate の `persist_adjacency_tests.rs` に 2 本（`a_defaultx_gap_on_the_left_is_marked_left`「left: [(1, "none")] right: [(1, "left")]」・`a_side_edge_scope_is_skipped_as_the_previous_chain_window`「left: [(1, "none")] right: [(2, "left")]」）、本体の新しい `frame_chain_rule_tests.rs` に 6 本（494 の例 `the_defaultx_gap_is_saved_as_left_and_survives_100_to_200`「left: Some("none") right: Some("left")」・記憶から戻した `defaultx` の隙間の組の 200% → 100%「left: 214 right: 36」・覚えた印 `left` を `defaultx` の隙間で付ける「left: 0 right: 36」・495 ⑴ 覚えた右の辺のスコープ「left: -156 right: 0」・495 ⑵ 台本が辺へ動かしたスコープ「left: 1868 right: 0」（scope 2 が kero の右へ反転した）・詰め直しが辺のスコープを飛ばす「隙間 184」）。494 の例のテストは印の確かめで止まったので、200% で重なったまま残る赤は、同じ決め（変わる直前の隙間 36 を `apart`）を通る 200% → 100% のテストで示した。配置の crate の `chain_finalize_adjacency_tests.rs` に足した `chain_adjacency_accepts_the_defaultx_gap_on_the_left_only` は新しい関数のテストで、直す前のコードでは組めない（RED は対象外。振る舞いの赤は上の 8 本が示す）。
- 期待を変えたテスト: 本体の `frame_chain_finalize_align_tests.rs` の 496 のテスト（`boot_finalize_keeps_a_side_edge_scope_as_the_base_of_a_remembered_next_scope`）だけ。裁定で「次が記憶なら辺のスコープを残す」の枝が消えたので、直した後のコードで「left: 2146 right: 2302」の赤になり、`boot_finalize_skips_a_side_edge_scope_for_a_remembered_next_scope` に改めた（覚えた印 `left` を本体との組として読み、本体の幅を準備と変えて、scope 2 が今の本体の隣へ動くことまで見る）。ほかのテストは期待を変えていない（今までのテストは `defaultx` 無し・下端・材料無しのどれかで、決めが変わらない）。新しいテストファイルから使うために、`frame_chain_finalize_align_tests.rs` の部品（`WA`・`SPAWN`・`spawn_resolved`・`sizes`・`x_of`）を `pub(super)` にした。
- doc: `doc/COMPAT_ARCHITECTURE.md` の判断の表の 2 行（拡大率が変わった後の詰め直しの対象・隣り合いの印）に、鎖の規則と印 `left` の読みを足した。コードの doc（`chain_finalize.rs` の `Adjacency`・`AdjacencyMark`・`ScopeChainState`・`boot_pair_decision`・`remembered_decision`・`PairDecision::JoinLeftOffset`・モジュールの説明、`chain_realign.rs` のモジュールの説明、`persist_save.rs` の `with_adjacency_marks`、`spawn.rs` の覚えた印）の「組 (n−1, n)」を、鎖の前の窓との組に改めた。完了した spec の文書は変えていない。
- 追随の持ち物は 1 行も触っていない。追随の `place_chain_in_work_area` は `JoinLeftOffset` をもう解く（485 の直し）。追随の `keep_restored_groups` の `joined_with_prev`（記憶の組を作業領域へ寄せるときの組の判定）は、今も組 (n−1, n) と隙間 0 の `judge_adjacency` で判じる。`defaultx` の隙間の組は印 `left` で繋がるが、辺のスコープを挟んだ組は番号が続かないので窓ごとに寄せる。追随の持ち物なので、タスク 8.2 が `audit-placement-follow` へ申し送る（下の申し送り）。

#### 申し送り（タスク 8.2 が書く）

- この回の新しい申し送りは無い（追随の持ち物には 1 行も触っていない。494 の直しの形も、追随の `place_chain_in_work_area` が `JoinLeftOffset` をもう解くので、追随の持ち物を要らない）。
- 494・495 の直しの後: `audit-placement-follow` へ、追随の `work_area_clamp.rs` の `joined_with_prev` が鎖の規則（`chain_finalize.rs` の `joins_chain`・`judge_chain_adjacency`）を使わず、組 (n−1, n) と隙間 0 で記憶の組を判じること（辺のスコープを挟んだ組は窓ごとに寄せる）を 1 行。
- 494・495 の直しで範囲の外で触った所: 本体の `runtime/frame/drain_resnap.rs`（801 行）・`frame.rs`（テストの module を 1 つ）・`frame_chain_finalize_align_tests.rs`（部品の `pub(super)` と 496 のテスト）・新しい `frame_chain_rule_tests.rs`・`doc/COMPAT_ARCHITECTURE.md`（表の 2 行）。走っている spec と重なる所は無い。本体の `switch_finish_tests.rs`（999 行）には何も足していない。
- 範囲の外で触った所: 496・498 のために本体の `runtime/frame/drain_resnap.rs`（記録の文とコメントの 3 行）・`frame_chain_finalize_align_tests.rs`（テスト 1 本）・`frame_chain_finalize_tests.rs`（コメント 1 行）。走っている spec と重なる所は無い。本体の `switch_finish_tests.rs`（999 行）には何も足していない。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 969 件が通り 1 件 ignore、手本 2 件が通る（この回は配置の crate にテストを足していない）。
- `cargo test -p areka`: 本体の bin 3188 件が通り 6 件が ignore（496 の 1 件の分だけ増えた）、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る。
- `cargo test -p log-capture-kit --test file_length_guard_test --test with_default_guard_test`: 6 件と 24 件が通る（`chain_finalize.rs` 629 行・`script_alignment_tests.rs` 404 行・本体の `drain_resnap.rs` 780 行・`frame_chain_finalize_align_tests.rs` 206 行・`switch_finish_tests.rs` 999 行（触っていない））。
- `cargo fmt --all -- --check`: 緑。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。
- 壊し方は狙った編集で入れて、元のファイルの写しから戻した（496: `drain_resnap.rs` の条件を消して赤を確かめ、写しで戻して差分が無いことを確かめた）。`git checkout`・`restore`・`stash`・`reset` は使っていない。読み手の下請けは読むだけで、直しは進行役が順に入れた。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を指す `<ファイル>.rs:<行>` の形を、ツリー全体（`.kiro/specs/`・`vendors/`・`.kiro/steering/roadmap-history.md` を除く）から拾い、今の行と見比べた。当たったのは `doc/ukadoc-coverage/briefing-assets.md` の `resolver.rs:293` だけで、この回の `resolver.rs` の直し（479 行の doc）より前の行なので動いていない。
- 494・495 の直しの後（2026-11-15）: `cargo test -p areka-placement --features test-support` の lib 972 件が通り 1 件 ignore（足した 3 件）・手本 2 件、`cargo test -p areka` の本体の bin 3194 件が通り 6 件が ignore（足した 6 件・期待を変えた 1 件）・ほかのテストの的も全部通る（知られた揺れは出なかった）、`cargo test -p areka-sylphya` 221 件、`cargo test -p ukadoc-survey` 全部、1,000 行の番人 6 件と 24 件（`chain_finalize.rs` 699 行・`chain_realign.rs` 471 行・`persist_save.rs` 436 行・`persist_adjacency_tests.rs` 316 行・`chain_finalize_adjacency_tests.rs` 217 行・本体の `drain_resnap.rs` 801 行・`frame_chain_rule_tests.rs` 192 行・`frame_chain_finalize_align_tests.rs` 215 行・`frame.rs` 566 行）、`cargo fmt --all -- --check` が緑。`git diff --stat Cargo.lock` と `vendors/` の差分は空、追随の持ち物は 1 行も触っていない。494・495 の直しで触った `.rs` を `<ファイル>.rs:<行>` の形で指す所を、ツリー全体（`.kiro/specs/`・`vendors/`・`.kiro/steering/roadmap-history.md` を除く）から拾い直した。当たったのは本体の `input_events/balloon.rs` の `runtime/frame.rs:170` だけで、`frame.rs` へ足したのは 502 行より後なので動いていない（この直しの前から指す行と食い違っているが、範囲の外の古さで動かしていない）。

#### 判定

8 回目の新しい指摘は 9 件（必ず直す 2・直したほうがよい 1・できれば 6）で、0 件ではない（7 回目の 18 件から減った）。直した 6 件（496〜501）・裁定待ち 3 件（494・495・502）。

8 回目には「必ず直す」が 2 件（494・495）あり、どちらも本番の振る舞い（拡大率の変化の後と次の起動の窓の並び）を変える指摘で、直さずに裁定待ちとして残る。ほかの 7 件は本番の振る舞いを変えない（498 は記録の文から番号を消すだけ・502 は記録の行の数だけ）。したがって、開発者の決めた打ち切りの形（8 回目に「必ず直す」と本番の振る舞いを変える指摘が無ければ 8 回目で終える）には当たらず、タスク 8.1 はまだ終わらない。494・495 の裁定（直し方と、覚えた印 `left` の読み替え）と 502 の裁定を仰ぎ、その直しを入れた後に 9 回目をかけるかを開発者に仰ぐ。

494・495 は開発者の裁定（2026-11-15「推奨どおりで」）で推奨案のとおりに直し（上の「494・495 の直し」）、502 は同じ裁定で今のまま。これで 8 回目の 9 件は、直した 8 件と今のままの 1 件（502）になり、裁定待ちは 0 件。494・495 の直しは本番の振る舞い（拡大率の変化の後と次の起動の窓の並び）を変えたので、打ち切りの形にはまだ当たらない。この直しを入れた後に 9 回目をかけるかを開発者に仰ぐ。

### 18.9 9 回目（2026-11-15）

#### 取り込みと範囲の大きさ

- 494・495 の直しの後、開発者が 9 回目を認めた（2026-11-15）。打ち切りの決まりつき: 9 回目に「必ず直す」と本番の振る舞いを変える指摘が無ければ、9 回目を最後にしてタスク 8.1 を終える。
- `main` は進んでいない（`git fetch` の後の `HEAD..origin/main` が 0 件・最後の取り込みは `246b6d75f`）。
- 範囲は 1.1 節と 18.1〜18.8 と同じ数え方で、直す前に測り直した。今までの 117 ファイルに、この spec が範囲の外に足した本体のテストの 2 ファイル（7 回目の `runtime/frame_chain_finalize_align_tests.rs`・8 回目の `runtime/frame_chain_rule_tests.rs`）を、1 回目の `persist_remove_tests.rs` と同じ「足したファイル」として入れた（D の組）。ファイルは 119。

| 区分 | ファイル | 行 |
|---|---|---|
| コード（`crates/areka-placement/` の `.rs` でテストでないもの・`examples/` 込み） | 28 | 10,929 |
| テスト（`crates/areka-placement/` の `_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`） | 77 | 22,468 |
| テスト（`crates/areka/src/placement_tests/` の `transition_*` でないもの・`mod.rs` 込み） | 10 | 3,051 |
| `Cargo.toml` | 1 | 51 |
| 足したファイル（`areka-sylphya` の `persist_remove_tests.rs`・本体の `frame_chain_finalize_align_tests.rs`・`frame_chain_rule_tests.rs`） | 3 | 780 |
| 計 | 119 | 37,279 |

- ponytail は 5.3.3（1〜8 回目と同じ版）。作法も 1.3 節と同じ（6 つの物差し・具体的な場合・消す前の grep・件数の上限なし・全部読む）。分け方も 18.1 と同じ 6 つで、6 つの組のどれからも、全ファイルを全行読んだ旨と行数の一覧を受け取った（A 24／6,505・B 21／6,931・C 20／5,981・D 19／6,015・E 15／5,931・F 20／5,916。計 37,279 行で上の表と合う）。D の組は、本体の `drain_resnap.rs` の `finalize_chain_once_with` を読み合わせに読んだ（行は数えない）。読み手は読むだけで何も変えず、進行役が指摘を集めて番号を付けてから、触るファイルが重ならないように直した（鎖の規則の組と保存の組は順に、小さな直しは進行役が）。
- 読み手には 8 回目と同じ線（各回の「指摘にしなかったもの」・古い文は今の事実と食い違うものだけ・決して落ちない確かめはどの直しでも赤にならない理由を言えるものだけ）と、8 回目の直しの中身（`b7b880996`・494・495 の直しの `a8427eee8`・485 の `20223ddd3`）を渡した。8 回目のレビューの任意の提案 2 つ（`chain_finalize.rs` の「n−1」の doc・材料が無いときの保存の黙った落ち方）も候補に入れた（516・507）。
- 進行役から、別のワークツリーの全体テスト（`b7b880996` の上）の赤を 1 件受けた（522）。

| 組 | 読んだファイル | 行 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---|---|---|---|
| A | 24 | 6,505 | 0 | 1 | 2 | 3 |
| B | 21 | 6,931 | 0 | 1 | 3 | 4 |
| C | 20 | 5,981 | 0 | 0 | 3 | 3 |
| D | 19 | 6,015 | 1 | 0 | 4 | 5 |
| E | 15 | 5,931 | 0 | 0 | 2 | 2 |
| F | 20 | 5,916 | 0 | 2 | 0 | 2 |
| 全体テストの赤（`log-capture-kit` の見張り） | — | — | 1 | 0 | 0 | 1 |
| 計 | 119 | 37,279 | 2 | 4 | 14 | **20** |

- 514 は D と F の 2 つの組の読み手が同じ所を出したもので、規則の置き場（`chain_finalize.rs`）の D に 1 件だけ数えた。B と D の読み手が出した本体の新しいテストの名前の無い指摘番号は、D の 517 に 1 件だけ数えた。503 は A の組の読み手が出し、直しは D の組のファイルにも及ぶ。521 は場所（`chain_realign.rs`）の F に数え、テストは本体の `frame_chain_rule_tests.rs` に足した。

#### 指摘の一覧（503〜522）

扱いの印は 18.1〜18.8 と同じ。

| 番号 | 組 | 群 | 一言 | 場所 | 扱い |
|---|---|---|---|---|---|
| 503 | 直したほうがよい | A | 鎖に入る揃え方（左右の辺は入らない）と左の隙間（下端・上端の `defaultx`）の決まりが、resolver の P2（`prev` を進める腕と `bottom_x`）と、494・495 で足した `chain_finalize::joins_chain`／`chain_gap` の 2 か所に書いてある。片方だけを変えると（たとえば上端のスコープを P1' と同じ別の連なりにすると）、起動の並べ直し・隣り合いの印・詰め直しが resolver と食い違う（485 の形）が、どのテストも赤にならない（本体のテストに `top` の場合が無い） | `resolver.rs` の `resolve_placement`・`chain_finalize.rs` | 直した（`joins_chain`・`chain_gap` を `resolver.rs` の `Anchor` の隣へ移し、`resolve_placement` の P2 がそれを呼ぶ: `bottom_x = base_x − chain_gap(anchor, default_x)`・`if joins_chain(anchor) { prev = Some(x) }`。`chain_finalize.rs` は `pub use` で同じ名を出し続けるので、呼び手（本体の `drain_resnap.rs`・`persist_save.rs`・`chain_realign.rs`・テスト）の道は変わらない。依存の向き（resolver は chain_finalize を引かない）は保つ。結果が同じことを確かめた: x を書いた Free は `bottom_x` を x に使わず、x の無い Free は `default_x` が無く、左右の辺は `bottom_x` を読まない。RED は対象外（置き場を 1 つにするだけ。T-R5 ほかの resolver のテストと、`chain_finalize` と本体の起動・詰め直しのテストが両側を押さえる）。494・495 の裁定の「規則の置き場は `chain_finalize.rs`」は、規則の節と 3 か所の呼び口が `chain_finalize.rs` に残るので変わらない） |
| 504 | できれば | A | テストの補助 `pos` が `reseed::test_support::position_of` と同じ中身（458 と同じ形） | `script_alignment_tests.rs` | 直した（`position_of` を引いて `pos` を消した） |
| 505 | できれば | A | `primary_monitor` の `match monitors.first() { Some(first) => {…} None => None }` は `?` で短く書ける | `lib.rs` | 直した（0 台で記録しないのは同じ） |
| 506 | 直したほうがよい | B | 並べ終えた時点の保存で印の無い古い記憶へ印を足す道で、鎖の規則（`SavedChain::of`）をどのテストも通さない。`SavedChain(None)` にしても全部緑のまま（`old_memory_rig` は材料を置かない）。壊れると `kero.defaultx,36` の古い記憶へ `none` が書かれ、拡大率が変わると重なったまま残る（494 の形） | `persist_save.rs` の `persist_unremembered_char_positions`・`persist_finalize_save_tests.rs` | 直した（`old_memory_mark_uses_the_defaultx_gap_of_the_descript`。RED: `SavedChain(None)` にする壊し方で「left: Some("none") right: Some("left")」） |
| 507 | できれば | B | 材料（`DefaultPlacementBasis`）が無いとき、保存の印の判じ方が記録を残さずに全スコープを下端・`defaultx` 0 に落とす（詰め直しは `debug!`・起動の確定は `warn!`）。8 回目のレビューの提案 | `persist_save.rs` の `SavedChain::of` | 直した（`debug!` を 1 行。`missing_basis_marks_with_the_bottom_rule_and_logs_debug`。RED: 直す前は「left: 0 right: 1」。「記録が 0 行」を確かめる `missing_partner_scope_is_skipped_silently` の 2 つの World には空の材料を置いた（期待は変えていない）） |
| 508 | できれば | B | 1 回の並べ終えた時点の保存で、`SavedChain::of` を 2 度組む（`persist_unremembered_char_positions` と、送り口の中の印の付け足し）。descript の読めない値の `warn!`（`kero.defaultx,abc` ほか）が 1 回の出来事に 2 行出る（502 の残り。502 の裁定は「1 回の出来事に 1 度」） | `persist_save.rs` | 直した（並べ終えた時点の保存で 1 度だけ組み、内側の `send_entries` へ渡す。公開の `persist_entries` の形は同じで、自分で組む。`one_finalize_save_warns_once_for_an_unreadable_defaultx`。RED: 直す前は「left: 2 right: 1」。シェル位置初期化（`reset_pos.rs`）で 2 度組むのは、502 の裁定（シェルの切り替えと同じ 1 回の出来事に 2 行）の線に入れて変えていない） |
| 509 | できれば | B | 印を足す所の doc が「k を鎖の前の窓とする次のスコープの組」と書くが、コードは k が左右の辺のスコープでも、k より上の番号で鎖に入る一番近いスコープの組を書く（テスト `a_side_edge_scope_is_skipped_as_the_previous_chain_window` が今のコードを固定している） | `persist_save.rs` の `with_adjacency_marks`（今は `adjacency_marks_with`） | 直した（doc を「スコープ k の組と、k より上の番号で鎖に入る一番近いスコープの組」に） |
| 510 | できれば | C | 指摘 4 のために足した手本のテスト `each_scope_balloon_uses_its_own_scope_picture` が、手本の呼び出し（`build_balloon_target(…, scope_u32)`）を通らない。3 つ目の引数を `0` に戻しても（指摘 4 の誤り）緑のまま。確かめているのは `target_scope(1) == Some(1)`（次のテストも確かめる）と、`areka-emo-present` のバルーンの絵の選び（その crate の `balloon_series_tests.rs` が確かめる）だけ | `examples/window-placement.rs` | 直した（消した。`Cargo.toml` の `[[example]]` の説明から「絵の選び」を外した。守りを残すには選びを関数へ移してコードを足すことになる。手本の `#[test]` は 1 件になる） |
| 511 | できれば | C | 追随の支えが共有の `test_support::zorder_pair_deferred::code_only` の写しと、`SharedFakeIo` の小さな写しを持つ（64・76・143 の残り。2 回目の申し送りの `Store` とは別の物） | 追随の `follow_test_support.rs` の `code_only`・`SharedFakeIo` | 申し送り（`audit-placement-follow`。共有のものを使う。直しが追随の持ち物に要る） |
| 512 | できれば | C | 本体のテストが `SharedFakeIo`（`PersistIo` の偽物）を 3 か所に手で写す（共有の `areka_placement::test_support::SharedFakeIo` がある・80・474 と同じ形） | 本体の `boot_resolve_tests.rs`・`input_events/drag_test_support.rs`・`main_persist_wiring_seam_tests.rs` | 申し送り（`audit-areka-app`。範囲の外） |
| 513 | 必ず直す | D | `defaultx` が大きく負のとき、隙間 `defaultx` の組を隣り合いと見ない。左右を窓の中心で先に決め、`defaultx` の隙間は中心で左のときしか見ないので、`defaultx ≤ −(w(n)+w(前の窓))/2` だと右の組として重なりと判じる。たとえば 100% で w0=191・w1=168・`kero.defaultx,-200` のゴーストを記憶なしで起こすと、起動の確定は kero を本体の x+32 に置く（485）が、保存は印 `none` を書き、詰め直しは `Keep(Overlapped)` で動かさない。100% から 200% へ移ると kero は resolver の位置から 180px ほど外れて残り、次の起動からは本体の幅に付いてこない（494 の形）。494 の裁定の文は「隙間が 0 か、そのスコープの `defaultx` の ±2px」で、中心で左という絞りは無い | `chain_finalize.rs` の `judge_chain_adjacency` | 直した（`defaultx` が 0 でなければ、中心を比べる前に左の隙間 `前の窓.x − (n.x + n.w)` を見て、縦が重なり `defaultx` ±2px なら `Left`。doc と「鎖の規則 ⑵」の文も直した。`a_negative_defaultx_gap_past_the_centre_is_still_left`。RED: 直す前は「left: Overlapped { gap: -159 } right: Left { gap: -200 }」。18.8 の「左の組は `defaultx` の隙間も」の「左」は、この直しの後は中心でなく左の隙間で決まる） |
| 514 | できれば | D | `defaultx` が ±1・±2 のとき、詰め直しが隙間を 0 にする（`Left{g}` で `|g| ≤ 2` をいつも `JoinLeft` にする）。`kero.defaultx,2` は起動と次の起動では隙間 2 だが、拡大率が変わると 0 になる | `chain_finalize.rs` の `adjacency_decision` | 直した（隙間が `defaultx` より 0 に近いか同じときだけ `JoinLeft`、ほかは `join_left(defaultx)`。`a_small_defaultx_gap_is_kept_not_rounded_to_zero`。RED: 直す前は「left: JoinLeft right: JoinLeftOffset { offset: 2 }」） |
| 515 | できれば | D | 起動の確定で、組の決めは descript の揃え方（`boot_default_decision(anchor, …)`）、鎖に入るかは窓の今の揃え方（`joins_chain(live)`）で判じる。descript が右の辺の kero を、最初の確定より前に台本 `\![set,alignmenttodesktop,bottom]`（`free` も）が揃え直すと（x はそのまま・`ScriptPlaced` の印は付かない）、決めは `Keep(DesktopEdge)` で kero は辺に残るのに、鎖の基準になる。記憶の無い下端のスコープ 2 は、本体の隣から右の辺の kero の隣へ動く（495 ⑵ の裏返し。テストが無い） | 本体の `runtime/frame/drain_resnap.rs` の `finalize_chain_once_with` | 開発者の裁定で直した（2026-11-15「推奨通り」・18.10） |
| 516 | できれば | D | 「n−1」が、前のスロットとの組になった後も doc に残る（`PairDecision::JoinLeft`／`JoinLeftOffset`／`JoinRight`・`resolve_chain` と事後条件 `x(n−1) == x(n) + w(n)`・モジュールの式）。左右の辺のスコープ 1 を飛ばすと、スコープ 2 の事後条件は `x(0) == x(2) + w(2)`（497 の残り・8 回目のレビューの提案） | `chain_finalize.rs` | 直した（「前のスロット」・`x(prev)`・`w(prev)` に。ふつうの場合を言う「ふつうは組 (n−1, n)」は正しいので残した） |
| 517 | できれば | D | spec の名の無い指摘番号が、8 回目に足した本体のテストの doc に残る（「494 の例」「495 ⑴」「495 ⑵」「指摘 1・SSP の測り 210」。498 の残り） | 本体の `frame_chain_rule_tests.rs`・`frame_chain_finalize_align_tests.rs` | 直した（前の 3 つは番号を消した。文が中身をもう言っている。最後の 1 つは `audit-placement-core 指摘 1` に） |
| 518 | できれば | E | 台帳の `has_base` の欄が、`groups` が持つ事実（先頭が descript 由来の組か）をもう 1 度持ち、4 か所で手で揃える。揃え忘れると `reset_to_descript` がタグの組を基底として残すか、本物の基底を落とす（今は 4 か所とも揃っている） | `zorder_group_ledger.rs` | 直した（欄を消し、先頭の組の出所を見る私有の `has_base()` に。公開の形は変わらない） |
| 519 | できれば | E | テストが `Vec<Option<Entity>>` を番号の順に埋めてから全部を `expect` で開ける（`None` は残りえない） | `zorder_chain_tail_order_tests.rs` | 直した（`push` で同じ順に積む） |
| 520 | 直したほうがよい | F | 494 の鎖の規則 ⑴（左右の辺のスコープを飛ばす）の後、利用者が左右の辺のスコープの隣へドラッグで付けた窓が、拡大率が変わると離れて重なる。たとえば 3 スコープのゴーストで kero が右の辺、さくらとスコープ 2 が下端のとき、スコープ 2 を kero の左に隙間 0 で付けてから 100% → 200% にすると、494 の前は組 (1, 2) を `JoinLeft` で付け直したが、今は組 (0, 2) を `Apart` と判じて動かさず、kero は右の辺から左へ広がるので、幅が 336 → 672 なら 504px 重なったまま残る。保存の印も kero との `left` でなく、さくらとの `none` になる。8 回目の決め (a) の限り（台本で辺へ動かした窓）には入らない | `chain_realign.rs` の `realign_chain_once_with`（`chain_finalize.rs` の規則 ⑴） | 開発者の裁定で直した（2026-11-15「推奨通り」・18.10） |
| 521 | 直したほうがよい | F | 詰め直しが、辺かどうかを窓の今の揃え方（`Anchored`）で決める所に、壊れたら落ちるテストが無い。descript の揃え方だけで決める形にしても全部緑のまま。そうすると、最初の確定より前に台本が kero を左の辺へ動かしたゴーストで、拡大率が変わると kero が鎖に残り、組 (0, 2) を判じず、スコープ 2 と本体のあいだが開く（494 が直した形の離れ方） | `chain_realign.rs` の `realign_chain_once_with` の `terms` | 直した（本体の `frame_chain_rule_tests.rs` に `realign_skips_a_boot_script_side_edge_scope_as_the_chain_base`。RED: descript の揃え方だけで決める壊し方で「scope 2 が本体の左に詰まっていない（隙間 184）」。壊し方は写しから戻し、`chain_realign.rs` の差分は 0） |
| 522 | 必ず直す | — | 全体テストの x64 のワークスペースのテストで、`log-capture-kit` の見張り `temp_path_guard_test` の 3 本（`each_exception_declares_whether_it_uniquifies_by_process_identifier`・`every_exception_still_has_a_real_hit`・`the_measurement_is_not_vacuous_and_matches_the_allow_table`）が赤。例外表が `crates/areka-placement/src/measure_tests.rs`（プロセス識別子で一意化すると名乗るが、もう使っていない）と `placement_monitor_tests.rs`（読み出しのみ）を載せたままで、測りではどちらにももう当たりが無い。この spec の前の回の直しが 2 つのファイルの一時パスの自前の作り方をやめたのに、表を直していなかった。前の回の確かめは `log-capture-kit` の 2 つの見張り（行数・`with_default`）しか回しておらず、この見張りを見落としていた（前の回の借り） | `crates/log-capture-kit/tests/temp_path_guard_test.rs` の `ALLOWED_ENTRY_POINT_USES` | 直した（2 行を表から消し、件数を 3 → 1・一意化 2 → 1・読み出しのみ 1 → 0 にした。表へは足していない。消した経緯は表の中のコメントに書いた。測った集合は `crates/areka/src/install/fetch_url.rs` の 1 つだけで表と合う（`the_measurement_is_not_vacuous_and_matches_the_allow_table` が緑）。RED: 直す前は進行役が全体テストで 3 本の赤を確かめ、この回の鎖の規則の組も `cargo test -p log-capture-kit` で同じ 3 本の赤を見た。この回から確かめは `cargo test -p log-capture-kit`（crate 全体）にする） |

- 申し送りとの対応（要件 1.6）: 1〜502 と同じ所を指す指摘は 0 件。前の回の直しの残りは新しく数え、どの番号の残りかを一言に書いた（504＝458、508＝502、511＝64・76・143、512＝80・474、513・514＝494、515＝495、516＝497、517＝498、520・521＝494）。522 は前の回の直しが作った赤で、全体テストで見つかった。
- 513・514 は要件 5 に当たるかを確かめた: どちらも、8 回目に開発者が裁定した鎖の規則 ⑵（「隙間が 0 か、そのスコープの `defaultx` の ±2px」）に今のコードを合わせる直しで、裁定の文を変えない（391・454・472 と同じ判断）。進行役が直した。
- 515・520 は要件 5 の 3 種類目（利用者に見える窓の位置）に当たり、どちらも 8 回目の裁定の規則の当て方を決め直すので、進行役は直さずに裁定を仰ぐ。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を `<ファイル>.rs:<行>` の形で指す所を、ツリー全体（`.kiro/specs/`・`vendors/`・`.kiro/steering/roadmap-history.md` を除く）から拾い、今の行と見比べた。当たったのは `doc/ukadoc-coverage/briefing-assets.md` の `resolver.rs:293`・本体の `examples/collision-probe.rs` の `lib.rs:581`／`:382-456`／`:395-406`・`collision-probe/setup.rs` の `window-placement.rs:251-255` で、この回の直しはどれもそれより後ろの行なので動いていない。

#### 指摘にしなかったもの

- 本体の `boot_default_decision` の決めに窓の今の揃え方を使う案（515 の直し方の候補として）。`free` で x の分からない窓を本体の隣へ引き寄せてしまう。
- `reset_pos.rs` のシェル位置初期化で設定を 2 度組む（502 の裁定の線。508 は並べ終えた時点の保存だけを直した）。
- `send_entries` を作業の無いときに早く返す形（508 の直しの中。結果は直す前と同じ）。
- `persist_save.rs` の `terms` が設定に無いスコープを下端・`defaultx` 無しに落とすこと（`detect_scopes` は 0 と 1 だけを出し、詰め直しの `ScopeConfig::default()` と同じ）。
- 本体の `frame_chain_rule_tests.rs` の `a_remembered_defaultx_gap_pair_rejoins_after_200_to_100` が詰め直しの読まない既定の位置を消すこと（494 の仕組みで赤になる）。
- 起動の確定が、飛ばす辺のスコープの決めも作ってから外すこと・`has_default` の線形の探し（窓の一式ごとに 1 度・スコープは数個）。
- 詰め直しで辺のスコープと先頭のスロットに組の `info!` が無いこと（組が無い・辺のスコープは `debug!` が 1 行）。
- 辺の kero の隣へさくらをドラッグした 2 スコープの場合に、どちらも動かないこと（494 の前は `JoinRight` で kero を辺から外していた）。
- 上端と下端のスコープが隣り合いと判じられないこと（縦の範囲が重ならない・`dpi-realign-remembered-chain` の要件 2.1）。
- `defaultx` が 0 のときの 1〜2px の隙間を 0 に詰めること（許す幅のわざとの形）。
- `zorder_group_ledger.rs` の `reset_to_descript` の `if self.has_base() { None } else { … }` の守り（不変条件をその場で言う）・`set_descript_base` が空の台帳で版を進めないこと（版を読むのはテストだけ）・`RebaseOutcome` の `Default`（使い手は無いが、公開の型から外すと公開の形が狭まる・215・246 の線）・`vocab_public_consts` が `pub static` を見ないこと（t_zpd40 の全ファイルの走査が拾う）・`scan_z_once` の全部 `None` の入力（呼び手が無い）・`Rig::drop` の 2 度目の `DestroyWindow`（doc に書いた形）・`seriko.zorder` の末尾の読点（18.5 の `sticky-window` と同じ線）。
- `config.rs` の読めない値の `warn!` に `event` の欄が無いこと（機械で読む所が無く、文が分かれている）・採寸でスコープ 1 と 2 が同じバルーンの面を焼くこと（起動に 1 度・本番はスコープ 2 を作らない）・`assert_no_blind_spot_literals` の誤検知（赤の向きで、今の走査の対象に無い）・`code_only` の行の続きの扱い（`contains` だけで読む）・`DefaultSurfaces::builtin()` の本体の呼び手（`#[allow(dead_code)]` のテストだけの関数の中）。
- `resolver.rs` の P2 の doc の「n−1」（同じ箇条が辺のスコープを連なりから外すと言うので、鎖の前のスコープと読める）・`lib.rs` の届かない枝（面 0 の `warn!`・u32 に収まらないスコープ・配置表に無いスコープ）・`PreparedPlacement.titles` と材料の `titles` の二重持ち（1 度の読みから作る）・`windowposition.x,3000000000`（警告して落とす）・`script_alignment.rs` の戻り値の「当てたら真」（本番の呼び手が戻り値を捨てる）・`placement_monitor_tests.rs` のテストの名の `primary_work_area`（名で、主張ではない）。
- 組から上がって、確かめて外したもの: `balloon_offset_supply_tests.rs` の節の番号の飛び（足りないだけ）・`persist_restore_group_tests.rs` ほかの「task」の番号（同じ括弧に spec の名がある）・`apply_face_windowposition` の全スコープの解き直し（5 回目の線）・`SavedChain::of` を `GhostWindows` が無くても組むこと（守りの道で軽い）・`BalloonSyncScaleState` を窓の一式を閉じても消さないこと（読み手が無い・要件 6.6）・`apply_set_sticky` の `members.clone()`（台本の命令ごとに 1 度）・`reset_balloon_pos_tests.rs:491` の文（正しい）。

#### 裁定が要るもの（要件 5・8.6）

| 番号 | 種類 | 中身 | 推奨 | 裁定 |
|---|---|---|---|---|
| 18.8 の決め (a) | 8 回目の 494・495 の直しで進行役が決めた所（要件 5 の 3 種類目） | 隣り合いの印を書く保存が、辺かどうかを descript の揃え方（`DefaultPlacementBasis` → `build_placement_config`）で決め、起動の並べ直しと詰め直しは窓の今の揃え方（`Anchored`・無ければ descript）で決める | 今のまま | 開発者の裁定で今のまま（2026-11-15「推奨どおりで」。コードは変えない）。理由: 保存した印は次の起動が読み、次の起動は descript の揃え方で組を作り直す。台本の吸着はその回かぎりで記憶に書かない。受け入れた限り: 台本が窓を辺へ動かした回の保存は、次のスコープに `none` を書きうる（まれ） |
| 515 | 要件 5 の 3 種類目（起動のときの窓の位置）。8 回目の裁定の「鎖に入るかは窓の今の揃え方」と、485 の「記憶の無い既定のスコープの決めは descript の揃え方」が、台本が最初の確定より前に揃え直した窓で食い違う | descript が左右の辺で、台本が最初の確定より前に下端・Free へ揃え直した窓を、起動の確定で鎖の基準にするか | **直す**: 起動の確定で、descript か今の揃え方のどちらかが左右の辺なら鎖から外す（`joins_chain(live) && joins_chain(descript)`・本体の `drain_resnap.rs` の 1 行）。その窓は辺に残るので、記憶の無い次のスコープは本体の隣のまま（resolver の位置）で、保存（descript の揃え方で組む＝決め (a)）と次の起動の組とも揃う。詰め直しは今のまま（窓の今の揃え方）。RED を先に（台本が `bottom` に揃え直した辺の kero で、スコープ 2 が動かない）。規模は S | 開発者の裁定で直した（2026-11-15「推奨通り」・18.10） |
| 520 | 要件 5 の 1 種類目（8 回目に裁定した鎖の規則 ⑴「左右の辺のスコープは鎖の外」と、完了した `dpi-realign-remembered-chain` の「変わる直前に隣り合っていた組を付け直す」）と 3 種類目（拡大率の変化の後の窓の並び） | 利用者が左右の辺のスコープの隣へ付けた窓を、拡大率が変わったときに付け直すか | **詰め直しだけ直す**: 詰め直しで、飛ばした辺のスコープ k に、変わる直前の次のスコープ n が隙間 0 で左に付いていたときは、n の組を k との組として判じて k の左へ付ける（`chain_realign.rs`・規模は S〜M）。印の保存と起動は今のまま（次の起動で辺のスコープは辺へ戻り、n は覚えた位置のまま動かない）。RED を先に（上の 3 スコープの例で、200% の後も n が kero の左に隙間 0）。直さないなら、規則 ⑴ の限りとして `chain_realign.rs` に `shortcut:` で書く | 開発者の裁定で直した（2026-11-15「推奨通り」・18.10） |

この回の新しい裁定待ちは 2 件（515・520）。決め (a) は開発者の裁定で今のまま。515・520 は、その後の開発者の裁定（2026-11-15「推奨通り」）で直した（18.10）。

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（持ち主の spec はもう完了しているので、8.2 が扱いを決める）: 511（`follow_test_support.rs` の `code_only`（465 行あたり）を消して `crate::test_support::zorder_pair_deferred::code_only` を使い、`SharedFakeIo`（495 行あたり）を共有の `test_support::SharedFakeIo` に）。
- `audit-areka-app` へ: 512（本体の `boot_resolve_tests.rs`・`input_events/drag_test_support.rs`・`main_persist_wiring_seam_tests.rs` の `SharedFakeIo` の写しを、`areka_placement::test_support::SharedFakeIo` に）。
- 範囲の外で触った所: 517・521 のために本体の `runtime/frame_chain_rule_tests.rs`・`frame_chain_finalize_align_tests.rs`、522 のために `crates/log-capture-kit/tests/temp_path_guard_test.rs`。走っている spec と重なる所は無い。本体の `switch_finish_tests.rs`（999 行）には何も足していない。

#### テスト

- `cargo test -p areka-placement --features test-support`: lib 977 件が通り 1 件 ignore（足した 5 件）、手本 1 件が通る（510 で 1 件消した）。
- `cargo test -p areka`: 本体の bin 3195 件が通り 6 件が ignore（521 の 1 件の分だけ増えた）、ほかのテストの的も全部通る（鎖の規則の組の 1 回目に知られた揺れ `spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits` が 1 度だけ赤になり、単独で緑・回し直しで全部緑。最後の通しでは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。
- `cargo test -p ukadoc-survey`: 全部通る。
- `cargo test -p log-capture-kit`（crate 全体・この回から）: 全部通る（`temp_path_guard_test` 16 件・`with_default_guard_test` 24 件・`file_length_guard_test` 6 件ほか）。直す前は `temp_path_guard_test` の 3 件が赤（522）。
- `cargo fmt --all -- --check`: 緑。
- 1,000 行の近く: `lib.rs` 927 行・`persist_finalize_save_tests.rs` 748 行・`zorder_group_ledger.rs` 739 行・`chain_finalize.rs` 700 行・`resolver.rs` 559 行・`persist_save.rs` 468 行・本体の `switch_finish_tests.rs` 999 行（触っていない）。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。
- 壊し方は狙った編集で入れて、元のファイルの写しから戻した（506 の `persist_save.rs`・521 の `chain_realign.rs`。どちらも戻した後に差分が狙った直しだけ、または 0 であることを確かめた）。`git checkout`・`restore`・`stash`・`reset` は使っていない。読み手の下請けは読むだけ。直しは、鎖の規則の組（503・513・514・516・517・521）と保存の組（506〜509）を下請けが順に入れ、小さな直し（504・505・510・518・519・522）は進行役が入れた。触るファイルは重ならない。

#### 判定

9 回目の新しい指摘は 20 件（必ず直す 2・直したほうがよい 4・できれば 14）で、0 件ではない（8 回目の 9 件から増えた。D と F の組が、8 回目に足した鎖の規則の当て方を読んだ分が多い）。直した 16 件・申し送りだけのもの 2 件（511・512）・裁定待ち 2 件（515・520）。あわせて、8 回目の決め (a) は開発者の裁定で今のままと決まった。

9 回目には「必ず直す」が 2 件（513・522）ある。513 は本番の振る舞い（`defaultx` が大きく負のゴーストの、拡大率の変化の後と次の起動の窓の並び）を変える指摘で、直した。522 は全体テストの赤で、本番の振る舞いには関わらない。ほかに本番の振る舞いを変える指摘が 3 件ある（514 は直した・2px までの差。515・520 は裁定待ち）。したがって、開発者の決めた打ち切りの形（9 回目に「必ず直す」と本番の振る舞いを変える指摘が無ければ 9 回目で終える）には当たらず、タスク 8.1 はまだ終わらない。515・520 の裁定を仰ぎ、その直し（または残す理由）を入れた後に 10 回目をかけるかを開発者に仰ぐ。

### 18.10 連鎖の決まりの総括（2026-11-15）

#### いきさつと範囲

- 9 回目の後、開発者の指示（2026-11-15）: 「必ず直す」は 0 件にする。515・520 を 1 件ずつ継ぎ足さず、これまでに出た鎖（キャラ窓の横の連なり）の直しを全部まとめて洗い出し、場合を全部並べ、どの道も 1 つの規則から答えを出すようにする。
- `main` は `1feefe6d1`（9 回目の直し `98077a7ec` の上に取り込み済み）。この回は audit のかけ直しではなく、鎖の規則の総括と、その規則への寄せ。
- 読んだもの: 18.1〜18.9 の鎖にかかわる指摘と裁定、完了した spec の要件（`areka-P0-window-placement` 要件 2・`areka-P0-scope-chain-gap` 要件 2・7・`dpi-realign-remembered-chain` 要件 1〜3・6・`window-work-area-clamp` 要件 1・2・4）、道のコード（`resolver.rs`・`chain_finalize.rs`・`chain_realign.rs`・`persist_save.rs`・`persist.rs` の読み戻し・`reseed.rs`・`reset_pos.rs`・本体の `runtime/frame/drain_resnap.rs`・`switch.rs`・追随の `follow/work_area_clamp.rs`）。

#### 今までの鎖の直しの一覧

| 番号 | 回 | 道 | 変えたこと |
|---|---|---|---|
| 1 | 要件の段・タスク 2.1 | 既定の並び（resolver） | `seriko.alignmenttodesktop` を 4 つの辺で解く（P1'）。左右の辺のスコープは辺に置き、辺ごとに下から上へ連ね、横の連なり（P2）を進めない |
| 210 | 2 回目（裁定 3 回目の前） | 既定の並び | 右（左）の辺のスコープと下端のスコープが角で重なるのは今のまま（SSP も離さない） |
| 296 | 3 回目（裁定 5 回目） | 既定の並び・台本の吸着 | 揃え方の語の大文字小文字を 1 つの読み（`Anchor::from_word`）にそろえた。どのスコープが辺かの判定がこれで決まる |
| 391 | 4 回目 | 印の保存（並べ終えた時点） | 記憶の位置を寄せて表示した起動では、印の無い古い記憶へ印を書き足さない（隣の窓の位置を書く束からも外す） |
| 434 | 5 回目（裁定 6 回目の前） | 印の保存（ドラッグ・連れ動き） | 391 と同じ外し方を、送り口の本体（`send_entries`）で全部の呼び手に効かせた |
| 454・472 | 5・6 回目 | 切り替えの解き直し | バルーン・シェルの切り替えで、全スコープを resolver に通し直すときのバルーンの値（作者の DPI・今出している面）を正した。キャラ窓の位置は捨てる（鎖は解き直さない） |
| 485 | 7 回目（裁定） | 起動の確定 | 記憶に位置の無い既定のままのスコープを、resolver と同じ揃え方の規則で解く（`defaultx` の隙間を残す・左右の辺は動かさず基準にもしない・x を書いた Free は動かさない）。`JoinLeftOffset`・`DesktopEdge`・`FreePosition` を足した |
| 494 | 8 回目（裁定） | 印の保存・詰め直し | 485 の規則を保存と詰め直しにも当てた。⑴ 鎖の前の窓は辺のスコープを飛ばす ⑵ 左の組は `defaultx` の隙間も隣り合い ⑶ 覚えた印 `left` は `defaultx` の隙間で付ける |
| 495 | 8 回目（裁定） | 起動の確定 | 辺かどうかを窓の今の揃え方（`Anchored`）で決め、覚えた辺のスコープと、台本が最初の確定より前に辺へ動かしたスコープを鎖から外す |
| 決め (a)(b) | 8 回目（(a) は 9 回目に裁定） | 印の保存・起動の確定 | (a) 印を書く保存は descript の揃え方、窓を今動かす道は今の揃え方 (b) 494 の前に書いた印は、辺のスコープを飛ばした前の窓との組として読む |
| 496・497・516・517 | 8・9 回目 | 起動の確定（テスト・doc） | 「次が記憶なら辺のスコープを残す」の枝の檻（494 で枝ごと消えた）と、「組 (n−1, n)」の文の直し |
| 502 | 8 回目（裁定） | 記録 | descript の読めない値の `warn!` が 1 回の出来事に道の数だけ出るのは今のまま |
| 503 | 9 回目 | 既定の並び・3 つの道 | 鎖に入る揃え方（`joins_chain`）と隙間（`chain_gap`）を resolver の 1 か所に置いた |
| 506〜509 | 9 回目 | 印の保存 | 古い記憶へ印を足す道の檻・材料が無いときの `debug!`・1 回の保存で設定を 1 度だけ組む・doc |
| 513 | 9 回目 | 隣り合いの判定 | `defaultx` が大きく負でも、左の隙間が `defaultx` なら左の組と読む（中心の左右より先に見る） |
| 514 | 9 回目 | 詰め直しの決め | `defaultx` が ±1・±2 のとき隙間を 0 に丸めない |
| 521 | 9 回目 | 詰め直し（テスト） | 辺かを窓の今の揃え方で決めることの檻 |
| 515 | 9 回目（裁定 2026-11-15「推奨通り」）→ この回 | 起動の確定 | 下の表 T24 |
| 520 | 9 回目（裁定 2026-11-15「推奨通り」）→ この回 | 詰め直し | 下の表 T32 |

読み取れたこと: 485 から後の直しは、どれも「resolver が決めた並びの規則を、もう 1 つの道が知らない」形だった。道ごとに「どのスコープが鎖に入るか・前の窓はどれか・隙間はいくつか」を別々に組んでいた（起動は本体の `drain_resnap.rs`、詰め直しは `chain_realign.rs` の `terms`、保存は `persist_save.rs` の `SavedChain`）ので、1 つを直すたびに残りとの食い違いが次の回の指摘になった。

#### 1 つの規則（鎖の規則）

置き場は `crates/areka-placement/src/chain_rule.rs`（説明と、3 つの道が共有する `ChainRule`）。隣り合いと決めの式は今までどおり `chain_finalize.rs`、鎖に入る揃え方と隙間は `resolver.rs`。

- 言葉: 「descript の揃え方」はシェルの descript から 4 段の優先で決めた揃え方と `defaultx`（並べ直しの材料 `DefaultPlacementBasis` から組む。次の起動でも効く）。「今の揃え方」は窓の `Anchored`（台本の吸着を含む。その回かぎり）。
- ⑴ 鎖に入るスコープと鎖の前の窓: スコープは、揃え方が左右の辺でなければ鎖に入る。左右の辺のスコープは辺に置かれ、横には動かさず、鎖の組の基準にもしない。鎖の前の窓は、n より下の番号で鎖に入る一番近いスコープ。無ければ n は鎖の頭で、組を持たない（動かさない・印を書かない）。どの揃え方で見るかは道で決まる。
  - 既定の並び（resolver）と印の保存: descript の揃え方（印は次の起動が読むので）。
  - 起動の確定: descript の鎖から、今左右の辺に揃っている窓を外す（既定の並びと覚えた印を今の画面へ当てる道）。
  - 詰め直し: 今の揃え方だけ（変わる直前の画面を読む道）。
- ⑵ 隙間と隣り合い: 組（鎖の前の窓, n）の決まった隙間は、n の descript の `defaultx`（下端・上端だけ。Free と辺は 0。DPI で換算しない）。resolver は `x(n) = x(鎖の前の窓) − w(n) − 隙間` で置く。組が隣り合うのは、向かい合う端の隙間が 0 か、左の隙間がこの隙間のとき（どちらも ±2px）。右は隙間 0 だけ。縦の範囲が重ならなければ隣り合わない。
- ⑶ 印: n の印は、descript の鎖の前の窓との組の ⑵ の結果（`left`・`right`・`none`）。n が辺のスコープか窓の組なら `none`。覚えた `left` は ⑵ の隙間で付け直す。
- ⑷ 辺のスコープの隣に付けた窓（詰め直しだけ）: 鎖の前の窓へ付け直さない窓（鎖の頭を含む）が、変わる直前に、飛ばした辺のスコープ（n に近い順）の隣に隙間 0 で付いていたら、その辺のスコープを動かさない基準にして同じ左右の側へ付け直す。
- 道がすること: 既定の並びは ⑵ の式で置く。起動の確定は、記憶に位置の無い既定のままの窓を resolver と同じ決めで、記憶の窓を覚えた印で、今の幅へ当て直す（鎖の頭は動かさない）。印の保存は画面の並びから ⑶ の印を書く。詰め直しは変わる直前に ⑵ で隣り合っていた組を付け直し、⑷ を当てる。シェルの切り替えは鎖を解き直さず、descript の値（材料）と窓の揃え方を入れ替えるだけ。

⑴〜⑶ は 494・495 の裁定の規則と同じで、起動の確定の見方に「descript が辺のスコープも外す」（515）を足し、⑷（520）を足した。

#### 場合の表

列は道。「R」既定の並び（`resolve_placement`。起動の準備・並べ直し・切り替えの解き直しが通る）、「B」起動の確定（`finalize_chain_once_with` → `chain_rule::boot_chain`）、「S」印の保存（`persist_entries`・`persist_unremembered_char_positions`）、「D」拡大率の後の詰め直し（`realign_chain_once_with`）、「W」シェル・バルーンの切り替え、「F」追随の寄せ（`follow/work_area_clamp.rs` の `keep_restored_groups`・読んだだけ）。p は鎖の前の窓、g は ⑵ の隙間。「〃」は T1 と同じ、「—」はその道が見ない場合。W はどの行でもキャラ窓を横へ動かさない（T28）ので、T28 のほかは「—」。扱いの「合う」は今までのコードがもう表のとおり、「直した」はこの回で直した、「限り」は下の裁定の表。

| # | 場合 | R | B | S | D | W | F | 出どころ | 扱い |
|---|---|---|---|---|---|---|---|---|---|
| T1 | n も p も下端・`defaultx` 無し | `x(p) − w(n)` | 既定のままなら今の幅で `x(p) − w(n)`。記憶の窓は印（T19） | 隙間 0（±2）なら `left`／`right`、ほかは `none` | 隣り合っていたら今の幅で付け直す | — | 番号が続く組 (n−1, n) を、印 `left`／`right` か記憶の矩形（離れていない）で 1 つの組にする | `scope-chain-gap` 2.1・7.1、`dpi-realign` 1.1・6.2・6.3、`work-area-clamp` 4.1 | 合う |
| T2 | n が上端 | x は〃・y は上端 | x だけ〃 | 縦の範囲が p と重ならなければ `none` | 縦が重ならなければ動かさない | — | 〃 | 485（上端も同じ式）、`dpi-realign` 1.2・2.1 | 合う（18.9 で指摘にしなかった） |
| T3 | n が Free・x 無し | 〃（隙間 0） | 〃 | 〃 | 〃 | — | 〃 | `window-placement` 2.6、485 | 合う |
| T4 | n が Free・x あり | 左上から x。次のスコープの基準になる | 動かさない（`free-position`）。基準になる | 隙間 0 で付いていれば `left`／`right`（`defaultx` は位置なので隙間にしない） | 隙間 0 で付いていれば付け直す | — | 〃 | 485 ⑶、494 ⑵ | 合う |
| T5 | n が左右の辺 | 辺に置き、辺ごとに縦に連ねる。鎖を進めない | 鎖の外（動かさない） | `none` | 鎖の外（動かさない） | — | 番号が続けば記憶の矩形で組にする（重なりも組・`work-area-clamp` 4.1） | 指摘 1、485 ⑵、494 ⑴ | 合う |
| T6 | n の 1 つ下が辺のスコープ | p は辺を飛ばした窓 | 〃（覚えた印も p との組として読む・決め (b)） | p との組の印。位置を書いたスコープ k の次の鎖のスコープの印も書く | 〃 | — | 番号が続かない組 (p, n) は窓ごとに寄せる。n−1 が辺で n の印が `left`／`right` だと、辺のスコープと n を組にする（規則と違う） | 485 ⑷、494 ⑴、495 ⑴ | 合う。F は申し送り |
| T7 | スコープ 0 が左右の辺 | 次の鎖のスコープが頭（`right − w − g`・角で重なる） | 頭は動かさない | 頭は印を書かない | 頭は組を持たない（T32 は当たる） | — | 番号が続けば記憶の矩形で組 | 210、18.8 で指摘にしなかったもの | 合う |
| T8 | 中のスコープが無い（0 と 2 だけ） | p は下の一番近いスコープ | 〃 | 〃 | 〃 | — | 番号が続かないので窓ごと | `GhostWindows::scopes` の昇順 | 合う。F は申し送り |
| T9 | `defaultx` が正（36） | 36 空ける | 既定のまま・覚えた `left` とも 36 の隙間（`left-offset`） | 隙間 36（±2）は `left` | 36 の隙間のまま付け直す | — | 印 `left` で組になる | `scope-chain-gap` 2.8、485 ⑴、494 ⑵⑶ | 合う |
| T10 | `defaultx` が負（−36・重なる） | 36 重ねる | 〃（−36） | 左の隙間 −36 は `left` | −36 のまま | — | 〃 | 494 ⑵ | 合う |
| T11 | `defaultx` が大きく負（n の中心が p の中心より右） | 同じ式 | 〃 | 左の隙間が `defaultx` なら `left`（中心より先に見る） | その隙間のまま | — | 〃 | 513 | 合う |
| T12 | `defaultx` が ±1・±2 | 同じ式 | その隙間 | `left` | 隙間が `defaultx` より 0 に近いか同じなら隙間 0、ほかはその隙間 | — | 〃 | 514 | 合う |
| T13 | `defaultx` ≠ 0 の組を利用者が隙間 0 に付けた | — | 次の起動は `defaultx` の隙間で付く | `left` | その回は隙間 0 のまま付け直す | — | 〃 | 494 ⑶（裁定「読み替える」） | 合う |
| T14 | n が p の右に隙間 0 | — | 覚えた `right` は今の幅で右に付ける | `right`（右は隙間 0 だけ） | 右に付け直す | — | 〃 | `dpi-realign` 1.3・6.3 | 合う |
| T15 | Free・左右の辺の `defaultx` | Free は位置、辺は使わない | 隙間 0 | 隙間 0 | 隙間 0 | — | — | 485、494 ⑵ | 合う |
| T16 | 起動: 記憶に位置が無く既定のまま | — | resolver と同じ決め（T1〜T5・T9〜T12） | — | — | — | — | 485 | 合う |
| T17 | 起動: 記憶に位置が無く、動かされた・台本の `move` が置いた | — | 動かさない。鎖に入っていれば実位置が次の基準 | 並べ終えた時点では位置を書かない | — | — | — | `scope-chain-gap` 7.3 | 合う |
| T18 | 起動: 記憶から戻した・印 `left`／`right` | — | 今の幅で p の左（隙間 g）／右（隙間 0）へ | — | — | — | T1 | `dpi-realign` 6.3、494 ⑶ | 合う |
| T19 | 起動: 記憶から戻した・印 `none`／印なし（古い記憶） | — | 動かさない。印なしは並べ終えた時点で印だけ書き足す | 古い記憶へ印だけ（T36 の起動では書かない） | — | — | 記憶の矩形で組 | `dpi-realign` 6.4・6.5 | 合う |
| T20 | 起動: 記憶から戻した・窓の組／台本の印 | — | 動かさない | 窓の組は `none` | — | — | — | `dpi-realign` 6.4 | 合う |
| T21 | 494 の前に書いた印（辺のスコープの次のスコープ） | — | p との組として読み、`debug!` を 1 行 | 次にその組を書いたときに書き直る | — | — | — | 決め (b) | 合う（受け入れた限り） |
| T22 | descript は鎖・台本が最初の確定より前に辺へ動かした | descript で解く | 鎖の外。次の窓は飛ばした前の窓との組 | descript で見る（鎖に入れて印を決める） | 鎖の外 | — | — | 495 ⑵、決め (a) | 合う（S は (a) の限り） |
| T23 | descript は鎖・台本が確定の後に辺へ動かした | — | — | T22 と同じ | 鎖の外。次の窓は前の窓との組 | — | — | 521、決め (a) | 合う |
| T24 | descript は左右の辺・台本が最初の確定より前に下端／free へ揃え直した（x は辺のまま）【515】 | descript で解く（辺） | 鎖の外。辺に残り、次の記憶の無い窓は本体の隣のまま（resolver の位置） | descript で見る（`none`・次の窓は本体との組） | 今の揃え方で鎖に入る（T25 と同じ・下の限り 4） | — | — | 485 ⑷（辺のスコープの次の基準は resolver と同じ）、決め (a) | **直した**（前は決めだけ `desktop-edge` で鎖に残り、次の窓が辺の窓の隣へ動いた） |
| T25 | descript は左右の辺・台本が最初の確定より前に辺から外して動かした（台本の印つき） | 〃 | 鎖の外（次の窓は本体の隣のまま） | 〃 | 今の揃え方で鎖に入る | — | — | 515 の式（descript か今が辺なら外す） | **直した**の式が効く（前は実位置が次の基準）。下の裁定 3 |
| T26 | descript は左右の辺・記憶から戻した・台本が確定の前に外した | 〃 | 鎖の外（保存が書いた印の相手とそろう） | 〃 | 鎖に入る | — | — | 決め (a) | 直したの式が効く |
| T27 | descript は鎖のスコープ 0・台本が確定の前に `right` へ揃えた（x は変わらない） | 鎖の頭 | 鎖の外。次のスコープが頭になり、今の幅で付け直されない | descript で見る | 鎖の外 | — | — | 495 の裁定の文（今の揃え方が辺なら外す） | 今のまま。下の裁定 2 |
| T28 | シェルの切り替え（バルーンの切り替えは揃え方を変えない） | 全スコープを解き直すが、キャラ窓の位置は捨てる | 走らない（確定は窓の一式に 1 度） | 書かない。以後の保存は新しい descript | 以後の詰め直しは新しい descript の隙間と今の揃え方 | 材料（descript）を入れ替え、台本が変えていない窓の揃え方を入れ替えて、その辺へ置き直す。鎖は解き直さない | — | `shell-balloon-switch` 2.7、454・472 | 合う |
| T29 | 拡大率が変わる直前に離れていた・重なっていた | — | — | `none` | 動かさない | — | 重なりは組・離れは窓ごと | `dpi-realign` 2.1・2.2 | 合う |
| T30 | n が窓の組・p か n がドラッグの最中・n を武装の後に台本が置いた・控えが無い | — | — | 窓の組は `none` | 動かさない | — | — | `dpi-realign` 2.3・2.4・3.1・7.2 | 合う |
| T31 | 拡大率を上げる・下げる | — | — | — | どちらも同じ決め（`defaultx` は px のまま） | — | — | `dpi-realign` 1.7 | 合う |
| T32 | 利用者が左右の辺のスコープの隣へ付けた窓（3 スコープの kero の隣・2 スコープの本体の隣）【520】 | — | 次の起動は覚えた位置のまま（辺のスコープは辺へ） | `none`（鎖の頭なら印なし）＝記憶に残らない | 辺のスコープを動かさない基準にして、同じ側へ付け直す | — | — | `dpi-realign` 1.1（隣り合っていた組は隣り合わせたまま）、494 ⑴ | **直した**（D）。S・B は下の裁定 1 |
| T33 | T32 で、鎖の前の窓の隣にも付いていた | — | — | p との組の印 | p へ付け直す（鎖の組が先） | — | — | 494 ⑴ | 直した（T32 の当て方） |
| T34 | T32 で、付けた窓が窓の組・ドラッグの最中・武装の後の台本、または辺のスコープがドラッグの最中 | — | — | — | 動かさない | — | — | `dpi-realign` 2.3・2.4・3.1 | 直した（T32 の当て方） |
| T35 | 辺の kero の隣へ本体（番号の小さい側）を付けた | — | — | — | どちらも動かさない | — | — | 18.9 で指摘にしなかったもの | 合う |
| T36 | 記憶の位置を寄せて表示した起動 | — | — | 印の無い古い記憶へ印を書き足さない（並べ終えた時点・ドラッグ・連れ動き） | — | — | 組の全員が寄せられたときだけ同じ量を動かす | `work-area-clamp` 4.6、391・434 | 合う |
| T37 | ドラッグの確定・連れ動きの保存 | — | — | 書いたスコープ k の印と、k の次の鎖のスコープの印 | — | — | — | `dpi-realign` 6.2、494 ⑴ | 合う |
| T38 | 並べ直し（`resetwindowpos`） | 今の窓の大きさで解く | — | resolver の位置と印 | 武装中なら動かさない窓として記す | — | — | `move-canon` 8.6、`dpi-realign` 6.2 | 合う |
| T39 | 出ている絵の幅が準備と違う | — | 鎖の頭は動かさず、既定のままの窓を今の幅で resolver の式へ。入らなければ反対の側・端（`defaultx` の隙間の組は端） | — | — | — | — | `scope-chain-gap` 7.1・7.2、`work-area-clamp` 1.2、485 | 合う |
| T40 | 並べ直しの材料（`DefaultPlacementBasis`）が無い | — | 下端・`defaultx` 0 で解く（`warn!`） | 同じ（`debug!`） | 同じ（`debug!`） | — | — | 485、507 | 合う |

規則から外れる所（F の列の 2 つ）は追随の持ち物で、下の申し送りに書いた。表の「限り」は下の裁定の表。

#### 裁定が要るもの（要件 5・8.6）

515・520 の裁定（2026-11-15）: 開発者は、9 回目の裁定の表の 2 件の推奨案に「推奨通り」と答えた。

- 515: 推奨案は「起動の確定で、descript か今の揃え方のどちらかが左右の辺なら鎖から外す」。そのとおりに直した（`ChainView::Boot`）。
- 520: 推奨案の文は「詰め直しで、飛ばした辺のスコープ k に、変わる直前の次のスコープ n が隙間 0 で左に付いていたときは、n の組を k との組として判じて k の左へ付ける」。入れた形はこの文より広い: ⑴ 右に付いていた窓（左の辺のスコープの右）も同じ側へ付け直す ⑵ n が鎖の頭のとき（辺のスコープが本体の 2 スコープのゴースト）も当てる ⑶ 飛ばした辺のスコープがいくつもあれば、n に近い順に見て最初に付いていたものを相手にする ⑷ 鎖の前の窓の隣にも付いていた窓は、鎖の前の窓へ付け直す（辺のスコープは見ない） ⑸ 付けた窓が窓の組・ドラッグの最中・武装の後の台本のとき、辺のスコープがドラッグの最中のときは動かさない。この広さは、進行役が開発者に確かめを仰いでいる（裁定の文に無い分＝⑴〜⑶）。その後、下の「この仕様で行く」で、このままにすると最終の判断をした。

総括で、推奨案の式が及ぶ場合と、規則が残す限りが 4 つ見つかった。どれも台本の吸着か辺のスコープがかかわるまれな場合で、コードは下の「今の振る舞い」のとおり。この 4 つは、下の「この仕様で行く」の残す限り 1〜4 として最終の判断にした（どれも推奨のとおり。理由は「まれ」でなく、そこに書き直した）。

| 番号 | 中身 | 今の振る舞い | 推奨 |
|---|---|---|---|
| 1（T32 の S・B） | 辺のスコープの隣に付けた窓の並びは記憶に残らない。印は descript の鎖の前の窓との組だけを表すので、辺のスコープとの組を書く場所が無い | その回の拡大率の変化では付いたまま（520 の直し）。起動し直すと、付けた窓は覚えた位置のまま・辺のスコープは辺へ戻るので、幅が保存のときと違えば離れるか重なる | **今のまま**（9 回目の 520 の推奨と同じ）。覚えるには印に相手のスコープを書き足す＝保存の形が変わる（sylphya の読み・要件 8 の相手）。`chain_rule.rs` と `persist_save.rs` の doc と `doc/COMPAT_ARCHITECTURE.md` に限りとして書いた |
| 2（T27） | descript が鎖のスコープ 0 を、台本が最初の確定より前に `right`（`left`）へ揃え、x が変わらなかったとき、495 の裁定の文（今の揃え方が辺なら鎖の前の窓にしない）で本体が鎖から外れ、kero が鎖の頭になって今の幅で付け直されない（出ている幅が準備と違うぶんだけ隙間か重なりが残り、その位置が記憶に書かれる） | 495 の裁定の文のとおり（この回は変えていない） | **今のまま**（最初の確定より前に本体を辺へ揃える台本で、しかも出ている幅が準備と違うときだけ）。直すなら、起動の確定で「既定の位置から動いていない窓は descript の揃え方で鎖に入れる」を足す（`chain_rule::boot_chain` の 1 か所・規模 S・RED を先に） |
| 3（T25・T26） | 515 の推奨案の式（descript か今の揃え方が左右の辺なら起動の鎖から外す）は、台本が辺から外して `move` で動かした窓と、記憶から戻した辺の窓にも効く。前は、その窓の実位置が次の記憶の無いスコープの基準になり、次のスコープがその窓の隣へ動いた | 次のスコープは本体の隣のまま（resolver の位置）。保存が書く印（descript の鎖の本体との組）ともそろう | **今の形で認める**（式は 9 回目の推奨案そのもの。起動の鎖を「descript の鎖から今辺にある窓を外したもの」の 1 文で言える）。戻すなら、台本の印の付いた窓だけ今の揃え方で決める枝が要る |
| 4（T24・T25 の D） | descript が左右の辺で、台本が下端／free へ揃え直した窓は、詰め直しでは今の揃え方で鎖に入る。その窓が本体と離れている（重なっている）と、次のスコープは本体の隣に付いていても、組の相手がその窓になるので付け直されない（3 スコープ以上のときだけ） | 詰め直しは今の揃え方だけ（9 回目の 515 の推奨「詰め直しは今のまま」・521 の檻） | **今のまま**（descript が辺・台本の揃え直し・3 スコープ・拡大率の変化が重なるときだけ）。descript も見る形にすると、台本が辺から外した窓を利用者が本体の隣へ付けたとき、その窓が付け直されなくなる（2 スコープでも起きる） |

#### 採らなかった形

- 詰め直しの組の相手を「下の番号の窓のうち、変わる直前に隣り合っていたもの全部」から選ぶ形。裁定 4 の場合も塞がるが、番号が続かない鎖の窓どうし（たとえば下端の 3 スコープで、スコープ 2 を本体の隣へ付けた）を付け直す新しい振る舞いが入り、解きが 1 本の列でなくなる（追随の `place_chain_in_work_area` は前のスロットを基準にする 1 本の列を受ける）。完了した `dpi-realign-remembered-chain` の組の作り方を広げるので、この回では採らない。
- 詰め直しの見方を「descript かつ今」にする形（裁定 4 の推奨の理由）。
- 起動の見方で、既定の位置のままの窓だけ descript で決める形（裁定 2 の「直すなら」）。495 の裁定の文を当て直すことになるので、裁定なしには入れない。
- 辺のスコープとの組にも印 `left`／`right` を書く形。次の起動は印を descript の鎖の前の窓との組として読むので、相手を取り違えて本体の隣へ引き寄せる（裁定 1）。

#### RED（直す前のコード・`1feefe6d1`）

`cargo test -p areka --bin areka chain_rule_tests`（足した 2 本が赤・ほかの 7 本は緑）:

- 515（T24）`a_descript_side_edge_scope_realigned_by_the_boot_script_is_not_the_chain_base`: 「scope 2 の基準は本体のまま（辺にいる窓の隣へ動かさない） left: [(Bottom, -156), (Free, -156)] right: [(Bottom, 0), (Free, 0)]」（scope 2 が本体の隣から、右の辺の kero の隣へ 156px 動いた）。
- 520（T32）`a_window_attached_to_a_side_edge_scope_stays_attached_after_100_to_200`: 「付けた窓は辺のスコープの隣のまま（重なったまま残らない） left: [("kero.seriko.alignmenttodesktop", "right", 428), ("kero.seriko.alignmenttodesktop", "left", -428), ("sakura.seriko.alignmenttodesktop", "right", 573)] right: [(…, 0), (…, 0), (…, 0)]」（3 スコープの右の辺・左の辺、2 スコープの本体が辺、の 3 行とも、付けた窓が動かず辺のスコープと 428px・573px 重なった）。

T33・T34 の 2 本（`a_window_beside_both_follows_the_previous_chain_window`・`a_window_attached_to_a_side_edge_scope_is_kept_by_the_exclusions`）は 520 の直しの当て方の檻で、直す前のコードでは辺のスコープへ付け直す道が無いので緑になる（RED は対象外）。壊し方で赤を確かめた: 鎖の前の窓より先に辺のスコープを見る形で「left: 61 right: 0」、ドラッグの最中の辺のスコープを外さない・付けた窓の「動かさない」を見ない形で「left: [0, 0, 0, 0] right: [428, 428, 428, 428]」、起動の見方を今の揃え方だけへ戻す形で 515 の赤。壊し方は狙った編集で入れ、元のファイルの写しから戻して差分が無いことを確かめた（`git checkout`・`restore`・`stash`・`reset` は使っていない）。

配置の crate の `chain_rule_tests.rs` の 7 本は新しい型（`ChainRule`・`ChainView`）のテストで、直す前のコードでは組めない（RED は対象外。振る舞いの赤は上の 2 本）。

#### 直したこと

- `crates/areka-placement/src/chain_rule.rs`（新しい・規則の置き場）: 1 スコープの見方 `ChainTerms`（descript の揃え方・`defaultx`・今の揃え方）、道の見方 `ChainView`（`Saved`・`Boot`・`Realign`）、窓の一式の見方 `ChainRule`（`of`・`of_ghost_windows`・`from_terms`・`joins`・`gap`・`prev`・`next`・`skipped`）、起動の決め `boot_chain_decision`、起動の解きの列 `boot_chain`（本体の `drain_resnap.rs` から移した）、⑷ の相手 `edge_partner`。本体から使う公開の口は `boot_chain` と `BootChain` の 2 つだけを足し、ほかは crate の中の口（`pub(crate)`）。今までの公開の形は変えていない。
- 3 つの道が持っていた写しを消した: 本体の `drain_resnap.rs` の「材料から設定を組む → 揃え方と今の揃え方を引く → 決めを改める → 辺のスコープを外す」（`boot_chain` を呼ぶ 1 行に）、`chain_realign.rs` の `terms` と辺のスコープを外す絞り、`persist_save.rs` の `SavedChain`（`ChainRule` に）。どの道も、鎖に入るか・鎖の前の窓・隙間を `ChainRule` から引く。
- 515（T24〜T26）: 起動の見方 `ChainView::Boot` を「descript も今の揃え方も左右の辺でない」にした。
- 520（T32〜T34）: 詰め直しで、鎖の前の窓との組が「動かさない」になった窓（鎖の頭を含む。窓の組・ドラッグの最中・武装の後の台本・控えの無い窓は除く）について、飛ばした辺のスコープ（近い順・ドラッグの最中と控えの無いものは除く）と変わる直前に隙間 0 で隣り合っていたら、辺のスコープを `Keep(DesktopEdge)` のスロットとして列に入れ、`JoinLeft`／`JoinRight` で付け直す。組ごとの `info!` は辺のスコープの行（`desktop-edge`）と付けた窓の行が出て、入れたことを `debug!` に 1 行残す。
- 起動の確定の「覚えた印を、辺のスコープを飛ばした前の窓との組として読む」の `debug!` は、前のスロットとのあいだに本当に飛ばしたスコープがあるときだけ出す（前は番号が続かないだけで出たので、中のスコープが無いゴーストでも出ていた）。
- doc: `chain_finalize.rs` の「鎖の規則」の節を `chain_rule.rs` の説明へ移して指す形にし、`KeepReason::DesktopEdge` に詰め直しの使い方を足した。`chain_realign.rs` のモジュールの説明に ⑷ を足した。`resolver.rs`・`spawn.rs` の「`chain_finalize` の鎖の規則」を `chain_rule` に。`doc/COMPAT_ARCHITECTURE.md` の判断の表の 2 行（詰め直しの対象・隣り合いの印）に ⑷ と、辺のスコープの隣の並びが印に残らないことを足した。
- 規則が残す限りは、コードの `shortcut:` の行に書いた（`ChainView::joins` の起動の腕＝下の裁定 2、詰め直しの腕＝裁定 4、`persist_save.rs` の印の決め＝裁定 1）。
- 期待を変えた今までのテストは無い。網羅して match する所への追随も無い（`PairDecision`・`KeepReason`・`Adjacency`・`AdjacencyMark` に腕を足していない）。追随の持ち物は 1 行も触っていない。

#### テスト

- 配置の crate `chain_rule_tests.rs`（7 本・表の World を持たない行）: `chain_membership_by_view`（⑴ の見方の 11 行）・`previous_chain_window_skips_scopes_outside_the_chain`（T6・T8）・`a_scope_after_a_side_edge_head_is_the_chain_head`（T7）・`the_gap_is_the_descript_defaultx_of_bottom_and_top`（T15）・`boot_decision_table`（T16〜T20 の 16 行）・`every_path_puts_a_default_pair_where_the_resolver_does`（T1・T9〜T12: `defaultx` が 0・±1・±2・±36・−500 の 8 つで、resolver が置いた組に保存が `left` を書き、幅が変わった後の起動の確定（既定のまま・覚えた印）と詰め直しが、どれも resolver が新しい幅で解く位置へ置く）・`edge_partner_is_the_nearest_side_edge_scope_attached_with_no_gap`（⑷ の 6 行）。
- 本体の `frame_chain_rule_tests.rs`（足した 4 本）: T24（515）・T32（520 の 3 行）・T33・T34（4 行）。
- 今までの檻が押さえる行: T4・T5・T6・T9・T39 は本体の `frame_chain_finalize_align_tests.rs`、T6・T9・T18・T22・T23 は `frame_chain_rule_tests.rs` の前からの 7 本、T29〜T31 は本体の `frame_chain_realign_adjacency_tests.rs`・`frame_chain_realign_exclusion_tests.rs`、T2（縦が重ならない）・T11〜T14 は `chain_finalize_adjacency_tests.rs`、T6・T9・T37・T40 の保存は `persist_adjacency_tests.rs`、T19・T36 は `persist_finalize_save_tests.rs`、T5・T7 の resolver は `resolver_resolve_tests.rs`（`t_r5_top_left_right_resolve_on_their_own_edge`・`t_r5_mixed_edges_chain_per_edge`）。T27・T28・T35・T38 に、この回で足した檻は無い（T27 は裁定待ち・ほかは今までの振る舞いで、この回は触っていない）。その後の穴の見直しで、T27・T35・T38 の檻を足し、T28 の今までの檻を確かめた（下の「これで終わる理由」）。

#### 申し送り（タスク 8.2 が書く）

- `audit-placement-follow` へ（追随の持ち物・この回は読んだだけ。18.8 の申し送りの 1 行を、行と場合を付けて置き換える）: `crates/areka-placement/src/follow/work_area_clamp.rs` の `joined_with_prev`（L488-L496）と `RestoredWindow::mark` の doc（L392-L393「組 (scope−1, scope) の覚えた印」）・`keep_restored_groups` の doc（L404-L411）が、記憶の組を「番号が続く対 (n−1, n)」で作る。鎖の規則では印は descript の鎖の前の窓との組なので、表の T6・T8 で食い違う: ⑴ 辺のスコープを挟んだ組 (p, n)・中のスコープが無い組は番号が続かないので、印が `left`／`right` でも窓ごとに寄せる ⑵ n−1 が辺のスコープで n の印が `left`／`right`（本体との組の印）だと、辺のスコープと n を 1 つの組にする。どちらも、寄せた後に起動の確定が覚えた印で鎖の前の窓の隣へ付け直すので、最後の位置は規則のとおりになる（寄せの途中の組だけが違う）。直すなら、記憶の矩形と一緒に「鎖に入るか」（`ScopePlacement::anchor` と `resolver::joins_chain`）を渡して、組の相手を鎖の前の窓にする。呼ぶ共有の関数（`judge_adjacency`・`resolve_chain`）は直すところが無い。
- 同じく `audit-placement-follow` へ: `opposite`（L298-L305）は `JoinLeftOffset` を反対の側へ回さない（7 回目の記録の決めのまま）。⑷ で足した辺のスコープとの組は `JoinLeft`／`JoinRight` なので、今の `opposite` のまま反対の側へ回る。
- この回に範囲の外で触った所: 本体の `runtime/frame/drain_resnap.rs`（起動の解きの列の組み立てを `chain_rule::boot_chain` へ移した・729 行）・`runtime/frame_chain_rule_tests.rs`（4 本を足した・415 行）・`doc/COMPAT_ARCHITECTURE.md`（表の 2 行）。走っている spec と重なる所は無い。本体の `switch_finish_tests.rs`（999 行）には何も足していない。

#### 確かめ（総括の時点）

- `cargo test -p areka-placement --features test-support`: lib 984 件が通り 1 件 ignore（足した 7 件）、手本 1 件が通る。
- `cargo test -p areka`: 本体の bin 3199 件が通り 6 件が ignore（足した 4 件）、ほかのテストの的も全部通る（知られた揺れは出なかった）。
- `cargo test -p areka-sylphya`: 221 件が通る。`cargo test -p ukadoc-survey`: 全部通る。
- `cargo test -p log-capture-kit`（crate 全体）: 全部通る（`temp_path_guard_test` 16 件・`with_default_guard_test` 24 件・`file_length_guard_test` 6 件ほか）。
- `cargo fmt --all -- --check`: 緑。
- 1,000 行の近く: `lib.rs` 929 行・本体の `drain_resnap.rs` 729 行・`chain_finalize.rs` 688 行・`chain_realign.rs` 501 行・`persist_save.rs` 438 行・本体の `frame_chain_rule_tests.rs` 415 行・`chain_rule_tests.rs` 401 行・`chain_rule.rs` 341 行・本体の `switch_finish_tests.rs` 999 行（触っていない）。
- `git diff --stat Cargo.lock` と `vendors/` の差分は空。追随の持ち物（`src/follow*`・`src/transition_*`・本体の `placement_tests/transition_*`）は 1 行も触っていない。行末は元のまま（`git diff --numstat` は中身の行だけ）。
- 直しで古くなった行の番号の確かめ: この回で触った `.rs` を `<ファイル>.rs:<行>` の形で指す所を、ツリー全体（`.kiro/specs/`・`vendors/`・`.kiro/steering/roadmap-history.md` を除く）から拾った。当たったのは `doc/ukadoc-coverage/briefing-assets.md` の `resolver.rs:293` だけで、この回の `resolver.rs` の直し（519 行の doc）より前の行なので動いていない。記録の文を字面で読む所（テスト・`tools/`）は、移した `warn!`・`debug!` の文について grep して 0 件。

#### 判定（総括の時点）

- 鎖にかかわる「必ず直す」の残りは **0 件**。9 回目の裁定待ちの 2 件（515・520）は、開発者の裁定（2026-11-15「推奨通り」）で、9 回目の推奨案のとおりに 1 つの規則の中で直した（520 の入れた形の広さは、上の裁定の節のとおり確かめを仰いでいる）。
- 表の 40 の場合のうち、今までのコードがもう表のとおりだったものが 33、この回で直したもの（直しの式や当て方が効くものを含む）が 6（T24〜T26・T32〜T34）、今のまま裁定を仰ぐものが 1（T27）。規則が残す限りは上の裁定の表の 4 つ（推奨は 1・2・4 が今のまま、3 が今の形で認める）で、どれも「必ず直す」ではない（台本の吸着か辺のスコープがかかわるまれな場合）。
- 追随の寄せ（F の列）の 2 つの食い違いは、追随の持ち物なので申し送りにした（最後の位置は起動の確定が規則のとおりに直す）。

#### 穴の見直しと足した場合（2026-11-15・開発者の指示「指摘の穴も含め確認」）

総括の表（T1〜T40）と規則を、表に無い場合・通していない道・檻の無い欄・レビューの 4 点（520 の広さ・T35・純関数だけの檻・T27 の「まれ」の根拠）で見直した。コードの振る舞いは変えていない（足したのは檻と記録だけ）。

表に足す場合（列の意味は上の表と同じ）:

| # | 場合 | 道と答え | 出どころ | 扱い |
|---|---|---|---|---|
| T41 | スコープ 0（鎖の頭）の `defaultx` | R: 右の辺から `defaultx` だけ左。B・D: 頭は動かさない。S: 頭は印を持たない | `window-placement` 2.10、`scope-chain-gap` 7.2 | 合う |
| T42 | 同じ左右の辺に 2 つ以上のスコープ（縦の連なり） | R: 辺ごとに下から上へ連ねる。B・D: 縦は扱わない（出ている絵の高さが準備と違っても、縦の連なりは解き直さない）。S: 辺のスコープの印は `none` | 指摘 1、`scope-chain-gap` 7.2（Y は扱わない）、`dpi-realign` 1.4 | 合う（縦の連なりは既定の並びだけ。残す） |
| T43 | 作業領域に入らず反対の側へ回した組・端へ寄せて重なった組 | B・D: 追随の `place_chain_in_work_area` が置く。S: 画面の並びから印（反対の側へ回した組は `right`、重なった組は `none`） | `work-area-clamp` 1.2・2.1、`dpi-realign` 6.2 | 合う |
| T44 | 並べ直し（`resetwindowpos`）のとき、台本が揃え方を変えた窓がある | R と S だけを通る（どちらも descript の鎖）。`Anchored` は変えない | `move-canon` 8.6、決め (a) | 合う |
| T45 | バルーン位置初期化・面の `windowposition` の置き直し | キャラ窓を動かさず、キャラ窓の位置の鍵を書かないので、印も書かない | `dpi-realign` 6.2（位置を書くときだけ） | 合う |
| T46 | ゴーストの読み直し・切り替え | 窓の一式を作り直し、起動と同じ道（B は窓の一式に 1 度） | `dpi-realign` 5.1・5.2 | 合う |
| T47 | 拡大率の変化が最初の確定より前 | D は武装しない。B が新しい大きさで解く | `dpi-transition-atomicity` 設計 C4 | 合う |
| T48 | 幅が 0 以下の窓 | B・D: 動かさず、今の x を次の基準にする（`non-positive-width`） | `scope-chain-gap` 7.1 の縮退 | 合う |
| T49 | シェルを替えて descript の鎖が変わった後の起動で、前のシェルのときに書いた印を読む | B: 今の descript の鎖の前の窓との組として読む（決め (b) と同じ。記憶はゴーストごとで、印に相手の番号は無い） | 決め (b) | 合う（残す） |
| T50 | 窓の組の連れ動きの保存 | S: 書いた全スコープと、その次の鎖のスコープの印。組の窓は `none` | `dpi-realign` 2.3・6.2 | 合う |

道の見直し: 鎖を読む道は R・B・S・D の 4 つと、鎖を解き直さない W、追随の F。ほかに位置や印を書く口は、並べ直し（T38・T44＝R と S）、ドラッグ・連れ動きの保存（T37・T50＝`persist_entries`＝S）、バルーン位置初期化と面の置き直し（T45＝鎖を読まない）、読み直し・切り替え（T46＝B）で、どれも上の 4 つの道のどれかを通る。追随の持ち物が鎖の式を自分で持つのは `joined_with_prev` だけ（申し送り）。

足した檻（どれも今の振る舞いを固定する。壊し方で赤を確かめた）:

- 本体 `frame_chain_rule_tests.rs`:
  - `a_descript_side_edge_scope_realigned_by_the_boot_script_is_not_the_chain_base` を T24〜T26 の 4 行の表にした（T25 台本が動かした・T26 記憶から戻した、を足した）。起動の見方を今の揃え方だけへ戻す壊し方で「left: [-156, -156, 1446, -156] right: [0, 0, 0, 0]」。
  - `a_main_window_aligned_to_a_side_edge_by_the_boot_script_leaves_the_chain`（T27）。起動の見方を descript だけにする壊し方で「left: 2102 right: 2168」。
  - `a_lower_scope_attached_to_a_side_edge_scope_is_not_moved`（T35）。
  - `every_world_path_keeps_the_defaultx_gap`（T1・T9〜T12 を本番の道で: `defaultx` が 0・2・36・−36・−500 の 5 つで、起動の確定（出ている幅が準備と違う）→ 並べ終えた時点の保存 → 詰め直し → 覚えた印 `left` の起動、の 4 つがどれも `defaultx` の隙間）。隙間を 0 にする壊し方で 2・36・−36・−500 の行の保存・詰め直し・覚えた印の起動が赤。レビューの指摘（純関数の `every_path_puts_a_default_pair_where_the_resolver_does` は道そのものを通らない）への答え。
  - `a_missing_middle_scope_is_not_logged_as_a_skipped_side_edge_scope`（T8・T21）。記録の条件を番号の続きだけへ戻す壊し方で「辺のスコープを飛ばしていないのに記録が出た」。
- 配置の crate `reset_pos_tests.rs` の `shifted_windows_return_to_default_and_are_persisted` に、並べ直しの保存が印 `left` も書く確かめを足した（T38）。
- 本体 `frame_chain_finalize_align_tests.rs` に、スコープの番号を列で渡す `spawn_resolved_scopes` を足した（中のスコープが無い World 用。`spawn_resolved` はそれを呼ぶ）。

#### 根拠（ukadoc）

鎖の規則の一次の根拠は ukadoc。沈黙している所は、完了した areka の spec と開発者の裁定に従う。SSP の振る舞いは参考で、正しいとは限らない（この回は SSP を動かしていない）。

| 番号 | ukadoc の所 | 引用 | 規則のどこを支えるか |
|---|---|---|---|
| U1 | `seriko.alignmenttodesktop` https://ukatech.shillest.net/ukadoc-spec/descript_ghost.html#seriko.alignmenttodesktop と補足 https://ukatech.shillest.net/ukadoc-spec/descript_ghost.html#note-1 | 「既定値: bottom」「free: 自由移動。」 | 揃え方の既定は下端。補足が挙げる descript の値は top・bottom・free・osuwari で、left・right は載っていない（沈黙） |
| U2 | `\![set,alignmenttodesktop,方向]` https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-alignmenttodesktop | 「現スコープ側サーフェスを…吸着」「左右方向へのドラッグ移動ができなくなる」「ゴースト終了まで有効。」 | 台本の吸着は現スコープの窓だけに効き、その回かぎり（決め (a)）。左右の辺の窓は横に動かさない（⑴）。ほかのスコープの並びには沈黙 |
| U3 | `sakura.defaultx` https://ukatech.shillest.net/ukadoc-spec/list_shiori_resource.html#sakura.defaultx | 「表示位置の記憶が無いとき…に使われる」「X座標はその設定で決まるため無効」 | 既定の並びは記憶が無いときだけ。左右の辺に沿う設定のとき `defaultx` は効かない（辺のスコープの隙間は 0） |
| U4 | `kero.defaultx` https://ukatech.shillest.net/ukadoc-spec/list_shiori_resource.html#kero.defaultx・`char*.defaultx` https://ukatech.shillest.net/ukadoc-spec/list_shiori_resource.html#char-star-.defaultx | 「本体側の左隣に置かれる。」「1つ前のキャラクターの左隣に置かれる。」 | 指定の無いスコープは前のキャラクターの左隣＝鎖（隙間 0）。前のキャラクターが左右の辺に沿うときの扱いには沈黙 |
| U5 | `\![set,sticky-window,…]` https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-sticky-window-ID-ID | 「その制限が優先され」 | 吸着の制限は、ほかの窓に合わせて動かす決まりより先（辺の窓を鎖の都合で横へ動かさない） |
| U6 | `\![execute,resetwindowpos]` https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-execute-resetwindowpos | 「各ウインドウの位置を初期位置に戻す。」 | 並べ直しは既定の並び（R）へ戻す（T38・T44） |
| U7 | `\![move]` https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-move | 「指定座標まで移動。」 | 台本が置いた窓は引き戻さない。ほかのスコープの既定の並びへの効きには沈黙 |

ukadoc が沈黙していて、areka の決めに従う所:

- 拡大率が変わったときの並び・記憶の形（隣り合いの印）・出ている絵の幅が準備と違うときの解き直し: ukadoc に記述が無い。`scope-chain-gap` 要件 7・`dpi-realign-remembered-chain` 要件 1〜3・6・`window-work-area-clamp` 要件 1・2・4 に従う。
- descript の `alignmenttodesktop` の left・right: U1 の補足に無い（台本の指令 U2 にはある。U3 は「左端/右端に沿って表示する設定」があることを前提にしている）。この spec の指摘 1 の裁定（4 つの辺で解く）に従う。
- 前のキャラクターが辺に沿うときの「左隣」（U4）: 字面は「1つ前のキャラクター」で、辺のスコープを飛ばすとは書かない。辺の窓の隣へ並べるとも書かない。指摘 1・485・494 の裁定（辺のスコープは鎖の外・鎖の前の窓は辺を飛ばす）に従う。
- 台本が本体を辺へ揃えたとき、相方を動かすか（U2）: 沈黙。495 の裁定（今辺にある窓は鎖の外）に従う。

ukadoc と食い違う所（鎖の規則の外・この回は変えない）: U3 は `defaultx` を「基準点(通常はサーフェスの中央下)をこの位置に置く」画面の X 座標と書き、負の値を作業領域の右端からの距離と書く。[2.9.13] の印が付くのは、SHIORI リソースの `sakura.defaultx`（U3）では負の値・`--` の重ね書き・descript にも書けることの 3 つの文で、基準点をこの位置に置く文には付いていない。descript の側（https://ukatech.shillest.net/ukadoc-spec/descript_ghost.html#sakura.defaultx）では、基準点をこの位置に置く文に [2.9.13] が付く。areka は `defaultx` を「鎖の前の窓（本体は作業領域の右端）から左へ空ける量」と読む（完了した `areka-P0-window-placement` 要件 2.10 の de-facto・`areka-P0-scope-chain-gap` 要件 2.8「意味を変えない」）。鎖の規則 ⑵ はこの読みの上に立つ。読みそのものを ukadoc に合わせるかは、鎖の規則とは別の件で、タスク 8.2 で起票する（下の申し送り）。

#### 食い違う指摘と裁定の整理（2026-11-15・開発者の指示）

鎖にかかわる指摘と裁定のうち、逆の向きに引き合うものを並べ、採る側と理由、採らなかった側の行き先を決めた。

| 番号 | 引き合うもの | 採る側と理由 | 採らなかった側の行き先 |
|---|---|---|---|
| C1 | 指摘 1・210（辺のスコープは辺に置き、下端の鎖と角で重なってよい）⇔ `scope-chain-gap` 2.1（スコープは隣り合って並ぶ） | 指摘 1・210。U3 が辺に沿う設定では X が辺で決まると書き、U4 は辺の窓の隣へ並べるとは書かない | 「隣り合って並ぶ」は鎖に入るスコープどうしだけ。辺のスコープと鎖のスコープは組にならない（重なってよい） |
| C2 | `scope-chain-gap` 7.1（出ている幅で隙間 0 に解き直す）⇔ 同 2.8（`defaultx` の意味を変えない） | 2.8（485）。解き直しは resolver の式の当て直しで、隙間は `defaultx` | 7.1 の「隙間 0」は「resolver の隙間（`defaultx`・無ければ 0）」と読む |
| C3 | 485（起動は `defaultx` の隙間を残す）⇔ `dpi-realign` 2.1・6.2（離れていた組は動かさない・印は `none`） | 494 ⑵。既定の並びの組を「離れていた」と読むと、拡大率の変化と次の起動で並びが崩れる | 利用者がわざと `defaultx` ±2px だけ離して置いた組は、既定の並びの組と見分けない（隣り合いとして扱う） |
| C4 | 494 ⑶（覚えた `left` は `defaultx` の隙間で付ける）⇔ `dpi-realign` 6.3（覚えたときの並びを今の幅で） | 494 ⑶（裁定「読み替える」）。印の語を足すと保存の形が変わる | `defaultx` ≠ 0 のスコープを利用者が隙間 0 に付けた組は、その回の詰め直しでは隙間 0、次の起動では `defaultx` の隙間になる（T13） |
| C5 | 494 ⑴（辺のスコープを飛ばして鎖の前の窓と組む）⇔ `dpi-realign` 1.1（隣り合っていた組は隣り合わせたまま） | 両方。鎖の組を先に見て、付け直さない窓だけ ⑷ で辺のスコープの隣へ付け直す（520） | 辺のスコープとの組は記憶に残らない（印は鎖の前の窓との組だけ・残す限り 1） |
| C6 | 495（起動: 今の揃え方が辺なら外す）⇔ 485 ⑷（辺のスコープの次の基準は resolver と同じ＝descript） | 両方（515）。起動の鎖は「descript の鎖から、今辺にある窓を外したもの」 | `scope-chain-gap` 7.3 の「動かされた窓の実位置が次の基準」は、descript の鎖に入る窓だけに当たる（T25・残す限り 3） |
| C7 | 495 の文（今辺にある窓は外す）⇔ `scope-chain-gap` 7.1・7.3（鎖の頭が台本で右の辺へ寄っても、相方を今の幅で付け直す） | 495 の文を頭にもそのまま当てる。descript で右の辺に揃えた本体と相方は鎖の組にならない（210・C1）ので、台本で揃えたときだけ組にすると、辺の本体が鎖に入るかどうかが書き方で変わる。そろえるのは鎖に入るかどうかで、窓の位置ではない（descript で揃えたときは相方が右の辺の角で本体と重なる＝T7。台本で揃えたときは相方が準備の幅で解いた位置のままで、出ている幅との差だけ隙間か重なりが残り、その位置が記憶に書かれる）。U2 は現スコープの窓だけを動かすと書く | 台本が最初の確定より前に本体を辺へ揃えると、相方は鎖の頭になり今の幅で付け直されない（T27・残す限り 2） |
| C8 | 決め (a)（保存は descript）⇔ 494・495（動かす道は今の揃え方） | 両方。U2「ゴースト終了まで有効」＝台本の吸着は次の起動に残らないので、次の起動が読む印は descript で決める | 台本が揃え方を変えた回の保存は、その回の画面の鎖と違う組の印を書きうる（T22・T23） |
| C9 | 515（起動は descript かつ今）⇔ 521（詰め直しは今だけ） | 両方。起動は既定の並びと印（どちらも descript のもの）を当てる道、詰め直しは変わる直前の画面だけを読む道（`dpi-realign` 3.3「どうやってそこへ来たかでなく」） | descript が辺で台本が外した窓は、起動では鎖の外・詰め直しでは鎖の中（T24・T25 の D・残す限り 4） |
| C10 | `dpi-realign` 6.2・6.5（位置を書くとき・並べ終えた時点で印を書く）⇔ `work-area-clamp` 4.6（寄せた起動では古い記憶へ印を書かない） | 4.6（391・434）。寄せた矩形から印を作ると、覚えていない並びへ詰めてしまう | 寄せた起動では、古い記憶の印は寄せずに起動した回まで書かれない（T36。`persist_save.rs` の `shortcut:`） |
| C11 | 494 ⑵・513（負の `defaultx` の組は左の組）⇔ `dpi-realign` 2.2（重なっていた組は動かさない） | 513。既定の並びが重ねて置いた組を「重なり」と読むと、C3 と同じ崩れ方をする | 利用者が `defaultx` と同じだけ重ねて置いた組は、既定の並びの組と見分けない |
| C12 | 514（`defaultx` が ±1・±2 の隙間を保つ）⇔ `dpi-realign` 1.1・1.2（±2px 以内は隙間なく付ける） | `defaultx` ≠ 0 のスコープは 514、`defaultx` が 0 のスコープは今までどおり 0 へ詰める | — |
| C13 | 520・⑷（辺のスコープの隣に付けた窓を離さない）⇔ `dpi-realign` 1.3・`scope-chain-gap` 7.2（番号の小さい側・鎖の頭は動かさない） | 1.3・7.2。本体を動かすと、本体に連なる全部の窓が動く | 辺のスコープの隣へ番号の小さい側の窓を付けた組は付け直さない（T35・残す限り 5） |
| C14 | `work-area-clamp` 1.2（入らなければ反対の側へ）⇔ 485（`defaultx` の隙間の組は resolver の式のまま） | `defaultx` の隙間の組は反対の側へ回さず端へ寄せる（resolver の P4 と同じ向き・7 回目の記録） | 追随の `opposite` は今のまま（申し送り） |
| C15 | 454・472（切り替えで全スコープを解き直す）⇔ `scope-chain-gap` 7.4（確定の後は横へ動かさない） | 7.4。切り替えの解き直しはバルーンの値と揃え方だけを取り、キャラ窓の位置は捨てる | シェルを替えて幅が変わると、組のあいだが開きうる（次に位置を書いたときの印と、次の拡大率の変化・起動が規則どおりに扱う） |
| C16 | 494 ⑴（印は鎖の前の窓との組）⇔ 追随の `joined_with_prev`（番号の続く対） | 494 ⑴ | 追随の持ち物の読みは申し送り（最後の位置は起動の確定が規則どおりに直す） |

#### この仕様で行く

開発者の指示（2026-11-15「整理したうえで『この仕様で行く』と判断されるなら、それはそれでよい」）への答え。鎖は次の仕様で確定とする。コードはこの仕様のとおりで、この回に仕様とコードの食い違いは見つかっていない。

1. 既定の並び: 指定の無いスコープは前のスコープの左隣（U4）。鎖に入るのは、descript の揃え方が左右の辺でないスコープ。左右の辺のスコープは辺に置き、鎖の組にしない（U3・指摘 1・210）。
2. 隙間: 組（鎖の前の窓, n）の隙間は n の `defaultx`（下端・上端だけ。DPI で換算しない）。この読みは areka の完了した spec の決め（上の「ukadoc と食い違う所」）。
3. 鎖の前の窓: n より下の番号で鎖に入る一番近いスコープ。無ければ n は鎖の頭で、動かさず、印も持たない。
4. 起動の確定（窓の一式に 1 度）: descript の鎖から、今左右の辺に揃っている窓を外す。記憶に位置の無い既定のままの窓は resolver の式を今の幅で当て直し、記憶の窓は覚えた印で付け直す（`left` は `defaultx` の隙間・`right` は隙間 0）。動かされた窓・台本が置いた窓・窓の組は動かさない。
5. 隣り合いの印の保存: 位置を書くときだけ、descript の鎖の前の窓との組の並びを書く（左は隙間 0 か `defaultx`、右は隙間 0、どちらも ±2px）。辺のスコープと窓の組は `none`。寄せた起動では古い記憶へ書き足さない。
6. 拡大率の後の詰め直し: 今の揃え方の鎖で、変わる直前に 5 と同じ見方で隣り合っていた組を、同じ側・同じ隙間で付け直す。離れていた・重なっていた・窓の組・ドラッグの最中・武装の後の台本の窓は動かさない。鎖の前の窓へ付け直さない窓が、下の番号の辺のスコープの隣に隙間 0 で付いていたら、その隣へ付け直す。
7. シェルの切り替え: 鎖は解き直さない。descript の値と、台本が変えていない窓の揃え方を入れ替える。
8. 台本の吸着（U2）はその回かぎり。記憶と印は descript で決める。

520 の入れた形の広さ（右の側・鎖の頭・飛ばした辺のスコープの近い順）は、このままにする。6 の「同じ側で付け直す」は左右で同じ決まりで（`dpi-realign` 1.3「隣り合っていた左右の側を保つ」）、左の辺の窓には右隣しか無い。鎖の頭を外すと、辺のスコープが本体の 2 スコープのゴーストだけ直らない。近い順は、相手が 2 つあるときに答えを 1 つにするため。

残す限り（最終の判断。どれも「このままにする」）:

| 番号 | 限り | このままにする理由 | コードの印 |
|---|---|---|---|
| 1 | 辺のスコープの隣に付けた窓の並びは記憶に残らない（T32 の S・B） | 印は「鎖の前の窓との組」という 1 つの意味だけを持つ。相手を書き足すと保存の形と読み手が変わり、今までの印が全部読み替えになる。位置そのものは記憶に残るので、幅が同じなら並びは保たれる | `persist_save.rs` の `shortcut:`（印の決め） |
| 2 | 台本が最初の確定より前に本体を辺へ揃えると、相方は鎖の頭になり今の幅で付け直されない（T27） | C7。辺の本体と相方を鎖の組にしないのは、descript で辺に揃えたときと同じ扱い（鎖に入るかどうかが同じ。窓の位置は同じではない: descript のときは角で重なり、台本のときは相方が準備の幅で解いた位置のまま残る）で、台本のときだけ組にする理由が無い。利用者が相方を本体の隣へ付ければ、6 の後段で付いたままになる | `chain_rule.rs` の `ChainView::joins` の `shortcut:`（起動の腕） |
| 3 | descript が辺のスコープは、台本が辺から外して動かしても起動の鎖の外（T25・T26） | C6。起動の鎖を「descript の鎖から今辺にある窓を外したもの」の 1 文で言え、保存が書く印の相手とそろう | 同じ腕（規則そのもの） |
| 4 | descript が辺で台本が外した窓は、詰め直しでは鎖に入り、その次のスコープの組の相手になる（3 スコープ以上） | C9。詰め直しは画面の今だけを読む。descript も見ると、台本が外した窓を利用者が本体の隣へ付けた組（2 スコープでも起きる）が付け直されなくなる | `ChainView::joins` の `shortcut:`（詰め直しの腕） |
| 5 | 辺のスコープの隣へ番号の小さい側の窓を付けた組は付け直さない（T35） | C13。番号の小さい側（本体）を動かさないのは完了した spec の決めで、本体を動かすと連なる全部が動く | `chain_realign.rs` の辺の相手の所のコメント |
| 6 | 494 の前に書いた印・前のシェルのときに書いた印は、今の descript の鎖の前の窓との組として読む（T21・T49） | 決め (b)。保存の形が同じなので見分けられない。次にその組の位置を書いたときに書き直る | 起動の確定の `debug!`（辺のスコープを飛ばしたとき） |
| 7 | 同じ辺の縦の連なりは、既定の並びだけで、出ている絵の高さでは解き直さない（T42） | 起動の確定と詰め直しは縦を扱わない（`scope-chain-gap` 7.2・`dpi-realign` 1.4） | — |
| 8 | 台本が揃え方を変えた回の保存は、descript の鎖で印を決める（T22・T23） | 決め (a)・C8 | `persist_save.rs` の印の決めの doc |

#### SSP で測るなら（候補・進行役が決める）

ukadoc が沈黙している所で、測れば仕様の裏付けになるもの。この回は測っていない。SSP が正しいとは限らないので、測りは参考にとどめる。

| 番号 | 何を送るか | 何を読むか | 何が分かるか |
|---|---|---|---|
| S1 | 3 スコープのゴーストのシェルの descript に `kero.seriko.alignmenttodesktop,right` を書き、位置の記憶を消して起動する | 起動の直後の 3 つのキャラ窓の矩形 | スコープ 2 が本体の左隣か、辺の kero の左隣か（U4 の「1つ前のキャラクター」の読み・C1・規則 3） |
| S2 | `kero.defaultx,36` を書き、位置の記憶を消して起動する | kero の矩形と本体の矩形 | `defaultx` が画面の X 座標（U3）か、本体から左へ空ける量（areka）か（規則 2 の読み） |
| S3 | 起動の後に本体のスコープで `\![set,alignmenttodesktop,right]` を送る | 送る前後の本体と kero の矩形 | 本体を辺へ揃えたとき kero が動くか（C7・残す限り 2） |
| S4 | 3 スコープで kero のスコープに `\![set,alignmenttodesktop,left]` を送ってから `\![execute,resetwindowpos]` | 3 つの矩形 | 並べ直しが台本の吸着の窓をどう置くか（T44） |

#### これで終わる理由

鎖の所を閉じる条件と、その確かめ方。

1. 場合を全部並べた: 上の表 T1〜T40 と、穴の見直しで足した T41〜T50。軸は揃え方（下端・上端・Free の x の有無・左右の辺）、`defaultx`（正・0・負・中心を越える負・±1・±2）、記憶と印、descript と今の揃え方の食い違い（確定の前後）、拡大率の上げ下げ、保存の時機、寄せた起動、中のスコープの欠け、鎖の頭が辺、幅の変化、幅 0、縦の連なり、切り替え、並べ直し、読み直し。
2. どの道も 1 つの規則から答える: `rg "ChainRule::of|joins_chain\(|chain_gap\(" crates --glob "!*_tests.rs"` で、鎖の見方を組む所は `chain_rule.rs`（起動）・`chain_realign.rs`（詰め直し）・`persist_save.rs`（保存）の 3 つ、`joins_chain`・`chain_gap` を呼ぶのは `resolver.rs`（既定の並び）・`chain_rule.rs`・`chain_finalize.rs` の `boot_default_decision` だけ。本体は `boot_chain` を呼ぶ 1 行だけで、鎖の式を持たない。追随の持ち物で鎖の式を持つのは `joined_with_prev` だけ（申し送り）。
3. 表の行ごとの檻（無印は配置の crate・「本体」は `crates/areka/src/runtime/`）:
   - T1・T9〜T12: 本体 `every_world_path_keeps_the_defaultx_gap`、`every_path_puts_a_default_pair_where_the_resolver_does`、`a_negative_defaultx_gap_past_the_centre_is_still_left`、`a_small_defaultx_gap_is_kept_not_rounded_to_zero`、本体 `boot_finalize_keeps_the_defaultx_gap`
   - T2: `vertically_disjoint_pair_is_apart`、`t_r5_top_left_right_resolve_on_their_own_edge`
   - T3・T15〜T20: `boot_decision_table`、`the_gap_is_the_descript_defaultx_of_bottom_and_top`、本体 `finalize_does_not_pull_back_an_explicitly_moved_scope`・`finalize_does_not_pull_back_a_scope_moved_onto_its_default_position`・`remembered_left_pair_stands_adjacent_at_current_widths`・`remembered_separate_pair_is_not_tightened`・`old_memory_without_mark_stands_at_its_position`
   - T4・T5: 本体 `boot_finalize_leaves_a_free_scope_with_x_in_place`・`boot_finalize_leaves_side_edge_scopes_on_their_edge`、`t_r5_mixed_edges_chain_per_edge`
   - T6: 本体 `boot_finalize_skips_side_edge_scopes_as_the_chain_base`・`boot_finalize_skips_a_side_edge_scope_for_a_remembered_next_scope`・`realign_skips_a_side_edge_scope_as_the_chain_base`、`a_side_edge_scope_is_skipped_as_the_previous_chain_window`
   - T7・T8・T21: `a_scope_after_a_side_edge_head_is_the_chain_head`、`previous_chain_window_skips_scopes_outside_the_chain`、本体 `a_missing_middle_scope_is_not_logged_as_a_skipped_side_edge_scope`、`missing_partner_scope_is_skipped_silently`
   - T13・T14: `chain_adjacency_accepts_the_defaultx_gap_on_the_left_only`、本体 `a_remembered_left_mark_joins_with_the_defaultx_gap`・`a_pair_adjacent_on_the_right_stays_on_the_right`
   - T22・T23: 本体 `a_boot_script_side_edge_scope_is_not_the_base_of_the_next_scope`・`realign_skips_a_boot_script_side_edge_scope_as_the_chain_base`、`chain_membership_by_view`
   - T24〜T26: 本体 `a_descript_side_edge_scope_realigned_by_the_boot_script_is_not_the_chain_base`
   - T27: 本体 `a_main_window_aligned_to_a_side_edge_by_the_boot_script_leaves_the_chain`
   - T28: `t_rs01_new_offset_and_alignment_reach_the_char_window_parts`、本体 `shell_finish_moves_char_windows_to_the_new_alignment`・`shell_finish_replaces_the_default_placement_basis`
   - T29〜T31: 本体 `a_pair_apart_before_the_change_is_not_moved`・`a_pair_overlapped_before_the_change_is_not_moved`・`a_window_in_a_sticky_group_is_not_moved`・`a_dragging_window_and_its_neighbour_are_not_moved`・`a_window_moved_by_script_after_arming_is_not_moved`・`a_window_missing_from_the_snapshot_is_not_moved_and_others_tighten`・`a_round_trip_200_100_200_keeps_the_pair_adjacent`
   - T32〜T35: 本体 `a_window_attached_to_a_side_edge_scope_stays_attached_after_100_to_200`・`a_window_beside_both_follows_the_previous_chain_window`・`a_window_attached_to_a_side_edge_scope_is_kept_by_the_exclusions`・`a_lower_scope_attached_to_a_side_edge_scope_is_not_moved`、`edge_partner_is_the_nearest_side_edge_scope_attached_with_no_gap`
   - T36・T37・T40・T45・T50: `clamped_boot_adds_no_mark_to_old_memory_through_the_neighbours_position`・`clamped_boot_drag_save_adds_no_mark_to_old_memory`・`position_batch_adds_left_and_right_marks_of_touched_pairs`・`missing_basis_marks_with_the_bottom_rule_and_logs_debug`・`balloon_offset_only_batch_gets_no_marks`・`sticky_group_member_is_marked_none`
   - T38・T44: `shifted_windows_return_to_default_and_are_persisted`（位置・印・`Anchored` が変わらないこと）
   - T39・T41: 本体 `boot_finalize_corrects_a_real_width_change_with_the_gap`・`finalize_closes_the_gap_by_moving_the_follower_only`、`every_world_path_keeps_the_defaultx_gap`（本体の `defaultx` 1000）
   - T42: `t_r5_mixed_edges_chain_per_edge`（既定の並び）。起動の確定と詰め直しの解きの指示は x だけ（`ChainMove`）で、縦の連なりを解く道が無い
   - T43: 本体 `a_realigned_window_outside_the_work_area_flips_to_the_other_side`・`remembered_left_partner_outside_rejoins_inside_work_area`（置き方）と T37 の檻（印は画面の並びから）
   - T46・T47: 本体 `restart_writes_once_more_to_the_new_store_only`・`finalize_writes_only_once_per_window_set`、本体 `frame_chain_realign_arm_tests.rs` の `boot_and_finalize`（起動のフレームでは武装しないことを確かめる）
   - T48: `non_positive_width_is_kept_and_is_next_base`・`mark_non_positive_widths_rewrites_only_unusable_joins`
   - T49: T21 と同じ読み（印に相手の番号が無い）。前のシェルの印を読む場合だけの檻は置いていない（T6 の `boot_finalize_skips_a_side_edge_scope_for_a_remembered_next_scope` が同じ道を通る）
4. 残す限りは 8 つで、どれも上の表に最終の理由とコードの印がある。
5. 追随の持ち物の食い違い（`follow/work_area_clamp.rs` の `joined_with_prev`）は申し送りで、最後の位置は起動の確定が規則どおりに直す。
6. 鎖にかかわる「必ず直す」は 0 件。仕様とコードの食い違いは 0 件。

申し送りの足し（タスク 8.2 が書く）: `defaultx` の読みが ukadoc（U3: 基準点の画面の X 座標・負の値は右端からの距離）と違う。持ち主の決めは完了した `areka-P0-window-placement` 要件 2.10 と `areka-P0-scope-chain-gap` 要件 2.8 で、この spec の範囲の外。8.2 で `/kiro-discovery` により起票する（開発者の方針: 意味論は ukadoc から・未対応は範囲外でも全部起票）。`areka-P0-scope-chain-gap` は「別途 SSP 確認済み」と書いて今の読みを決めたので、SSP 2.9.13 で意味が変わった見込みがある。そこは起票した spec で確かめる（S2 の測りはその材料）。

確かめ（この見直しの後）: `cargo test -p areka-placement --features test-support` は lib 984 件が通り 1 件 ignore・手本 1 件、`cargo test -p areka` は本体の bin 3203 件が通り 6 件が ignore（足した 4 件）・ほかの的も全部通る、`cargo test -p areka-sylphya` 221 件、`cargo test -p ukadoc-survey` 全部、`cargo test -p log-capture-kit`（crate 全体）全部、`cargo fmt --all -- --check` 緑。`git diff --stat Cargo.lock` と `vendors/` の差分は空、追随の持ち物は 1 行も触っていない。1,000 行の近く: 本体の `frame_chain_rule_tests.rs` 582 行・`frame_chain_finalize_align_tests.rs` 230 行・`reset_pos_tests.rs` 497 行・`chain_rule.rs` 342 行・`chain_realign.rs` 502 行・本体の `switch_finish_tests.rs` 999 行（触っていない）。

### 18.11 かけ直しの打ち切り（2026-11-15・開発者の決め）

- 開発者の決め（2026-11-15）: 「audit はもう実施しなくて良い。現在判明している絡まった状況に対して回答を作る」。10 回目はかけない。タスク 8.1 のかけ直しは、9 回目（18.9）と鎖の規則の総括（18.10）で止める。
- 9 回目の 20 件は、直した 16 件・申し送りだけの 2 件（511・512）・裁定で直した 2 件（515・520）。0 件の判定には届いていない（9 回目の指摘の後にかけ直していない）が、開発者の決めで打ち切る。
- 「必ず直す」の残り: 0 件。本番の振る舞いを変える指摘で直していないもの: 0 件。
- 残す限り: 18.10「この仕様で行く」の 8 つ（最終の判断・このままにする）。
- 申し送り（タスク 8.2 が書く）: `audit-placement-follow` へ 18.2〜18.10 の各回の申し送り（鎖では `joined_with_prev` と `opposite`）、`audit-areka-app` へ 512 ほか、`defaultx` の読みと ukadoc の食い違いの起票（`/kiro-discovery`・18.10）、SSP の測りの候補 S1〜S4（18.10）。
- タスク 8.2 はここから: 全体テスト（`tools/test-all.ps1`）・clippy・申し送りの書き出し・80 件の突き合わせ。

## 19. 全体テストと申し送り（タスク 8.2・2026-11-15）

### 19.1 clippy（要件 4.4）

- `cargo clippy -p areka-placement --all-targets --features test-support` を枝の頭と、着手の前（`main` との分かれ目 `2f664eb6`）で回して比べた。この spec の直しと重なる指摘は 0 件・消えた指摘も 0 件。直しの途中で足した `type_complexity` 2 件（`chain_rule_tests.rs` の表の行・`reseed_tests.rs` の壊し方の表）は、`type` の別名にして消した（振る舞いは同じ・lib 984 件が通る）。残る 20 件（本番 6・テスト 14）はどれも着手の前からある。`clippy-199-lints` の brief に書いた。

### 19.2 完了の申し送り

- 指摘 13 で、本体の `crates/areka/src/runtime/move_cue_place.rs` の主モニタの作業領域の写しを、配置の crate の公開の関数 `areka_placement::work_area_px` に替えた（要件 2.10）。
- 追随の持ち物の古い文（設計の `follow/work_area_clamp.rs:L18` の「examples は `#[path]` で配置の層を取り込む」）は、`audit-placement-follow` の着地（#460）でもう消えている（今の `work_area_clamp.rs` に `examples`・`#[path]` の文は 0 件）。
- `audit-areka-app` の brief へ: 18.1〜18.9 の本体の分（81 の残り・略称・古い名指し・見送り点の数・記憶の継ぎ目・`assets_tests.rs`・記憶の帳面の名・task 番号・呼び手の数・届かない腕・512 ほかの写し）を件 1〜10 に、行を引き直して書いた。1 回目の 159（`collision-probe.rs` の行の番号と `prepare_stages`）は 2 回目の 295 と 7 回目で直したので渡していない。
- `audit-test-kits` の brief へ: `log-capture-kit/src/lib.rs:28` の「19 crate」（今は 20）。
- `audit-placement-follow` は完了済み（`.kiro/specs/completed/`）なので、追随の持ち物の申し送りは受け手が無い。タスク 8.2 は brief を作らず、進行役へ一覧で返した（起票は進行役が決める）。
- 起票の候補（進行役が `/kiro-discovery` で起票する）: 164（`DraggingState` の「位置が分からない」・C20 の後）と、`defaultx` の読みが ukadoc（2.9.13）と違う件（18.10・SSP の測りの候補 S1〜S4 はその材料）。

### 19.3 突き合わせ

- 80 件: 設計の「80 件と単位の対応」の表は 1〜80 を 1 回ずつ持ち、受け持つタスク（1〜8.1）は全部済み。裁定で残す指摘は 0 件、2 本目の spec は 0 本（議題 7）。かけ直しで出た分の残す 1 件（88）・回す 1 件（164）は 18 節の裁定のとおり。
- `git diff --stat main...HEAD -- vendors/ Cargo.lock` は空。終了の道（`app_exit`・`smoke_exit` ほか）は触っていない（差分は `quit_app` を名指すコメントの言い換えだけ）。
- `cargo test -p log-capture-kit`（crate 全体）・`cargo fmt --all -- --check`・`cargo test -p areka-placement --features test-support` は緑。
- 全体テスト（da71b351d）は 14 段とも緑・回し直しなし（`pwsh -NoProfile -File tools/test-all.ps1`・走らせ始めの未コミットの変更 0）。段: sample fetch・add i686 target・build i686 artifacts・fmt --check・x64 workspace tests（897 秒）・dola toml feature tests・i686 tests (host-32)（48 秒）・crates.io pre-publish check・encoding check・release notes check・roadmap overview check・i686 helper sync check・sample fetch check・i686 helper beside areka.exe。どれも exit code 0。
