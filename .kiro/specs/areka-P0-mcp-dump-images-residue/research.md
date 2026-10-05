# 調査と設計判断: areka-P0-mcp-dump-images-residue

## ギャップ分析（要件とコードの差）

> 2026-10-05・本ブランチ（`claude/areka-p0-mcp-dump-images-7ad1fb`・main `44fc0a61` の上）のコードを読んで書いた。対話なしで書いた。
> 対象: [requirements.md](requirements.md)・[brief.md](brief.md)・前の spec（[completed/areka-P0-mcp-dump-images](../completed/areka-P0-mcp-dump-images/) の design.md・design-validation.md・verification/signoff.md）・`.kiro/steering/`（product・tech・structure・roadmap・logging）。
> コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。確かめていないものは「未確認」と書く。ここに書くのは選択肢と判断の材料で、決めるのは設計の段。

---

### 1. 要約

- **土台はそろっている**。2 本の入口（`crates/areka/src/mcp/dump_surface.rs` と `dump_balloon.rs` の関数 `handle`）、判断と絵の写しを UI スレッドで行う `answer`、残りを符号化のスレッドで仕上げる `reply_elsewhere`、後から答える置き場 `later`（`crates/areka/src/mcp/mod.rs`・覗く関数は `FnMut` で状態を持てる・`'static` で `Send` を求めない）が在る。件 2・3・5 は、この形を広げるだけで `mod.rs` に触らずに直せる見込み。
- **件 1（読み戻し）だけは新しい部品が要る**。今の `TextSurface::read_back`（`crates/areka-emo-text/src/surface.rs`）は「写しを積む → `Map(READ)` で GPU の終わりを待つ」を 1 回の呼び出しの中で行う。待たない形にするには、文字の層に「写しを積むだけの口」と「待たずに読めたら読む口」（`D3D11_MAP_FLAG_DO_NOT_WAIT` で `DXGI_ERROR_WAS_STILL_DRAWING` を『まだ』として扱う）を足し、`dump_balloon` は `later` で毎フレーム覗く形になる。今の `read_back` の呼び手（約 90 か所）はそのまま残せる。
- **件 4（符号化のスレッドの記録）に、要件が想定していない制約が 1 つ見つかった**。いちばん小さい直し方（呼んだスレッドの記録の受け手を符号化のスレッドへ引き継ぐ）は、ワークスペースの見張り `crates/log-capture-kit/tests/with_default_guard_test.rs`（記録の受け手を直接差す呼び出しを例外表の外で禁じる）に引っかかる。例外表を足すのは要件の境界の外なので、境界の内で閉じる別の判定（答えで判定する・仕事を直接受け取って捕まえる）を選ぶか、止めて開発者に諮るかになる。
- **件 5 は今のコードで起きうる（狭い道）**。切替先のゴーストが装着の前に失敗して既定のゴーストへ戻ると、預けた覗く関数が既定のゴーストの結線状態を読んで、その絵を切替先の名前を添えて返す。さらに件 1・3 の直しで「答えを待たせる時間」が装着の後にも生まれるので、要件 4.2 の確かめは覗く関数の全部に要る。
- **規模は M・危険は中**。新しい種類の GPU の使い方（待たない読み出し）と、既存の GPU を通るテストの待ち方の変更（フレームを回さずに待っている）が主な不確かさ。

---

### 2. 今のコードの姿

#### 2.1 入口と答え方

| 何 | 定義 | 振る舞い |
|---|---|---|
| 入口 | `handle`（`mcp/dump_surface.rs`・`mcp/dump_balloon.rs`） | `answer` が `Some(step)` なら `reply_elsewhere` へ。`None`（装着の前）なら `super::later` へ覗く関数を預ける。受け取った `_ghost: &ActiveGhost` は今は使っていない |
| UI スレッドの側 | `answer`（同 2 ファイル） | 結線状態 `Emo2Wiring` を読み、無ければ「窓が無い」。`attached()` が偽なら `None`。判断（`judge::judge_surface`・`judge::judge_balloon`）の後、絵を写して `Step::Encode(scope, job)` を返す |
| 答えの種 | 列挙 `Step`（`Now(ToolOutcome)`・`Encode(u32, Job)`）・型 `Job`（`FnOnce(Duration) -> ToolOutcome + Send`） | `Step::here` は今のスレッドで `job` を走らせる（装着の前に預けた組だけが使う＝件 3） |
| 別のスレッドで仕上げる | `reply_elsewhere`（`dump_surface.rs`） | スレッド名 `mcp-encode`（定数 `ENCODE_THREAD`）を起こし、`catch_unwind` で受けて `fail(…, "the encoding thread panicked")`、起こせなければその場で `fail(…, e.to_string())`。`ReplyTo` ごとスレッドへ渡し、**そのスレッドから直接答える**（`later` を通らない） |
| 判断の失敗 | `refuse`（`debug!` だけ） | 文言は `judge` の定数 `NO_WINDOW`・`NO_SUCH_SCOPE`・`NO_SUCH_SURFACE`・`NOT_SHOWN_YET`（`dump_surface_judge.rs`） |
| 想定外の失敗 | `fail`（`error!` 1 件） | 呼んだスレッドで記録する。符号化のスレッドで呼ばれると、記録もそのスレッドで出る |
| 成功の記録 | `picture` の中の仕事・`dump_balloon` の `answer` の中の仕事 | 符号化のスレッドで `debug!(…, ui_us, encode_us, "[mcp] 絵を返す")` |

