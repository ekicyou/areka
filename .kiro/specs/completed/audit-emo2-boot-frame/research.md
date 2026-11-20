# ギャップ分析（audit）: audit-emo2-boot-frame

> 2026-11-20 `/kiro-validate-gap`。この spec では、ギャップ分析の段で audit をかける（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。指摘には 3 つの組を通した番号 1〜114 を付けた。以後の段はこの番号で指す。この文書は分析と選択肢を並べるだけで、直す対象は要件の討議で確定する。

## 0. 要約

- 受け持つ **103 ファイル・34,968 行**を全部、全行読んだ。内訳は本番の名前のファイル 18 本・6,332 行、テスト 85 本・28,636 行（`areka-runtime` 49 本・19,135 行と本体 36 本・9,501 行）。
- **指摘は 114 件**。組ごとの数は、必ず直す 1・直したほうがよい 34・できれば 79。
- 必ず直すのは 1 件だけ。
  - 1: `frame_text_scale_tests.rs` の「変わった 1 窓だけを拾う」の表明が `Result::iter()` を数えていて、何が起きても 1 になる。
- 本番のコードで重いのは次の 3 件（どれも直したほうがよい）。
  - 2: 位置の指令の預かりの列（`move_hold.rs`）に上限が無い。面を出さない窓への `\![move]` は永久に溜まり、毎フレーム O(n²) で見直す。
  - 3: バルーンの文字層の装着に失敗すると、説明の「次のフレームで再試行」は起きず、そのゴーストの間ずっと文字が出ない。
  - 4: 窓の側の scope を `2*scope+1` に直す所に `i32::MAX` の上限が無い〔件 9〕。今は scope が {0,1} だけなので届かない。
- brief に預かった 14 件（要件 5）は、今の `main` で全部確かめ直した（3 節）。どれも今も当てはまる。
  - 件 13（要件 5.14）: 本番の相の並びを読む限り、札（`DpiSyncHold`）が外れるフレームに拡大率の相は `DpiReproject` でその窓を、入れ直した揃え方で書く（5.15 には当たらない）。ただし、それを通る決定論のテストは範囲の中に **1 本も無い**。問い 4 の仮定の「無いと分かったときだけ議題にする」に当たる（6 番・議題 4）。
  - 件 3（要件 5.3）: 要件の書く直しの文「読み手の絞り込みは前方一致なので漏れはせず」は、`areka-runtime` の側から見ると事実と違う（46 番・議題 5）。
- 開発者の裁定が要る候補は 9 件（2・46・55・64・67・68・72・80・94）。外への約束（要件 1.6）に響く指摘は 4 節。C23 の約束に触れるのは 7 節（2 番は `doc/COMPAT_ARCHITECTURE.md` §8 の行に響く）。
- 規模は **24〜28 タスク**と見込む。20 をはっきり越える（議題 1）。

---

## 1. 測り直しと作法（要件 1.1〜1.4）

### 1.1 取り込んだ main と範囲の大きさ

- 作業のブランチ（`claude/kiro-start-audit-emo2-boot-frame-c1ff35`）の HEAD は `6f2d6ffc`。main `9bf6e32e` の上に、spec の初期化のコミットが 1 つ載っている（`spec.json`・`requirements.md` だけ）。`git fetch` の後に `git merge-base --is-ancestor origin/main HEAD` で、main を取り込み済みと確かめた。コードは main と同じ。
- 範囲は Project Description のとおり。`crates/areka-runtime/src/` の `frame.rs`・`frame/`・`frame_*.rs`（66 本）、`crates/areka/src/runtime_tests/` の `frame.rs`・`frame/`・`frame_*.rs`（33 本）、`crates/areka/src/session/ghost_quit*.rs`（4 本）。

| 区分 | ファイル | 行 |
|---|---:|---:|
| 本番の名前のファイル（`frame.rs`・`frame/` の `_tests`／`_test_support` でないもの・`ghost_quit.rs`） | 18 | 6,332 |
| `areka-runtime` のテストと支え（`frame_*.rs`・`frame/*_tests.rs`・`frame/*_test_support.rs`） | 49 | 19,135 |
| 本体のテスト（`runtime_tests/frame*`・`session/ghost_quit_*tests.rs`） | 36 | 9,501 |
| **計** | **103** | **34,968** |

- 要件の Project Description の実測（103 ファイル・34,968 行・本番 6,332・テスト 28,636）と合う。
- 1,000 行を越えるのは `frame_transition_branch_tests.rs`（1,271 行・`crates/log-capture-kit/tests/file_length_guard_test.rs` の上限越えの表に載る）だけ。900 行台は `frame_test_support.rs` 955・`frame_attach_tests.rs` 947・`frame_balloon_offset_follow_tests.rs` 947。どれもテストを 1〜2 本足す直しで 1,000 行を越える。直す前に縮められる写し（26・29・30・31 番）がある。

### 1.2 ponytail の版

- 今入っているのは 5.3.3。`~/.claude/plugins/installed_plugins.json` の `ponytail@ponytail`（user scope・`lastUpdated` 2026-11-06）で確かめた。brief の起票のときは 5.3.2。物差し（`skills/ponytail-audit/SKILL.md`）は 5.3.2 と 5.3.3 で同じ（`audit-emo2-boot-spine` の研究の 1.2 節で確かめた結果と同じ版）。

### 1.3 作法

- ponytail-audit の 6 つの物差し（バグ・危ない所・負荷・テストの抜け・遅い所・要らないコード）で見た。どの指摘にも具体的な場合を付けた。「使われていない」と書く前には、ツリー全体を grep した。
- 外した作法は 2 つ。
  - 「最大 20 件」を外し、全件を出した（要件 1.3）。
  - 「読まなかった所」を外し、103 ファイルを全部、全行読んだ（要件 1.4）。範囲を 10 の束に分けて読み手を立てた。どの束からも、全ファイルを全行読んだ旨と、ファイルごとの行数の表を受け取った。合計は 34,968 行で、上の表と合う。
- `shortcut:`・`ponytail:` の注釈と、doc つきの `#[allow(dead_code)]` は、範囲の中に **0 件**。範囲の中の `#[allow(...)]` は、`frame.rs` と `frame/wiring.rs` の `unused_imports` 23 本（件 4・5 番）と、`frame/switch.rs:L524` の `clippy::too_many_arguments`（理由は今の事実・引数 9 本）だけ。
- プロジェクトの決まりへの違反も指摘に数えた。記録の無い失敗の道（`let _ =` で失敗を捨てる）・1,000 行・決定論のテスト（壁時計の待ち）・一時フォルダは `target\` の下・終了の道・1 フレーム遅らせない・古くて事実と違う説明、の 7 つ。
- 想定した負荷は次のとおり。
  - 利用者 1 人のデスクトップで areka が 1 つ動き、ゴーストの窓が数枚ある。ECS の相が毎フレーム（60fps 前後）回る。何時間も動き続ける。
  - 他人のゴースト・シェル・バルーンのフォルダ（descript・台本）は信用できない入力として扱う。
  - テストは `cargo test -p areka -j 4` で GPU のテストと CPU を取り合う状態を、ふだんの負荷とみなした。
- cargo のテストは回していない（読むのと grep だけ）。cargo を回したのは件 4 の確かめの 2 回だけ（`cargo rustc -p areka-runtime --lib -j 4 -- --force-warn unused_imports` と、同じものを `--profile test` で。どちらも成功・それぞれ約 2 分）。`--features test-support` 付きは回していない（3 節の件 4）。

### 1.4 読んだ分け方（10 の束・全部読んだ）

| 束 | 中身 | ファイル | 行 |
|---|---|---:|---:|
| G1 | 本番 `frame.rs`・`frame/{wiring,attach,balloon_face,balloon_offset_follow,scale_text,scale_notice,status_report,window_state,move_shift,move_hold}.rs` | 11 | 3,269 |
| G2 | 本番 `frame/{dpi,drain_resnap,switch,work_area_sync,zorder_descript,zorder_drain}.rs`・`session/ghost_quit.rs` | 7 | 3,063 |
| G3 | `frame/` の下のテストと支え 14 本 | 14 | 3,607 |
| G4 | `frame_attach_tests`・`frame_balloon_offset_*` 4 本・`frame_balloon_runtime_test_support`・`frame_test_support`・`frame_shared_test_support` | 8 | 4,227 |
| G5 | `frame_chain_finalize_tests`・`frame_chain_realign_*` 4 本・`frame_default_pos_track`・`frame_diag_route`・`frame_dpi_drag_grab`・`frame_exit_gate`・`frame_harness` | 10 | 3,820 |
| G6 | `frame_dpi_reproject_none`・`frame_dpi_reproject`・`frame_dpi_sync_hold`・`frame_dpi`・`frame_drain_text`・`frame_move_hold`・`frame_move_order`・`frame_resnap`・`frame_schedule`・`frame_sticky_no_follow` | 10 | 3,595 |
| G7 | `frame_text_scale`・`frame_transition_atomicity`・`frame_transition_branch`・`frame_window_link_drain`・`frame_work_area_resnap_hold`・`frame_work_area_resnap`・`frame_work_area_sync` | 7 | 3,886 |
| G8 | 本体 `runtime_tests/frame.rs`・`runtime_tests/frame/` 13 本・`session/ghost_quit_*tests.rs` 3 本 | 17 | 3,911 |
| G9a | 本体 `frame_attach_carry`・`frame_balloon_timeout_notice_e2e`・`frame_chain_finalize_*` 8 本 | 10 | 2,508 |
| G9b | 本体 `frame_chain_rule`・`frame_move_order`・`frame_scale_notice_integration`・`frame_schedule`・`frame_shell_box_*` 3 本・`frame_stay_on_top_lifecycle`・`frame_visibility_integration` | 9 | 3,082 |
| 計 | | **103** | **34,968** |

- ほかに、要件 5.5 の「開けた公開の使い手」を 1 つの読み手にツリー全体で grep させた（3 節の件 5）。
- 束の数え直し: 10 の束の一覧を合わせると 103 本で、範囲の一覧と一致し、重なりは 0。
- 主な指摘は、根拠の行まで読み直して確かめた（8 節）。

---

## 2. 指摘の一覧（通し番号・3 つの組を通す）

各指摘の印は次のとおり。

- 〔約束 a/b/c〕: 直すと「外への約束」に触れる。(a) ゴーストと SSP 互換の振る舞い、(b) `areka-runtime` の公開の面（テストの支えの公開の補助を含む）、(c) ログの行の語と target。響く読み手は 4 節。
- 〔裁定 ⑴/⑵/⑶〕: 要件 2.2 の 3 種類（⑴ 開発者の裁定に反する直し・⑵ わざと置いた先送りの印・⑶ SSP 互換か ukadoc の正典に関わる振る舞いの変化）に当たる、またはその候補。
- 〔件 n〕: brief に預かった件（3 節の件 n）と同じ所を指す（同じ直しを 2 回数えない・要件 1.8）。
- 〔範囲の外〕: 直しの本体か一部が、範囲の外のファイルに要る（持ち主を書く）。
- 〔C23〕: 直すと C23 のほかの 3 本の触るファイルに及ぶ（7 節）。
- 規模: S は 1 タスク以内の小さな直し、M は 1〜2 タスク。

### 件数

| 組 | 番号 | 件数 |
|---|---|---:|
| 必ず直す（Must fix） | 1 | **1** |
| 直したほうがよい（Should fix） | 2〜35 | **34** |
| できれば（Nice to have） | 36〜114 | **79** |
| 計 | | **114** |

- 束ごとの読み手が出した番号（G1-1 など）は、まとめるときに付け直した。読み手が出した 132 件を、同じ所か同じ直しを指すものどうしで 17 の指摘（8・22・26・27・29・30・31・33・36・41・43・46・47・57・72・84・101 番）にまとめ（21 件減）、読み手の注と件 5 の読み手から 3 件（6・113・114 番）を足して 114 件になった。対応は付録 A。

### 2.1 必ず直す（1）

#### 1. 「変わった 1 窓だけを拾う」の表明が、何が起きても 1 になる
- 場所: `crates/areka-runtime/src/frame_text_scale_tests.rs:L202-L216`（数えるのは L206-L212）
- 種類: バグ（落ちない表明）
- 何をする所か: 拡大率の相が持ち続ける観測器（`wiring.dpi_state`）が、実際に DPI が変わった窓を 1 枚だけ拾うことを確かめる。
- 問題（具体的な場合）: bevy_ecs 0.19.1 の `SystemState::get` は `Result<Query, SystemParamValidationError>` を返す。L206-L212 は `.get(&world).iter().count()` で、同じテストの L176・L195 と違い `.expect(..)` を挟んでいない。そのため `.iter()` は Query ではなく `Result::iter()` に当たり、Ok なら数は必ず 1 になる。観測器が変化を一切拾わなくなる退行でも、4 窓全部を拾う退行でも、`matched_after_change == 1` は緑のまま。L213-L216 の「検知が生きている」は何も確かめていない。
- 直し方: L210 の後ろに `.expect("DPI changed query validation should succeed")` を足す（1 行）。直す前に「観測器の結果を空にする」と赤になることを確かめる（要件 4.2）。
- 直さないと: 本番の経路で観測器が盲目になる退行を、この表明は見つけない。
- 規模: S

### 2.2 直したほうがよい（2〜35）

#### 本番のコード

#### 2. 位置の指令の預かりの列に上限が無い〔約束 a〕〔裁定 ⑴⑶〕〔C23〕
- 場所: `crates/areka-runtime/src/frame/move_hold.rs:L135-L148`・`L162-L168`・`L188-L271`（呼び手は `frame/drain_resnap.rs:L176-L182`）
- 種類: 負荷（危ない所でもある）
- 何をする所か: `\![move]` を、対象と基準の窓が最初の面を表示するまで預かる。同じ窓に関わる後の指令は、預かった指令を追い越さない。
- 問題（具体的な場合）:
  - 起動から一度も面を出さない窓（相方を `\1\s[-1]` のまま出さないゴースト）は、`applied_ratio` が `None` のまま。その窓を対象にした `\![move]` は永久に預かられる。
  - たとえば `OnSecondChange`・`OnMinuteChange` の台本が `\p[1]\![move,…]` を出すたびに、列が 1 件ずつ増え続ける。減るのは並べ直しの指令（`resetwindowpos`）が来たときだけ。
  - 毎フレーム列の全件を判定し直し、`Blocked.scopes` は `Vec` を `contains` で引く。件数 n に対してフレームごとに O(n²)。毎秒 1 件なら 1 時間で 3,600 件、毎フレームおよそ 1,300 万回の比べになる。
  - `\0\![move,…,--base=1]` が 1 件預かられると、その後の scope 0 への `\![move]` も全部「追い越さない」で止まる。止まりが記録に出るのは届いた巡の debug の `move_hold_behind` だけ。
- 直し方の候補（決めない）:
  - ⑴ 同じ scope への後の `Move` が届いたら、前に預かった `Move` を捨てる（並べ直しが移動を捨てる `move_hold_superseded` と同じ形）。列の長さは scope の数で頭打ちになる。
  - ⑵ 振る舞いを変えずに `Blocked.scopes` を `BTreeSet` にし、預かりの件数に上限を置いて越えた分を `warn!` で落とす。
  - ⑶ ⑵ の前半（集合にする）だけ。O(n²) は消えるが、列が伸びる問題は残る。
- 直さないと: 何時間も動く間に毎フレームの手間が膨らむ。相方を出さないゴーストでは scope 0 の移動まで効かなくなる。
- 〔裁定 ⑴〕: 完了した `window-work-area-clamp` の設計の話し合い 2（2026-10-29）の「待つ間は順のまま預かり、取りこぼさない」に触れる（上限は決めていない）。
- 〔裁定 ⑶〕〔C23〕: 預かりの振る舞いは `doc/COMPAT_ARCHITECTURE.md` §8 の沈黙ルール対応表の行（L439・「位置の指令は…表示の後に当てる。同じ窓への指令は順を追い越さず」）に記録されている。⑴ と ⑵ はこの行の書き換えを要る。§8 は C23 の `hit-id-pass`（末尾への追記）と `balloon-flip-ssp-review`（既にある別の行の書き換え）が触る（約束 ⑷）。議題 2。
- 規模: S〜M

#### 3. バルーンの文字層の装着に失敗すると、再試行が無いのに「次のフレームで再試行」と書いてある〔約束 a〕
- 場所: `crates/areka-runtime/src/frame/attach.rs:L160-L163`・`L413-L415`・`L433-L440`・`L462-L470`・`L494-L500`、`frame/scale_text.rs:L117-L127`
- 種類: 危ない所（古い説明でもある）
- 何をする所か: 装着の相が、バルーンの文字層のスロットへ actor を登録する。
- 問題（具体的な場合）:
  - `text_slot_view` が `None` の道を、doc は「次フレーム再試行へ委ねる（R4.2）」と書く。実際は L463 で `attached = true` が立ち、装着の相は二度と走らない。
  - 毎フレームの `run_text_scale_phase` は `refresh_actor_scale` を呼ぶだけで、未登録の actor は「装着は register_actor_view の領分」と debug で抜ける（`crates/areka-emo-text/src/actor_attach.rs:L374-L380`）。
  - 面 0 の `ShowSurface` で表示の状態が立たなかったバルーン（他人のバルーンの面 0 が読めない など）は、そのゴーストの間ずっと文字が出ない。記録は最初の `warn!` 1 行だけ。後で view が取れるようになると、「未登録 actor」の debug が毎フレーム（60 行/秒）出続ける。
  - `connect_balloon_text` が返す `bool` も L433 で捨てている。
- 直し方の候補: ⑴ `run_text_scale_phase` で、view が取れて未登録の actor なら `connect_balloon_text` で登録し直す。⑵ 再試行をやめ、doc を「再試行しない」に直して `error!` にする（記録の無い失敗の道を消す）。
- 直さないと: 文字の出ないバルーンが記録 1 行のまま残る。
- 未確認: 他人のバルーンで面 0 の `ShowSurface` が本当に失敗しうるか（資産を組む段で弾かれるか）は、設計の段で確かめる（Research Needed）。
- 規模: S

#### 4. 窓の側の scope を表示対象の番号に直す所に `i32::MAX` の上限が無く、形も所ごとに違う〔件 9〕〔範囲の外〕〔C23（入口の案だけ）〕
- 場所:
  - `crates/areka-runtime/src/frame/dpi.rs:L413-L426`（`u32::try_from` だけ）
  - `frame/drain_resnap.rs:L380`・`L675`（`scope as u32`＝桁を切り捨てる）
  - `frame/attach.rs:L105`（`try_from` だけ）・`frame/move_shift.rs:L53`（`as u32`）・`frame/switch.rs:L354-L359` と `frame/zorder_drain.rs:L196-L199`（`try_from(..).ok()` で黙って落とす）
  - 範囲の外: `crates/areka-runtime/src/balloon_visibility_phase.rs:L464`（`as u32`）
- 種類: 危ない所
- 何をする所か: 窓に付いた印の scope（`CharWindowMarker.scope: usize`）を `shell_target(2*scope)`／`balloon_target(2*scope+1)` へ渡し、表示側の寸を引く。
- 値の出どころ: 印の scope は `areka-placement/src/spawn.rs:L507` で付く。もとは `config.rs:L152-L164` の `detect_scopes` で、今は {0} か {0,1} しか作らない（`char{n}.` からは足さない・`extra-character-windows` が絞りを外す）。**今は信用できない入力が届く道は無い。**
- 問題（具体的な場合・`extra-character-windows` が `char{n}.` の n を他人の descript から読むようになった後）:
  - scope＝2^31 のとき、dpi.rs の `try_from` は通る。debug のビルドでは `2*scope` があふれて毎フレームの相の中で panic する。release のビルドでは折り返して `TargetId(0)`（scope 0 のシェル）になり、scope 0 の報告で別の窓の寸を書く。
  - scope＝2^32 のとき、drain_resnap.rs の `as u32` は 0 に切り捨て、scope 0 の物理寸でその窓の再スナップと連鎖の確定を判断する。
  - 台本の側（`target_map.rs:L39-L45` の `scope_of`）は `audit-emo2-boot-spine` で `s <= i32::MAX as u32` の上限を置いた。窓の側はこの入口を通らない。
- 直し方の候補: `target_map.rs` に `pub fn window_scope(scope: usize) -> Option<u32>`（`u32::try_from(scope).ok().filter(|&s| s <= i32::MAX as u32)`）を 1 本足し、`scope_of` の上限も同じ定数にそろえる。dpi.rs L415（`warn!` を残して捨てる・要件 5.10）・drain_resnap.rs L380・L675（`None` は今の `continue`／見送りの腕へ）・attach.rs L105 で使う。直す前に、あふれで赤になるテストを立てる（要件 5.10）。入口（`detect_scopes`）にも上限を置くのが本筋だが、`config.rs` は `crates/areka-placement/`（C23）なので、持ち主の候補 `extra-character-windows` の brief へ申し送る。
- 〔範囲の外〕: `target_map.rs` と `balloon_visibility_phase.rs` は範囲の外（持ち主の `audit-emo2-boot-spine` は完了済み・`balloon_visibility_phase*` は `audit-emo2-boot-switch` の範囲）。要件 3.2 の「直しのために要る範囲の外の変更」に当たる。
- 直さないと: 3 人目以降の窓を開けた日に、他人の descript 1 行で別の窓の寸を書くか、debug のビルドで落ちる。
- 規模: M

#### 5. `frame.rs` の import の多くが両方のビルドで死んでいて、許可の理由が事実と違う〔件 4〕
- 場所: `crates/areka-runtime/src/frame.rs:L92-L135`・`L147-L154`・`L171-L192`、`frame/wiring.rs:L11-L21`
- 種類: 痩せ（削除）／古い説明
- 何をする所か: テストの `use super::*;` のために import を残し、`#[allow(unused_imports)]` で抑えている。
- 問題（具体的な場合）:
  - 理由の文「`areka`（lib target を持たない bin クレート）」（`frame.rs:L96`・`L152`、`wiring.rs:L12`）は事実でない。ここは lib を持つ `areka-runtime`。
  - `--force-warn unused_imports` で測ると、lib でもテストでも使わない名が 14 ある（3 節の件 4 の表）。`L131`（`resize_window_keep_position`・`resize_window_to`）と `L133`（`SizePx`）は文ごと死んでいる。
  - `L153` の `pub use attach::{…}` は `pub mod frame` の下の公開の再輸出なので、どちらのビルドでも警告にならない。許可は要らず、`L147-L151` の「非 test ビルドで未使用は 2 名」も誤り（実際は 0 名）。
  - `wiring.rs` の 2 本（`World`・`TargetId`）は `cfg(any(test, feature = "test-support"))` のメソッドが使うので許可は要るが、理由の文が誤り。
- 直し方: 両方のビルドで死んでいる名は消す。テストでだけ使う名は `#[cfg(test)] use …` に分けて許可を外す（`L171`・`L178`・`L188` の 3 本の束もテストでだけ要る＝同じ形にできる）。`L153` の許可を外す。`wiring.rs` は許可を残し、理由を「test と test-support でだけ使う」に直す。
- 直さないと: 死んだ import と事実と違う理由が残る。
- 規模: S

#### テストの抜け（落ちない・空振りする檻）

#### 6. 整合待ちの札が外れるフレームに、シェルの切り替えで見送った窓を拡大率の相が書くことを、本番の相の並びで確かめるテストが無い〔件 13〕
- 場所: 足す先の候補は `crates/areka-runtime/src/frame_dpi_sync_hold_tests.rs`。本番は `frame/dpi.rs:L356-L401`（札の窓を対象に入れる和集合と札の付け外し）・`L403-L497`（書く）、`frame/switch.rs:L564-L573`（`apply_shell_descript`→`reanchor_char_windows`）、`crates/areka-placement/src/reseed.rs:L282-L296`（`reseed_held`）。
- 種類: テストの抜け
- 何をする所か: シェルの切り替えの置き直しは、札のある窓を見送る。札が外れるフレームに拡大率の相が `DpiReproject` で新しい揃え方に置く、という前提で成り立っている（`audit-placement-follow-residue` の決め 5）。
- 問題（具体的な場合）: 本番の流れは読みのうえでは成り立つ（3 節の件 13）。けれども範囲の中にこの流れを通るテストは 0 本。範囲の外の `reseed_tests.rs` の `t_rs11`（L577-L603）は札を手で外し `resize_window_to(.., DpiReproject)` を直に呼ぶ写しで、`dpi_phase_with` の和集合（`dpi.rs:L362-L372`）も札の解除（`apply_dpi_phase_gate`）も通らない。たとえば和集合の L362-L372 を消す（92 番の最適化で `Or<…>` に替えるときに書き損じる）と、札の窓は DPI の変化が無い限り二度と書かれず、シェルの切り替えの前の揃え方のまま残る。それでも全部のテストが緑になる。
- 直し方の候補（読み手の案）: `FrameHarness::new()`→整地（96）→拡大率の通知を先に届ける（札が付く）→`apply_shell_descript` で揃え方を `top` に入れ直す→`areka_placement::reseed::reanchor_char_windows` を呼ぶ（`reseed_held` の debug 1 行・書き込み 0）→表を 192 に追いつかせて `run_placement_phases`→その窓の書き込みがちょうど 1 回で経路の語が `DpiReproject`・上辺が作業領域の上端（揃え方 `top`）・札が外れている、を見る。陽性の対として札の無い scope 1 は切り替えの段で `AnchorChange` で動く。`areka-placement` は呼ぶだけで、ファイルには触らない。
- 直さないと: 配置の crate の前提が、本番の相の並びのどこでも固定されないまま残る。
- 議題 4（問い 4 の仮定の「範囲の中に 1 本も無いと分かったときだけ議題にする」に当たる）。
- 規模: S

