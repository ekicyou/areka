# 設計の検証: balloon-flip-ssp-review

- 日付: 2026-11-20
- 対象: `design.md`（`2c5bc18ab` の版）・`requirements.md`・`research.md`（2〜12 節）と、設計が名指すコード・テスト。`brief.md` と steering の本文は読み直していない（要件と `research.md` に写された決めごとで足りると見た）
- 進め方: `kiro-validate-design` の手順（分析 → 大きな問題 → 良い点 → GO／NO-GO）。対話なしで判定まで出した。SSP・areka は動かしていない。コードは読むだけで、テストは回していない。

## 設計の検証のまとめ

設計は、案 ⑵（押し込みだけ）を「足した段を外し、3 つの関門が同じ `limit_correction` を呼ぶ形へ戻す」削る直しとして書いており、要件 2.3・3・4・5・7・8 の全部に当てる所がある。外す名前（`place_balloon_in_work_area`・`BalloonPlace`・`counted_overlap`・`shown_char_rects`・`is_shown`・`TEXT_BOX_SURFACE_NAME`・`overlap_scope`）をリポジトリ全体で引き直したが、設計が挙げていない呼び手は無かった。手で計算した期待の位置はどれも合っていた。残る問題は檻（テスト）の書き方の細部で、実装の前に直せば足りる。

## 確かめたこと

### 外す名前の呼び手（リポジトリ全体・`.kiro/` を除く）

| 名前 | 見つかった所 | 設計の扱い |
|---|---|---|
| `place_balloon_in_work_area`・`BalloonPlace` | `balloon_limit.rs`・`follow.rs` 56 行・`follow/window_move.rs` 14・17・979・1079〜1091 行・`balloon_limit_flip_tests.rs` | 全部挙げている |
| `counted_overlap` | `balloon_limit.rs` だけ | 挙げている |
| `shown_char_rects` | `follow/other_chars.rs`・`follow.rs` 175 行・`window_move.rs` 18・1078 行・`follow_other_chars_tests.rs` | 挙げている |
| `is_shown`・`TEXT_BOX_SURFACE_NAME` | `follow/other_chars.rs` だけ（ほかの `is_shown` は別物の同じ綴り: `crates/areka/src/mcp/dump_surface_gpu_tests.rs` のテスト内の関数・テスト名の一部） | 挙げている |
| `overlap_scope` | `balloon_limit.rs`・`window_move.rs`・`balloon_limit_flip_tests.rs`・`follow_other_chars_tests.rs` 242 行・`doc/COMPAT_ARCHITECTURE.md` 438 行 | 挙げている。`tools/`・`crates/areka`・`crates/areka-runtime` には 0 件 |
| `doc/COMPAT_ARCHITECTURE.md` | 438 行だけが当たる（314 行・531 行は `is_shown` を名前に含む別のテストの名で、関係ない） | 438 行の書き直し |

- `crates/areka/src/main.rs` 793 行は `apply_balloon_limit` を呼ぶが、設計はこの関数のシグネチャを変えないので触らなくてよい。
- `crates/areka/src/main_restore_seam_tests.rs` は `BALLOON_LIMIT_CLAMP_TAG` と位置 `(0,310)` だけを見ており、押し込みのままなので緑のまま。
- `[work-area-clamp] Clamp` の `how="flipped"`（キャラクター窓の鎖の寄せ）は別の記録の語で、範囲の外（`research.md` 2.2 節のとおり）。

### 手で計算した位置（作業領域 `0,0,2880,1704`）

| 形 | 押し込み（案 ⑵） | 今の並べ替え | 設計の値 |
|---|---|---|---|
| 10-30（窓 `0,336` 1108×1368・バルーン 872×900 が `(-872,336)`） | x＝max(−872, 0)＝0 → `(0,336)` | 映し 0＋1108−(−872＋872)＝1108 → `(1108,336)` | 合う |
| 11-16（窓 `2524,1304` 336×400・バルーン 288×203 が `(2655,1188)`） | x＝min(2655, 2880−288)＝2592 → `(2592,1188)` | 映し 2524＋2860−(2655＋288)＝2441 → `(2441,1188)`。自分の窓との重なりは自分の窓の中なので数えない | 合う |
| 収まる（窓 `1200,904`） | `(328,904)`・動かない | 同じ | 合う |
| `limit=0`（窓 `0,904`） | `(-872,904)`・動かない | 同じ | 合う |
| 窓がはみ出す（窓 `-200,904`） | `(0,904)` | 窓が収まらないので押し込み | 合う |
| 縦にはみ出す（ずらし `(-872,1000)`） | y＝1704−400＝1304 → `(328,1304)` | 横は動かないので押し込み | 合う |