#### 2.2 後から答える置き場（`crates/areka/src/mcp/mod.rs`・触らない）

- `later(world, reply, poll)`: 置き場が在れば組を積む。無い（`install` の前・`close` の後）なら組をその場で落とす＝"shutting down"。
- `poll_later`: `drain`（`Input` の段・`dispatch_pointer_events` の後）の最後に、積んだ組を全件覗く。`reply.is_abandoned()`（待つ側が 10 秒の上限で去った）なら**覗かずに捨てる**＝覗く関数ごと落ちる。`Some` を返したら `reply.send` して外す。預けた直後の同じ `drain` の中でも 1 度覗かれる。
- `close`: 終了を始めた所で置き場ごと外す＝預けた組は "shutting down"。
- `ReplyTo::send`（`crates/areka-mcp/src/tools/bridge.rs`）は受け手が居なければ黙って捨てる。待ちの上限は `REPLY_WAIT`＝10 秒（`crates/areka-mcp/src/tools/mod.rs`）。

この形から言えること: 覗く関数が「受け取り口（`mpsc::Receiver`）」や「読み出し待ちの札」を手元に持ち、仕上がるまで `None` を返せば、`mod.rs` に触らずに件 1・3 を表せる。待つ側が去れば覗く関数が落ち、受け取り口も落ちるので、符号化のスレッドの送りは失敗して黙って終わる（要件 3.5 の形がほぼそのまま手に入る）。終了の途中も置き場ごと落ちる（要件 1.6）。

#### 2.3 文字の面の読み戻し（`crates/areka-emo-text/src/surface.rs`・817 行）

- 構造体 `TextSurface` は描画面 2 枚（`sources`・`front` が最新）と、読み戻し用の `staging`（CPU で読める写し先・**1 枚だけ**・装着のときに作る）と、`context`（`ID3D11DeviceContext`・UI スレッド専用の即時の文脈）を持つ。欄は非公開。
- `read_back(&self)`: `CopyResource(staging, front)` → `Map(staging, READ)`（GPU がそこまでの仕事を終えるまで待つ）→ 行ごとに密な配列へ写す → `Unmap`。失敗は内部の `device_err` が `error!` を出して `Err`。
- `size()` は面の物理寸。`back_tex` は `pub(crate)`、`front_tex` はテスト専用。
- デバイスは `D3D11_CREATE_DEVICE_SINGLETHREADED` なしで作られる（`crates/wintf/src/ecs/graphics/core.rs` の `GraphicsCore` の注記）が、即時の文脈は D2D と共有で、別のスレッドから `Map` を呼ぶのは安全と言えない（未確認・避けるのが無難）。
- `read_back` の呼び手は約 90 か所（文字の層のテスト・見本・`crates/areka/src/emo2_boot/spine_talk_close_tests.rs`・`dump_balloon_gpu_tests.rs`・`dump_balloon.rs`）。要件の Adjacent expectations どおり、今の振る舞いは変えずに残すのが前提。

#### 2.4 ゴーストの切り替えと結線状態

- 起動中のゴーストは置き場 `GhostSlot`（`crates/areka/src/ghost_session.rs`）。`resolve::active`（`mcp/resolve.rs`）は置き場から `ActiveGhost { name, root }` を作る。切り替えの途中で置き場が空になるのは `ghost_switch::switch_to`・`switch_to_default`（`crates/areka/src/emo2_boot/ghost_switch.rs`）の同期の手順の中だけで、MCP の `drain` からは見えない。
- 切り替えは `Update` の段の `ghost_quit_system` → `run_ghost_quit_phase` → `on_ghost_stopped` → `switch_to`／`switch_to_default` →（`take_down` → `boot_into`）で起き、`boot_into` の中の起動で新しい `Emo2Wiring` が World に入る（`crates/areka/src/emo2_boot/mod.rs` の手順 6 の `insert_non_send(wiring)`）。`GhostSession` にはゴーストを見分ける通し番号の欄が無い。
- 装着（`run_attach_phase`・`emo2_boot/frame/attach.rs`）の条件は資源 `GhostWindows` と GPU の資源。切り替えでは窓を閉じ（`app_exit.rs` で `GhostWindows` を外す）、`commit_ghost_windows`（`ghost_session.rs`）が窓の生成を**非同期に**投げるので、新しいゴーストは少なくとも 1 フレームは装着の前に居る見込み（フレーム数は未確認）。

#### 2.5 記録の捕まえ方（テスト）