#### 7. 拡大率の観測器が「毎回作り直して入れ直す」形に戻っても、本番の経路のテストが区別できない
- 場所: `crates/areka-runtime/src/frame_text_scale_tests.rs:L145-L200`（説明 L152-L154・空撃ちの get は L175-L179）
- 種類: テストの抜け（空振りする檻）
- 問題（具体的な場合）: L172-L179 でテスト自身が `get` を 1 回空撃ちして変化を消費してから「0 件」を見ている。`dpi_phase_with` の `state.get_or_insert_with(..)`（`frame/dpi.rs:L353`）を `*state = Some(SystemState::new(world))` に変える退行（毎 run 全窓に当たる churn）でも、`is_some`・空撃ちの後の 0・新しい観測器の >0 はどれも緑のまま。`dpi_phase_with` の単体の永続性は `frame_dpi_tests.rs:L319-L359` が押さえるが、wiring の欄を使う本番の経路は押さえていない。
- 直し方: `run_dpi_phase` を 2 回目も本番の経路で回し、2 回目の前に 1 窓だけ DPI を変えて、2 回目に拾った窓がその 1 窓だけであることを見る。空撃ちを消す。
- 直さないと: 本番の配線だけが churn に戻っても、どのテストも赤にならない。
- 規模: S

#### 8. 毎フレームの相の登録先のテストが、本番の関数でなく写しを試し、写しの見張りも必ず通る
- 場所: `crates/areka-runtime/src/frame_schedule_tests.rs:L15`・`L19-L21`・`L38-L40`・`L59-L64`・`L105-L110`・`L139`
- 種類: テストの抜け／痩せ（削除）／古い説明
- 問題（具体的な場合）: L107-L110 は `code_only(include_str!("frame_schedule_tests.rs"))` に `BOOT_REGISTRATION` の字面が含まれるかを見る。ところが L40 の `const BOOT_REGISTRATION: &str = "add_systems(Update, frame_system.after(update_typewriters));"` の字面そのものが本文に残る（`code_only` が落とすのは `//` の行だけ）ので必ず通る。`register_like_the_boot` を `add_systems(FrameFinalize, frame_system)` に書き換えても L107 は緑。写しを作った理由「本番の `wire_runtime` は実ゴーストの資産を要する」（L19-L21）も今は当たらない。登録は `lib.rs:L706-L710` の `pub fn register_frame_system(world)`（3 行）で、空の World に `Schedules` を置けば決定論で呼べる。説明の「`mod.rs` 手順 6」「`wire_runtime` 手順 6 の登録」（L15・L38）も古い（テストが読むのは `lib.rs`・登録は `register_frame_system`）。
- 直し方: `register_like_the_boot` と L105-L110 を消し、L139 で `crate::register_frame_system(&mut world)` を呼ぶ。説明を `register_frame_system` を名指す文に直す。
- 直さないと: 本番の登録を変えても、構造のテストは古い写しを試し続ける。
- 規模: S

#### 9. 本体の `frame_schedule_tests` が本番の `wire_kanade_stop` を呼ばず、登録の 1 行を写して字面で見張る
- 場所: `crates/areka/src/runtime_tests/frame_schedule_tests.rs:L44-L67`（`session/ghost_quit.rs:L116` の「`frame_schedule_tests` の 1 本が固定する」も）
- 種類: テストの抜け（写しを試す檻）／痩せ（削除）
- 問題（具体的な場合）: `wire_kanade_stop`（`session/ghost_quit.rs:L117-L122`）は `pub` で、`Schedules` と受け手だけで動く決定論の関数。今の形ではテストが自分で `insert_non_send(KanadeNoticeRx(rx))` するので（L64）、`wire_kanade_stop` が受け口を据えなくなっても緑。
- 直し方: `wire_kanade_stop(&mut world, rx)` を直接呼び、`QUIT_REGISTRATION` と `include_str!` の字面の確かめ（L44-L58）を消す。ファイルの名と置き場を中身（終わる相）に合わせて `session/ghost_quit_*_tests.rs` へ移すかは設計で決める（`session-module-gather` が後で `runtime_tests/frame*` のパスを直すので、その手間も減る）。
- 直さないと: 受け口の据え付けの抜けが緑のまま通り、写しと字面の見張りが残る。
- 規模: S

#### 10. `lib.rs` の同じ 2 つの字面を、範囲の中と外の 2 本のテストが確かめている〔範囲の外（消さない側）〕
- 場所: `crates/areka-runtime/src/frame_schedule_tests.rs:L88-L104` と、範囲の外の `crates/areka-runtime/src/zorder_wiring_tests.rs:L186-L198`（`t_zwi06`）
- 種類: 痩せ（merge）
- 問題（具体的な場合）: どちらも `squeeze(code_only(lib.rs))` に `add_systems(Update, frame_system.after(update_typewriters));` があることと `frame_system.before(apply_zorder_chain)` が無いことを確かめる。登録の書き方を変えると 2 か所を直すことになる。
- 直し方: 8 番で構造の側を本番の関数に替えたうえで、字面の照合は段の並びも見る `t_zwi06` に任せ、範囲の中の T-N10 ⑴ から消す。範囲の外には触らずに済む。
- 規模: S

#### 11. 隣り合いの不変量の表明が、字面の数どうしの引き算で決して落ちない
- 場所: `crates/areka-runtime/src/frame_chain_finalize_tests.rs:L76-L81`
- 種類: テストの抜け（落ちない表明）
- 問題（具体的な場合）: `assert_eq!(1483 - (1205 + 278), 0, …)` は窓を 1 つも読まない。確定が scope 1 の幅を 300 に書き換えても、x が 1205 のままなら緑。
- 直し方: `pos_of`／`size_of` で読んだ値から隙間を出す（`pos_of(char0).x - (pos_of(char1).x + size_of(char1).width)`）。
- 直さないと: 「隙間 0」の主張が実の窓を一度も見ない。
- 規模: S

#### 12. 「起点のスコープは置き直されている」の前提が `is_some()` で、何も確かめていない
- 場所: `crates/areka-runtime/src/frame_chain_finalize_tests.rs:L127`・`L137-L140`
- 種類: テストの抜け（空振りする檻）
- 問題（具体的な場合）: `moved_origin_x.is_some()` は窓に位置がある限り真。`resnap_from_sizes` が何もしなくなっても（scope 0 が 1483 のまま）緑。注の「観測の退化防止」と逆の働きになっている。
- 直し方: `assert_ne!(moved_origin_x, Some(1483))`（下端中央を保つなら 1450 との一致でもよい）。
- 規模: S

#### 13. 文字層が無いときの装着のテストが「登録しない」ことを確かめていない
- 場所: `crates/areka-runtime/src/frame_attach_tests.rs:L147-L175`
- 種類: テストの抜け（落ちない表明）
- 問題（具体的な場合）: 「register_actor_view 未呼出の担保」として `runtime.try_borrow_mut().is_ok()` を見るが、`connect_balloon_text` の借用は `Some` の腕の中で閉じる（`attach.rs:L485-L493`）ので、呼んだ後はいつでも成り立つ。`None` の腕が誤って `register_actor_view` を呼んでから false を返しても緑。`None` の腕が出す `warn!` も確かめていない。
- 直し方: `runtime.borrow().state().actors().next().is_none()` を主張し、`capture_logs` で WARN 1 行を数える。
- 規模: S

#### 14. 「切り替えで台帳を持ち越さない」テストが 2 つの別々の World で組まれ、持ち越しが起きても落ちない〔範囲の外（注 1 行）〕
- 場所: `crates/areka/src/runtime_tests/frame_visibility_integration_tests.rs:L540-L590`
- 種類: テストの抜け（落ちない表明）
- 問題（具体的な場合）: 台帳 `balloon_status` は `RuntimeWiring` の欄（`frame/wiring.rs:L124`・`L225`）なので、`RuntimeWiring::new` を呼べば作りからして空になる。台帳を World の資源へ移す直しが入ると、本番の切り替え（同じ World のまま）では持ち越しが起きるのに、このテストは新しい World を作るので緑のまま。L587 の「前のゴーストへは何も届かない」も、新しい World に前の送り口が無いので必ず真。しかも GPU の許可を 2 つ取り、`crates/areka-runtime/src/spine_wait.rs:L686-L687` に「2 つ取るのはこの 1 本だけ」という例外の注を作らせている。
- 直し方: 同じ World のまま前の結線を捨て、`seat_ghost` で置き場を据え直して `boot_wiring` を組み直す。許可は 1 つになり、例外の注（範囲の外・`spine_wait.rs`・持ち主の `audit-emo2-boot-spine` は完了済み）も消せる。同じ World で前の窓に装着し直せるかは、組むときに確かめる。
- 規模: M

#### 15. 箱の矢印とホイールのテストが「送れた」とは見るが、向きを見ない
- 場所: `crates/areka/src/runtime_tests/frame_shell_box_arrow_tests.rs:L134-L144`（`box_unmoved`）・`L236`・`L277`
- 種類: テストの抜け
- 問題（具体的な場合）: 上の矢印の押下が誤って新しい側へ 1 行送ると、箱は自動の位置に戻る。その後の `scroll_manual_at(Newer, 1)` は `Unchanged` を返す（`areka-emo-text/src/actor_scroll.rs:L62-L74`）ので `box_unmoved()` は偽になり、`assert!(!cage.box_unmoved())` は通る。ホイールの向きの取り違え（`vertical: 120` を新しい側へ）も同じく通る。
- 直し方: 送った後に新しい側へ 1 行送ると `Moved`、もう 1 行で `Stopped(ReachedAuto)` になる、と 2 段で見る。
- 規模: S

#### 16. 「話している最中の矢印の押下はいつもの箱の押下（中断）へ流れる」テストが、中断を確かめていない
- 場所: `crates/areka/src/runtime_tests/frame_shell_box_arrow_tests.rs:L251-L266`（`talking_box_arrow_press_flows_to_the_usual_box_press`）
- 種類: テストの抜け
- 問題（具体的な場合）: `press` の答えを捨て、受け手の `_lifecycle_rx`／`_kanade_rx` も一度も読まない。押下が何もせず飲み込まれても（`true` を返して中断を送らなくても）、箱は動かないので緑。
- 直し方: `press(up)` が真であることと、`UserBreakWiring` の受け手（lifecycle と kanade）へ中断が届いたことを表明する（同じファイルの L238 が空を見る形の逆）。
- 規模: S

#### 17. 再起動のテストの 2 回目の起動が、本番の「覚えた隣り合いの印を据える」をしていない
- 場所: `crates/areka/src/runtime_tests/frame_chain_finalize_persist_restart_tests.rs:L95-L122`（写しは `frame_chain_finalize_persist_nowrite_tests.rs:L52-L75` の `boot_remembered`・`frame_chain_finalize_remembered_tests.rs:L97-L112` の `boot_from`）
- 種類: テストの抜け（写しでもある）
- 問題（具体的な場合）: 本番（`crates/areka/src/ghost_session.rs:L310-L313`）は戻したスコープごとに `clear_default_char_pos(scope)` と `set_remembered_adjacency(scope, mark)` を呼ぶ。`second_run` は前の 1 つだけを呼び、`restored` の印を捨てる。印の無いスコープは並べ直しが `NoRememberedMark` で必ず動かさないので、テスト 13（`restart_after_body_drag_keeps_partner_at_previous_position`）は印がどうであっても緑。`adjacency_marks_with` が触れた次のスコープ（相方）の印を `Separate` に書き直さなくなる退行では、本番の 2 回目の起動は相方を動かした本体の隣へ 300 引き寄せる（要件 2.1 が破れる）のに、このテストは緑のまま。
- 直し方: 「窓を作る→戻した集合に既定の位置の消去と印を据える→置き直す」を支えのファイルに 1 つ置き、3 か所で使う。`ghost_session.rs` は C23 の `input-crate-split` が触るファイルだが、読むだけで触らない。
- 規模: S

#### 18. seriko の終わりを自前の 5 秒の空回し（`yield_now`）で待っている
- 場所: `crates/areka/src/runtime_tests/frame/switch_tests.rs:L678-L686`（`seriko_send_failure_empties_the_slot`）
- 種類: 負荷（決定論の決まり）／再利用
- 問題（具体的な場合）: 待ちの上限が壁時計の 5 秒しかなく、待つ側も CPU を 1 つ回し続けて seriko のスレッドと取り合う。`-j 4` で GPU のテストと取り合うと、誤りが無くても 5 秒を越えて赤になりうる。用の部品 `areka_runtime::spine::wait_until`（上限 30 秒・段々に休む）がすでにある。
- 直し方: `wait_until("seriko の閉鎖", Progress::Unknown, || rig.seriko.is_finished()).expect(..)` の 1 行にする。
- 規模: S

#### 19. 停止の知らせを 5 ms の sleep と壁時計の 30 秒で待っている
- 場所: `crates/areka/src/session/ghost_quit_logsink_tests.rs:L22-L23`・`L77-L85`
- 種類: 負荷（壁時計の sleep で待つ）／再利用
- 問題（具体的な場合）: 待ちの部品を自前で写している。kanade が固まると、進みの見えない「30 秒以内」の文言だけを残して落ちる。同じ crate の `ghost_quit_switch_tests.rs` は `wait_recv`／`wait_until` を使っている。
- 直し方: `wait_until("停止通知が終了相へ届く", Progress::Unknown, || run_ghost_quit_phase(&mut world))` にし、`BOUND` を消す。
- 規模: S

#### 20. 記憶の書き手のアクターの panic を `let _ =` で捨て、アクターの join に上限が無い
- 場所: `crates/areka/src/runtime_tests/frame/switch_finish_tests.rs:L296-L302`（`memory`）、`frame/switch_tests.rs:L274`（`seriko.join().unwrap()`）
- 種類: テストの抜け（記録の無い失敗の道）
- 問題（具体的な場合）: `reply_failure_records_one_error_and_writes_nothing` ほかは記憶が `old_memory()` のままであることを期待する。書き手のアクターが書く途中で panic しても `join` の `Err` は捨てられ、記憶は古いままなのでテストは緑。どちらの join も上限が無く、アクターが終わらないとテストが止まったままになる。
- 直し方: `areka_runtime::spine::join_bounded("sylphya", parts.handle).expect("記憶の書き手が panic した")`。seriko の join も `join_bounded` にする。
- 規模: S

#### 遅い

#### 21. 箱の檻が 1 フレーム回すたびに、置き場を埋める空の結線のために emo2 の資産を丸ごと組み直す
- 場所: `crates/areka/src/runtime_tests/frame_shell_box_integration_tests.rs:L135-L140`・`L206-L209`、`frame_shell_box_arrow_tests.rs:L156-L165`
- 種類: 遅い
- 問題（具体的な場合）: `Cage::frame`／`with_wiring` は、結線を `advance_frame` へ渡す間だけ `self.wiring` を空の結線で埋める。その空の結線が `boot_wiring` を呼ぶので、毎回 `build_boot_assets` が emo2 のシェルとバルーンの画像を WIC で全部読み直す（`assets.rs:L251-L260`）。1 本のテストで 3〜15 回、箱の 14 本で合わせて約 100 回。組み直しは GPU の許可を持ったまま走るので、`-j 4` で許可を待つほかの GPU のテストまで遅れる。
- 直し方: 空の結線を安い形にする（`shared_test_support::wiring().1` の合成の資産）。または `frame_scale_notice_integration_tests.rs:L30-L33` と同じく `Option<RuntimeWiring>` と `take()` にする。
- 規模: S

#### 写し・重なり・置き場（痩せ）

#### 22. `capture_logs` の写しが同じ crate の中に 3 つ残っている〔件 11〕
- 場所: 元は `crates/areka-runtime/src/frame_test_support.rs:L116-L121`。写しは `frame_chain_finalize_tests.rs:L287-L316`（`L27` の `use`・`L293-L300` の注も同じ文）・`frame/zorder_descript_tests.rs:L48-L51`・`frame/zorder_drain_test_support.rs:L147-L154`
- 種類: 痩せ（再利用）
- 問題（具体的な場合）: `frame_chain_finalize_tests.rs` の私有の `capture_logs` は、元と diff を取ると `#[cfg(test)]` と `pub` の 2 点だけが違い、doc の行・本体・戻り値の取り出しは同じ。L293-L300 の 8 行の注も元の L107-L114 と一字一句同じ。`lines_of_level(..).len()` は `count_level` と同じ数え方。捕捉の形（`LineFormat`）を元の側で変えると、写しの側の檻だけが古い形の行を見る。
- 直し方: 3 か所とも元を `use` して写しを消す。`lines_of_level` は `digest` に行が要る 2 か所のためだけに残す。本体の写し（`runtime_tests/spine.rs:L124` ほか）は元が `#[cfg(test)]` だけで本体から読めないので理由があり、範囲の外。
- 規模: S

#### 23. `code_only` の写し〔件 11〕
- 場所: `crates/areka-runtime/src/frame/zorder_drain_tests.rs:L460-L470`
- 種類: 痩せ（再利用）
- 問題（具体的な場合）: `source_text_test_support::code_only`（`source_text_test_support.rs:L4-L10`）と 1 字ずつ比べると、違うのは引数の名・可視性・改行の位置だけで振る舞いは同じ。走査の決まり（ブロックコメントの扱いなど）を共通の側だけで変えると、この檻だけが古い決まりで走る。
- 直し方: 私有の関数と L460-L464 の doc を消し、`use crate::source_text_test_support::code_only;` にする（`zorder_cue_tests.rs` ほか 6 か所と同じ形）。
- 規模: S

#### 24. 書き換えられる時計の写し〔件 11〕
- 場所: `crates/areka-runtime/src/frame_drain_text_tests.rs:L14-L27`（関数は L16 と L25）
- 種類: 痩せ（再利用）
- 問題（具体的な場合）: `talk_clock.rs:L136-L150` に `#[cfg(test)] pub(crate)` で同じ名・同じ中身の関数がある（閉包を変数に置くかの違いだけ）。`talk_lifecycle_signals_tests.rs:L16` はそちらを使っている。時計の注入の形が変わると 2 か所を直すことになる。
- 直し方: L14-L27 を消し、`use crate::talk_clock::{controllable_clock, set_now};` を足す。
- 規模: S

#### 25. `zorder_descript_tests.rs` が `zorder_drain_test_support.rs` の道具を 1 組まるごと写して持っている
- 場所: `crates/areka-runtime/src/frame/zorder_descript_tests.rs:L38-L59`（`GROUP_TARGET`・`lines_with`）・`L84-L128`（`placement`・`world_with_scopes`・`window_of`）。元は `frame/zorder_drain_test_support.rs`（`GROUP_TARGET` は L171）
- 種類: 痩せ（再利用・merge）
- 問題（具体的な場合）: 元の L4-L5 は「二重に持つと、片方だけを直したときに檻の前提が静かにずれる」と書く。`placement` の `balloon_offset_base` や `GROUP_TARGET` の出力先を片方だけ直すと、もう片方の檻は古い前提のまま緑で残る。写しができたのは `zorder_drain.rs:L403-L405` の `mod test_support` が私有で、`zorder_descript` から届かないため。
- 直し方: `zorder_drain.rs` の宣言を `pub(super) mod test_support;` にし、`zorder_descript_tests.rs` は `use super::super::zorder_drain::test_support::*;` で引いて写しを消す。ファイルの名とパスは変えない（`placement_tests/spawn_zorder_pair_deferred_tests.rs:L114` が `src/frame/zorder_drain_test_support.rs` のパスを字面で読み、本体の `placement_tests/zorder_group_branch_coverage_tests.rs` の `COVERAGE` がテストの関数の名を文字列で持つ）。
- 規模: S

#### 26. `RuntimeWiring::new` の手組みが範囲の中に 16 か所ある〔約束 b〕
- 場所:
  - `crates/areka-runtime/src/frame_attach_tests.rs`: L184-L195・L225-L236・L270-L281・L333-L342・L410-L421・L464-L475・L505-L516・L569-L580・L699-L710・L888-L897（10 か所。うち `TalkClock::new(Arc::new(|| 0.0))` の 4 か所は `zero_clock()` の写し）
  - `frame_drain_text_tests.rs:L228-L239`・`L277-L288`・`L319-L330`・`L364-L375`
  - `frame_resnap_tests.rs:L365-L377`・`frame_move_hold_tests.rs:L35-L50`（`wiring_with`）・`frame_window_link_drain_tests.rs:L35-L50`（`sink_and_wiring`）
  - 似た組み立てが支えに 2 つ: `frame_test_support.rs:L142-L155`（`headless_wiring_with`）・`frame_shared_test_support.rs:L22-L37`（`wiring`）
- 種類: 痩せ（再利用）
- 問題（具体的な場合）: 受け口を 1 本足す変更（`zorder_rx` が増えたときがそう）で、テストの中の 16 か所と支えの 2 つを直すことになる。`frame_attach_tests.rs` は 947 行で、これを縮めないと次の 1 本で 1,000 行を越える。
- 直し方: 支え（`frame_test_support.rs` は 955 行なので `frame_shared_test_support.rs` 側）に、受け口を既定で埋めて assets・文字層・表示層だけを取る補助を 1 つ置き、`wiring()`・`headless_wiring_with`・16 か所をそこへ寄せる（attach は約 100 行縮む）。
- 〔約束 b〕: 支えは本体のテストも使う公開の面。足すだけなら壊さない。
- 規模: M

#### 27. 共有の支えの定数と補助が、隣のテストに写しとして残っている
- 場所: 元は `crates/areka-runtime/src/frame_shared_test_support.rs:L40-L155`。写しは `frame_chain_realign_tests.rs:L39-L52`・`L75`・`L93-L100`・`L137-L183`・`L189-L201`、`frame_chain_realign_arm_tests.rs:L39-L57`、`frame_default_pos_track_tests.rs:L102-L136`、`frame_chain_realign_adjacency_tests.rs:L42-L49`、`frame_move_hold_tests.rs:L53-L58` と `frame_resnap_tests.rs:L356-L358`（`steady_harness`）
- 種類: 痩せ（写し）
- 問題（具体的な場合）: `realign_tests` は shared の `HIGH_DPI`〜`GAP_WITHOUT_REALIGN`・`low_sizes`・`lower_the_scale`・`gap` を同じ値のまま私有で持つ。`raise_the_scale`・`high_sizes` は realign と adjacency の 2 か所、`char_x`・`default_x`・`drag_char_window_by` は realign と default_pos_track の 2 か所。arm_tests の私有の `settled_sizes`（L55）は `test_support::settled_sizes` と名が同じなのに値が違う。実機の幅 764 を測り直して shared の側だけ直すと、`realign_tests` は古い 359 のまま緑で残る。shared の頭の注は「写しを作らない」と言い、使い手の一覧（2 本）も古い（実際は exclusion と本体の remembered も使う）。
- 直し方: 写しを消して shared から引く。`high_sizes`・`raise_the_scale`・`default_x`・`drag_char_window_by` は shared へ移す。arm_tests の寸は shared の名に寄せ、私有の `settled_sizes` は別の名にする。`boot_at_high` は既定の位置を外さない点が違うので残す。shared の頭の使い手の説明を直す。
- 規模: M

#### 28. 3 体の鎖の支えがテストのファイルの中に `pub(super)` で置かれ、別のテストのファイルから引かれている
- 場所: `crates/areka-runtime/src/frame_chain_realign_adjacency_tests.rs:L51-L61`・`L266-L369`（使う側は `frame_chain_realign_exclusion_tests.rs:L33-L35`）
- 種類: 痩せ（置き場）
- 問題（具体的な場合）: 決まり（テーマ間で共有する補助は `<stem>_test_support.rs` へ）に反する。adjacency のファイルを x64 の門の外へ動かしたり分けたりすると、exclusion のテストがビルドできなくなる。
- 直し方: `place_char_window_x`・`chain3_world`・`halve_keeping_bottom_center`・`chain3_scan`・`chain3_pos` を `frame_shared_test_support.rs` へ移す。
- 規模: S