- 件 2 の書き換え: `resolver.rs` 384〜390 行で、右に出すと基準が窓の右端・ずらしは「総量−2×descript の分」。descript の分 d では右の x＝窓の右端−d、左の x＝窓の左端−幅＋d で、左右の間合いはどちらも −d になり等しい。`windowposition.x` の分 −237（d＝0）では右の x＝窓の右端−237・左の x＝窓の左端−幅−237。設計の書き換え後の主張と合う。
- 書き込みの関門の形の作り方（形と違う中ほどで spawn して `move_window_to`）は、6 つの形のどれも作れる（10-30 は y＝336 のまま x を 872 以上へ、縦の形は窓の y を 304 以下で spawn すればバルーンが収まる）。
- `enqueue_window_set_pos` は関門の前に同じ位置の書き込みを飛ばさない（706 行）。判定し直しの書き込みは毎回関門を通り、押し込みのたびに `Clamp` を 1 行残す。設計の Risks の記述と合う。
- 行数の番人: `window_move.rs` は 1,234 行で例外表に載り（`crates/log-capture-kit/tests/file_length_guard_test.rs` 72 行）、数行減っても 1,000 行を割らない。

## 大きな問題（3 件）

### 問題 1: 新しい檻の「直す前」の色が、行ごとの欄の主張と食い違う

- **何が問題か**: Testing Strategy の表は「キャラクター窓がはみ出す」「縦にはみ出す」の 2 行を直す前も「緑」と書く。ところが同じ節で、記録のある行は全部 `field("overlap_scope")` が `None` であることを確かめるとしている。今は押し込みの行にも `overlap_scope=None` の欄が毎回載っている（`research.md` 12.1 節・`balloon_limit.rs` 330 行・`window_move.rs` 1091 行）ので、この主張は直す前に赤になる。また、新しく置く定数 `BALLOON_LIMIT_PUSHED_HOW` は直す前には無いので、新しい檻がこの定数を使うと、値で赤になる前にビルドで落ちる。
- **なぜ困るか**: 要件 7.2 は「変わる形のテストを直す前に赤で確かめる」ことを求め、7.5 は「変わらない形は位置が今と変わらない」ことを求める。どの主張が赤でどれが緑かの見込みがずれていると、実装係と見直し係が「赤の確かめ」をどう読むかで割れる。
- **直し方**: 表の「直す前」を、位置の主張と欄の主張で分けて書く（変わらない形の 2 行は「位置は緑・欄が無いことの主張は赤」）。または欄が無いことの主張を別の 1 本にまとめ、その 1 本だけを「直す前は赤」とする。新しい檻は直す前のコードでビルドが通るよう、`how` の値を檻の側の字（`"\"pushed\""`）で比べる（今の檻と同じ書き方）か、定数を先に足すタスクを置く。
- **要件**: 7.2・7.5・4.3・8.6
- **design.md の所**: Testing Strategy「新しい檻（PushCages）」の表とその下の段落・Components「BalloonLimitCore」の Service Interface（定数）

### 問題 2: 判定し直しの檻が、系が走ったか走らないかを位置では見分けられなくなる

- **何が問題か**: 案 ⑵ では、判定し直しが走っても走らなくても本体のバルーンの位置は同じ（押し込みの位置）になる。書き換える `rejudge_after_chain_finalize_pushes_the_balloon_onto_the_moved_partner_once` の手順のうち「印の無い巡で相方を動かしても動かない」は、位置だけを見るなら直した後は外れようがない主張になる。系が 1 度だけ走ったことを示せるのは `Clamp` の行の数だけで、設計は「印の入った巡で 1 行」「印の後の巡で 0 行」としか書いていない。`reset_pos_tests.rs`・`chain_realign_script_moved_tests.rs` の 2 本も、直した後は判定し直しの直の呼び出しが無くても緑になる（設計はこれを承知で名前と注だけ直すと決めている）。
- **なぜ困るか**: このウェーブでは系の登録と直の呼び手を残す（決めごとの 5）。残すものの振る舞い（印の巡で 1 度だけ走る・印の無い巡では走らない）を固定する檻がこの 1 本だけになるので、その 1 本の主張が弱いと、残した系が壊れても気付けない（「正しく動くこと」と「固定されていること」は別）。
- **直し方**: 書き換える 1 本で、3 つの巡のそれぞれに `Clamp` の行の数を主張する。印の無い巡は、相方を動かす `move_window_to` と同じ捕捉の中で回し、`[diag.window_move]` の行で捕捉が生きていることを確かめてから `Clamp` が 0 行。印の入った巡はちょうど 1 行（`how="pushed"`・`overlap_scope` の欄なし・位置は `(0,904)` のまま）。印の後の巡も同じやり方で 0 行。`reset_pos`・`chain_realign` の 2 本は設計の決めどおり名前と注だけでよいが、注に「判定し直しが起きたかはこの檻では見ない（系の檻が持つ）」と書く。
- **要件**: 3.5・7.6
- **design.md の所**: Testing Strategy「書き換える檻・除く檻」の `rejudge_after_chain_finalize_…` の行・`reset_pos_tests.rs`／`chain_realign_script_moved_tests.rs` の行・Open Questions の議題 3

