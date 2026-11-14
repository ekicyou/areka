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

- 5: 資産の道で焼くのは `areka-emo-present` の `shell_target.rs:L621-L622`（`bake`／`bake_quiet`）と `balloon.rs:L806-L807`。測りの 2 本が動く絵を登録するのはシェルの側だけ（`measure_bake_log_tests.rs:L169-L180`）なので、上限の口はシェルの側だけに足す。本体は `areka-emo-atlas` に直に依存している（`crates/areka/Cargo.toml:L58`）。`build_shell_assets` の呼び手は本番 2 か所（`assets.rs:L252`・`switch_assets.rs:L219`）とテスト。
- 13: 3 か所とも、主モニタを引いた後の `RECT`→`RectPx` の写しが同じ。並べ直しは DPI も要るので、`Option<RectPx>` を返す形でなく、モニタ 1 つを写す `work_area_px` にした。
- 17: sylphya の書き込みは、投函 `SylphyaMsg::PersistPut` → `SylphyaCore::apply` の効果 `SetDottedGlobal`・`PersistSave` → アクターの実行（`actor.rs:L715`・`L731`）→ `save_scope`（`persist/mod.rs:L301-L417`・錠・既存の読み・`apply_entry`・`to_toml_string_over(base, doc)`・commit）。`to_toml_string_over` は `doc` を `base` に重ねるだけなので、消すには `base` の表からも消す要がある。`LastRunning` の「空文字は無し」の約束は、バルーンのずらしでは壊れた値と見分けがつかないので取らない。
- 36: `apply_descript`（`sticky_group_ledger.rs:L262`）は既に `action`（`descript_seed`／`descript_replace`）を受けるが、`L275` が `"seed"` を固定で渡す。呼び手は crate の中の 2 か所だけ。
- 73: `ChainPlan.absent` の読み手は本体の `runtime/frame/zorder_drain.rs:L124`・`L258-L277` とテスト。`GroupElement` は配置の型なので wintf に置けない。値は `(u32, char, u32)` にする。

## 12. 設計の総合（`design-synthesis`）

- **まとめられる形**: 記録の無い縮退（16・26・35・40）は、どれも「今の縮退の手前に `warn!` を 1 行」で同じ形。写し（11・14・32・37・64・65・76・77・80）は、どれも今ある支え（`test_support.rs`・`placement_shared_test_support.rs`・`spawn_test_support.rs`）か新しい 2 つの支えへ寄せる。公開を狭める 6 件（8・19・42・45・51・62）は、表の 3 種類の置き方で 1 つに決めた。
- **作るか使うか**: 新しい依存は使わない（`visibility` のような crate は足さない）。`image` の上限は確保の前の確かめを自前の `pixel_len` で持つ（`image` の `Limits` は `new` で効かない）。
- **削ったもの**: 指摘 27 の `macro_rules!`（テストの網羅の `match` で足りる）・指摘 13 の `primary_work_area`（`work_area_px` で足りる）・指摘 2 の「動かした印」（追随の持ち物に触れる）・指摘 5 のバルーンの側の上限の口（測りのテストが使わない）。

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
| U28 | 追随の `follow/work_area_clamp.rs:L18` の古い文（「examples は `#[path]` で…」） | 追随の側が直した（今の `L15-L20` に `examples`・`#[path]` の語は無い）。完了の申し送りに書く件は 1 つ減る |

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