- `log_capture_kit::capture`・`count_levels`（`crates/log-capture-kit/src/capture.rs`・`event.rs`）は**呼んだスレッドの**既定の受け手を窓の間だけ差し替える。符号化のスレッドの記録は入らない。
- 全スレッドの捕捉 `install_global_capture_all`（`global.rs`）はプロセスに 1 度だけ据えて取り消せず、そのテストバイナリの全スレッドの記録を溜め続ける。areka の単体テストは 1 本のバイナリなので向かない（同じ理由で使わなかった前例: `crates/areka/src/session_end_deadline_tests.rs` の注記。答えや呼ばれた回数で代わりに判定している）。
- 見張り `crates/log-capture-kit/tests/with_default_guard_test.rs` は、ワークスペースの全ソース（テストを含む・コメントは除く）で、受け手を直接差す 3 つの呼び出しの綴り（`with_default(`・`set_default(`・`set_global_default(`）を例外表 `ALLOWED_DIRECT_CALLS`（4 件・件数も別の定数で固定）の外で禁じている。**`tracing::dispatcher` の同名の関数も同じ綴りで引っかかる**。

#### 2.6 既存のテストの待ち方

- `WaitAnswer::wait_answer`（`dump_surface.rs` の `#[cfg(test)]`）は `spin_wait_until` で `try_answer` を回すだけで、**フレームを回さない**。GPU を通るテスト（`dump_surface_gpu_tests.rs`・`dump_balloon_gpu_tests.rs`）の成功の呼び出しはこれで待っている。今は符号化のスレッドが直接答えるので届く。
- フレームを回して待つ口は `GpuRig::frames_until`（`dump_surface_gpu_test_support.rs`）。装着の前の呼び出しのテスト `before_attachment_answers_after_the_frames_run` はこちらで待っている。
- `spin_wait_until` は `crate::emo2_boot::spine` に在る（`dump_surface.rs`・`dump_surface_gpu_test_support.rs` が使う）。

---

### 3. 要件ごとの対応

印: **Missing**＝今は無い・**Unknown**＝確かめが要る・**Constraint**＝決まりによる縛り・**OK**＝今のままで足りる。