### 問題 3: 直した後に回すテストの範囲に `areka-runtime` が入っていない

- **何が問題か**: 設計は「直した後に `areka-placement` と `areka` の全部のテストを回す」と書き、続けて「フレームの檻（`crates/areka/src/runtime_tests/`・`crates/areka-runtime/src/`）が赤になったら止める」と書く。止める条件には `areka-runtime` が入っているのに、回す範囲には入っていない。`areka-runtime` のテストもバルーンの書き込みで書き込みの関門を通る。
- **なぜ困るか**: 並べ替えを暗に前提にしたテストがあると、直した後の確かめで見落とし、完了の全体テストで初めて赤になる。静的に見た限りでは、`areka-runtime` で `balloon_limit: true` を使うのは `frame_chain_realign_adjacency_tests.rs`（バルーンがキャラクター窓の中に重なる形）と `move_cue_apply_move_tests.rs`（ずらしを保つことだけを見る）の 2 本で、どちらも並べ替えの形にならない見込み。危うさは小さいが、確かめの手順としては欠けている。
- **直し方**: 回す範囲を「`areka-placement`・`areka`・`areka-runtime`」にする（または workspace の全体）。あわせて、名前を変える檻のファイルを名指す古い注（`follow_other_chars_tests.rs` 147 行の「`balloon_limit_flip_tests.rs` と同じ形」）も、注を直すファイルの一覧に足す。
- **要件**: 7.2・8.2・8.5
- **design.md の所**: Testing Strategy「緑のまま残る檻」の最後の段落・File Structure Plan の `follow_other_chars_tests.rs` の行

## 良い点

1. **削る直しとして正しく小さい**: 案 ⑵ では今の純関数が `limit_correction` と同じ答えしか返さないことを読み取り、関数・型・引数・欄・重なりの列を一緒に外して、離したときの補正と同じ「関門が `limit_correction` を直に呼ぶ」形に戻している。新しい部品も依存も足さず、同じウェーブの約束（`ghost_session.rs`・`frame/` に触らない）を守り、`follow.rs` の取り込みの行で `audit-emo2-boot-frame` と近づくことも先に挙げている。
2. **数と呼び手の裏付けが確か**: 外す名前の呼び手はリポジトリ全体で設計の一覧と一致し、檻の一覧（`balloon_limit_flip_tests.rs` の 24 本・`follow_other_chars_tests.rs` の 8 本と 4 本）も実物と合う。手で計算した期待の位置（10-30 の `(0,336)`・11-16 の `(2592,1188)`・直す前の `(1108,336)`・`(2441,1188)`・変わらない 4 つの形）はどれも計算どおりだった。

## 最後の判定

- **判定: GO**
- **理由**: 要件の全部に当てる所があり、既にある仕組みの形に沿った削る直しで、設計の挙げていない呼び手や依存は無かった。3 件の問題はどれも檻の主張と確かめの手順の細部で、設計の形を変えずに design.md の Testing Strategy を数行直せば済む。
- **次の段**:
  1. 設計の話し合い（`kiro-design-discussion`）で、上の 3 件を design.md の Open Questions の議題 1〜8 と一緒に扱う。
  2. 直した design.md で `/kiro-spec-tasks balloon-flip-ssp-review` へ進む。タスクの順は「新しい檻（直す前のコードでビルドが通る形）を足して赤を確かめる → コードを直し、外した名前を名指す檻を同じタスクで書き換える → 3 つの crate のテスト → §8 の書き直しと檻の扱いの記録」。