#### 29. 書き込みの指令を絞る補助・整地の手順・拡大率の定数が、9 つのファイルで何重にも写されている
- 場所:
  - `writes_for`: `frame_balloon_offset_{converge,follow,keyword_gate,roundtrip}_tests.rs`・`frame_transition_atomicity_tests.rs:L139`・`frame_transition_branch_tests.rs:L126`・`frame_work_area_resnap_hold_tests.rs:L72`・`frame_work_area_resnap_tests.rs:L55`・`frame_dpi_sync_hold_tests.rs:L60`（9 か所）
  - `offset_of`（4）・`follow_of`（3）・`offset_lines`（3）・`raise_scale_without_moving_the_work_area`・`attach_material`・`KEYWORD_MODE/ADJUST`（2 ずつ）・`transition_to`・`BASE_OFFSETS`（2）: `frame_balloon_offset_converge_tests.rs:L51-L109`・`follow_tests.rs:L137-L266`・`L415-L447`・`keyword_gate_tests.rs:L54-L188`・`roundtrip_tests.rs:L94-L97`・`L209-L279`
  - `scaled`（atomicity L118・branch L112）・`warnings_containing`（branch L139・hold L85）
  - 整地: atomicity `boot_at_base` L215・branch `settle_at` L204・hold `settle` L107・resnap `settle` L68・sync `settle_at_boot_dpi` L54・offset の 4 本の `settle`／`settle_at`
  - バルーンの等倍寸 `(223, 158)`: atomicity L273-L275・branch L95（元は `frame_test_support.rs:L189`・`L202`）
  - `BASE_DPI`・`SCALE_120`・`SCALE_192`: atomicity L96-L102・branch L80-L86・hold L60-L66・offset の 4 本の `LOW_DPI/HIGH_DPI`
- 種類: 痩せ（再利用・merge）／テストの抜け
- 問題（具体的な場合）:
  - converge の L173-L177 は `writes_for(&writes, scope_u32, "char")` が空であることを主張する。種別を文字列で渡すので、`"chr"` と打ち間違えると 0 件の主張が空振りで緑になる（branch L126 は `WindowKind` で受けていて、この穴が無い）。
  - converge と keyword の `settle` は `base.dpi.is_some()` しか見ず、follow と roundtrip は基準の対の全値を見る。同じ「整地」の自己検査の強さが揃っていない。
  - `resnap_placements` のバルーン寸を変えると、atomicity と branch が別々の古い寸で報告を組む。
  - follow は 947 行で、次の 1 本で番人を越える。35 番の分割の妨げにもなる。
- 直し方: 兄弟の支え（例 `frame_transition_test_support.rs`・`frame_test_support.rs` は 955 行なので足さない）に `WindowKind` 受けの `writes_for`・強い方の `settle_at(dpi)`・`scaled`・`warnings_containing`・`offset_of`/`follow_of`/`offset_lines`・`pub const BALLOON_SPAWN_SIZE`・拡大率の定数を 1 つずつ置き、9 本から消す。`resnap_placements` もその定数を使う。
- 規模: M

#### 30. 偽の HWND の付け方が、配置の支えに既にある補助の写し
- 場所: `crates/areka-runtime/src/frame_test_support.rs:L157-L162`・`L244-L255`、本体 `crates/areka/src/runtime_tests/frame_chain_finalize_persist_test_support.rs:L227-L239`、`frame_chain_realign_adjacency_tests.rs:L307`・`frame_window_link_drain_tests.rs:L170`
- 種類: 痩せ（再利用）
- 問題（具体的な場合）: `areka_placement::test_support::{fake_handle, give_fake_handles}`（pub・dev-dependency の test-support から届く）が同じ仕事をする。同じ輪が範囲の中に 5 か所、範囲の外（`move_cue_apply_move_tests.rs:L53`・`move_cue_place_tests.rs:L85`）にも 2 か所ある。生の値の刻み（0x10 と 1）は違うが、生の値を読むテストは無い（grep で確かめた）。説明の「follow.rs の fake_handle 相当」も今は `test_support.rs` にあるので古い。
- 直し方: 範囲の中の 5 か所を `give_fake_handles(world, &gw);` の 1 行にし、私有の `fake_handle` を消す（約 30 行減る）。範囲の外の 2 か所は `audit-emo2-boot-switch` の範囲（`move_cue*`）なので申し送る。
- 規模: S

#### 31. GPU の World を組む手順が 4 か所に写され、COM の初期化の失敗を捨てている〔約束 b〕
- 場所: `crates/areka-runtime/src/frame_test_support.rs:L941-L955`（`gpu_attach_world`）・`frame_balloon_runtime_test_support.rs:L30-L43`、本体 `runtime_tests/frame_visibility_integration_tests.rs:L49-L70`（`gpu_frame_world`）・`L135-L140`（`bump_window_dpi`＝範囲の外の `spine_test_support.rs:L77` と同じ判断）
- 種類: 痩せ（再利用）／記録の無い失敗の道
- 問題（具体的な場合）: MTA の COM 初期化→`GpuPermit::take`→`GraphicsCore::new`→d2d→`WucGraphicsResource::new` の並びと、「許可は装置より先に取り組の先頭で返す」決まり（`spine_wait.rs:L683`）が 4 か所に写っている。`let _ = CoInitializeEx(...)` は S_FALSE・RPC_E_CHANGED_MODE だけでなく E_OUTOFMEMORY などの本当の失敗も捨て、原因の HRESULT を残さずに後の「GraphicsCore::new 失敗」だけが残る（`areka_placement::test_support::with_com_initialized` はこれを分けている）。許可の取り方の決まりが変わると片方が取り残され、`-j 4` の GPU の取り合いで揺れる。
- 直し方: `areka-runtime` の支えに「DPI つきの World へ GPU を載せて許可を返す」補助を 1 つ置き、4 か所から呼ぶ。COM は `hr.is_ok() || hr == RPC_E_CHANGED_MODE` を assert する（プロセスの寿命の MTA があるなら行ごと要らないかを確かめる）。`gpu_attach_world` の署名は本体の `frame_attach_carry_tests` も使うので変えない。
- 規模: S

#### 32. 最小のゴーストの一時フォルダが `temp-path-kit` の写しで、消せなかったことを黙って捨てる
- 場所: `crates/areka/src/runtime_tests/frame_chain_finalize_persist_test_support.rs:L246-L293`（`let _ =` は L266 と L291）
- 種類: 痩せ（再利用）／危ない所（記録の無い失敗の道）
- 問題（具体的な場合）: 名の組み立て（pid＋連番）と破棄で消すことは `temp-path-kit::TempPath` の写し（本体は dev-dependency で既に引いている・kit は「ワークスペースで 1 か所だけ定義する」と言う）。置き場も違う（写しは `target\test-tmp\`、kit は `target\test-roots\`）。破棄のときに索引付けやウイルス対策が `sylphya.toml` を握っていると消せず、フォルダは毎回別の名で黙って溜まる（kit は標準エラーへ 1 行書く）。
- 直し方: `MinimalGhost(TempPath)` にして `TempPath::new(&format!("cpsoe-{label}"))` で作る。`NEXT_GHOST`・L256-L266・`Drop` を消す。
- 規模: S

#### 33. 本体の項目を 1 つも使わないテストが本体に居る（鎖の規則 13 本・起動の揃え 6 本ほか）〔約束 b〕
- 場所: `crates/areka/src/runtime_tests/frame_chain_rule_tests.rs:L1-L589`（13 本）・`frame_chain_finalize_align_tests.rs:L1-L230`（6 本全部）・`frame_chain_finalize_clamp_tests.rs:L85-L132`（⑴）・`frame_chain_finalize_remembered_tests.rs:L217-L261`（⑶⑷⑸）。支えは `frame_chain_finalize_persist_test_support.rs:L213-L243`（`spawn_shown`）・`frame_chain_finalize_align_tests.rs:L34-L87`（`spawn_resolved`）
- 種類: 痩せ（在るべきでない所に在る）
- 問題（具体的な場合）: これらが使うのは `areka_placement` と `areka_runtime`（`finalize_chain_once_with`・`realign_chain_once_with_source`・`resnap_with`・`resolve_placement`）と、本体に置いた支えだけで、支えも本体の項目を使わない。本体に触れる 2 本（`frame_chain_rule_tests.rs:L76`・`L496`）も `PersistStore::wire` の中の `crate::insert_persist_wiring`（`main.rs:L816-L818`＝`world.insert_non_send(PersistWiring { publisher })` の 1 行の包み）だけ。`runtime_tests/mod.rs:L1-L5` の決まり（本体の部品も使うテストだけをここに置く）に反し、鎖の確定と詰め直しを持つ `areka-runtime` を直して `cargo test -p areka-runtime` が緑でも、ここで初めて赤になる。
- 直し方: `spawn_shown`・`spawn_resolved`・`sizes`・`x_of` と `PersistStore` の偽の保存先の半分を `areka-runtime` の支え（`frame_shared_test_support.rs` か新しい兄弟のファイル）へ移し、上のテストを `crates/areka-runtime/src/frame_chain_*_tests.rs` へ戻す。2 本は `PersistWiring` を直に置けば足りる。`areka-runtime` は `log-capture-kit`・`areka-sylphya`・`areka-placement/test-support` を既に dev-dependency で引いているので、`Cargo.toml` は変えずに済む見込み（組んで確かめる）。本体に残るのは本体の項目を使う persist の 3 ファイル・restart・remembered ⑵。
- 〔約束 b〕: 支えの公開の面が増える（読み手は本体の `chain_finalize_*` のテスト）。
- 規模: M

#### 34. 整合待ちのテスト 4 本が、兄弟のファイルの 192 の版に丸ごと含まれる
- 場所: `crates/areka-runtime/src/frame_dpi_sync_hold_tests.rs` の L150-L219・L224-L248・L252-L285・L349-L383。含む側は `frame_transition_branch_tests.rs` の L252-L336・L344-L426・L435-L494・L573-L638（`…_at_192`）
- 種類: 痩せ（削除・merge）
- 問題（具体的な場合）: 整地の値（96）・遷移先（192）・駆動の順はどれも同じで、主張は向こうがすべて上回る。こちらの `the_wait_gives_up_after_the_bounded_number_of_frames_and_proceeds` は doc（L250）で「警告の上で」と言うのに警告を確かめず、警告を消す誤りは向こうの `…_at_192` だけが捕まえる。整合待ちの判断を変えると 2 ファイルの同じ場面を直すことになる。
- 直し方: こちらの 4 本を消し、向こうに無い 4 本を残す。`frame_transition_branch_tests.rs:L10` の「対テストの持ち主」の表と、こちらの L12-L14 の見出しを直す。
- 規模: M

#### 35. 判断の分岐のテストが 1,271 行で、1,000 行の番人の上限越えの表に載っている〔範囲の外（表）〕
- 場所: `crates/areka-runtime/src/frame_transition_branch_tests.rs`（全体）。表は `crates/log-capture-kit/tests/file_length_guard_test.rs:L66-L82`（L68・理由は「分割は本仕様の範囲外」・`OVER_LIMIT_ALLOWED_COUNT = 3`）
- 種類: 痩せ（split）
- 問題（具体的な場合）: 上限越えで、例外の表がある限りこのファイルは今後も上限を越えて伸びられる。中身は群 A（待ち・L243-L547）と、群 B〜D・本番の入口・観測の条件（L549-L1271）にきれいに分かれる。
- 直し方: 群 A と残りの 2 ファイルに分け、`frame.rs:L446-L450` の `#[path]` を 2 本にする。共通の補助（L107-L241）は 29 番の支えへ寄せる。表から 1 行消し、件数を 3 から 2 にする。34 番で 4 本を消す分と合わせて行の数を確かめながら進める。
- 〔範囲の外〕: 表は `log-capture-kit` のテスト。C23 のあいだ表に触ってよいのはこの spec だけ（約束 ⑸・要件 7.4）で、この範囲のファイルが表から出る直しなので約束の中。
- 規模: M

### 2.3 できれば（36〜114）

- 113・114 番は本番のコードの指摘なので、最初の表（本番）に置いた。

#### 本番のコードの説明と小さな形（`areka-runtime` の `frame.rs`・`frame/`）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 36 | 相の並びの説明が実際と合わない。L207-L208 の一覧に 6 つの相（作業領域の同期・装置の作り直し・作業領域の変化の再スナップ・window_state・move_shift・scale_notice）が無い。L31・L351 は status-report を「並びの最後」と書くが最後は L357 の scale_notice で、L355 と矛盾。L37-L40 の「本ファイルは plan_attachments…を実装」は今は `attach.rs` ほか。テストの側にも「remove→3 フェーズ→insert」（今は 17〜20 の相） | `frame.rs:L1-L40`・`L207-L208`・`L31`・`L351-L352`、`frame_drain_text_tests.rs:L166-L171`・`L179` | 3 節の件 13 の並びに合わせて書き直す。テストの側は「各相」に | — |
| 37 | `wiring.rs` の説明が古い。L319 の `super::super::input_events::balloon` は無い。L325-L331 の「本番の呼び手は 3 箇所」は 6 か所（`balloon_exit`・`balloon_moved`・`balloon_pressed`・`balloon_wheel`・`char_watch`・`shell_box_handler`）。L290 の `super::hit_region` は `crate::hit_region`。L296-L299 の「dead_code 判定上は live」は要らない。L298 の `boot_ghost` の `wire_mouse_input` は今は `ghost_session_runtime.rs:L36`。L11-L17・L348-L354 は `#[cfg(test)]` と書くが `cfg(any(test, feature = "test-support"))`。L360-L367 の「兄弟モジュール `super::spine`」「以下 3 つ」は古い（`crate::spine`・5 つ）。L39・L42-L48・L50-L58 の「attach フェーズ」「三相」「5 つを `pub(crate)`・公開面は 1 つも増えない」は事実と違う（11 欄が `pub`） | `frame/wiring.rs` | 事実に合わせて文を直す。L319 はモジュールのパスを書かずに「本体の入力の層（バルーンの入力）」と名で指す（要件 5.1） | 〔件 1〕〔約束 b〕（文だけ） |
| 38 | 他ファイルを指す行番号がずれている（`window_move.rs:337-345` は今は手順 5。べき等の skip は L322-L331） | `frame/balloon_offset_follow.rs:L277` | 行番号をやめ「`resize_window_to` の手順 4」と名で指す | — |
| 39 | `attach.rs` の doc が、無い表明（「呼び手は `items.len()` を積極 assert」・実際は `info!` だけで、表明は本体の `runtime_tests/spine.rs:L494`）と、動かした例（`examples/emo-present.rs` は今は `crates/areka/examples/emo-present/systems.rs:L44`）を指す | `frame/attach.rs:L34-L35`・`L96-L97`・`L156-L157`・`L160`、`frame.rs:L211` | 表明の持ち主を spine と書き、例の場所を直す | 〔約束 c〕（`runtime attach: 装着計画を実行` は `crates/areka/tests/emo2_real_run/common.rs:L18` が字面で読む＝行は変えない） |
| 40 | 面の move のずれの注が、ドラッグ中・組の相手・持ち主のバルーンがドラッグ中の見送り（`debug!` の `move_shift_deferred`・`areka-placement/src/follow/move_shift.rs:L138-L158`）を書いていない | `frame/move_shift.rs:L75` | 「材料が無ければ warn!、ドラッグ中の見送りは debug!。どちらも記録を変えない」 | 〔件 8〕 |
| 41 | 略称 `atom`（＝`areka-P0-dpi-transition-atomicity`）と、spec の名の無い「task n」「tasks.md n」が残る。`task 6.1`・`task 6.5` は `areka-P0-balloon-offset-dpi` にも同じ番号があり読み分けられない | `atom`: `frame.rs:L243`・`L263`・`L337`・`frame/dpi.rs:L458`・`frame/drain_resnap.rs:L562`・`L607`・`L672`・`frame_balloon_offset_follow_tests.rs:L609`・`frame_transition_atomicity_tests.rs:L15`・`L418`・`L465`（11 か所・本体の側は 0）。task: `frame_diag_route_tests.rs:L11`・`L87`・`L123`・`L126`・`L134`・`L152`・`L268`（`areka-P0-dpi-window-vanish`）・`frame_chain_realign_tests.rs:L1`・`frame_chain_realign_arm_tests.rs:L1`・`frame_default_pos_track_tests.rs:L1`・`frame_harness_tests.rs:L446`・`frame_chain_finalize_tests.rs:L30`・`frame_text_scale_tests.rs:L9`・`L249`・`frame_work_area_resnap_hold_tests.rs:L1`・`L425`・`frame_transition_branch_tests.rs:L545` | `atom` を正式の名へ、task 番号の前に spec の名を書く | 〔件 14〕 |
| 42 | 誤字「櫯」（「檻」） | `frame.rs:L525` | 1 字直す | — |
| 43 | 「唯一の位置ライター」（L554・`move_window_to`）と「単一ライター」（L281・`resize_window_to`）が別々の関数を指し、互いに矛盾し、どちらも事実と違う（単一の書き口は `enqueue_window_set_pos`・`window_move.rs:L631`。`move_window_to` の doc 自身が「位置を書く道はほかにもある」と書く） | `frame/drain_resnap.rs:L554`・`L281`、`frame_chain_finalize_tests.rs:L204` | L554 は「反映は `move_window_to`（`MoveCue` を名乗る・バルーン随伴の offset 維持を内包）」、L281 は「`resize_window_to` が T の再適用と随伴を 1 呼出で書く」、テストは「台本の移動と同じ口（`move_window_to`）で動かす」 | 〔件 6〕 |
| 44 | `persist_unremembered_char_positions` の本番の呼び手を「ここ 1 か所だけ」と書くが、並べ直し（`areka-placement/src/reset_pos.rs:L135`）も呼ぶ | `frame/drain_resnap.rs:L595-L598` | 「確定の側の呼び手はここ。並べ直し（`reset_pos.rs`）も呼ぶ」 | 〔件 14〕 |
| 45 | 確定の見送りの `ChainDeferReason::DpiSyncHeld` の腕は届かない（作るのはツリー全体で `areka-placement/src/chain_realign.rs:L246`＝解き直し専用だけ。`collect_chain_states` は返さない）。腕を消すと網羅の `match` が止まるので腕は要る | `frame/drain_resnap.rs:L479` | 腕に 1 行「`collect_chain_states` は返さない（解き直し専用・網羅のため置く）」 | 〔件 14〕 |
| 46 | 記録の出口を wintf に揃える理由「grep 対象が 2 本に割れる」が事実と違う。本当の理由は、`areka-runtime` の中でマクロを呼ぶと出力先が `areka_runtime::frame::zorder_drain` になり、サインオフの `RUST_LOG`（`info,wintf::ecs::window=debug`・`signoff-procedure.md:L439` ほか）の前方一致で拾われず、debug の `[zorder-group] applied` の行が消えること（wintf の 2 つの出力先 `zorder_chain` と `zorder_chain_diag` は前方一致で漏れない）。要件 5.3 が書く直しの文「読み手の絞り込みは前方一致なので漏れはせず」をそのまま入れると、`areka-runtime` の側では嘘になる | `frame/zorder_drain.rs:L47-L53`・`frame/zorder_descript.rs:L30-L34`・`frame/zorder_drain_tests.rs:L443` | 3 か所を「こちらでマクロを呼ぶと出力先が `areka_runtime::…` になり、`wintf::ecs::window` の前方一致の絞り込みでは debug の受理の行が点かない。行の字面と出力先を wintf の 1 か所に保つ」という形に。要件 5.3 の文も合わせて改める（議題 5） | 〔件 3〕〔約束 c〕（文だけ・読み手はサインオフの手順）〔裁定 ⑴〕（要件の文言を改める） |
| 47 | 再スナップの説明が、今は使っていない `text_slot_view`・`surface_size`（原寸）を名指す。本体の L371-L380 は「native で駆動すると綱引き」と禁じており、説明どおりに直すと禁じた形へ戻る。テストの側にも同じ古い文 | `frame/drain_resnap.rs:L287-L300`、`frame_resnap_tests.rs:L214` | 「`PhysicalSizeSource`（本番は `target_physical_size`＝k 倍後の物理寸）を `shell_target` で引く」 | — |
| 48 | 待ち札の例外を「随伴バルーンの追従 1 つだけ」と書くが、今の見張り（`window_move.rs:L499-L551`）はドラッグ・`MoveCue`・`Restore`・`BalloonLimitRelease`・`BalloonFollow`・`SpawnInitial` を見送らない | `frame/work_area_sync.rs:L188-L191` | 「例外は明示の操作と随伴（`deferral_covers_route` が正本）」 | — |
| 49 | 作業領域の源の同期が、変化の無いフレームでも毎フレーム 2 つの源を作り直してから比べて捨てる（QueryState の作り直し・`Vec<Monitor>`・`MonitorSources` の Vec 2 本・`MonitorSnapshot` と `MonitorDpiTable` の clone）。`frame.rs:L246` の「定常フレームは無操作」も事実と違う。変わる時点は wintf の `monitor_systems` が知っている | `frame/work_area_sync.rs:L68-L111`、`frame.rs:L246` | ⑴ `Monitor` の Added／Changed／RemovedComponents を持続の SystemState で見て、無いフレームは早く戻る（範囲の中で済む）。⑵ clone をやめて借りたまま比べる（`same_monitors` の署名が `areka-placement` 側＝〔C23〕）。`frame.rs:L246` の文を事実へ | 〔C23〕（⑵ だけ）・M |
| 50 | 拡大率の相が、持続の SystemState を使いながら同じ関数で待ち札の窓を毎フレーム新しい query で集め、手で和集合を作る | `frame/dpi.rs:L362-L372`（型は L308-L317） | `DpiChangedQuery` のフィルタを `Or<(Changed<DPI>, With<DpiSyncHold>)>` にして 11 行を消す（6 番のテストを先に立てる） | — |
| 51 | `size_changed` は、書く前の寸の読み直しと届かない腕（i32 への変換の失敗）のためだけに在り、その腕を試すためだけに `pub(super)` と単体のテスト（`frame_chain_realign_arm_tests.rs:L307-L347`）がある | `frame/dpi.rs:L252-L272`・`L448-L450`・`L462` | L450 の `before` と書いた後の `WindowPos.size` を `before.is_some() && before != after` で比べ、`size_changed` と単体のテストを消す | — |
| 52 | 重なりの相が、グループ 0 本の既定の状態でも毎フレーム在庫の Vec を作る（`compose_chain` は空なら最初に `None` を返す） | `frame/zorder_drain.rs:L192-L203` | `compose_plan` の頭に `if ledger.groups().is_empty() { return None; }` | — |
| 53 | 差し替えの後始末の説明が「この順」と書きながら、カーソルの受け渡し（L541）・窓寸の報告の先の反映（L544）・動かせない指定の取り替え（L567-L570）・揃え方の置き直し（L573）と、バルーンの段の作者の DPI の入れ替え（L627-L638）を欠く | `frame/switch.rs:L9-L13`・`L521-L523` | 本体の順に合わせるか「順は本体のとおり」へ短く | — |
| 113 | 表示の指令の取り出しが、ちょうど 1,024 件のフレームでも（残りが無いのに）「残りは次のフレームで適用する」と warn を出す。境目を固定するテストも無い（`drain_resnap_tests.rs:L203` は MAX+5 だけ） | `frame/drain_resnap.rs:L103-L109` | `take(MAX + 1)` で 1 件多く取り、越えたときだけ warn して余りを次へ回す形か、文を「上限に達した（残りがあれば次のフレームで）」に | 〔約束 c〕（warn の条件だけ・字面で読む所は 0 件＝grep） |
| 114 | 開けた公開と `pub(crate)` のうち、狭められるもの。`StageObservation` は `areka-runtime` の外に使い手が無く私有に戻せる。`collect_bindings`（`status_report.rs:L72`）・`Transition`・`decide_transition`（`window_state.rs:L44`・`L56`）は同じファイルと子のテストだけ＝私有で届く。`reveal_pending`（`scale_text.rs:L273`）・`FaceCause`・`select_balloon_face`（`balloon_face.rs:L22`・`L34`）・`drain_move_directives`（`wiring.rs:L344`）は frame の兄弟だけ＝`pub(super)` で届く | `frame/status_report.rs:L52`ほか | 可視性を狭める。使われない `pub(crate)` は 0 件なので消すものは無い | 〔件 5〕〔約束 b〕（外の読み手 0） |