| 要件 | 今あるもの | 差 |
|---|---|---|
| 1.1 1 回の呼び出しが UI スレッドを塞ぐ時間がどのフレームでも 2 ms 以内 | 実機で `dump_balloon` の `ui_us` は 1,436〜14,233 µs（前の spec の signoff）。キャラクター 1 枚は最大 310 µs | **Missing**（待たない読み戻し）。**Unknown**: 読み出せた後の行ごとの写し（200% の面）と、写し先を毎回作るならその作成の時間が 2 ms に収まるか。最小の 1,436 µs に何が含まれるかの内訳も未測 |
| 1.2 GPU の終わりを UI スレッドで待たない | `TextSurface::read_back` が `Map(READ)` で待つ | **Missing**: 文字の層に「積むだけ」「待たずに読む」の 2 つの口。**Constraint**: 文字の層に新しいファイルを足さない（`surface.rs` に足す・817 行＋追加で 1,000 行以内）。今の `read_back` の結果は変えない |
| 1.3 背景と文字を同じ時点の内容にする | 背景は `last_shown` の絵を `to_vec`、文字は同じ呼び出しで読む | 写しを積んだ時点で GPU の仕事の並びが決まるので、背景の写しと文字の写しを同じ呼び出しで積めば同じ時点になる。**Constraint**: 写し先 `staging` が 1 枚だけなので、読み出す前に別の読み戻し（テストの `read_back`・次の `dump_balloon`）が同じ写し先へ積むと上書きされる。呼び出しごとに写し先を持つか、使用中を守る仕組みが要る |
| 1.4 答えを待たせても描画を遅らせない | `later` は覗くだけ・描画の段とは別 | **OK**（覗く関数が待たなければ満たす） |
| 1.5 読み出しの失敗 → `NG:`＋`error!` 1 件 | `fail` | **Constraint**: 文字の層の `device_err` も `error!` を出すので、今の `read_back` の失敗は ERROR が 2 件になる。新しい口で 1 件にするか（層の側で記録しない・または `fail` を呼ばない）を設計で決める。「まだ読めない」（`DXGI_ERROR_WAS_STILL_DRAWING`）は失敗として記録しないこと |
| 1.6 終了の途中は入口の決まりのまま・待たせない | `close` が置き場ごと落とす | **OK**（覗く関数が札を持つだけなら、落ちれば写し先も解放される） |
| 2.1 `compose_alone` が「無い」→ `NG:the shell of this scope is not ready` | `answer` の `SurfacePlan::Alone` の腕の `None` が `judge::NO_SUCH_SCOPE` を `fail` に渡す | **Missing**（定数 1 つと腕 1 行）。届かない枝なので、テストでどう踏むかが要る（**Unknown**: `compose_alone` が `None` を返す状態を作れるか。作れなければ「`fail` に渡す文言」を純粋な関数に切り出して固定するなど） |
| 2.2 判断の文言 4 つを想定外の失敗に使わない | 2.1 の 1 か所だけが違反 | 2.1 で直る。固定の仕方（文言の定数を `fail` に渡していないことを検査で縛るか）は設計で |
| 3.1 装着の前に預けた呼び出しも符号化を UI スレッドの外で | `Step::here` が UI スレッドで `job` を走らせる | **Missing**: 覗く関数が `Step::Encode` を受けたらその場で符号化のスレッドを起こし、受け取り口を手元に持って `None` を返す。`reply_elsewhere` の「起こす・`catch_unwind`・起こせなければ答える」をこの形でも使えるよう分けると重複しない |
| 3.2 仕上がるまで待たせる | `later` の覗き | **Missing**（3.1 と同じ部品） |
| 3.3 panic → `NG:the encoding thread panicked` | `reply_elsewhere` の中だけ | **Missing**（3.1 の部品を共有すれば同じ答え） |
| 3.4 スレッドを起こせない → `NG:`＋`error!` | `reply_elsewhere` の中だけ | **Missing**（同上） |
| 3.5 待つ側が去った後は黙って捨てる | `poll_later` が覗かずに捨てる・`mpsc::Sender::send` の失敗は無視できる | ほぼ **OK**。符号化のスレッドの側で送りの失敗を記録しないこと。去った後に仕事が panic したときの `fail` の `error!` は残る（要件は「仕上がった答え」についての決まりなので、これを残してよいかは設計で一言書く） |
| 4.1 解決したゴースト以外の絵を含めない | 覗く関数は毎フレーム World の結線状態を読み直す | **Missing**: 覗く関数が解決したゴーストを覚えて、覗くたびに確かめる。`handle` が受け取る `ActiveGhost` を写して持てる |
| 4.2 待たせている間に替わったら `NG:Specified ghost is not active`（ERROR なし） | 文言は `resolve::NOT_ACTIVE`（`mcp/resolve.rs`・`pub(crate)`） | **Missing**。**Constraint**: 「同じゴーストか」の見分け方。`ActiveGhost` の比較（名前とルートフォルダ）なら境界の内で済むが、同じゴーストを起こし直したときは見分けられない。通し番号を足すには `ghost_session.rs`（境界の外）に触る |
| 4.3 起きうるかを確かめて残す | — | 下の 4 節に結果。設計で決定論テストの形にする |
| 4.4 起きなくても振る舞いを保つ | — | 4.1・4.2 の確かめを覗く関数の全部に置けば満たす |
| 5.1 「ERROR が無い」を符号化のスレッドの分も判定 | `count_levels` は呼んだスレッドだけ | **Missing**。**Constraint**: 受け手を引き継ぐ直し方は見張り（2.5）に引っかかる。下の 5.4 の選択肢 |
| 5.2 捕まえる道具の較正 | — | **Missing**（わざと失敗させる仕事で赤になることを見るテスト） |
| 5.3 成功の記録の `ui_us`・`encode_us` を固定 | 記録は符号化のスレッドで出る | **Missing**。**Unknown**: 待たせる形になったとき `ui_us` を「全部のフレームの合計」か「1 フレームの最大」か（要件 7.2 ⑴ の判定に直結） |
| 6.1 前の振る舞いを保つ | — | 文言・順・画像の形はどの案でも変わらない。答えが 1 フレーム以上遅れる経路が増えるだけ |
| 6.2 既存のテストを緑のまま（期待する値は書き換えない） | `wait_answer` はフレームを回さない | **Constraint**: `dump_balloon` の成功を `later` 経由にすると、既存の `dump_balloon_gpu_tests.rs` の `.wait_answer()` は届かずに赤になる。待ち方（フレームを回して待つ）の書き換えは「期待する値」ではないが、要件の言い方との関係を設計で明記する |
| 7.1 ⑴〜⑺ の決定論テスト | GPU の土台 `GpuRig`・`SwitchRig` | ⑹（ゴーストが替わる状態）を作る口が今の土台に無い。下の 5.3 |
| 7.2 実機確認 | 前の spec の手順・`target\` の下 | **OK**（手順は前の spec を写せる）。`ui_us` の意味（5.3）を先に決める |

---

### 4. 件 5（預けている間にゴーストが替わる）の確かめ

**結論: 今のコードで起きうる（狭い道が 1 本ある）。**

道筋（ファイルと定義の名前で再び確かめられる形）:

1. ゴースト A から B へ切り替える。`ghost_switch::switch_to` が `take_down`（A を降ろす）→ `boot_into`（B を起こして `GhostSlot` に入れる・`Emo2Wiring` は B のものに替わる・`emo2_boot/mod.rs` の手順 6）。段は `SwitchStage::Welcoming { attempt: WelcomeAttempt::Target }`。
2. B の窓は `commit_ghost_windows` が非同期に作るので、B の `Emo2Wiring::attached()` は少なくとも 1 フレーム偽のまま。
3. その間に `dump_surface`／`dump_balloon` が届く。`resolve::active` は B を返し、`ReplyTo::for_ghost` に B の名前が付き、`answer` が `None` を返して `later` に預けられる。
4. B の SHIORI が装着の前に失敗して止まると、`run_ghost_quit_phase` → `on_ghost_stopped` の `(Welcoming { Target }, Fault)` の腕 → `switch_to_default` → 既定のゴースト（emo2）の `Emo2Wiring` が入る。
5. 預けた覗く関数は毎フレーム `world.get_non_send::<Emo2Wiring>()` を読み直すので、emo2 が装着されたフレームで **emo2 の絵を、B の名前を添えて**返す。

起きない枝: 最初の起動（`fn main()` の起動）で A が装着の前に止まると、予約が無いので `quit_app` で終わり、別のゴーストは起きない。装着の後の呼び出しは今は預けないので、今のコードでは上の 1 本だけ。

本 spec の直しで広がる所: 件 1（バルーンの読み戻しを待つ）と件 3（装着の前の符号化を待つ）で、**装着の後の呼び出しも 1 フレーム以上待たせる**ようになる。待っている間に A → B の切り替えが起きると、絵は A の時点の写しを持っているので「別のゴーストの絵」にはならないが、要件 4.2 の「解決したゴーストが今動いていない」に当たる。よって確かめは装着の前の覗く関数だけでなく、待つ覗く関数のすべてに置く必要がある。

未確認: 2. の「少なくとも 1 フレーム」の実際のフレーム数（窓の生成の投函がいつ着くか）。4. の SHIORI の失敗が装着より先に届く時間の窓の広さ。どちらも道の有無は変えない。

---

### 5. 実装の道の選択肢

#### 5.1 件 1: 文字の面を待たずに読む

**案 A（推奨の見込み）: 文字の層に「積む口」と「待たずに読む口」を足し、`dump_balloon` は `later` で覗く**

- `surface.rs` に、例えば `TextSurface::begin_read_back(&self) -> Result<PendingReadBack, TextLayerError>`（写し先へ `CopyResource` を積み、`Flush` で GPU へ送る）と、`PendingReadBack::try_finish(&self) -> Result<Option<Vec<u8>>, TextLayerError>`（`Map` に `D3D11_MAP_FLAG_DO_NOT_WAIT`・`DXGI_ERROR_WAS_STILL_DRAWING` なら `Ok(None)`・読めたら密な配列にして `Unmap`）を足す。札は写し先と文脈の参照（COM の参照の写し）を持つだけで、`TextSurface` を借りない。今の `read_back` は触らない（または同じ部品で「待つ版」として組み直す）。
- `dump_balloon` の `answer` は判断・背景の写し・文字の写しを積むところまで行い、`handle` は `later` に覗く関数を預ける。覗く関数は「読めたら → 符号化のスレッドを起こす → 仕上がったら答える」の段を順に進める（件 3 の部品と同じ）。
- 使う Windows の定数は既に有効な機能（`Win32_Graphics_Direct3D11`・`Win32_Graphics_Dxgi`）に在る見込みで、`Cargo.toml` に触らない（`windows` の版での定数の名前は未確認）。
- 写し先: 呼び出しごとに新しく作るか、`TextSurface` の 1 枚を使用中の印つきで使い回すか（下の 7 節 議題 4）。新しく作れば、テストの `read_back` や面の作り直し（拡大率の変更で `TextSurface` が替わる）と干渉しない。
- ✅ UI スレッドで待たない。今の呼び手に影響なし。`mod.rs` に触らない。終了・上限は `later` の決まりのまま。
- ❌ 文字の層に公開の口が 2 つ増える（`surface.rs` の行数）。答えが最低 1 フレーム遅れるので、既存の GPU を通るテストの待ち方を変える要がある。GPU の仕事が送られていないと永久に「まだ」になるので `Flush` の扱いを確かめる要がある。

**案 B: 写しは今のまま 1 回で、`Map` だけを別のスレッドへ**

- 即時の文脈を UI スレッドの外から触ることになる。D2D と共有の文脈で、多重の守り（`ID3D10Multithread`）を areka は立てていない（2.3）。**危険が高く、勧めない**。

**案 C: 文字の層の毎フレームの処理の中で読み戻しを進める**

- 読み出しの完了を文字の相（`run_text_phase` など・`crates/areka/src/emo2_boot/frame.rs`）で覗く。境界の外（`emo2_boot/`・`budoux-reveal-reflow` の持ち物の近く）に触る。**境界に反する**。

**案 D: GPU の写しを待つ代わりに、問い合わせ（`D3D11_QUERY_EVENT`）で終わりを確かめてから待たずに `Map`**

- 案 A の変種。「まだ」の判定を `Map` の失敗でなく問い合わせで行う。部品が 1 つ増えるだけで利点は小さい。案 A で `DO_NOT_WAIT` が期待どおりに振る舞わない（WARP での振る舞いなど）ときの退避先として覚えておく。

#### 5.2 件 3: 装着の前に預けた呼び出しの符号化

**案 A（推奨の見込み・brief と要件の裁定 2 どおり）: 覗く関数が符号化のスレッドを起こし、受け取り口を持って待つ**

- `reply_elsewhere` の「スレッドを起こす・`catch_unwind` で受けて `fail`・起こせなければ `fail`」を、「答えを `ReplyTo` へ送る版」と「答えを `mpsc` で返す版」の 2 つで共有できるよう、1 つの関数に分ける（例: 仕事と送り先を受けて起こすだけの関数）。`Step::here` は使われなくなるので消せる。
- 覗く関数は状態（装着待ち → 符号化待ち）を持つ。`dump_balloon` は件 1 の読み出し待ちが間に入る。
- ✅ `mod.rs` に触らない。panic も起こせない失敗も、装着の後と同じ答えになる。
- ❌ 覗く関数が状態を持つぶん少し長くなる（2 ファイルで同じ形になるので、`dump_surface.rs` に共通の部品を置くのが自然）。

**案 B: `mod.rs` の `later` に「別のスレッドで答える」形を足す**

- 要件の裁定 2 と C4 の約束（`mcp/mod.rs` に触らない・`mcp-author-tools` が触る）に反する。**取らない**。

#### 5.3 件 5（要件 4）: 「同じゴーストか」の見分け方

| 案 | 中身 | 利点 | 欠点 |
|---|---|---|---|
| A | `handle` が受け取る `ActiveGhost`（名前・ルートフォルダ）を写して持ち、覗くたびに `resolve::active(world)` と比べる | 境界の内（`mcp/` の 2 ファイル）で閉じる。新しい欄が要らない | 同じゴーストを起こし直した（自分への切り替え・読み直し）ときは「同じ」と見なす。そのときの絵も同じゴーストのものなので要件 4.1 は満たすが、4.2 の「今動いているゴーストでなくなる」の読み方を設計で一言決める |
| B | `GhostSession` に起こした回ごとの通し番号を足して比べる | 起こし直しも見分けられる | `ghost_session.rs`（境界の外・C4 の他の spec の近く）に触る。止めて報告になる |
| C | 結線状態 `Emo2Wiring` の取り違えを見る（例: 覚えた参照と今の参照が同じか） | ゴーストごとに結線状態が作り直されるので見分けられる | 安全な見分けの口が `Emo2Wiring`（`emo2_boot/frame/wiring.rs`・境界の外）に無い。アドレスの比較は脆い |

確かめを置く所も決める要がある: 待つ覗く関数の全部（装着待ち・読み出し待ち・符号化待ち）の毎回の覗きの入口か、答えを返す直前だけか。前者なら替わった直後のフレームで答えが返る。

`dump_surface` の装着の後の経路（符号化のスレッドが直接答える `reply_elsewhere`）は、絵を写した時点で解決したゴーストのものなので 4.1 は満たす。符号化の 10〜20 ms の間に替わったときに 4.2 を当てるかは、要件の「答えを待たせている間」の範囲の読み方しだい（下の 7 節 議題 2）。当てるなら全部の答えを `later` 経由にする必要があり、既存の決定論テスト `success_is_finished_on_the_encoding_thread_and_answered_from_there`（`dump_surface_tests.rs`）と GPU を通るテストの待ち方が広く変わる。

7.1 ⑹ の状態の作り方:
- GPU を通る土台 `GpuRig::new`（`dump_surface_gpu_test_support.rs`・本 spec が触ってよいファイル）は `SwitchRig::new` に A だけを渡している。`SwitchRig` は A と B のフォルダを用意する（`ghost_switch_test_support.rs` の `SwitchRig::new`）ので、B の台本を渡せば `rig.boot("B")` で置き場を B に替えられる見込み（A を先に降ろす手順の要否は未確認）。`ghost_switch_test_support.rs` そのものは C4-① の持ち物なので触らない。
- GPU の要らない形: 覗く関数の「替わったか」の判断を純粋な関数に切り出し、`ActiveGhost` の組で固定する（配線は GPU のテスト 1 本で踏む）。

#### 5.4 件 4（要件 5）: 符号化のスレッドの記録の数え方

| 案 | 中身 | 境界 | 利点 | 欠点 |
|---|---|---|---|---|
| A | 符号化のスレッドを起こす所で、呼んだスレッドの記録の受け手を取り（`tracing::dispatcher::get_default`）、スレッドの中でその受け手を差して仕事を走らせる | **外**: 差す呼び出しの綴りが見張り `with_default_guard_test.rs` に引っかかり、例外表（`crates/log-capture-kit/tests/`）を足す要がある | 本番の配線のまま `count_levels` が両方のスレッドを数える。数行で済む。本番では受け手が全体の既定なので振る舞いは変わらない | 要件の Boundary Context（共有のテストの道具とその見張りに触らない）に反する。取るなら止めて開発者に諮る |
| B | 答えで判定する: 符号化のスレッドで起きる失敗は必ず `fail` を通って `NG:` の答えになる（成功の記録は `debug!`）。GPU を通るテストは「答えがすべて `NG:` でない」＋「UI スレッドの ERROR 0 件」で判定する | 内 | 本番もテストの道具も変えない。前例（`session_end_deadline_tests.rs`）と同じ考え方 | 「符号化のスレッドで `error!` を出すのは `fail` だけ」という前提が崩れたら見逃す。前提を縛る検査（例: 仕事の中の記録の呼び出しの数を数える構造の検査）を足すかを決める。5.3 の `ui_us`・`encode_us` は別の手段が要る |
| C | 仕事を直接受け取って捕まえる: テストが `answer` を直接呼んで `Step::Encode` の `job` を取り出し、`log_capture_kit::capture` の窓の中で走らせる（テストのスレッドでも、捕まえる窓ごと別のスレッドで走らせてもよい） | 内 | 本物の仕事の記録（成功の `debug!` の欄・`fail` の `error!`）をそのまま数えられる。5.2 の較正（わざと失敗させる仕事）と 5.3 の欄の固定が同じ形で書ける | `handle` → `reply_elsewhere` の配線そのものは通らない（配線は既存の「符号化のスレッドで仕上がる」テストが固定している）。テストが `answer` の戻り値の形に依存する |
| D | 符号化のスレッドの記録を出さず、結果と一緒に UI スレッドへ戻して UI スレッドで記録する | 内 | 全部の記録が UI スレッドで出るので今の捕まえ方のまま数えられる | `later` を通らない `reply_elsewhere` の経路では UI スレッドへ戻る口が無い。全部の答えを `later` 経由にするなら成り立つ（5.3 の議題と連動） |
| E | 全スレッドの捕捉 `install_global_capture_all` | **外**（見張りの別表に足す要）＋ areka の単体テストのバイナリ全体に効く | — | 取らない |

組み合わせの見込み: C（5.2・5.3 と GPU を通らない判定）＋ B（GPU を通るテストの判定）が境界の内でいちばん少ない変更。A は最も素直だが、見張りの例外表を 1 件足すことを開発者が認めた場合に限る。

---

### 6. 規模と危険

- **規模: M（3〜7 日・タスク 6〜9 の見込み）**。件 2 は数行、件 3 と件 5（要件 4）は覗く関数の状態の整理、件 1 は文字の層の新しい口と GPU を通るテストの待ち方の変更、件 4（要件 5）はテストの判定の組み直し、実機確認 1 回。brief の見立て S〜M（5〜9）と合う。
- **危険: 中**。待たない読み出しはこのリポジトリで初めての GPU の使い方（`DO_NOT_WAIT` と `Flush` の振る舞い・WARP での振る舞い）。既存の GPU を通るテストの待ち方を変える範囲が広い。2 ms の線は、読み出せた後の写し（200% の面）と写し先の作成の時間しだいで、実機で測るまで確かでない。見張り（記録の捕まえ方）の制約で、要件 5 の判定の形が一番素直な形から外れる。

---

### 7. 設計の段へ持ち越すもの

#### 7.1 調べもの（Research Needed）

1. `Map` に `D3D11_MAP_FLAG_DO_NOT_WAIT` を付けたときの戻り（`DXGI_ERROR_WAS_STILL_DRAWING`）と、`windows` の今の版での定数・型の名前。WARP（テストの土台）でも同じ振る舞いか。
2. `CopyResource` の後に `Flush` が要るか（描画の無いフレームが続くと GPU へ送られず「まだ」が続くおそれ・10 秒の上限で切れる）と、`Flush` の UI スレッドでの所要時間。
3. 実機（配布用のビルド・200%）での内訳: ⑴ 背景の写し（`to_vec`）、⑵ 写し先の作成（毎回作る案のとき）、⑶ 読み出せた後の行ごとの写し、⑷ 最初の 1 回だけ重い理由（前の spec の最大 14.2 ms は最初の 1 回）。⑶ が 2 ms に近いなら、`Map` を開いたまま写しを別のスレッドで行い、次のフレームで `Unmap` する形を検討する（即時の文脈は UI スレッドだけで触る）。
4. 写しを積んでから読めるまでのフレーム数（テストの待ちの上限と、実機での答えの遅れ）。
5. `compose_alone` が `None` を返す状態をテストで作れるか（要件 2.1 の枝を踏む方法）。
6. GPU を通る土台で、A を降ろさずに B を起こして置き場を替えられるか、降ろしてから起こす必要があるか（要件 7.1 ⑹）。

#### 7.2 並走との接点

- `crates/areka/src/emo2_boot/spine.rs` の `spin_wait_until` は C4-①（`ghost-session-test-load-flake`）が新しいファイルへ出す予定。本 spec のテスト（`dump_surface.rs` の `WaitAnswer`・`dump_surface_gpu_test_support.rs`）はこれを使うので、後からマージする側が使う所の道を直す。
- `ghost_switch_test_support.rs` は C4-① の持ち物。本 spec は使うだけで触らない。
- `crates/areka-emo-text/src/` は C4-②（`budoux-reveal-reflow`）が `lib.rs` を持つ。本 spec は `surface.rs` だけに足し、新しいファイルを足さない（文字の層の新しい口のテストも `surface.rs` の中の `mod tests` に置くか、`dump_balloon` の GPU を通るテストで踏む。`surface.rs` の 1,000 行の線に注意）。

---

### 8. 設計の分かれ目（要件ディスカッションへ）

1. **件 4（要件 5）の判定の形**: 見張り `with_default_guard_test.rs` の例外表に 1 件足して「受け手を符号化のスレッドへ引き継ぐ」形（素直・境界の外）を開発者に認めてもらうか、境界の内で「答えで判定する（案 B）＋仕事を直接捕まえる（案 C）」で閉じるか。要件 5.1 は「共有のテストの道具を変えない」としているが、見張りは同じ道具の crate の `tests/` に在るので、触れば境界の外になる。
2. **要件 4.2 の範囲**: 「答えを待たせている間」に、`later` を通らず符号化のスレッドが直接答える経路（`dump_surface` の装着の後）の 10〜20 ms も含めるか。含めるなら全部の答えを `later` 経由にする（答えが常に 1 フレーム以上遅れ、既存のテストの待ち方が広く変わる）。含めないなら「`later` に預けた答えだけ」と要件か設計に書く。
3. **同じゴーストの見分け方**: 名前とルートフォルダの比較（境界の内・起こし直しは同じと見なす）でよいか。起こし直しも「替わった」とするなら `ghost_session.rs` に通し番号が要り、境界の外になる。
4. **読み戻しの写し先**: 呼び出しごとに新しく作る（干渉しない・作成の時間が毎回かかる）か、`TextSurface` の 1 枚を使用中の印つきで使い回す（作成の時間なし・続けて撮ると後の呼び出しが待つ・テストの `read_back` と干渉しうる）か。7.1 の調べもの 3 の結果で決まる面がある。
5. **`ui_us` の意味**: 答えが複数のフレームにまたがるとき、成功の記録の `ui_us` を「全部のフレームの UI スレッドの時間の合計」にするか「1 フレームの最大」にするか（両方載せるか）。要件 1.1 は「どのフレームでも 2 ms」、要件 7.2 ⑴ は「成功の記録の UI スレッドの側の時間がすべて 2 ms 以内」なので、合計を載せるなら判定が厳しくなる。
6. **既存の GPU を通るテストの待ち方の変更**: `dump_balloon` の成功が `later` 経由になると、`dump_balloon_gpu_tests.rs` の `.wait_answer()`（フレームを回さない）は届かなくなる。待ち方をフレームを回す形に替えるのは、要件 6.2 の「期待する値を書き換えない」に反しない、と設計に明記してよいか。
7. **読み出しの失敗の ERROR の件数**: 文字の層の `device_err` が出す `error!` と、`fail` の `error!` で 2 件になる今の形（`read_back` の失敗）を、新しい口では 1 件にそろえるか（要件 1.5 は 1 件）。そろえるなら、新しい口は層の側で記録せず `Err` だけ返すか、`dump_balloon` の側が `fail` を呼ばずに `NG:` を返すか。
8. **`dump_surface` の `surface` 指定の UI スレッドの時間**: 前の spec の実機で `compose_alone` の経路は `ui_us` 2,226 µs（2.2 ms）だった。要件 1.1 の 2 ms の線は `dump_balloon` だけに掛かっているので本 spec の範囲外だが、同じ線を `dump_surface` に当てる話が出たときのために、記録として残すか起票するかを決める。

---

### 9. 要件ディスカッション（2026-10-05）での扱い

- **要件に書き込んで閉じたもの**: 議題 5（`ui_us` は 1 フレームの最大＝要件 5.3・7.2 ⑴）・議題 6（待ち方の変更は期待値の書き換えでない＝要件 6.2）・議題 7 の「何件か」（全体で 1 件・「まだ読めない」は記録しない＝要件 1.5。どちらの層で記録するかは設計）・議題 8（範囲外の観測として Boundary Context に記録）・3.5 の「去った後の panic の `error!` は残す」・件 6 の引き継ぎ（`areka-P0-mcp-kanade-tools` の brief の末尾に追記）。
- **設計の段で決めるもの**:
  1. 要件 5 の判定の形。要件 5.1 は「共有のテストの道具を変えない」としているので、見張りの例外表を足す案 A は取らず、境界の内の **案 C（仕事を直接受け取って捕まえる）＋案 B（答えで判定する）**を本筋とする。符号化のスレッドで記録を出すのは仕事の中（C で数える）と、仕事を包む `catch_unwind`／起こせないときの `fail`（必ず `NG:` の答えになる＝B で判定する）だけ、という前提を設計に書き、崩れたら赤になる形（較正＝要件 5.2）を置く。
  2. 読み戻しの写し先（呼び出しごとに作るか・使用中の印つきで使い回すか）。7.1 の調べもの 3 の結果と合わせて決める。
  3. 読み出しの失敗の記録をどちらの層で出すか（要件 1.5 の 1 件）。
  4. 要件 2.1 の届かない枝をテストで踏む方法（7.1 の調べもの 5）。
  5. 7.1 の調べもの 1〜6 の全部。
- **開発者と決めたもの（議題 1・案 a）**: 8 節の議題 2・3。「替わったか」を確かめるのは絵を写す前（装着を待つ間）の覗く関数だけ。写した後（読み出し待ち・符号化待ち・符号化のスレッドが直接答える経路）は写した時点の絵を返す（要件 4.5）。見分けは `ActiveGhost` の名前とルートフォルダの比較（5.3 の案 A）で、`ghost_session.rs` に通し番号は足さない。全部の答えを `later` 経由にはしない。