#### `areka-runtime` の `frame/` の下のテスト

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 54 | 「この相はマクロを持たない」の番人が `zorder_descript.rs` を見ない（同じ決まりを名乗る `apply_descript_rebase` に `debug!` を足しても t_zdb* のどれも落ちない） | `frame/zorder_drain_tests.rs:L427-L458` | t_zdr09 の `here` を `zorder_drain.rs` と `zorder_descript.rs` の 2 本にする | 〔約束 c〕 |
| 55 | 取り出しの檻 ① は、元の誤り（適用しながら取り出す形）へ戻ったとき、目印が増え続けるので赤になるまで 300 秒かかる（`spine_wait.rs:L247-L271`） | `frame/drain_resnap_tests.rs:L102-L124` | 適用の閉包の中に `assert!(applied <= MAX_DRAIN_PER_FRAME)`。元の形なら約 10 秒で `［相手が居ない］` | 〔裁定 ⑴〕（目印の決め方は完了した `areka-test-binary-stability` 6.5 の決め。assert を足すだけなら触れない） |
| 56 | 「`load-repro.md` の ⑹」に持ち主の spec が無い（⑹ を持ちこのテストを名指すのは `areka-test-binary-stability/load-repro.md` の L160・L380。`areka-P0-ghost-session-test-load-flake` の方には ⑹ が無い） | `frame/drain_resnap_tests.rs:L103` | 「`areka-test-binary-stability` の `load-repro.md` の ⑹」 | 〔件 12〕 |
| 57 | テストを関数の単位で割った後も、両側のファイルの見出しが割る前の全体を語る（どちらを開いても、もう半分の確かめがここにあるように読める）。`missing_slot_resource_sends_nothing` の「置き場そのものが World に無くても」は、試しているのが「口が据わっていない→`NO_BODY`」。本体の `frame_move_order_tests.rs:L1-L17`・`frame_schedule_tests.rs:L1-L26` は `areka-runtime` の同名のファイルの見出しと一字一句同じで、無いテスト（`the_frame_calls_the_move_drain_…`・`BOOT_REGISTRATION`）を名指す。`switch_finish_tests.rs:L8` の「判定は集めてから 1 回」は 2 本に当てはまらない | `areka-runtime` の `frame/status_report_tests.rs:L1-L10`・`L100-L102`・`frame/status_report_stage_tests.rs:L1-L8`・`frame/window_state_tests.rs:L1-L3`、本体の `runtime_tests/frame/status_report_tests.rs:L1-L10`・`status_report_stage_tests.rs:L1-L8`・`window_state_tests.rs:L1-L7`・`switch_finish_tests.rs:L8`・`runtime_tests/frame_move_order_tests.rs:L1-L17`・`runtime_tests/frame_schedule_tests.rs:L1-L26` | 各見出しを自分の持つテストだけにし、残りは「もう一方の側（`<相手のパス>`）」と 1 行で指す | 〔件 5 続き〕 |
| 58 | 支えの見出しが「2 本の兄弟テスト」と書くが、使う兄弟は 3 本（25 番を直すと 4 本） | `frame/zorder_drain_test_support.rs:L4` | 数を書かない | — |
| 59 | 書き手の「定義の束が無い scope」の道（`error!` を出して何も変えずに戻る）を踏むテストが、ツリー全体で 0 本 | `frame/balloon_face_tests.rs`（本番は `frame/balloon_face.rs:L41-L49`） | 束なしの結線で `select_balloon_face(…, 0, 2, FaceChange)` を呼び、`error` 1 行と `balloon_faces`・`balloon_models` が空のままを見る 1 本（GPU は要らない） | — |
| 60 | 窓の entity の一覧の作り方が、支えとテストの 2 か所にある | `frame/window_state_tests.rs:L89-L93`・`frame/window_state_test_support.rs:L28-L32` | 支えに `all_windows(gw)` を置き両方から呼ぶ | — |

#### `areka-runtime` の `frame_*` のテストと支え

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 61 | 会話の再生のテストの「待機込み」の説明が事実と違う（台本 `\0あい\w9\1うえ\e` では最後の文字の終端 0.65 が待機の終端 0.55 より後＝compile が待機を落としても緑。`talk_lifecycle_tests.rs:L355-L364` がこの台本を落とし穴と名指す） | `frame_attach_tests.rs:L582`・`L628-L632` | 説明と assert の文から「待機込み」を消す（テストの名は本体の e2e が名指すので変えない） | — |
| 62 | 当たり判定の読み口のテストが `hit_region.rs:L122` の `unshown_scope_resolves_region_none` と同じ主張 | `frame_attach_tests.rs:L454-L489` | 消す（約 36 行・`hit_region.rs` には触らない） | — |
| 63 | move の送り口のテストが既存のテスト（`move_cue_move_sink_tests.rs:L20`・`L70`・`frame_drain_text_tests.rs:L226-L252`）と重なり、「wire_runtime の配線の縮図」と書くが `wire_runtime` を呼ばない | `frame_attach_tests.rs:L491-L547` | 消す。名を引く `frame_drain_text_tests.rs:L204` と、範囲の外の `move_cue_apply_move_tests.rs:L248`（「frame.rs」も誤り）の説明を直す | 〔範囲の外〕（`move_cue*` は `audit-emo2-boot-switch` の範囲） |
| 64 | 背景の順の字面の検査が、常に回る実 GPU のテスト（L693）と重なる（「実 GPU の檻が環境都合で回らない場での二重の備え」は今は当たらない） | `frame_attach_tests.rs:L635-L674` | 消し、名を引く `attach.rs:L476`・範囲の外の `assets.rs:L83` の説明を実 GPU のテストへ向け直す | 〔裁定 ⑵〕（doc にわざと置いた二重の備え） |
| 65 | 走査点の見張りの説明が古い（`present_frame` は今は `actor_present.rs:L71`・expect の「crates/areka の 2 つ上」） | `frame_attach_tests.rs:L765`・`L782` | 字面を直す | — |
| 66 | 数の説明が古い（「本ファイルの 7 本」は 8 本・「948 行で 52 行しか残していない」は 947 行・53 行） | `frame_balloon_offset_follow_tests.rs:L41-L50`・`frame_balloon_offset_roundtrip_tests.rs:L38-L39` | 数を書かない形に | — |
| 67 | アンカーの網羅の檻が、新しいアンカーの取りこぼしを捕まえない（`ANCHORS` に足し忘れても `len()==5` で緑） | `frame_balloon_offset_follow_tests.rs:L929-L947` | match の各腕が `ANCHORS` 上の添字を返し、`ANCHORS[i] == anchor` を主張する | 〔裁定 ⑴〕（完了した `areka-P0-balloon-offset-dpi` の tasks.md:L329 のレビューの是正として入った形） |
| 68 | キーワードの門のテスト 2 本が同じ場面を回し、片方は他方の部分集合（増えるのは探針の `assert_ne` 1 行） | `frame_balloon_offset_keyword_gate_tests.rs:L365-L406`・`L194-L242` | 1 行を移して 1 本にし、モジュールの doc の名指し（L22-L24・L369-L370）を直す | 〔裁定 ⑴〕（2026-08-27 の開発者の裁定の「受容した残余」を名で固定している） |
| 69 | 支えの中の「下の注記」（実際は上）と `apply_hide` の場所（今は `presenter/hub.rs:L227`）が古い | `frame_test_support.rs:L104`・`L192`・`L205` | 字面を直す | — |
| 70 | `balloon_is_manual` は確かめるだけで手動を自動へ戻すのに、そう書いていない（続けて 2 回呼ぶと 2 回目は偽）。`frame/balloon_face_tests.rs:L170-L171` は私有の `restart_manual` で補う | `frame_balloon_runtime_test_support.rs:L72-L81` | doc に「手動なら自動へ戻す」と書き、`restart_manual` を支えへ移す | — |
| 71 | `FrameHarness::run_work_area_resnap` は pub だが呼び手は自分の `run_placement_phases` だけ | `frame_test_support.rs:L824-L827` | 畳む。`frame/work_area_sync.rs:L54` の名指しも直す | 〔約束 b〕（外の読み手 0） |
| 72 | 32 bit の見張りが二重。支えの `const _: () = assert!(cfg!(target_pointer_width = "64"))` が支え全体に効くので、32 bit ではハーネスを使わないテストまで組めない（スキップでなくコンパイルの失敗）。`frame.rs` の各所の「`cfg(all(test, target_pointer_width = "64"))` で構造的に守る」は効いておらず、それを字面で見張るテスト（`frame_harness_tests.rs:L420-L440`）も何も上乗せしない | `frame_test_support.rs:L565-L570`、`frame.rs:L376-L456`・`L486-L487`、`frame_harness_tests.rs:L420-L440` | どちらか一方に寄せ、注を事実に合わせ、字面の見張りを消す（残すなら担い手を const の表明と書く） | 〔裁定 ⑴〕（完了した `areka-P0-dpi-transition-atomicity` の要件 7.5 がこの形を名指している可能性・未確認） |
| 73 | 落ちない数と使わない値（`visits == 5` は 5 要素の固定配列の輪・`before` は集めて使わない） | `frame_balloon_offset_roundtrip_tests.rs:L375`・`L392-L394`、`frame_balloon_offset_follow_tests.rs:L296-L313` | 両方消す | — |
| 74 | `synth_assets` の空の値の理由「`plan_attachments` が読むのは…のみ」が今の使い方より狭い（実 GPU の装着の相と `headless_wiring_with` にも渡る） | `frame_test_support.rs:L49-L53` | 「装着の相にも渡る。空の値で装着は成り立つ」 | — |
| 75 | もう無い `areka_placement::test_support::capture_logs`（`pub(crate)`）を観測の境界として名指す（`areka-placement` に `fn capture_logs` は 0 件。実際の捕捉は L24 の `log_capture_kit::capture`） | `frame_diag_route_tests.rs:L19-L20` | 「観測の境界は `log_capture_kit::capture` が捕まえる tracing のイベント」 | 〔件 14〕 |
| 76 | `run_dpi_phase` の doc の場所が古い（今は `frame/dpi.rs:L584-L599`） | `frame_diag_route_tests.rs:L89-L91` | `frame/dpi.rs` の `run_dpi_phase` と書く | — |
| 77 | doc の内部リンクが無いテスト名を指す（今は `the_harness_tests_are_connected_under_an_x64_only_gate`） | `frame_harness_tests.rs:L24-L25` | 名を直す | — |
| 78 | `SPAWN_SIZE_1 == (278, 357)` の表明が何も確かめていない（このテストは 278 も 357 も使わない） | `frame_harness_tests.rs:L355-L359` | 消す | — |
| 79 | 「scope1 に 1 行残る」と書くが、テストは `any(..)`＝「1 行以上」しか見ない（同じ窓を 2 度寄せる退行を見逃す） | `frame_chain_realign_adjacency_tests.rs:L421-L422`・`L472-L477` | 条件に合う行を数えて `== 1` | — |
| 80 | 陽性の対の関数が同じ入力で 6 回呼ばれ、中身は adjacency の 1 本と同じ駆動（ハーネスの起動が 6 回余分） | `frame_chain_realign_exclusion_tests.rs:L151-L166`（呼ぶのは L202・L216・L229・L241・L262・L281） | 1 本に残すか adjacency に任せる | 〔裁定 ⑴〕（「零件の主張は同じテストに陽性の対を置く」の決まりでわざとの形かもしれない） |
| 81 | 陽性の対の説明「ドラッグした事実では外さない」が確かめていることより強い（`drag_char_window_by` は位置を書くだけで印を残さず、−120→+120 はドラッグしなかった状態と区別がつかない） | `frame_chain_realign_tests.rs:L467-L480` | 「離していなければ詰まる（詰め直しは生きている）」 | — |
| 82 | route の檻のファイルに、route と関係の無い `AuthorDpis::for_target` のテストが入っている | `frame_diag_route_tests.rs:L329-L358` | `frame_dpi_tests.rs` へ移す | — |
| 83 | 本文の走査で、`code_only`・`index_of`（`source_text_test_support.rs:L4-L19`）と同じ処理を 2 回手で書いている | `frame_move_order_tests.rs:L145-L154`・`L182-L192` | `use super::super::source_text_test_support::{code_only, index_of};` | — |
| 84 | 「見送り点は 4 つ」が古い。`HoldSite` は 5 つ（`Dpi`・`Reconcile`・`Resnap`・`WorkAreaResnap`・`PatternMove`・`areka-placement/src/dpi_sync.rs:L134-L151`）で、テストは `PatternMove` の呼び口（`frame/move_shift.rs:L71`）の綴りを固定していない。ほかに `HoldSite` に入らない見送りとしてシェルの切り替えの `reseed_held` もある | `frame_transition_branch_tests.rs:L496-L546`（L501・L527）、`frame_dpi_sync_hold_tests.rs:L291-L296`・`L463-L464` | 5 本目として `include_str!("frame/move_shift.rs").contains("dpi_sync::defers_window_write(world, window, HoldSite::PatternMove)")` を足し、「4 つ」を「5 つ（＋シェルの切り替えの見送り）」に。`areka-placement/src/follow/window_move.rs:L666` の「見送りの 4 点」も古いが、そちらは〔C23〕なので申し送る | 〔件 14〕〔C23〕（placement 側の文だけ） |
| 85 | None の道の見出しが、もう別ファイル（`frame_dpi_reproject_tests.rs:L233-L263`）にあるテストを「上の `s2_red_*`」と呼ぶ。「⚠ 区分」の段落は兄弟の L273-L283 の写し | `frame_dpi_reproject_none_tests.rs:L18`・`L33-L43` | L18 を相手のファイルの名で指し、⚠ の段落は片方を残す | — |
| 86 | 接地させる手順の 14 行が同じファイルに 2 か所（`use` もファイルの途中） | `frame_dpi_reproject_tests.rs:L139-L152`・`L443-L456`・`L37-L39` | `fn ground_all_at(world, gw, scopes, dpi)` に出し、`use` を先頭へ | — |
| 87 | 再スナップの同寸と異寸のテストが 2 本ずつ重なり、同寸の 2 本の「冗長駆動ゼロ」「再書込が起きない」は書き込みの印の無い World で位置と寸を比べるだけ（`drain_resnap.rs:L277-L280` の読み飛ばしを消しても `resize_window_to` 側が吸うので緑） | `frame_resnap_tests.rs:L17-L49`・`L52-L71`・`L247-L337` | 2 本ずつを 1 本に。「書かない」を主張するなら `dpi_world` と `assert_no_write` で見る | — |
| 88 | 「書込の件数を覗く API が無い」が古い（今は `wintf::ecs::window::drain_window_pos_commands` がある） | `frame_dpi_tests.rs:L15-L19` | 理由の文を今の事実へ | — |
| 89 | `reset_window_pos_skipped` の WARN 1 行と線が空になることを 3 本が確かめ、この 1 本はほかの 2 本（`frame_drain_text_tests.rs:L314-L352`・本体の `frame_move_order_tests.rs:L32-L70`）に含まれる | `frame_move_order_tests.rs:L118-L139` | 消す | — |
| 90 | 装置の作り直しの置き場を見るテストが move の置き場のファイルにあり、見出しにも無い | `frame_move_order_tests.rs:L178-L202` | 見出しに 1 行足すか、相の並びを見るファイル（`frame_exit_gate_tests.rs` など）へ移す | — |
| 91 | 期待値を `BTreeSet` に通して `Vec` に戻して並べている | `frame_dpi_tests.rs:L295-L310` | `want.sort_unstable()` | — |
| 92 | debug でだけ鳴る見張りを待つ `#[should_panic(expected = "DpiSyncHold")]` に `cfg_attr(not(debug_assertions), ignore)` が無い（`--release` だけで赤になる。範囲の外の同じ形の 3 本は付けている）。対の零件のテスト（L388-L415）が頼る手写しの `INVARIANT_WARNING`（L69）は、在ることをどこも確かめない | `frame_work_area_resnap_hold_tests.rs:L428-L453`・`L69` | L429 の下に兄弟と同じ 1 行を足し、`--release` の合否が変わることを記録に残す（要件 5.8）。`--release` でテストを回す道は今は無い（3 節の件 7） | 〔件 7〕 |
| 93 | 原子性のテストの説明が他ファイルを行番号で指し、行も語の数もずれている（3 節の件 2 の表・「`PlacementRoute` 12 語」は今は 13 語） | `frame_transition_atomicity_tests.rs:L47-L54`・`L527-L528` | 行番号をやめ関数名（`WM_DPICHANGED` の処理・`enqueue_window_set_pos`・`apply_window_pos_changes`・`pair_fix_command`）で指し、「12 語」を「13 語」に | 〔件 2〕 |
| 94 | 同じ本体を水準違いで 2 か所に持つテスト（作業領域だけの変化・定常フレームの書き込み 0・モニター 0 台）と、同じ観測の条件のテスト（`s2_monitors` が 2 台）が 2 本ずつ | `frame_work_area_resnap_tests.rs:L103-L134`・`L156-L209`・`L260-L300` と `frame_transition_branch_tests.rs:L648-L757`・`L776-L826`、`frame_work_area_sync_tests.rs:L251-L275` と branch `L843-L902`、`frame_transition_atomicity_tests.rs:L586-L613` と branch `L1247-L1271` | 同じ本体を `[96, 120, 192]` で回す 1 本に。観測の条件のテストは支えの側に 1 本 | 〔裁定 ⑴〕（branch の L16-L19 は要件 7.2 の 2 水準を兄弟と別に持つ意図を書いている） |
| 95 | 「19 本はいずれも `run_placement_phases` を駆動する」が 1 本ずれている（20 本のうち L504 の 1 本は字面の走査だけ） | `frame_transition_branch_tests.rs:L23`・`L1072-L1074` | 数を書かない | — |
| 96 | 「`frame_work_area_resnap_tests.rs` は 1 文字も触っていない」が事実と違う（同じ spec の PR #125 が `.offset` を `.offset()` に 2 か所変えた） | `frame_transition_atomicity_tests.rs:L422-L424` | 「表明は変えていない」 | — |
| 97 | 上限の定数（`*_MAX`・`*_BOUND`）を等号で比べていて、判定器（`transition_judge_verdict.rs:L162` の「絶対値がそれ以下」）と意味が違う。判定器の側で `GROUND_DIFF_MAX` を 1 に緩めると、diff 0 で正しく接地した遷移がこのテストで赤になる（モジュールの説明の「片方だけ緩めて食い違わないため」と逆の結果） | `frame_transition_atomicity_tests.rs:L357-L361`・`L367-L371`・`L404-L408`・`L518-L521`、`frame_transition_branch_tests.rs:L300-L304`・`L608`・`L704-L708`・`L946`・`L1034` | `diff.abs() <= MAX`・`len <= MAX` に。窓あたりの本数は陽性の対として「1 本以上」も足す | — |

#### 本体のテスト（`runtime_tests/frame*`・`session/ghost_quit_*tests.rs`）

| 番号 | 一言 | 場所 | 直し方 | 印 |
|---|---|---|---|---|
| 98 | `nothing_arrived_keeps_waiting_silently` の中に、名に無い `NO_BODY` の確かめが紛れ、落ちようがない（`NO_BODY` の口は World を読まない定数の閉包） | `runtime_tests/frame/switch_tests.rs:L479-L514` | L479-L499 と L500-L514 の表明を消し、`plant_ghost_switch` もこの 1 本では外す | — |
| 99 | `log_ghost_falls_back_to_dash_and_raise_reports_normal_band` は本体の項目を本当には使っていない（`hooks::install` の 1 行だけ・`band=normal` の確かめはツリー中この 1 本） | `runtime_tests/frame/window_state_tests.rs:L142-L154` | `areka-runtime/src/frame/window_state_tests.rs` へ移す（`hooks::install` の行を消し、絞り込みを中に書き、`use log_capture_kit::capture;` を足す。`window_state_events` は本体の残り 4 本が使うので写さない。test-support へ出すと `Cargo.toml` に触るので出さない）。本体の口 `ghost_folder` の `?` を通る確かめは失う | 〔件 10〕 |
| 100 | `shell_split_builds_the_seriko_fallback_from_the_same_values_as_the_box_bundle` は本体の項目を何も使わない純関数の確かめ。ただし土台 `shell_built()` を本体の 4 本も使う | `runtime_tests/frame/switch_tests.rs:L846-L886`（土台 L130-L163） | 今のままが最小。移すなら `shell_built` を `frame/switch` の test-support の口へ出す（M） | — |
| 101 | `runtime_tests/frame.rs` の注が古い。L25 の「`carry_tests` の本物の親は `frame_attach_tests`」は事実でない（`areka-runtime` 側は `carry_tests` を宣言しない）。L39 の「同じくハーネスに乗るので」は最初の注なので「同じく」の先が無い（`areka-runtime/src/frame.rs` の写し）。L56-L57 の「使い手は 3 つのテストファイル」は今は 7 つ | `runtime_tests/frame.rs:L25`・`L39`・`L56-L57`、`runtime_tests/frame_chain_finalize_persist_test_support.rs:L4-L5` | 事実に合わせる（数は書かない） | — |
| 102 | 常に手前の 2 本が、関係の無い読み直しのテストファイルの子として宣言されている（`switch_reload_tests.rs` を消すと、Rust は宣言の無いファイルを警告しないので 2 本が黙って走らなくなる） | `runtime_tests/frame/switch_reload_tests.rs:L190-L192`、`switch_stay_on_top_tests.rs:L4`・`L12` | 3 行を `switch_finish_tests.rs` の `mod` の並び（L811〜）へ移し、`super::super::*` を `super::*` に（親は 821 行） | — |
| 103 | 装着の控えの檻のファイルの説明が 3 本目（⑶ 控えた着せ替えを着せ直し、名の無いものは落とす）を挙げていない | `runtime_tests/frame_attach_carry_tests.rs:L6-L7` | ⑶ の 1 行を足す | — |
| 104 | 時間切れの通しの檻は「バルーンの組の届けだけ読み捨てる」と言うが、腕が `KanadeMsg::ExecutionState(_)` なので `NoUserBreak` も黙って捨てる | `runtime_tests/frame_balloon_timeout_notice_e2e_tests.rs:L148-L159` | 腕を `ExecutionState(ExecutionStateUpdate::Balloons(_))` に狭める | — |
| 105 | 部品の確かめのテストが、同じファイルのテスト 9（値まで見る）と再起動のテストの `second_run`（読み直し）に重なる。固有なのは置き直しの大きさの前提（L50-L62）だけ | `runtime_tests/frame_chain_finalize_persist_tests.rs:L27-L91` | 偽の保存先の半分を消し、実物のファイルの 1 回だけに | — |
| 106 | 記憶のファイルの道を、テストの側でもう一度組み立てている（`ghost_master()`・`profile_areka_root(..)`・`"sylphya.toml"` の写し。書き終えを待つためだけに `let _ = store.load()`） | `runtime_tests/frame_chain_finalize_persist_nowrite_tests.rs:L30-L34` | 支えに `PersistStore::file_text()`（`barrier` の後に読む）を置き、`memory()` を消す | — |
| 107 | 可視性の通しのテストの説明が、他ファイルを行番号や古い道で指してずれている（`spine_test_support.rs:80-83`→L77・`refresh.rs:74-80`→L104・`surface.rs:249-262`→L335-L338・`wintf/src/ecs/graphics/visual.rs:26`→`visual/mod.rs:L221`）。「毎フレーム 9 つの相」も古く、節は ⑴⑵⑶ の次が ⑸ | `runtime_tests/frame_visibility_integration_tests.rs:L134`・`L258`・`L335`・`L393-L394`・`L172`・`L249`・`L328`・`L456` | 行番号を外し関数名で指す。相の数は書かない。節の番号を詰める | — |
| 108 | 拡大率の知らせの通しのテストの先頭の場面の表が ⑷ で終わり、⑸（送り先なし・警告 1 件）が無い | `runtime_tests/frame_scale_notice_integration_tests.rs:L8-L15`（⑸ は L275-L303） | 表と送り先の文に ⑸ を足す | — |
| 109 | kanade の返事の待ちが壁時計の 30 秒の締め切り（進みの目印の無い待ち） | `runtime_tests/frame_scale_notice_integration_tests.rs:L249-L255` | kanade を同じ受け箱への `Close` で止め `join_bounded` で待つ（`ReplyReceiver` は `wait_recv` に渡せない・実装で改めた） | 〔範囲の外の持ち主と重なる〕（`.kiro/specs/areka-test-wait-wording/brief.md:L13`・`L79` がこの行を古い道 `emo2_boot/…` で載せている＝どちらで直すかは議題 9） |
| 110 | 鎖の規則のテストに、ほかのテストとほぼ同じ道を通るものがある（`a_remembered_left_mark_joins_with_the_defaultx_gap` は L496-L546 の `defaultx=36` の列と幅だけが違う。L573-L588 の後半は `frame_chain_finalize_align_tests.rs:L187-L212` と同じ組み方・同じ表明） | `runtime_tests/frame_chain_rule_tests.rs:L128-L146`・`L573-L588` | 前者を消し、後者は記録の確かめを align の 1 本へ寄せる | — |
| 111 | 本体の `a_script_reset_sends_nothing_to_kanade` の 3 つ目の表明は決して落ちない（`PendingResetRaise` は本体の `pub(crate)` で、置くのは `menu/reset_pos_frame.rs:L92` だけ＝move drain からは呼べない） | `runtime_tests/frame_move_order_tests.rs:L63-L69` | 3 つ目を消す | — |
| 112 | `scaling_text` を本体の出し直し（`crate::ghost_session::ghost_session_scope_scale`・`pub(crate) use`）経由で引いていて、本体に依っているように見える（本物は `areka_runtime::scope_scale::scaling_text`） | `runtime_tests/frame_scale_notice_integration_tests.rs:L23` | `use areka_runtime::scope_scale::scaling_text;` | — |

### 判定と減らせる量

- 判定: 範囲は総じて健全。本番のコードの必ず直すは 0 件で、必ず直すの 1 件はテストの表明。直す順の頭は、1 番（落ちない表明）・2 番（預かりの列の上限）・3 番（文字層の再試行）・4 番（scope の上限・件 9）・6 番（件 13 のテスト）。
- 減らせる量の見込み: テストの写しと重なり（22〜30・33・34・62〜64・78・89・98・105・110・111 番ほか）と死んだ import（5 番）で、**約 −1,200〜−1,600 行**。依存は増えも減りもしない（−0）。

---

## 3. 申し送りの件（要件 5・1.7）

行番号は今の木（HEAD `6f2d6ffc`＝main `9bf6e32e`）。

| 件 | 要件 | 出どころ | 今も当てはまるか・今の場所 | 指摘 |
|---|---|---|---|---|
| 1 | 5.1 | `runtime-module-untangle` | 当てはまる。`frame/wiring.rs:L319`。同じ所の L290・L296-L299・L325-L331 も古い | 37 |
| 2 | 5.2 | `audit-wintf-window` 件 1 | 当てはまる。`frame_transition_atomicity_tests.rs:L47-L54`・`L527-L528`。正しい行は下の表。加えて「12 語」が今は 13 語 | 93 |
| 3 | 5.3 | `audit-wintf-window` 件 2 | 当てはまる。`zorder_drain.rs:L51-L53`・`zorder_descript.rs:L32-L34`・`zorder_drain_tests.rs:L443`。**要件 5.3 の直しの文は、`areka-runtime` の側では事実と違う**（46 番・議題 5） | 46 |
| 4 | 5.4 | `runtime-crate-split` ⑵ | 当てはまる。下の表 | 5 |
| 5 | 5.5 | `runtime-crate-split` ⑶ | 下の「件 5 の数」 | 114 |
| 5 続き | 5.6 | 同 ⑶ 案 C | 同じ名前のテストは両側で **0 件**（`window_state_tests`・`status_report_tests`・`status_report_stage_tests`・`frame_move_order_tests`・`frame_schedule_tests` の全部の組で関数名を突き合わせた）。中身の写しも 0。残るのは見出しの写しだけ | 57 |
| 6 | 5.7 | `runtime-crate-split` ⑷ の 1 つ目 | 当てはまる。`frame/drain_resnap.rs:L554`・`frame_chain_finalize_tests.rs:L204`。同じ形の誤りが `drain_resnap.rs:L281` にもある | 43 |
| 7 | 5.8 | 同 ⑷ の 2 つ目 | 当てはまる。`frame_work_area_resnap_hold_tests.rs:L429`。見張りは `areka-placement/src/follow/window_move.rs:L686-L689` の `debug_assert!`（`warn!` は L680-L685）。**`--release` でテストを回す道は無い**（`tools/test-all.ps1:L75` は `cargo test --workspace --no-fail-fast -j 4`＝debug・`tools/load-flake.ps1` は debug・`.github/workflows/release.yml`・`crates-io.yml` はテストを回さない・`package.ps1` は `cargo build --release` だけ・`tools/perf/` にも無い）。範囲の外の同じ形の 3 本（`areka-placement/src/dpi_sync_tests.rs:L351`・`follow_window_move_hold_watch_tests.rs:L158`・`L184`）は `#[cfg_attr(not(debug_assertions), ignore = "debug_assert! に依る")]` を付けている | 92 |
| 8 | 5.9 | 同 ⑷ の 3 つ目 | 当てはまる。`frame/move_shift.rs:L75` | 40 |
| 9 | 5.10 | `audit-emo2-boot-spine` ア | 当てはまる。`frame/dpi.rs:L413-L426`。今は窓の scope が {0,1} だけなので届かない。`drain_resnap.rs:L380`・`L675` の `as u32` は同じ穴のもっと弱い形 | 4 |
| 10 | 5.11 | 同 イ | 当てはまる。`runtime_tests/frame/window_state_tests.rs:L142-L154`。新しい依存なしで戻せる（dev-dependency の `log-capture-kit` は `areka-runtime/Cargo.toml:L44` に既にある） | 99 |
| 11 | 5.12 | 同 オと完了の時の棚卸 | 3 つとも当てはまる。`zorder_drain_tests.rs:L460-L470`・`frame_chain_finalize_tests.rs:L303-L307`（注は L293-L300）・`frame_drain_text_tests.rs:L14-L27`。同じ crate に `capture_logs` の写しがあと 2 つ（`zorder_descript_tests.rs:L48`・`zorder_drain_test_support.rs:L151`） | 22・23・24 |
| 12 | 5.13 | 同 完了の時の棚卸 | 当てはまる。`frame/drain_resnap_tests.rs:L103`。⑹ の持ち主は `areka-test-binary-stability`（その `load-repro.md` の L160 の節と L380 の行がこのテストを名指す） | 56 |
| 13 | 5.14・5.15 | `audit-placement-follow-residue` | 本番は書く（下の「件 13 の確かめ」）＝5.15 には当たらない。ただし範囲の中にそのフレームを通る決定論のテストは 0 本 | 6 |
| 14 | 5.16 | `audit-areka-app` の brief の件 2〜4・9 | 全部当てはまる。`atom` は範囲の中に 11 か所（本体の側は 0）。`frame_diag_route_tests.rs:L19`。`frame_transition_branch_tests.rs:L501`・`L527-L546`・`L545`。`frame/drain_resnap.rs:L595-L598`・`L479` | 41・44・45・75・84 |

### 件 2 の今の行

| 説明の書き方 | 今の場所 |
|---|---|
| `window_pos.rs:464`（`guarded_set_window_pos` の直呼び） | `crates/wintf/src/ecs/window_proc/window_pos.rs:L474-L475`（`WM_DPICHANGED` の処理の中） |
| `:468`（観測の行）・527 行の `window_pos.rs:468-489` | 同 L479（`emit_line(&write_line(..))`）・`WriteRecord` は L479-L502 |
| `follow/window_move.rs:730` | `crates/areka-placement/src/follow/window_move.rs:L719-L722`（`with_tag(transition_diag::write_tag(..))` は L721・`enqueue_window_set_pos` の中） |
| `graphics/systems/window_pos.rs:102` | wintf の `graphics/systems/window_pos.rs:L106-L109`（`origin: ORIGIN_WINDOW_POS` は L107・`apply_window_pos_changes` の中） |
| `zorder_pair_maintain.rs:212` | wintf の `window/zorder_pair_maintain.rs:L217-L220`（`origin: ORIGIN_ZORDER_PAIR` は L218・`pair_fix_command` の中） |

- 「タグを付ける 3 か所」は今も正しい。

### 件 4 の測り（`--force-warn unused_imports`）

| 許可の行 → use の行 | lib で未使用 | テストで未使用 | 判断 |
|---|---|---|---|
| `frame.rs` L98→99 `RefCell` | 未使用 | 使う | テストでだけ要る（`cfg(test)` へ） |
| L100→101 `BTreeSet, HashMap` | 未使用 | **未使用** | 文ごと消せる |
| L102→103 `Rc` | 未使用 | 使う | テストでだけ |
| L104→105 `Receiver` | 未使用 | **未使用** | 消せる |
| L107→108 `Entity` | 未使用 | 使う | テストでだけ |
| L109→110 `Changed, Query` | 未使用 | **未使用** | 消せる |
| L111→112 `SystemState` | 未使用 | 使う | テストでだけ |
| L114→115 `debug, error, info, warn` | `error`・`info`・`warn` | `error`・`info`・`warn` | 3 名を消し `debug` だけ残す（許可は要らない） |
| L117→118 `EmoPresenter, PresentCommand, TargetId, TextSlotView` | 4 名 | `TextSlotView` | `TextSlotView` を消す。残り 3 名はテストでだけ |
| L119→120 `TextLayerRuntime, present_frame` | 未使用 | 使う | テストでだけ |
| L121→122 `BalloonModel` | 未使用 | **未使用** | 消せる |
| L123→124 `ActorKey` | 未使用 | 使う | テストでだけ |
| L125→126 `DPI, FrameTime, GraphicsCore, SizeI, WindowPos, WucGraphicsResource` | 6 名 | `DPI`・`FrameTime`・`SizeI`・`WindowPos` | 4 名を消す。2 名はテストでだけ |
| L128→129 `DESPAWNED_SKIP_TAG, PlacementRoute` | 未使用 | `DESPAWNED_SKIP_TAG` | 1 名を消す。`PlacementRoute` はテストでだけ |
| L130→131 `resize_window_keep_position, resize_window_to` | 未使用 | **未使用** | 文ごと消せる |
| L132→133 `SizePx` | 未使用 | **未使用** | 消せる |
| L134→135 `BalloonWindowMarker, CharWindowMarker, GhostWindows` | 未使用 | 2 つの Marker | Marker を消す。`GhostWindows` はテストでだけ |
| L153→154 `pub use attach::{…}` | 警告なし | 警告なし | **許可は要らない**（公開の再輸出）。L147-L151 の説明も誤り |
| L171→172 `connect_balloon_text` | 未使用 | 使う | テストでだけ |
| L178→179-183 dpi の束 | 4 名（L173-L177 の説明と一致） | 警告なし | テストでだけ |
| L188→189-192 drain_resnap の束 | 5 名（L184-L187 の説明と一致） | 警告なし | テストでだけ |
| `wiring.rs` L13→14 `World` | 未使用 | 使う | test と test-support で要る。許可は要るが理由の文が誤り |
| `wiring.rs` L20→21 `TargetId` | 未使用 | 使う | 同上 |

- lib のビルドの警告は 23 件（全部 `frame.rs` と `wiring.rs`）、テストのプロファイルは 11 件（全部 `frame.rs`）。
- `--features test-support` 付きは回していない。test-support を付けても `frame.rs` のテストのモジュールは組まれないので lib と同じ結果、`wiring.rs` は test-support だけで組まれるメソッドが `World`・`TargetId` を使うので警告が消える、と読んだ（実装の段で 1 回回して確かめる）。

### 件 5 の数（要件 5.5・問い 2）

- 2.5 の 10 件（`frame.rs` の再輸出 2・`DiscardReason`・`log_stay_on_top_discard`・`RuntimeWiring::{scope_scale_table, presenter, shown_talk_id, attached, runtime}`・`ZOrderAbsentReports`）: どれも `areka-runtime` の外に使い手がある（本番の `app_exit.rs`・`session/ghost_switch.rs`・`ghost_session_runtime.rs`・`input_events/*`・`mcp/*` と本体のテスト）。
- 4.2 ⑴: `StageObservation`（`frame/status_report.rs:L52`）だけが **中だけ（私有に戻せる）**。`BalloonObservation`・`StayOnTopState` は外で名を書く所は無いが、外が呼ぶ公開の関数の型に入るので開いたままが要る（戻すと `private_interfaces` の警告）。ほかの項目と `RuntimeWiring` の欄 11・メソッド 4 は、どれも本体のテストに使い手がある。
- 4.2 ⑵（`test_open` 4 件）・⑶（宣言の印 6 つ）・⑷（渡し口 14）: どれも本体のテストに使い手がある。
- 6.1 examples: `pub` にした項目は 0 件（変わらず）。
- frame の本番のファイルの `pub(crate)` は 9 件。**誰も使わない（消せる）ものは 0 件。** 狭められるのは 114 番の 7 件（私有に 3・`pub(super)` に 4）。`resolve_talk_time`（`scale_text.rs:L251`）と `seed_zorder_descript_base`（`wiring.rs:L267`）は frame の外の crate の中で使うので `pub(crate)` が要る。
- 一覧の外: `ShownRatioSource`・`PhysicalSizeSource`・`ScaleReportSource`・`SnapshotChange` も `pub` で、外で名を書く所は無いが、外が使う公開の関数の型に入るので開いたままが要る。
- 数え方は問い 2 の仮定（本体のテストだけが使う項目は使い手に数える）に従った。`crate_source!` で字面を読むだけの所は使い手に数えていない。

### 件 13 の確かめ（要件 5.14）

結論: **本番の相の並びの中で、札の外れるフレームに拡大率の相は、その窓を `DpiReproject` で、入れ直した揃え方で書く。** 読みの根拠は次のとおり（cargo は回していない）。

1. 札は `crates/areka-placement/src/dpi_sync.rs:L126-L130` の `DpiSyncHold { since_frame }`。付け外しは `apply_dpi_phase_gate`（同 L226-L256）の 1 か所だけで、付けるのは L230-L234、外すのは L246（30 フレームを越えたとき・`warn!` の上で）と L249-L251（Proceed）。本番のコードで `remove::<DpiSyncHold>` はこの 2 行だけ。呼ぶのは `frame/dpi.rs:L397` だけ。
2. 1 フレームの相の並び（`frame.rs` の `frame_system`・L226-L359）:
   0 終了の門 → 1 作業領域の同期（L249）→ 2 装着（L251）→ 3 装置の作り直し（L255）→ **4 拡大率の相（L262）＝札を付け外し・`DpiReproject`** → 5 作業領域の変化の再スナップ（L270・札を読むだけ）→ 6 取り出し（L274）→ **7 差し替え（L278）＝`reanchor_char_windows`** → 8 可視性（L283）→ 9 報告寸の突き合わせ（L296・札を読むだけ）→ 10 重なり（L308）→ 11 window_state（L311）→ 12 再スナップ（L315・札を読むだけ）→ 13 move drain（L327）→ 14 面の move のずれ（L332・札を読むだけ）→ 15 連鎖の確定（L336）→ 16 連鎖の解き直し（L343）→ 17 文字の拡大率（L348）→ 18 文字（L349）→ 19 状態の報告（L354）→ 20 拡大率の知らせ（L357・最後）
3. 同じフレームの中では外すのが先・書くのが後。`dpi_phase_with` は 2 巡で、第 1 巡（`dpi.rs:L374-L401`）で対象の全部の窓の札を付け外しし、第 2 巡（L403-L497）で通った窓だけを書く。対象は `Changed<DPI>` と札を持つ窓の和集合（L356-L372）なので、**札の付いていた窓は DPI の変化が無くても必ず入る**。
4. 札が外れたキャラ窓は、報告が `Some` なら `reconcile_window_size`→`resize_window_to(.., PlacementRoute::DpiReproject)`（L451-L457→L167）、`None` なら `reproject_char_window_at_current_size(.., DpiReproject)`（L478-L482→L562-L570）。どちらの腕でも `resize_window_to` を 1 回呼ぶ。書けないのは `WindowPos.size` が無い（L544-L560 で warn）・entity が消えた・`u32::try_from(scope)` の失敗（L415・今は届かない）の 3 つだけ。バルーン窓は `Some` のときだけ書く（L484）。
5. 新しい揃え方が使われる理由: 切り替えのフレーム N の差し替えの相（`switch.rs:L564` の `apply_shell_descript`→`reseed.rs:L83` の `replace_anchor`）が `Anchored` を新しいシェルの値へ入れ替える。同じ相の `reanchor_char_windows`（`reseed.rs:L285-L296`）は札のある窓を読み飛ばす（`reseed_held`）。外れるフレーム M（> N）で `resize_window_to` は書く時点の `Anchored` を読む（`window_move.rs:L192`）ので、新しい揃え方で置かれる。
6. 札が外れたのと同じフレームに切り替えが着地した場合は、`DpiReproject`（古い揃え方）の後に同じフレームの差し替えの相が `AnchorChange`（新しい揃え方）で書く。これは札の無い切り替えでも `finish_shell` の `reconcile_reported_sizes`（`switch.rs:L544`）が先に書く形と同じで、札に固有の害ではない。
7. テスト: 範囲の中にこの流れを通る決定論のテストは **0 本**（範囲の全体を `DpiSyncHold`・`sync_hold`・`reseed`・`reseed_held`・`reanchor`・`DpiReproject`・`since_frame`・`AnchorChange`・`alignmenttodesktop` で grep した）。範囲の中で札を使うのは `frame_dpi_sync_hold_tests.rs` ほか 7 本で、どれも「拡大率の知らせが先に届く」駆動か手で付けた札。本体の `runtime_tests/frame/switch*_tests.rs` はシェルの切り替えを通るが札を使わない。範囲の外の `reseed_tests.rs` の `t_rs10`（L557-L573）・`t_rs11`（L577-L603）は見送りと、札を手で外した後の直の `resize_window_to` だけ。→ 6 番・議題 4。
8. 名について: 依頼の文にある `reseed_char_windows` という関数はツリーに無い。`reseed_held` を出すのは `reanchor_char_windows`（`reseed.rs:L291`）だけ。要件と brief の文は `reanchor_char_windows`（brief）か配置の crate の置き直し（要件）で、要件の文に直しは要らない。

---

## 4. 外への約束に響く指摘（要件 1.6）

| 指摘 | 約束 | 響く読み手 | 扱いの案 |
|---|---|---|---|
| 2 | (a) `\![move]` の当たる時点（面を出さない窓への移動を捨てる・上限で落とす）。`doc/COMPAT_ARCHITECTURE.md` §8 の L439 の行 | ゴーストの台本（SSP 互換）・§8 の文書（C23 の 2 本が触る） | 議題 2（案 ⑶ なら振る舞いは変わらない） |
| 3 | (a) 装着に失敗したバルーンで、後から文字が出るようになる（案 ⑴）。案 ⑵ は記録の level だけ | ゴースト。字面で読む所は 0 | 直す（失敗の道の直し） |
| 4 | (a) は今は変わらない（scope が {0,1}）。dpi.rs の `error!` の文が変わりうる | 字面で読む所は 0（grep） | 直す |
| 5・114 | (b) 公開の名前は減らない（`StageObservation` 1 つ・外の読み手 0）。`pub(crate)` を狭めるだけ | 外の読み手 0 | 戻す |
| 26・27・29・31・33 | (b) テストの支え（`frame_test_support`・`frame_shared_test_support`）の公開の補助が増える・移る | 本体の `runtime_tests/` | 足すだけ・呼び手を同じ変更で直す |
| 39 | (c) 行は変えない（直すのは doc だけ）。`runtime attach: 装着計画を実行` は `crates/areka/tests/emo2_real_run/common.rs:L18` が字面で読む | 実機の確かめ | 行に触らない |
| 46・54 | (c) 文だけ。サインオフの `RUST_LOG` の指定（`signoff-procedure.md`）が読み手 | 手順書 | 行に触らない |
| 113 | (c) warn の出る条件が 1 件ぶん変わる（文は変えない案もある） | 字面で読む所は 0（grep） | 直す |

- ゴーストと SSP 互換の振る舞い（(a)）が変わりうるのは 2 番だけ（3 番は失敗の道の直し）。2 番は §8 に SSP 2.9.26 の測りの記録つきで載っている行に触れる。
- ログの行の語と target（(c)）を変える指摘は 0 件（46・54・113 は文か条件だけ）。

---

## 5. 直さずに残す候補（要件 2.2）

- **⑴ 開発者の裁定に反する直し**（候補 8 件）
  - 2: `window-work-area-clamp` の設計の話し合い 2（2026-10-29）「待つ間は順のまま預かり、取りこぼさない」。
  - 46: 要件 5.3 の文言そのもの（「前方一致なので漏れはせず」）を改める必要がある。
  - 55: `areka-test-binary-stability` 6.5 の目印の決め（assert を足すだけなら触れない見込み）。
  - 67: `areka-P0-balloon-offset-dpi` のレビューの是正で入った網羅の檻の形。
  - 68: 2026-08-27 の開発者の裁定の「受容した残余」を名で固定したテスト。
  - 72: `areka-P0-dpi-transition-atomicity` の要件 7.5 が x64 の門の形を名指している可能性（未確認＝設計の段で requirements.md を開く）。
  - 80: 「零件の主張は同じテストに陽性の対を置く」の決まり。
  - 94: 判断の分岐のテストが要件 7.2 の 2 水準を兄弟と別に持つ意図。
- **⑵ わざと置いた先送りの印**（候補 1 件）
  - 64: 背景の順の字面の検査（doc に「二重の備え」とわざと置いた形）。
  - `shortcut:`・`ponytail:` の注釈と、doc つきの `#[allow(dead_code)]` は範囲の中に 0 件（指摘にしていないものも無い）。
- **⑶ SSP 互換か ukadoc の正典に関わる振る舞いの変化**（候補 1 件）
  - 2: `\![move]` の預かりは ukadoc が沈黙し、§8 に SSP 2.9.26 で測った記録がある（`window-work-area-clamp/research.md` の 12 節）。面を出さない窓への移動を捨てる・上限で落とすと、SSP とどう違うか（SSP が見えない窓への `\![move]` をどう扱うか）は測っていない。要件 2.3 のとおり、議題にするなら ukadoc の確かめと、要るなら SSP の MCP の測り（要件 8.2・開発者の許しと排他テストの机）を添える。案 ⑶（集合にするだけ）なら振る舞いは変わらず、⑶ に当たらない。

---

## 6. 範囲の外で見つけたこと（持ち主へ渡すもの）

| 件 | 場所 | 持ち主の候補 | 中身 |
|---|---|---|---|
| ア | `crates/areka-placement/src/config.rs:L152-L164`（`detect_scopes`） | `extra-character-windows`〔C23〕 | 窓の scope を作る入口に `i32::MAX` の上限を置く（4 番の本筋の片割れ） |
| イ | `crates/areka-placement/src/follow/window_move.rs:L666` | `areka-placement` の見直し（完了済み）〔C23〕 | 「見送りの 4 点」は今は 5 点（84 番と同じ型） |
| ウ | `crates/areka-placement/src/follow_window_move_hold_watch_tests.rs:L35` | 同上〔C23〕 | `INVARIANT_WARNING` の手写し。在ることを確かめる側が無い（92 番と同じ型） |
| エ | `crates/areka-placement/src/config.rs:L104-L109`・`follow/work_area.rs:L239`・`follow_visibility_balloon_wiring_tests.rs:L824` | 同上〔C23〕 | 古い道 `runtime::frame::…`・「本体の」（今は `areka_runtime::frame::…`） |
| オ | `code_only` の写し: `crates/areka/src/tick_gate_config_producers_tests.rs:L70`・`crates/areka-placement/src/test_support.rs:L424` | 本体の見直し（`audit-areka-app`）・placement〔C23〕 | `source_text_test_support::code_only` と同じ |
| カ | `crates/areka-placement/src/follow_window_move_diag_tests.rs:L173` | placement〔C23〕 | `window_move_lines` が `frame_test_support.rs:L912` と同じ働き（上流の crate の `test_support` へ移せば両方が 1 つを使える） |
| キ | `crates/areka-runtime/src/move_cue_apply_move_tests.rs:L53`・`L248`・`move_cue_place_tests.rs:L85` | `audit-emo2-boot-switch` | 偽の HWND の輪の写し（30 番）・`move_cue_sink_reaches_runtime_wiring_receiver` の場所の誤り（63 番） |
| ク | `crates/areka-runtime/src/balloon_visibility_phase.rs:L464` | `audit-emo2-boot-switch` | `marker.scope as u32`（4 番と同じ穴）。4 番で共通の関数を置けば、それを使うだけ |
| ケ | `crates/areka-runtime/src/placement_tests/transition_judge.rs:L709-L717` | `placement_tests/` の持ち主（完了済みの `audit-placement-follow-residue`） | 経路 A の述語を範囲の中の `frame_transition_atomicity_tests.rs:L156-L172` が手で写している（説明は「同一の述語」）。`pub fn` に切り出して両方から呼ぶ形が候補（範囲の中の写しは 29 番と一緒に直せる） |
| コ | `let _ = parts.handle.join();` 9 か所（`crates/areka/src/boot_resolve_tests.rs:L483`・`L576`・`L757`・`L805`、`input_events/drag_test_support.rs:L283`、`install/names_tests.rs:L28`、`main_persist_wiring_seam_tests.rs:L85`・`L128`、`crates/areka-runtime/src/move_cue_apply_move_tests.rs:L256`） | 各ファイルの持ち主（`input_events/` は〔C23〕） | 記憶の書き手の panic を捨てる（20 番と同じ型） |
| サ | 本体の `capture_logs` の写し（`runtime_tests/spine.rs:L124`・`input_events/choice_drain.rs:L258`・`readme/readme_cue_tests.rs:L32`）と GPU の World の写し（`runtime_tests/spine.rs:L93-L106`） | 本体の `runtime_tests`（`audit-areka-app` の見込み）・`input_events`〔C23〕 | 元が `#[cfg(test)]` だけで本体から読めないための写し。本体の側で 1 か所にまとめる |
| シ | 起動の文脈 `BootContext` を組む補助 5 か所（`install/desk_tests.rs:L737`・`menu/ghost_frame_tests.rs:L43`・`readme/opener_submit_tests.rs:L31`・`session/ghost_switch_tests.rs:L87`・`session/reload_tests.rs:L68`） | `boot_config` の test-support（`menu/` は〔C23〕） | 写し |
| ス | `.kiro/specs/areka-test-wait-wording/brief.md:L13`・`L79` | `areka-test-wait-wording` | 古い道 `emo2_boot/frame_scale_notice_integration_tests.rs`（今は `runtime_tests/`）。109 番の扱いと合わせる |
| セ | `crates/areka-runtime/src/spine_wait.rs:L686-L687` | 範囲の外（`audit-emo2-boot-spine` は完了済み） | 14 番を直すと「2 つ取るのはこの 1 本だけ」の例外の注が要らなくなる（直しに要る変更として要件 3.2 で触るか、申し送るかは設計で） |

- 申し送りの書き方（brief に書き足すか、走っているセッションへ伝えるか）は要件 7.1・7.7・7.8 に従う。〔C23〕の件（ア〜カ）は C23 のあいだ触らない（議題 7）。

---

## 7. 並走の約束（要件 7・C23 の約束 ⑴〜⑹）に触れる指摘

- **`doc/COMPAT_ARCHITECTURE.md` §8**: 2 番の案 ⑴⑵ は §8 の L439 の行の書き換えを要る。§8 は `hit-id-pass`（末尾への追記）と `balloon-flip-ssp-review`（既にある別の行の書き換え）が触る（約束 ⑷）。行が別なので文字の衝突は起きにくいが、要件 7.3 は §8 に手を入れないと書く。案 ⑶ なら §8 に触れない。議題 2。
- **`crates/areka-placement/`**: 直しの本体が及ぶのは 49 番の案 ⑵（`same_monitors` の署名）と、6 節のア〜カ（申し送り）だけ。49 番は案 ⑴ なら範囲の中で済む。6 番の新しいテストは `areka_placement::reseed::reanchor_char_windows` と `reseed::test_support` を呼ぶだけで、ファイルには触らない。
- **`crates/areka-runtime/src/hit_region.rs`**: 触る指摘は 0 件（62 番は `frame_attach_tests.rs` の中だけ・37 番は `wiring.rs` の文だけ）。
- **本体の `input_events/`・`main.rs`・`ghost_session.rs`・`ghost_session_runtime.rs`・`app_exit.rs`・`menu/`**: 触る指摘は 0 件。17 番・33 番は `ghost_session.rs`・`main.rs` を読むだけ。`input-crate-split` の後も本体の箱のテスト（15・16・21 番）は `use areka_input as input_events;` の付け替えで字面のまま通る。
- **`Cargo.toml`・`Cargo.lock`**: 依存を足す指摘は 0 件（約束 ⑹・要件 6.6）。33・99 番の戻しも既にある dev-dependency で足りる見込み。99 番は test-support の口へ出すと `Cargo.toml` に触るので、`cfg(test)` の中に閉じる。
- **`crates/log-capture-kit/tests/file_length_guard_test.rs`**: 35 番が表から 1 行消して件数を 3→2 にする（範囲のファイルが表から出る＝約束 ⑸・要件 7.4 の中）。新しい支えのファイル（29 番ほか）は 1,000 行に収める。
- **`crates/log-capture-kit/src/event.rs`**: 触る指摘は 0 件。
- **後の spec の先回り**: 2 番は `window-resetwindowpos-hold`（`frame/move_hold.rs` の働き・C24）と同じファイルに触る。37 番は `balloon-text-visual-tree`（`frame/wiring.rs`）・3 番は同（`frame/attach.rs`・`scale_text.rs`）と同じファイルに触る。どちらもこの spec の着地を待つ側なので、この spec が先に直すのは約束の中。9・33・99 番の移しは `session-module-gather`（`runtime_tests/` の `frame*` と `session/ghost_quit*` のパスの直し）の対象を減らす方向。

---

## 8. 確かめた指摘

読み手の報告のうち、次の指摘はこの文書を書く前に根拠の行を読み直した。

- 1 番: `frame_text_scale_tests.rs:L170-L216`（L176・L195 は `.expect` を挟み、L206-L212 だけ挟まない）。
- 8 番: `frame_schedule_tests.rs:L36-L42`・`L100-L112`（`BOOT_REGISTRATION` の字面が同じファイルの L40 にある）。
- 11 番: `frame_chain_finalize_tests.rs:L74-L82`。
- 113 番: `frame/drain_resnap.rs:L98-L112`。字面の「取り出す上限」を読む所はツリーで `drain_resnap.rs` だけ（grep）。
- 2 番の §8: `doc/COMPAT_ARCHITECTURE.md:L439` は「## 8. 沈黙ルール対応表」の中。
- `frame.rs:L348` の `let _ = run_text_scale_phase(..)` は、返り値（拡大率を当てた scope の一覧・テストの観測用）を捨てているだけで、失敗を捨てていない＝指摘にしない。
- `frame/dpi.rs:L357-L358` の `.expect("DPI changed query validation should succeed")` は本番の毎フレームの道にあるが、Query だけの `SystemState` の検証は失敗する場合が無い（具体的な場合が無い）＝指摘にしない。
- 件 13 の結論は G2 と G6 の 2 つの読み手が別々に同じ並びを出した。

それ以外の指摘は、読み手が報告に書いた根拠の行による。走らせての確かめ（1 番の赤・4 番のあふれの赤・6 番の新しいテスト・件 4 の test-support の 1 回・33 番の移しが組めるか）は実装の段で行う。

---

## 9. 要件と今の姿の突き合わせ

| 要件 | 要るもの | 今の姿 | 差（Missing / Unknown / Constraint） |
|---|---|---|---|
| 1 | 範囲の測り直しと全件の audit | この文書 | 無し |
| 2 | 直す対象の確定と、直さずに残すものの裁定 | 候補 9 件（5 節） | Constraint: 開発者の裁定待ち |
| 3 | 114 件の直し・呼び手の直し・消す前の grep | 指摘ごとに場所と直し方がある | Missing: 直しそのもの。4 番は範囲の外の `target_map.rs` に及ぶ |
| 4 | バグと危ない所ごとのテスト | 1・2・3・4・6・113 番に足すテストを書いた | Missing: 1 番の赤・2 番の上限の檻・3 番の再試行（か error）の檻・4 番のあふれの赤・6 番の流れの檻 |
| 5 | 預かった 14 件 | 3 節 | Missing: 件 1〜12・14 の直し。件 13 は確かめ済み（テストは 6 番） |
| 6 | 決まり（1,000 行・記録の無い失敗・依存を足さない ほか） | 写しの直しは行を減らす。新しい支えのファイルは 1,000 行に収める | Constraint: 900 行台の 3 本に足すときは先に縮める（26・29 番） |
| 7 | 範囲の外への直しと C23 の約束 | 7 節 | Constraint: 2 番の §8・49 番の案 ⑵・6 節のア〜カ |
| 8 | 重い操作を回さない | 決定論のテストだけで確かめられる | Unknown: 2 番を ⑶ 以外にするなら SSP の測りが要るかもしれない（議題 2） |
| 9 | かけ直しと全体テスト | — | Missing: 実装の後 |

### 直し方の選び方

- **案 A（ファイルのまとまりごとに切る）**: 束（1.4 節の表）に沿ってタスクを切る。同じファイルを 2 つのタスクが触らないので、レビューと衝突が少ない。重さの違う指摘が 1 つのタスクに混ざる。
- **案 B（組ごとに切る）**: 必ず直す → 直したほうがよい → できれば、の順に切る。大事なものが先に着地するが、同じファイル（`frame_attach_tests.rs`・`frame_transition_branch_tests.rs`・支え）を何度も触るので、行番号のずれと衝突が増える。
- **案 C（混ぜる・推す）**: 1 番と、本番のコードの 2〜5 番、件 13 の 6 番を先に独立のタスクにする（それぞれ赤のテストから始める・要件 4.2）。次に支えの寄せ（22〜31 番＝ほかの指摘の土台になる）を 2〜3 本のタスクにまとめ、その後に移し（33・99 番）・分割（34・35 番）・残りをファイルのまとまりで切る。支えを先に寄せると、900 行台の 3 本と 35 番の分割が楽になる。

### 手間と危うさ

- 手間: **L**（1〜2 週間）。114 件のほとんどは S だが数が多い。支えの寄せ（26・27・29 番）は 10〜16 か所ずつ、33 番は 20 本余りのテストの移し、35 番は 1,271 行の分割。
- 危うさ: **中**。
  - 支え（`frame_test_support.rs`・`frame_shared_test_support.rs`）は本体のテストも使う公開の面で、寄せると両方の crate のテストに響く。
  - 2 番は振る舞いに触れる案があり、§8 と C23 の約束が絡む。
  - 33 番の移しは、本体の支えが `areka-runtime` の dev-dependency だけで組めるかを組んで確かめるまで確かでない。
  - どれも既にある型の延長で、新しい技術は要らない。

### タスクの数の見立て（要件 2.6）

| まとまり | 指摘 | タスク |
|---|---|---:|
| 落ちない表明（赤→直し） | 1・7 | 1 |
| 預かりの列 | 2 | 1 |
| 文字層の装着の失敗の道 | 3 | 1 |
| scope の上限（件 9・範囲の外の `target_map.rs`） | 4・ク | 1 |
| 死んだ import と許可（件 4）・可視性（件 5） | 5・114 | 1 |
| 件 13 の流れのテスト | 6・50 | 1 |
| 登録と終わる相の写しの檻 | 8・9・10 | 1 |
| 確定・装着・可視性・箱の空振りの檻 | 11〜16 | 2 |
| 再起動の印・待ちと join | 17〜20 | 1〜2 |
| 箱の檻の遅さ | 21 | 1 |
| 写しの寄せ（件 11・支え） | 22〜25・30・31 | 2 |
| 結線の手組み・共有の支え・鎖の支え | 26〜28 | 2 |
| 書き込みの補助と整地の寄せ | 29 | 1 |
| 一時フォルダ・本体から `areka-runtime` への移し | 32・33・99 | 2 |
| 整合待ちの重なりと分岐のテストの分割 | 34・35・94 | 1〜2 |
| 本番の説明と小さな形 | 36〜53・113 | 2 |
| `frame/` のテスト・`frame_*` のテストのできれば | 54〜98 | 2〜3 |
| 本体のテストのできれば | 100〜112 | 1 |
| かけ直しの audit と全体テスト（要件 9） | — | 1〜2 |
| **計** | | **24〜28** |

- 20 をはっきり越える。要件 2.6 と問い 1 の仮定（後の決め 2026-11-16 と前例の `audit-emo2-boot-spine` の 23 タスクに従い、1 本のまま進めて見立てを記録だけする）が要る（議題 1）。

### 設計へ持ち越す調べ（Research Needed）

- 3 番: 他人のバルーンで面 0 の `ShowSurface` が本当に失敗しうるか（資産を組む段で弾かれるか）。
- 14 番: 同じ World のまま前の窓へ装着し直せるか。
- 31 番: プロセスの寿命の MTA があるとき、テストの COM の初期化の行を消してよいか。
- 33 番: 本体の支え（`spawn_shown`・`PersistStore` の偽の半分）が `areka-runtime` の dev-dependency だけで組めるか。
- 49 番: 案 ⑴（`Monitor` の変化を持続の SystemState で見る）で、wintf の `monitor_systems` の書き方が Changed を正しく立てるか。
- 72 番: 完了した `areka-P0-dpi-transition-atomicity` の要件 7.5 が x64 の門の形を名指しているか。
- 件 4: `--features test-support` 付きの `--force-warn unused_imports` の 1 回。
- 2 番を ⑶ 以外の案にするなら: SSP が見えない窓への `\![move]` をどう扱うか（ukadoc で読めなければ SSP の MCP の測り）。

---

## 10. 要件の討議へ渡す議題（答えで作業が変わるもの）

1. **規模（問い 1・要件 2.6）**: 24〜28 タスクの見込み。推す＝1 本のまま進め、見立てを記録だけする（2026-11-16 の決め・前例 23 タスク）。
2. **2 番の預かりの列**: 案 ⑴（同じ scope の後の移動で前を捨てる）・⑵（上限で落とす）・⑶（集合にするだけ・振る舞いは変えない）。⑴⑵ は `window-work-area-clamp` の「取りこぼさない」の決めに触れ、§8 の行（C23 の 2 本が触る文書）の書き換えと、場合によって SSP の測りが要る。推す＝C23 の間は ⑶ で O(n²) を消し、列が伸びる件は後の `window-resetwindowpos-hold`（同じ `move_hold.rs` を触る・C24）の brief へ申し送る、か、開発者が ⑴ を裁定するなら §8 は C23 の着地の後に直す。
3. **4 番の上限の置き場**: 共通の関数を範囲の外の `target_map.rs` に置く（要件 3.2 の直しのための変更）。入口の `detect_scopes`（C23・`areka-placement`）は `extra-character-windows` の brief へ申し送る。推す＝この形。
4. **6 番（件 13・問い 4）**: 本番は書くと分かったが、範囲の中にそのフレームを通るテストが 0 本。推す＝足す（50 番の最適化の前の檻にもなる）。
5. **46 番（件 3）と要件 5.3 の文言**: 要件 5.3 の「前方一致なので漏れはせず」は `areka-runtime` の側では事実と違う。推す＝要件 5.3 の文を「こちらでマクロを呼ぶと出力先が `areka_runtime::…` になり、`wintf::ecs::window` の前方一致の絞り込みでは debug の受理の行が点かない。行の字面と出力先を wintf の 1 か所に保つ」に改める。
6. **92 番（件 7・問い 3）**: `--release` でテストを回す道は今は無い（3 節）。推す＝仮定どおり付ける。
7. **〔C23〕の申し送り（問い 5）**: 6 節のア〜カと 84 番の placement 側の文・49 番の案 ⑵ は C23 の間は触らない。推す＝仮定どおり、placement の分は持ち主の brief（`extra-character-windows`・placement の見直しの後の spec）へ書き足す。
8. **5 節の裁定の候補**: 46・55・64・67・68・72・80・94 番（と 2 番）を直すか残すか。推す＝55・72 は直す（触れないか未確認）、64・67・68・80・94 は直す（決めの狙いを保つ直し方がある：64 は実 GPU のテストが常に回る・67 は狙いを強める・68 は名を残して 1 本にまとめる・80 は陽性の対を 1 本に残す・94 は 3 水準を 1 本で回す）。どれも開発者の確認が要る。
9. **109 番の持ち主**: `areka-test-wait-wording` の brief が同じ行を持つ。推す＝この spec で直し、向こうの brief の行を「済み」に書き換える申し送りをする（向こうは未着手）。
10. **33・99 番の移し**: 本体から `areka-runtime` へテストを戻す（約 20 本）。`session-module-gather` の手間を減らす方向で、後の spec の先回りには当たらない。推す＝この spec で行う。

### 設計の段で決めること（要件の討議の分類 B）

要件の討議で、次は how の判断として設計（`/kiro-design`）へ回した。

- 3 番: 案 ⑴（文字の拡大率の相で登録し直す）か案 ⑵（再試行をやめて `error!`）か。他人のバルーンで面 0 の `ShowSurface` が失敗しうるかの調べと一緒に。
- 4 番: scope を表示対象の番号に直す共通の関数の形と置き場（範囲の外の `target_map.rs` に置く案・要件 3.2 の直しのための変更）。範囲の中の 6 か所と範囲の外の `balloon_visibility_phase.rs:L464` の寄せ方。
- 49 番: 案 ⑴（範囲の中で済む）を基に、wintf の `monitor_systems` が Changed を正しく立てるかの調べ。案 ⑵ は C23 の間 `areka-placement` に触るので取らない。
- 33・99 番: 本体の支えが `areka-runtime` の dev-dependency だけで組めるか、`cfg(test)` の中に閉じる形。
- タスクの切り方: 9 節の案 A・B・C（推す＝C）。900 行台の 3 本に足す前に縮める順（26・29 番）。
- セ: 14 番を直したときの範囲の外の `spine_wait.rs:L686-L687` の注を、直しに要る変更として触るか、申し送るか。
- 9 節「設計へ持ち越す調べ」の 14・31・72 番と件 4 の `--features test-support` の 1 回。72 番は、完了した `areka-P0-dpi-transition-atomicity` の要件 7.5 が「常時テストは x86 を用いず純 x64 の決定論テストとして走る」とだけ書き、見張りの形を名指していないことを要件の討議で確かめた（形を 1 つに寄せても決めには触れない）。

---

## 付録 A: 束の読み手の番号との対応

| 番号 | 束の番号 |
|---|---|
| 1 | G7-1 |
| 2 | G1-1 |
| 3 | G1-2 |
| 4 | G2-1 |
| 5 | G1-3 |
| 6 | G6 の件 13 の節・G2 の 6 節（この文書で番号を付けた） |
| 7 | G7-2 |
| 8 | G6-1・G6-2 |
| 9 | G9b-5（G2 の 5 節にも） |
| 10 | G6-3 |
| 11 | G5-2 |
| 12 | G5-3 |
| 13 | G4-7 |
| 14 | G9b-1 |
| 15 | G9b-3 |
| 16 | G9b-4 |
| 17 | G9a-1 |
| 18 | G8-1 |
| 19 | G8-2 |
| 20 | G8-3 |
| 21 | G9b-2 |
| 22 | G5-1・G4-5・G3-2 の一部 |
| 23 | G3-3 |
| 24 | G6-4 |
| 25 | G3-2（G2 の 5 節にも） |
| 26 | G4-2・G6-5（`frame_window_link_drain_tests` は G6 の 5 節） |
| 27 | G5-4・G4-3 |
| 28 | G5-5 |
| 29 | G4-1・G7-8 |
| 30 | G4-4・G9a-4 |
| 31 | G4-6・G9b-12 |
| 32 | G9a-3 |
| 33 | G9a-2・G9b-6 |
| 34 | G6-7 |
| 35 | G7-7 |
| 36 | G1-4・G6-14 |
| 37 | G1-5 |
| 38 | G1-6 |
| 39 | G1-7 |
| 40 | G1-8 |
| 41 | G1-9・G2-5・G7-6・G5-12・G4-13 の一部・G7-5 の一部 |
| 42 | G1-10 |
| 43 | G2-2・G5-6 |
| 44 | G2-3 |
| 45 | G2-4 |
| 46 | G2-6・G3-1 |
| 47 | G2-7・G6-12 |
| 48 | G2-8 |
| 49 | G2-9 |
| 50 | G2-10 |
| 51 | G2-11 |
| 52 | G2-12 |
| 53 | G2-13 |
| 54 | G3-4 |
| 55 | G3-5 |
| 56 | G3-6 |
| 57 | G3-7・G8-7・G6-15・G9b-7 |
| 58 | G3-8 |
| 59 | G3-9 |
| 60 | G3-10 |
| 61〜71 | G4-8〜G4-18 |
| 72 | G4-19・G5-10 |
| 73 | G4-20 |
| 74 | G4-21 |
| 75 | G5-7 |
| 76 | G5-8 |
| 77 | G5-9 |
| 78 | G5-11 |
| 79 | G5-13 |
| 80 | G5-14 |
| 81 | G5-15 |
| 82 | G5-16 |
| 83 | G6-6 |
| 84 | G7-5・G6-8 |
| 85 | G6-9 |
| 86 | G6-10 |
| 87 | G6-11 |
| 88 | G6-13 |
| 89 | G6-16 |
| 90 | G6-17 |
| 91 | G6-18 |
| 92 | G7-3 |
| 93 | G7-4 |
| 94 | G7-9 |
| 95 | G7-10 |
| 96 | G7-11 |
| 97 | G7-12 |
| 98 | G8-4 |
| 99 | G8-5 |
| 100 | G8-6 |
| 101 | G8-8・G9a-5・G9b の 5 節（`runtime_tests/frame.rs:L39`） |
| 102 | G8-9 |
| 103 | G9a-6 |
| 104 | G9a-7 |
| 105 | G9a-8 |
| 106 | G9a-9 |
| 107 | G9b-8 |
| 108 | G9b-9 |
| 109 | G9b-10 |
| 110 | G9b-11 |
| 111 | G9b-13 |
| 112 | G9b-14 |
| 113 | G3 の 5 節（この文書で番号を付けた） |
| 114 | 要件 5.5 の読み手（件 5） |

---

## 11. 設計の段の調べと決め（2026-11-20 `/kiro-spec-design`）

- 発見の型: 軽い調べ（既にある仕組みの延長・新しい技術も新しい依存も無い）。外の調べ（Web）は要らない。
- 読んだ手引き: `.claude/skills/kiro-spec-design/rules/` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、`.kiro/settings/templates/specs/design.md`、前例 `.kiro/specs/completed/audit-emo2-boot-spine/design.md`。ponytail の「いちばん小さい完全な変更」（同じ大きさなら端の場合に正しい方）を直し方の選び方に使った。
- 調べは 2 つの読み手（3・14 番／33・99・31 番）と、この段の grep と cargo 1 回で行った。

### 11.1 設計の段で決めること（10 節の分類 B）の調べの結果

| 件 | 調べ | 結果 | 決め（design.md） |
|---|---|---|---|
| 3 番 | `EmoPresenter::text_slot_view`（`crates/areka-emo-present/src/presenter/read.rs` L100-L115）は mount・applied・native_size が揃うときだけ `Some`。どれも `apply_show` が通ったときだけ書かれる（`presenter/show.rs` L326-L341）。面 0 の `apply_show` が早く戻るのは、装置が無い（装着の門が見るので届かない）・装置の失敗（装着のあいだに装置を失ったときだけ）・`SurfaceNotFound`（面 0 は必ず在るので届かない）・`EmptyComposition`／`ExtentTooLarge`（`Hide` へ縮退して Ok）。他人のバルーンで現実にありうるのは面 0 の絵が GPU の上限より大きい `ExtentTooLarge`。面 0 の絵が読めないバルーンは資産を組む段（`build_balloon_assets_with_limits`→`resolve_balloon_faces`）で `Err` になり、ゴーストは表示なしの縮退で起きる | 後のフレームで配置先が `Some` になる道はある（面の切り替え `after_drain`・装置の失敗からの戻り）。今は `run_text_scale_phase` が未登録の actor を `refresh_actor_scale` の debug で読み飛ばすだけで、文字は永久に出ない | 案 ⑴（文字の拡大率の相で繋ぎ直す・`RuntimeWiring.unconnected_text` に scope と背景色を控える）。案 ⑵ は小さいが端の場合（装着の時だけの装置の失敗・面 0 だけ組めない）に正しくない。問い 1 |
| 4 番 | 範囲の中で scope を `u32` に直す所は `frame/{attach,dpi,scale_text,drain_resnap×2,move_shift,switch,zorder_drain}.rs` の 8 か所（研究の 4 番の 7 か所に `scale_text.rs` L169 を足した）。設計の検証で `frame/attach.rs` L458 の `filter_map` を見つけ、9 か所になった | 範囲の外の `balloon_visibility_phase.rs` L464 は `box_scopes`（`u32`）との突き合わせで、表示対象の番号は作らない | `target_map.rs` に `MAX_SCOPE` と `window_scope(usize) -> Option<u32>` を置き、`scope_of` も同じ定数を使う。8 か所で使う。ク・アは渡す。問い 2 |
| 49 番 | wintf の `apply_monitor_snapshot`（`crates/wintf/src/ecs/layout/systems/monitor_systems.rs` L299-L412）は、値が変わったモニターだけ `get_mut` で書き（`differs_in_value`）、新しいモニターは `spawn`、消えたモニターは `despawn` する。areka は `clear_trackers` を呼ばない（grep）ので、消去は `RemovedComponents` でなく数で見るのが確か | Changed は値の変化と追加で立ち、消去は数で見える | 案 ⑴ を World の資源 `MonitorWatch`（持続の `SystemState` と前の数）で。`sync_monitor_snapshot` の署名は変えない（ハーネスと字面の確かめがそのまま）。問い 3 |
| 33・99 番 | 移すテストが使うのは `bevy_ecs`・`wintf`・`areka_placement`・`areka_sylphya`・`areka_actor`・`areka_ghost`・`windows`（`areka-runtime` の普通の依存）と `log_capture_kit`（dev）だけ。本体の項目は `crate::insert_persist_wiring`（`PersistWiring` は `areka_placement::persist` の `pub` の型・欄も `pub`）と、remembered ⑵ の `crate::app_exit::close_windows_for_restart` だけ。`MinimalGhost` を `temp-path-kit`（dev）で包むと test-support に置けない | `Cargo.toml` を変えずに組める | `PersistStore` ほかを `frame::persist_test_support`（test-support）へ移し、`wire` は `PersistWiring` を直に置く。本体には `MinimalGhost` だけを残す。99 番は絞り込みをテストの中に書いて移す。問い 6 |
| 14 番 | 本番の `switch_to` は同じ World のまま `close_windows_for_restart` で窓を畳み、窓を開け直し、新しい `RuntimeWiring` を入れる。同じ窓に新しい presenter を装着し直すと前の presenter の子の entity が残る。前例 `frame_chain_finalize_restart_tests.rs` L90（`close_windows_for_restart` → `spawn_resnap_windows`）。`spawn_resnap_windows` は `dpi_world` の `DPI(96)` と配置の component を付けない | 同じ World で組める（付け足しが要る） | 本番と同じ畳み方で切り替える。GPU の許可は 1 つ |
| セ | `spine_wait.rs` L683-L687 は「2 つ取るテストは `new_ghost_holdings_start_with_a_fresh_ledger_after_a_switch` の 1 本だけ」。ほかに 2 つ取るのは自分の数え（`GpuSlots::new(2)`）の `spine_wait_tests.rs` だけで、`GPU_SLOTS` からは取らない | 14 番の直しで文が事実でなくなる | 直しに要る変更として 2 行を直す（範囲の外・持ち主は完了済み） |
| 31 番 | 両方のテストのバイナリにプロセスの寿命の MTA がある（`crash_report_test_support.rs` L126-L153 の `.CRT$XCU` の `CoIncrementMTAUsage`・本体の `runtime_tests/mod.rs` L10-L17） | 各所の `let _ = CoInitializeEx(..)` は要らない | `frame_test_support::gpu_on` を置き、範囲の中の 3 か所から呼ぶ。本体の `runtime_tests/spine.rs` の写しは範囲の外（サ） |
| 72 番 | 完了した `areka-P0-dpi-transition-atomicity` の要件 7.5 は形を名指さない（要件の討議で確かめ済み） | どちらへ寄せても決めに触れない | 宣言の cfg へ寄せ、支えの `const` の表明と字面の見張りを消す。問い 4 |
| 件 4 | `cargo rustc -p areka-runtime --lib --features test-support -j 4 -- --force-warn unused_imports` を 1 回回した（成功・16 件・全部 `frame.rs`・`wiring.rs` は 0 件） | 3 つのビルドを突き合わせると、どのビルドでも使わない名は 20、テストでだけ使う名は 8（`Entity`・`SystemState`・`PlacementRoute`・`connect_balloon_text`・`GhostWindowClass`・`classify_ghost_window`・`reproject_char_window_at_current_size`・`resnap_from_sizes`）、テストと test-support で使う名は 16（`RefCell`・`Rc`・`EmoPresenter`・`PresentCommand`・`TargetId`・`TextLayerRuntime`・`present_frame`・`ActorKey`・`GraphicsCore`・`WucGraphicsResource`・`GhostWindows`・`dpi_phase_with`・`PhysicalSizeSource`・`resnap_with`・`finalize_chain_once_with`・`realign_chain_once_with_source`） | 消す・`#[cfg(test)] use`・`#[cfg(any(test, feature = "test-support"))] use` の 3 種に分け、許可を 0 本にする（`wiring.rs` の 2 本も cfg で）。3 節の件 4 の表の「テストでだけ」のうち test-support でも使う名は、`cfg(test)` だけにすると test-support のビルドが壊れるので、この 3 種で扱う |
| タスクの切り方 | 9 節の案 A・B・C | — | 案 C で 27 本（design.md の「タスクの束と順」）。900 行台の 3 本は、足す前に縮める順（13 番の数行のほかは T14・T16 の後） |
| ケ | `placement_tests::transition_judge::summarize` と `TransitionSummary.path_a_writes`・`sync_stage_writes` は `pub` | 範囲の中の写し（`frame_transition_atomicity_tests.rs` の `path_a_write_lines`）は、範囲の外を変えずに `summarize` を呼ぶ形にできる | 範囲の中で直す（T16）。渡す件は残らない |
| 63・64 番 | 消すテストを範囲の外の説明が名指す（`move_cue_apply_move_tests.rs` L248・`assets.rs` L83） | 消すと説明が宙に浮く（直しで壊れる所） | この spec の直しの中で 1 行ずつ直す（要件 3.2・3.3）。キの申し送りから 63 番の分を外す。問い 7 |

### 11.2 合成の 3 つの見方（design-synthesis）

- 一般化: 窓の側の scope の上限（4 番）は台本の側（`scope_of`）と同じ問題なので、定数 1 つと入口の関数 2 つを同じファイルに置いた。写しの寄せ（22〜31 番）は「両方の crate が使う補助は `areka-runtime` の test-support に 1 つ・dev-dependency を引く補助は `cfg(test)`」の 1 つの決まりで置き場を決めた。
- 作るか使うか: 待ち（`wait_until`・`join_bounded`・`wait_recv`）・一時フォルダ（`TempPath`）・偽の HWND（`give_fake_handles`）・記録の捕捉（`capture_logs`）・本文の走査（`code_only`）・判定器（`summarize`）は既にある部品を使う。新しく作る部品は `window_scope`・`gpu_on`・`MonitorWatch`・`unconnected_text`・結線の補助 1 つ・寄せ先の支え 2 本だけ。
- 削り: 2 番は上限や捨てる案を取らず集合にするだけ（確定 2）。49 番は `areka-placement` の署名を変えない案 ⑴。113 番は `Receiver` を覗けないので条件を変えず文だけ。9 番はファイルを移さない。

### 11.3 危うさ

- 3 番の縛るテストは、装着の相の中の控えの 1 行を通さない（面 0 を組めないバルーンを決定論で作れない）。控えの後の繋ぎ直しと、`None` で `false` が返ることは別々に縛る。
- 49 番で、資源 `MonitorSnapshot` を手で置き替えて同期に上書きさせるテストがあれば、モニターの component を書き替える形に改める（本数は実装で記録する）。
- 33 番の移しは組んで確かめるまで確かでない（調べでは依存の範囲で組める）。組めない所が出たら、`Cargo.toml` に触らずに済む置き方を探し、無ければ要件 6.7 の議題にする。

---

## 12. 預かった件と申し送りの記録（2026-11-21・T25・要件 5.8・5.14・5.17・7.7・7.8・7.10）

### 12.1 件 1〜14 の片づけ

出どころは 3 節の表のとおり。「直した場所」は tasks.md のタスクの番号（実装メモの記録に合わせた）。

| 件 | 要件 | 出どころ | 指摘 | 直した場所 | 片づけ |
|---|---|---|---|---|---|
| 1 | 5.1 | `runtime-module-untangle` | 37 | 5.1（T20） | 結線の古い説明を直した |
| 2 | 5.2 | `audit-wintf-window` 件 1 | 93 | 5.4（T23） | 行番号で指していた所を名前で指す形にした |
| 3 | 5.3 | `audit-wintf-window` 件 2 | 46 | 5.1（T20） | 「前方一致なので漏れはせず」の文言ごと直した |
| 4 | 5.4 | `runtime-crate-split` ⑵ | 5 | 1.5（T5） | 使わない import を 0 件にした。`--force-warn unused_imports` の警告は 3 つの組（lib・lib＋test-support・test）とも 0 件 |
| 5 | 5.5 | `runtime-crate-split` ⑶ | 114 | 1.5（T5） | 公開の面を狭めた（私有に 3・`pub(super)` に 4） |
| 5 続き | 5.6 | 同 ⑶ 案 C | 57 | 2.1・5.2・5.5 | 同じ名前のテストは両側で 0 件。残った見出しの写しだけ直した |
| 6 | 5.7 | `runtime-crate-split` ⑷ | 43 | 5.1（T20） | 説明の誤りを直した |
| 7 | 5.8 | 同 ⑷ | 92 | 5.4（T23） | 下の 12.2 |
| 8 | 5.9 | 同 ⑷ | 40 | 5.1（T20） | 説明を直した |
| 9 | 5.10 | `audit-emo2-boot-spine` ア | 4 | 1.4（T4） | scope の上限を 1 つの関数にして範囲の中の 9 か所を通した |
| 10 | 5.11 | 同 イ | 99 | 4.2（T18） | 本体から `areka-runtime` へ戻した（依存は足していない） |
| 11 | 5.12 | 同 オと完了の時の棚卸 | 22・23・24 | 3.1（T13） | `capture_logs` の写しを既にある支えへ寄せた |
| 12 | 5.13 | 同 完了の時の棚卸 | 56 | 5.2（T21） | 設計（要件 5.13）のとおり `areka-test-binary-stability` の名をコメントに書いた。CLAUDE.md のコメントの決まり（spec の名を書かない）とは食い違う。7.1 の audit で見る |
| 13 | 5.14・5.15 | `audit-placement-follow-residue` | 6 | 1.6（T6） | **確かめは済み（要件 5.14・5.17）**: 本番は札の外れるフレームに、入れ直した揃え方で書く（3 節「件 13 の確かめ」）。範囲の中にそのフレームを通る決定論のテストが無かったので 6 番として新しく立てた（5.15） |
| 14 | 5.16 | `audit-areka-app` の brief の件 2〜4・9 | 41・44・45・75・84 | 4.3・5.1・5.3・5.4（T19・T20・T22・T23） | `atom` の書き方・古い数・古い名を直した |

### 12.2 `--release` で合否の変わる 1 本（件 7・92 番・要件 5.8）

`frame::work_area_resnap_hold_tests::the_invariant_watch_still_fires_for_a_write_that_bypasses_the_sites`

- 見張りは `debug_assert!` なので `--release` では鳴らない。前は `--release` で赤、今は `#[cfg_attr(not(debug_assertions), ignore = "debug_assert! に依る")]` を付けて ignored になる（範囲の外の同じ形の 3 本と同じ書き方）。
- `--release` でテストを回す道は無い（`tools/test-all.ps1` は debug・release の CI はテストを回さない）ので、全体テストの結果は変わらない。
- 87 番では検知段の同寸 skip を消しても緑のまま（`resize_window_to` のべき等が吸う）なので、説明は「同じ寸を何度渡しても窓へ書かない」に弱めた。

### 12.3 規模と直さずに残す指摘

- 規模の見立ては **27 タスク**（要件 2.6・設計の見立ては 24〜28）。1 本のまま進めた（前例 `audit-emo2-boot-spine` の 23 タスク）。分けない。
- 討議で決めた**直さずに残す指摘は 0 件**（確定 7）。裁定に近い 8 件（2・46・55・67・68・72・80・94）と先送りの印の 1 件（64）も、裁定の中身には触れない形で直した。

### 12.4 範囲の外の件の回し先（要件 7.8・7.10）

どの持ち主も、書く前に `.kiro/specs/completed/` の下に無いことを確かめた（2026-11-21）。

| 件 | 回し先 | 書いた中身 |
|---|---|---|
| 2 番の列が伸び続ける件 | `window-resetwindowpos-hold` | 面を出さない窓への `\![move]` が預かられ続ける |
| ア | `extra-character-windows` | `detect_scopes` に `i32::MAX` の上限・窓の側は `target_map::window_scope` を使う |
| イ〜カ | `test-crash-report-merge` | `areka-placement` の古い注と写し（下の 4.1 の注も同じ所へ） |
| キ・ク・コ（`areka-runtime` の分） | `audit-emo2-boot-switch` | `move_cue*` の偽の HWND の輪の写し・`balloon_visibility_phase.rs` の `marker.scope as u32`・`move_cue_apply_move_sticky_tests.rs:256`・`move_cue_place_tests.rs:395` の `let _ = parts.handle.join()`（6 節コの `move_cue_apply_move_tests.rs:L256` は今はこの道） |
| オ（本体の分）・コ（本体の 7 か所）・サ・シ | `audit-areka-app` | `code_only` の写し・`let _ = parts.handle.join()`（6 節の「9 か所」は数え直して本体 8・`areka-runtime` 2。本体のうち `input_events/drag_test_support.rs:283` は `audit-areka-input-menu` の持ち物と brief に書いた）・本体の `capture_logs` と GPU の World の写し・`BootContext` の補助 |
| ス | `areka-test-wait-wording` | 5.5 で書き換え済み（`frame_scale_notice_integration_tests.rs` の古い道は新しい道へ・済みの印あり）。件は足さない |
| ケ・セ | 回さない | 範囲の中で直した（ケ＝3.5・セ＝3.2） |

### 12.5 実装で見つかった申し送り

- **4.1**: `crates/areka-placement/src/chain_rule_tests.rs` の 5 行目の説明が、移す前の置き場（本体の `frame_chain_rule_tests.rs`）を指したまま。今は `crates/areka-runtime/src/frame_chain_rule_tests.rs`。`areka-placement` は C23 で触らないので、`test-crash-report-merge` の brief へ件として足した（イ〜カと同じ種類の古い注）。
- **4.2**: `doc/COMPAT_ARCHITECTURE.md` §8（228 行）が `log_ghost_falls_back_to_dash_and_raise_reports_normal_band` を本体の `crates/areka/src/runtime_tests/frame/window_state_tests.rs` と書いている。今は `crates/areka-runtime/src/frame/window_state_tests.rs`。**持ち主を決められなかった**（下の 12.6）。

### 12.6 申し送り先の決まっていない件（完了時にその場で直した）

- `doc/COMPAT_ARCHITECTURE.md` §8 の 228 行目の、上の 4.2 のパス 1 か所。
- `doc/COMPAT_ARCHITECTURE.md` の 213 行目付近（⑴ の欄）の、消したテストのあるファイルの名 1 か所（13.7）。
  - `doc-compat-architecture-rename` は「中身を変えない・変えてよいのは改名で食い違う自分の名前だけ」で当たらない。`audit-areka-app` 等の見直しの持ち物は §8 の文書ではない。C23 の `hit-id-pass`（§8 の末尾への追記）と `balloon-flip-ssp-review`（別の行の書き換え）は違う行を触るので、渡さない。
  - spec は作っていない。次の棚卸（`/kiro-next`）で持ち主を決めるか、§8 に触る次の spec が 228 行目も直す。

### 12.7 C23 のほかの 3 本へ申し送る件（要件 7.7）

C23 のほかの 3 本は `hit-id-pass`・`balloon-flip-ssp-review`・`input-crate-split`。申し送る件は **0 件のまま**。12.5 の 2 つは、どれもこの 3 本の触るファイルではない（`areka-placement/src/chain_rule_tests.rs` は 3 本とも触らず、§8 の 228 行目は `hit-id-pass` の追記・`balloon-flip-ssp-review` の別の行の書き換えと重ならない）。

### 12.8 設計から外れた所（あとの audit で「ずれ」と数えない）

- 1.7: 同期のテストで、作り直しの後にもう一度 `SystemState::get` を呼ぶ（自分の `insert_resource` を次の呼び出しの入れ直しと数えないため）。
- 3.5: `BALLOON_SPAWN_SIZE`・`BASE_OFFSETS`・`SCALE_120` は新しい `cfg(test)` の支えでなく `frame_shared_test_support.rs` に置いた（`feature = "test-support"` でも組まれる `resnap_placements` が読むため）。
- 4.1: clamp ⑴⑻ の共通の配置は `persist_test_support::left_edge` に 1 つ置いた。移したテストは 22 本（規則 15・揃え 6・clamp ⑴ 1・設計の「規則 13 本」は数え違い）。
- 5.1・5.3・5.4: 41 番の「atom を spec の名で書く」は、spec の名を足すのでなく、spec の ID・要件番号・設計の部品の記号（C2・C5・C7・D15）の参照だけの行を消す形で読んだ。ログの文字列の中に残る ID は触らない（ログの文を変えるのは 3・4・113 番だけ）。
- 5.5: 109 番は `Progress::Unknown` にしても目印の無い待ちのままなので、同じ受け箱へ `KanadeMsg::Close` を送り `join_bounded` で待つ柵にした（打ち切りは［止まった］）。設計の 109 番の行と直し方の欄を実装に合わせて改めた。
- 5.2 の 56 番（spec の名をコメントに書く）と、`確定台帳` の参照が 17 ファイルの外に残ること、`attach.rs`・`frame.rs`・`left_edge::placements` に要件番号・task 番号が少し残ることは、7.1 のかけ直しの audit で見る。

## 13. かけ直しの結果（2026-11-21・T26・要件 1.1〜1.8・9.1〜9.3・9.6〜9.8）

### 13.1 測り直し（要件 1.2）

- 作業のブランチの HEAD は `b642fc15`。`git fetch` の後の `origin/main`（`aff8f397`）は HEAD の祖先で、main は取り込み済み。
- 物差しは ponytail 5.3.3（`~/.claude/plugins/installed_plugins.json` の `ponytail@ponytail`・user scope・`lastUpdated` 2026-11-06）。`skills/ponytail-audit/SKILL.md` を読み、上限 20 件と「読まなかった所」を外して当てた（1.3 節と同じ作法・7 つの決まりも同じ）。
- CLAUDE.md の `## Comments` の決まり（spec の名・要件番号・task 番号・設計の部品の記号・変わった経緯を書かない）は、この spec が足した行か書き換えた行（`origin/main` との差分）だけを指摘に数えた。`origin/main` のまま残る行は、事実と違わない限り数えない（設計の物差し「この spec が触らないコメントは掃除しない」）。
- 「変わった経緯」の物差し（5 回目で 1 本に決め、1〜5 回目の全部に当てた）。この spec が足した行か書き換えた行では、次の 4 つを指摘に数える。
  - ⑴ 直す前と後を比べる語（「是正前は赤・是正後は緑」「是正前の赤」「当初…差し替えた」「…していた・抜けていた」）。今の形の文（「待ちが無いと 2 段書込になる」「無いと赤になる」）に直す。
  - ⑵ 日付（「実測 2026-08-28」「2026-08-28 実装時是正」）。測って分かった事実（何を潰すと何が赤になるか）は理由として残し、日付だけ消す。
  - ⑶ だれが・どの段で（「レビューで実測」「レビューが名指しした」「開発中に一度」「先行仕様から引き継いだ」「本仕様の新規コード側」）。事実だけを今の形で残す。
  - ⑷ 日付つきの裁定（「開発者裁定 2026-08-27」）。「開発者の裁定」と日付なしで残す。条件は、同じ段落が理由（なぜ塞がないか・何を避ける判断か）を自分で述べていること。述べていなければ理由そのものを書く。今の 4 か所（`frame/balloon_offset_follow.rs`・`frame_balloon_offset_keyword_gate_tests.rs` の 2 か所・`frame_transition_branch_tests.rs`）はどれも理由を述べているので、日付だけを消した。
  - 数えないもの: `origin/main` のまま残る行の同じ語（「# ここが押さえる是正」の見出し・「是正前は赤・是正後は緑」ほか）、測りの中身を言う「実測の形」「実測:」、表明の文字列の中の要件番号（`origin/main` のまま）。書き換えた文が受けていて宙に浮く `origin/main` の行（`frame_transition_branch_tests.rs` の表の見出し「是正」）だけは一緒に直した。
- 範囲は 1.1 節と同じ置き場の決め方で、移したファイルと新しいファイルは移した先で数えた。

| 時点 | ファイル | 行 | 本番の名前のファイル | `areka-runtime` のテストと支え | 本体のテスト |
|---|---:|---:|---:|---:|---:|
| 着手のとき（1.1 節） | 103 | 34,968 | 18・6,332 | 49・19,135 | 36・9,501 |
| かけ直しの前（HEAD `b642fc15`） | 109 | 33,276 | 18・6,186 | 57・19,024 | 34・8,066 |
| かけ直しの後（4 回目の後） | 108 | 33,222 | 18・6,173 | 57・19,003 | 33・8,046 |
| かけ直しの後（5 回目の後） | 108 | 33,221 | 18・6,173 | 57・19,002 | 33・8,046 |

- 1,000 行を越えるファイルは 0 本（いちばん長いのは `frame_test_support.rs` 951 行）。

### 13.2 回ごとの件数

| 回 | 読んだ所 | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---:|---:|---:|---:|
| 1 | 範囲の 109 ファイルを全部、全行（9 の束） | 0 | 12 | 32 | 44 |
| 2 | 1 回目の直しが触った 37 ファイルを全行（3 の束）と、直しが及ぶ所 | 0 | 4 | 8 | 12 |
| 3 | 2 回目の直しが触った 12 ファイルを全行と、直しが及ぶ所 | 0 | 0 | 3 | 3 |
| 4 | 3 回目の直しが触った 2 ファイルを全行と、名指す先 | 0 | 0 | 0 | 0（物差しが 1〜3 回目とそろっていなかった・下） |
| 5 | `origin/main` との差分の足した行の全部を、13.1 の「変わった経緯」の物差しと番号の類の語で洗い、当たった 9 ファイルを全行 | 0 | 1 | 13 | 14 |
| 確かめ | 5 回目の直しが触った 9 ファイルを全行 | 0 | 0 | 0 | **0** |

- 4 回目の 0 件は、2 回目（B-3）と 1 回目（G6-5・G7-3）が指摘に数えた「是正前／是正後」と日付つきの注を、触った 2 ファイルの外では数えていなかった。レビューの差し戻しを受けて物差しを 13.1 に 1 本で書き、5 回目で差分の足した行の全部に当て直した。
- 5 回目の 14 件はどれもコメントだけの直しで、「必ず直す」は 0 件・本番の振る舞いに関わる指摘は 0 件。要件 9.6 のとおり、5 回目で「必ず直す」と本番の振る舞いの指摘が 0 件なので、かけ直しはここで区切ってよい。そのうえで 5 回目の直しが触った 9 ファイルを全行読み直し、同じ物差しで 0 件を確かめた（要件 9.2・9.3）。
- 同じ所から「必ず直す」が出続けた所は無い（要件 9.7 に当たらない）。
- 2・3 回目の指摘の多くは前の回の直しが呼んだもの（13.4・要件 9.8）。

### 13.3 1 回目（44 件）と直し

束 G1〜G9 は 1.4 節と同じ分け方を今の置き場に合わせたもの。番号は読み手の番号のまま。

**直したほうがよい（12）**

| 番号 | 場所 | 指摘 | 直し |
|---|---|---|---|
| G1-1 | `frame/dpi.rs` | 書き換えた説明に `S2 是正`・`診断レポート §1.2` が残る | 括弧ごと消した |
| G1-2 | `frame/balloon_offset_follow.rs` | 見出しに「2026-08-28 実装時是正」 | 消した |
| G1-3 | `frame/move_hold.rs` | 「件 5 の測り（MoveMeasure）」が残る | 理由の文だけにした |
| G3-1 | `frame/drain_resnap_tests.rs` | 説明に `areka-test-binary-stability` の `load-repro.md` ⑹（件 12・56 番） | 理由「自前の締切は負荷の下で赤になる」だけにした |
| G5-1 | `frame_chain_realign_arm_tests.rs`・`frame/dpi.rs` | 武装の条件の `wrote &&` が窓の寸の前後比べと重なって何も足さず、檻の ⑴ が落ちなくなっていた（説明は「落ちる」と言う） | `frame/dpi.rs` の条件から `wrote &&` を消し（振る舞いは同じ）、本番と檻の説明を 2 つの条件に直した |
| G6-1 | `frame_dpi_sync_hold_tests.rs` | 頭の行に「設計 C5・要件 5.8…」 | 消した |
| G6-2 | `frame_drain_text_tests.rs` | 書き換えた行に「7.4 の」 | 消した。同じ塊の task 番号も持ち主の名に替えた |
| G7-1 | `frame_transition_atomicity_tests.rs`・`frame_dpi_reproject_tests.rs` | 消したテストのあるファイルを名指す | `frame_transition_branch_tests.rs` の `a_work_area_only_change_writes_once` を指すようにした |
| G7-2 | `frame_transition_branch_tests.rs` | 「群 A」が指す先が無い | ファイルの名を書いた |
| G7-3 | `frame_transition_atomicity_tests.rs`・`frame_work_area_resnap_hold_tests.rs` | 書き換えた行に変わった経緯（「主張の書き換え」「当初…差し替えた」） | 今の理由だけにした |
| G9-1 | `runtime_tests/frame_schedule_tests.rs` | `ghost_quit_system.before(frame_system)` が抜けても落ちるテストが無い（止まったゴーストの毎フレームの相が 1 回余分に走る） | テスト `ghost_quit_system_runs_before_the_frame_system` を足した（曖昧さを誤りにして組み、実行順を見る）。`.before` を消すと赤・`.after` にすると赤・戻すと緑を確かめた |
| G9-2 | `runtime_tests/frame_chain_finalize_clamp_tests.rs` | 書き換えた頭の行に spec の名・task 番号・要件番号。移したテストの中身も書いたまま | G9-4 でファイルごと移した |

**できれば（32）**

| 番号 | 場所 | 直し |
|---|---|---|
| G1-4 | `frame/drain_resnap.rs` | 割れた文「直接呼ぶ／。」をつないだ |
| G1-5 | `frame.rs` | `mod` の上のモジュールの説明の言い直し 7 行を消し、同じ注 6 回を 1 回に、cfg の言い直しを消した |
| G2-1 | `frame/scale_text.rs` | 「7.1 の権威」「7.1 の未登録 skip」を `refresh_actor_scale` に替えた |
| G2-2 | `frame/work_area_sync.rs` | `let _ = watch.params.get(world)` の失敗を `warn!` 1 行で残す（今の引数では起きない道） |
| G3-2 | `frame/status_report_tests.rs` | 「台帳も動かさない」を「送った組（`last_sent`）も動かさない」に |
| G3-3 | `runtime_tests/frame/switch_stay_on_top_tests.rs` | 私有の `all_windows` を消し、`window_state::test_support::all_windows` を使う |
| G4-1 | `frame_transition_test_support.rs` | `attach_material` は `settle_at` の後に呼ぶ理由を足した |
| G4-2 | `frame_balloon_runtime_test_support.rs` | 途中の panic で GPU の許可が装置より先に捨てられる順を直した |
| G4-3 | `frame_balloon_offset_follow_tests.rs` | `anchor_enumeration_is_exhaustive` の腕が表の添字 `ANCHORS[n]` を返す形にした（67 番の「網羅の檻を強める」を保つ） |
| G4-4 | `frame_test_support.rs`・`frame_balloon_runtime_test_support.rs` | 書き換えた行の「診断レポート」「（レビューで確認）」「「4 点すべて」の 2 つ目」「中身を変えずに分けた」を消した |
| G4-5 | `frame_test_support.rs` | `synth_assets` の説明の無い欄 `model` と、事実と違う 2 行を消した |
| G4-6 | `frame_test_support.rs` | `FrameHarness` の「3 つの源」を表の 4 つに合わせた |
| G4-7 | `frame_balloon_offset_{follow,keyword_gate,roundtrip}_tests.rs` | 写しの `pos_of`・`size_of`／`size_px`・`ideal_alignment`／`expected_keyword_offset` を `transition_test_support` の `window_pos_of`・`window_size_of`・`keyword_alignment` 1 組にした |
| G4-8 | `frame_balloon_offset_converge_tests.rs` | 窓の種類の字面 `"char"`・`"balloon"` を `WindowKind::…as_str()` に |
| G4-9 | `frame_attach_tests.rs` | 読めないファイルを黙って飛ばす `unwrap_or_default()` を panic に |
| G5-2 | `frame_chain_realign_arm_tests.rs` | 消えた定数名 `HALVED_*` を `LOW_SIZE_*` に・「（1.7）」を消した |
| G5-3 | `frame_chain_realign_tests.rs` | 陽性の対の説明をコードどおり（隣に付け直した窓は詰まる）に戻した |
| G5-4 | `frame_diag_route_tests.rs` | 書き換えた行の「Flow 3 手順 5」の引用を消した |
| G5-5 | `frame_chain_finalize_remembered_tests.rs` | よそのファイルの番号 ⑶⑷⑸ を消した |
| G6-3 | `frame_dpi_sync_hold_tests.rs` | 見送り点 5 つのうち `site=pattern-move` の持ち主を書いた（2・3 回目で正した） |
| G6-4 | `frame_dpi_reproject_tests.rs` | 割れた見出し「⚠ 区分＝」を 1 文にした |
| G6-5 | `frame_dpi_reproject_tests.rs` | 書き換えた行の変わった経緯（無視属性を外した・拡充・主張の書き換え・さらに 1 段）を今の形に |
| G6-6 | `frame_resnap_tests.rs` | 新しい零件のテストに、違う寸なら同じ witness が書込を拾う陽性の対を足した |
| G7-4 | `frame_transition_branch_tests.rs` | 拠り所の消えた「5.5」を「現状維持（源も窓も触らない）」に |
| G7-5 | `frame_transition_branch_tests.rs` | 兄弟が叩く駆動口を「`FrameHarness` の駆動口（`run_placement_phases` ほか）」に |
| G7-6 | `frame_work_area_sync_tests.rs` | 写しの `sync_warnings` の数えを `warnings_containing` に |
| G7-7 | `frame_work_area_sync_tests.rs` | 手書きの絞り込みを `writes_for` に |
| G8-1 | `runtime_tests/frame/status_report{_stage,}_tests.rs` | 消えた凡例の ⑵⑶⑷ を節の見出しから消した |
| G8-2 | `runtime_tests/frame/switch_finish_tests.rs` | 前のバルーンの名を `kaku` に（コードどおり） |
| G9-3 | `runtime_tests/frame_balloon_timeout_notice_e2e_tests.rs` | 番号を付け直した親の節「⑸」を「⑷」に |
| G9-4 | `runtime_tests/frame_chain_finalize_clamp_tests.rs` | 1 本だけ残ったテスト `save_at_finalize_writes_clamped_partner_position` を `areka-runtime` の `frame_chain_finalize_tests.rs` の双子の隣へ移し、ファイルと `#[path]` の宣言を消した（参照は `.kiro` の完了 spec の記録だけ） |
| G3-4 | `frame/drain_resnap_tests.rs` の `let _ = done_tx.send(applied)` | **指摘に当たらない**として閉じた。送りが失敗するのは受け手が諦めてテストが既に赤のときだけで、具体の間違いの場合が無い（SKILL.md 3 節「具体の場合が無ければ指摘でない」） |

- 読み手が見て指摘に当たらないとしたもの: `frame_persist_test_support.rs` の `Drop for PersistStore` の `let _ = handle.join()`（`ActorHandle::join` が `warn!` を残してから返す・`origin/main` のまま）、`frame/work_area_sync.rs`・`frame_work_area_sync_tests.rs` の「確定台帳」（`origin/main` のまま・事実と合う）、`left_edge::placements` の「（要件 5.2）」（既に無い）、`attach.rs`・`drain_resnap.rs`・`scale_text.rs` のログの文字列の中の ID（ログの文は外への約束で、変えない）。
- 5.2 の 56 番（件 12）で設計どおりコメントに書いた spec の名は、CLAUDE.md の決まりに合わせて消した（G3-1）。design.md の 56 番の行は「spec の名を書く」のままなので、完了の棚卸で揃える。

### 13.4 2 回目（12 件）・3 回目（3 件）・5 回目（14 件）と直し

前の回の直しが呼んだもの（連鎖・要件 9.8）に「連鎖」と書いた。どれもコメントと檻の説明・テストの補助の直しで、本番の振る舞いに関わるものは無い。

| 回・番号 | 場所 | 指摘 | 直し |
|---|---|---|---|
| 2・A-1（連鎖） | `frame_balloon_offset_follow_tests.rs` | 書き換えた説明が、表の重なりや取りこぼしまで捕まえると言い過ぎる | 説明を正し、表の重なりを見る表明を足した |
| 2・A-2／B-1（連鎖・同じ指摘） | `frame.rs`・`frame_chain_realign_arm_tests.rs` | 「3 連言」が残る | 「2 連言」にし、節の番号を ⑴ 窓の種類・⑵ 寸の変化に振り直し、縮退の報告の節は番号なしに |
| 2・B-2（連鎖） | `frame_dpi_sync_hold_tests.rs` | 1 回目に足した `site=pattern-move` の持ち主が言い過ぎで、理由も別の点のもの | 門の在処と振る舞いを見るテストを分けて書き、記録を駆動するテストは無いが同じ `defers_window_write` を通ると書いた |
| 2・B-3（一部連鎖） | `frame_dpi_reproject_tests.rs`・`frame_dpi_sync_hold_tests.rs` | 書き換えた行に「是正前の構造」「是正後」「変異 M3／M6」 | 今の形の文に直した |
| 2・B-4（連鎖） | `frame_transition_atomicity_tests.rs`・`frame_dpi_reproject_tests.rs` | 1 回目に消した段落を受ける「変わったのは…」が宙に浮く | 消した・1 文に縮めた |
| 2・A-3 | `frame/work_area_sync.rs` | 「ほかの 3 点」（見送り点は 5 つ） | 「ほかの見送り点」に |
| 2・A-4 | `frame_balloon_offset_keyword_gate_tests.rs` | 判定の行が 0 行でも通る | 行の数の表明を 1 つ足した |
| 2・A-5 | `frame_balloon_offset_converge_tests.rs` | `window_pos_of` の写しが残る | `window_pos_of` を使う |
| 2・C-1（連鎖） | `runtime_tests/frame/status_report_tests.rs` | 1 回目に触った見出しに spec の名と設計の節の名 | `// 届けの相` に |
| 2・C-2 | `frame_transition_branch_tests.rs` | 割れた「⚠ 区分＝」と、名の消えた `research.md` の訂正の段落 | 1 文にし、段落を消した |
| 2・C-3（一部連鎖） | `frame_transition_branch_tests.rs` | 群 A が無いのに群 B・C・D が残る | 「群 X:」を消した |
| 2・C-4（連鎖） | `frame_transition_test_support.rs` | 1 回目に移した `window_pos_of`・`window_size_of` が既にある `pos_of`・`size_of` の写し | 既にある方へ委ねた |
| 3・1（連鎖） | `frame_dpi_reproject_tests.rs` | 2 回目に頭の説明を消したので、残る「是正前」「是正の前後」「4 件」が何を指すか分からない | `s2_red_*` の 4 件と名指し、是正の語を消した |
| 3・2（連鎖） | `frame_dpi_reproject_tests.rs` | 寸の変化の不変を守るテストとして、作業領域の変化のテストを名指した | `frame_balloon_offset_follow_tests.rs` の `a_surface_swap_at_a_constant_scale_does_not_move_the_offset` に |
| 3・3（連鎖） | `frame_dpi_sync_hold_tests.rs` | 振る舞いの持ち主として、相の写しを試すテストを名指した | 本物の相を試す `frame/move_shift_tests.rs` の `deferred_windows_are_shifted_after_the_hold` に |

**5 回目（14 件・どれもコメントだけ）**

| 番号 | 重さ | 場所 | 指摘 | 直し |
|---|---|---|---|---|
| 5-1 | 直したほうがよい | `runtime_tests/frame/switch_finish_tests.rs` | 1 回目（G8-2）で書き換えた説明に「（要件 3.12）」 | 消した |
| 5-2 | できれば | `frame_transition_branch_tests.rs` の頭 | 「是正そのものの対テスト（是正前は赤・是正後は緑）」と表の見出し「是正」 | 「対テスト（その仕組みが無いと赤になるテスト）」・見出し「仕組み」 |
| 5-3 | できれば | `frame_transition_branch_tests.rs` | 「（開発者裁定 2026-08-20）」 | 「（開発者の裁定）」（物差し ⑷） |
| 5-4 | できれば | `frame_transition_branch_tests.rs` の 2 か所 | 「窓が消える件を直したときの」 | 消した（決まりの中身「再導出不能なら現状維持」だけを残す） |
| 5-5 | できれば | `frame_transition_branch_tests.rs` の本番入口の檻 | 「レビューが名指しした申し送り」「レビューが実演で確定させた」「どこにも掛かっていなかった」 | 消し、「到達可能性はこの 1 本だけが押さえる」に |
| 5-6 | できれば | `frame_balloon_offset_follow_tests.rs` の頭 | 「（是正前は失敗する側）」「（実測 2026-08-28）」 | 消した（何を潰すと何が赤になるかは残る） |
| 5-7 | できれば | `frame_balloon_offset_follow_tests.rs` | 「追随の変更が上書きしない側…（拡大率遷移の側だけが反転した）」 | 「拡大率遷移では成り立たず、作業領域の再スナップでは成り立つ。ここが固定するのは後者」に |
| 5-8 | できれば | `frame_transition_branch_wait_tests.rs` の頭 | 「是正前の赤はここが持つ」 | 「待ちが無いと 2 段書込になることの対テストはここが持つ」 |
| 5-9 | できれば | `frame_work_area_resnap_tests.rs` の頭 | 「是正前の赤を示す対テスト」 | 「再スナップが無いと赤になる対テスト」 |
| 5-10 | できれば | `frame/balloon_offset_follow.rs`・`frame_balloon_offset_keyword_gate_tests.rs` の 2 か所 | 「（開発者裁定 2026-08-27）」 | 「（開発者の裁定）」（物差し ⑷） |
| 5-11 | できれば | 同じ 2 ファイル | 「本仕様の新規コード側」「追随の新規コード側」「1 文字も変えていない／触れていない」 | 「追随の側に置く」「そのまま保つ」 |
| 5-12 | できれば | `frame/balloon_offset_follow.rs` の 2 か所 | 「（先行仕様から引き継いだ約束／予算）」 | 消した |
| 5-13 | できれば | `frame_harness_tests.rs` の 2 か所 | 「開発中に一度、…実測で出している」「レビューで実測——…緑のままだった」 | 今の形の事実だけに。「60 回中 11 失敗」の数も残さない: 数の元だった `command.rs` のプロセス共有のカウンタは今は無く（窓書込指令のキューも観測の写しもスレッド局所）、今のコードで確かめられない数になっていた。頭の説明は「スレッド局所の状態（窓書込指令のキュー・観測の写し）が 1 つ残るだけで、同じスレッドで次に走るテストを汚す」に書き換えた |
| 5-14 | できれば | `frame_work_area_resnap_hold_tests.rs` の頭 | 「ところが列挙していた見送り点は 3 つ…抜けていた」「4 点目を足した後の」（1 回目の G7-3 と同じファイルの残り） | 「見送り点は…にも要る」「この 4 つ目が抜けると」「4 点目のある同じ駆動」に |

- 2・3 回目の連鎖はどれも同じ種類（説明を消す・名指しを替える直しが、残った説明の拠り所を外す）で、コメントの言葉と檻の補助に閉じた。
- 回をまたいだ食い違いは、4 回目の後には残っていた: 「是正前／是正後」と日付つきの注を 1 回目（G1-2・G6-5・G7-3）と 2 回目（B-3）は指摘に数えたのに、同じ語がこの spec の書いた別の行に残っていた（5-2・5-6・5-8・5-9・5-14）。3 回目の 1 番が `frame_dpi_reproject_tests.rs` から「是正」の語を消したのとも食い違っていた。5 回目で 13.1 の物差し 1 本を差分の足した行の全部に当て、確かめの読み直しで 0 件になったので、今は残っていない。
- 5 回目は、連鎖した指摘をまとめて直す回（要件 9.8）として Fable で行った。

### 13.5 本番のコードの変わった所

- `frame/dpi.rs`: 連鎖の並べ直しの武装の条件から `wrote &&` を消した。窓の寸の前後比べ（`before != after`）は書込が起きたときだけ真になるので、振る舞いは同じ。`frame::chain_realign_arm_tests` は全部緑。
- `frame/work_area_sync.rs`: `SystemState::get` の失敗を捨てずに `warn!` 1 行で残す（今の引数では起きない）。この行の字面を読む道具・手順書・テストは無い。この spec が足した新しいログの行（`[work_area_sync]` の文脈の `warn!`「見張りの印を進められない（次のフレームで 1 回作り直す）」）なので、仕上げのログの確かめ（T27）で新しい行として数える。
- ほかは説明（コメント）とテストだけ。公開の面・SHIORI へ送るもの・ゴーストに見える振る舞いは変わらない。

### 13.6 回すたびの確かめ

- `cargo test -p areka-runtime --lib -- frame::`（351 本緑）・`cargo test -p areka --bins -- runtime_tests::frame session::ghost_quit`（117 本緑）・`cargo test -p areka-runtime --lib -- zorder_wiring`（`frame.rs` の字面を読む檻・緑）・`cargo fmt --all -- --check`・`cargo test -p log-capture-kit --test file_length_guard_test`（緑）。
- `cargo clippy -p areka-runtime -p areka --all-targets`: 触った行を指す警告は 0 件（範囲の中に残る警告は前からの行）。
- 5 回目（コメントだけの直し）の後: `cargo fmt --all -- --check`（差分なし）・`cargo test -p areka-runtime --lib -- frame::`（351 本緑）。

### 13.7 範囲の外で見つけたこと（直さない・申し送り）

- `doc/COMPAT_ARCHITECTURE.md` の 213 行目付近（⑴ の欄）が、消したテストのある `frame_work_area_resnap_tests.rs` を「作業領域の再スナップ単独で追従オフセットが変わらない」の持ち主と書いている。今の持ち主は `frame_transition_branch_tests.rs` の `a_work_area_only_change_writes_once`。C23 で触らないファイルなので、12.6 の 228 行目と同じく「申し送り先の決まっていない件」に足す（完了の棚卸で起票する）。
- `crates/areka-placement/src/chain_rule_tests.rs` の 5 行目の古い置き場は、12.5 で `test-crash-report-merge` へ申し送り済み。
- `crates/areka-placement/src/chain_realign.rs` の 168〜170 行（`arm_chain_realign` の説明）は「キャラ窓の再射影が寸変化を伴って書込を起こしたときに呼ぶ。判断（寸が変わったか・書込が起きたか）は呼び手が持つ」と書く。この spec が武装の条件から `wrote &&` を消した（13.5）ので、今の呼び手（`frame/dpi.rs`）が見るのは窓の種類と窓の寸の前後比べの 2 つだけで、「書込が起きたか」は見ない（寸が変われば書込は必ず起きているので振る舞いは同じ）。`areka-placement` は C23 で触らないので、12.5 の件と同じ `test-crash-report-merge` の brief へ件として足した。

## 14. 仕上げの確かめ（T27・要件 9.4・9.5）

基準は `origin/main`（`aff8f397a`・この枝の merge-base と同じ）、確かめた枝の先頭は `e28eafa5d`。

### 14.1 既定の枝との差分の確かめ（要件 9.5）

| 確かめ | 結果 |
|---|---|
| 触らないファイル（`crates/areka-placement/`・`hit_region.rs`・本体の `input_events/`・`main.rs`・`ghost_session.rs`・`ghost_session_runtime.rs`・`app_exit.rs`・`menu/`・`spawn_assembly_tests.rs`・`log-capture-kit` の `event.rs`・`vendors/`・`Cargo.toml`・`Cargo.lock`・`crates/*/Cargo.toml`）の `git diff --stat origin/main...HEAD` | 空 |
| `doc/COMPAT_ARCHITECTURE.md` の差分 | 空 |
| `file_length_guard_test.rs` の差分 | `frame_transition_branch_tests.rs` の 1 項目（4 行）を表から消した分と、件数 3 → 2 の 1 行だけ |
| 範囲の外のファイルの差分（`frame*` の `areka-runtime`・本体の `runtime_tests/` の `frame*`・`session/ghost_quit*`・この spec の文書を除く） | `target_map.rs`・`spine_wait.rs`・`assets.rs`・`move_cue_apply_move_tests.rs`・本体の `runtime_tests/zorder_wiring_tests.rs`（`bfebe28d3`・2.1 の説明 1 行。`src/lib.rs` の対照の持ち主を `areka-runtime` の `t_zwi06` へ向け直した）・`file_length_guard_test.rs`・6 つの brief（`extra-character-windows`・`test-crash-report-merge`・`audit-emo2-boot-switch`・`audit-areka-app`・`areka-test-wait-wording`・`window-resetwindowpos-hold`）だけ。本体の `zorder_wiring_tests.rs` は設計の「範囲の外（持たないもの）」の一覧に無かったので、仕上げで一覧に足した。足した後の一覧と `git diff --name-only origin/main...HEAD` の範囲の外のファイルは一致する（ほかに漏れは無い） |
| 依存・フォーク・`[patch]`・`vendors/` の差分 | 0（`Cargo`・`vendors`・`.cargo`・`deny` を含む名前の差分ファイルが無い） |
| 端末へ出す文（`println!`・`eprintln!`・`print!`・`dbg!`）の足し | 0 |

### 14.2 外への約束の変わった所（「見直しが要る変化」に挙げた分）

- 公開の狭め（114 番）: `-pub(crate)`／`-pub` の行は、移した・狭めた分。外の読み手の grep は 0 件。`FrameHarness::run_work_area_resnap`（71 番）の削除も、ツリー全体で読み手 0 件。`frame::wiring` の公開は変わっていない（`origin/main` でも `#[cfg(feature = "test-support")] pub mod wiring;`／`#[cfg(not(feature = "test-support"))] mod wiring;`。差分は宣言の置き場を移し、上の説明の行を消しただけ）。
- 新しい公開の関数: `target_map::window_scope(usize) -> Option<u32>`（4 番）。
- ログの文の変化と、字面の読み手（道具・手順書・テスト・スキル）の数:
  - 消した `error!` 2 行（`dpi.rs` の「dpi: scope が u32 に収まらず…」・`scale_text.rs` の「dpi reconcile: scope が u32 に収まらず…」）: 読み手 0 件。
  - 足した `warn!`（設計の一覧にある分）: `target_map::window_scope`「window scope が上限を越え target を採番できない → この窓を読み飛ばす」（4 番）: 字面の読み手 0 件。
  - 足した `warn!`（設計の一覧の外だった分）: `frame/work_area_sync.rs`「[work_area_sync] 見張りの印を進められない（次のフレームで 1 回作り直す）」（7.1 のかけ直しで足した・13.5。今の引数では届かない道）: 字面の読み手 0 件。設計の「境界の約束」のログの行と「見直しが要る変化」に足して、設計とコードを揃えた。
  - 字面を変えた `warn!`（113 番）: `frame/drain_resnap.rs`「drain: 1 フレームに取り出す上限に達した（残りは捨てずに次のフレームで到着順に適用する）」→「…（残りがあれば次のフレームで到着順に適用する）」: 範囲の外の字面の読み手 0 件（範囲の中の読み手は同じ変更で足した `frame/drain_resnap_tests.rs` の 1 か所だけ）。
  - 足した `info!`: `frame/scale_text.rs`「text-scale: 装着で繋げなかった文字層を、配置先ができたので繋ぐ」（3 番）: 範囲の外の字面の読み手 0 件（範囲の中の読み手は同じ変更で足した `frame_text_scale_tests.rs` の 1 か所だけ）。
  - 字面を変えた `warn!`（5.1 の 41 番の一掃・設計の一覧の外だった分）: `frame/attach.rs`「runtime attach: 同 scope の balloon 資産が無い（DD-12 balloon_index None）→ 文字層接続なし」→「…（balloon_index None）…」: 字面の読み手 0 件。最終の確かめで見つけ、設計の「境界の約束」のログの行に足した。
  - 既存の `info!(skin_switch_done)` は字面を変えていない（説明の行だけ）。
  - 読み手の数は、それぞれの文の切れ端（「scope が u32 に収まらず」・「window scope が上限を越え」・「見張りの印を進められない」・「次のフレームで 1 回作り直す」・「取り出す上限」・「残りは捨てずに次のフレーム」・「残りがあれば次のフレーム」・「装着で繋げなかった文字層」）を `crates/`・`tools/`・`doc/`・`.claude/`・`.kiro/steering/` に grep して、出した行の外に当たる所で数えた。
- 設計との揃え: 設計の「指摘ごとの直し方」の 56 番の行は「`areka-test-binary-stability` の `load-repro.md` の ⑹」と spec の名を書くと言っていたが、コードは CLAUDE.md の `## Comments` に合わせて名を消し、理由「自前の締切は負荷の下で赤になる」だけを残した（G3-1・要件 5.13 は消してよいと書く）。設計の 56 番の行を、したことに書き直した。

### 14.3 全体テスト（要件 9.4）

`pwsh -NoProfile -File tools/test-all.ps1` を 1 回回した。確かめた commit は `e28eafa5d`（走り始めに未コミットの変更が 1 件あった: この節の追記）。結果は全部緑（`all steps green`・終了コード 0）。

| 手順 | 結果 |
|---|---|
| sample fetch・add i686 target・build i686 artifacts | OK |
| fmt --check | OK（6 秒） |
| x64 workspace tests | OK（413 秒） |
| dola toml feature tests・i686 tests (host-32)・crates.io pre-publish check | OK |
| encoding・release notes・roadmap overview・i686 helper sync・sample fetch・package・exe install・load flake・mouse input・app icon・cursor art の各 check | OK |
| perf loop selftest・i686 helper beside areka.exe | OK |

赤のテストは 0 件。
