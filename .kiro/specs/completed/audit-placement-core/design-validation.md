# 設計の検証: audit-placement-core

- 検証した日: 2026-11-14
- 読んだもの: `spec.json`（language ja・phase design-generated）・`requirements.md`・`design.md`・`research.md`（2 節の 1〜3・4 節・11〜13 節）・steering の `structure.md`（台帳の決まりの所だけ）と `tools/test-all.ps1`、設計が名指すコード（下の表）。`brief.md` と steering のほかのファイルは、要件と設計に写されている分で足りたので読み直していない
- 手順: `kiro-validate-design`（分析 → 大きな問題 → 良い点 → GO／NO-GO）。対話なしで判定まで出す。

## 要約

80 件の指摘は、28 の実装の単位（U1〜U28）に 1 件ずつ、漏れも重なりもなく割り当ててある（数えて確かめた: 80 件・重なり 0・抜け 0）。C20 の約束と実装の順（先の段 4 本の着地の後に始める・指摘 80 を最後にする・47・67・73 は相手の着地の後）も要件のとおりに守られている。ただし、振る舞いを変える指摘 1・17 の結果を書いた範囲の外の文書を直す計画が無いこと、指摘 2 の直しが一部の構成で効かないまま開発者の裁定を経ていないこと、指摘 4・61 の確かめが全体テストで走らないことの 3 点を、設計の討議で決める必要がある。

## コードと照らした確かめ（設計の主張のうち、外れると直しが崩れるもの）

| 設計の主張 | 確かめた結果 |
|---|---|
| 指摘 1: 今は `top`／`left`／`right` でも下端と右からの連なりで解き、`anchor` だけが辺を指す | 正しい。`resolver.rs:L278-L318` の P1〜P4 は Seam でも `bottom_y`・`bottom_x` を使い、`L392` の `anchor` は `Anchor::from_alignment`。`follow/anchor.rs:L144-L163` の `project_anchor` は上の辺で x を保ち、左右の辺で y を保つので、設計の「解いた位置は `project_anchor` を通しても変わらない」は成り立つ |
| 指摘 1: Seam の `warn!` は知らない値のときだけ | 補足が要る。`from_alignment`（`resolver.rs:L466-L472`）は知らない値に既に `warn!` を出す。`resolver` の側の `warn!` も残すと、知らない値で 1 スコープにつき 2 行になり、設計のテスト（呼び出し 1 回につき 1 行）と合わない。どちらか 1 つにする（タスクの段で決めてよい） |
| 指摘 2: 比べは `f.offset() == before.placement.balloon_offset_base.offset`（`face_windowposition.rs:L62-L64`） | 正しい。追随の値は `rescale_follow_offset`（`follow/offset_space.rs:L208-L246`）が基準を 1 回で換算した値で、解き直しは部品ごとに換算して足した値なので、丸めの差が出る |
| 指摘 3: `image` 0.25.10 の `PngDecoder::new` は `Limits::no_limits()` | 正しい。レジストリのソース `image-0.25.10/src/codecs/png.rs:L42-L43`。`Cargo.lock` の版も 0.25.10。`read_first_frame`（`image_arm.rs`）は寸法を確かめずに `open` するので、`super::pixel_len` を先頭に 1 行足す直しで足りる |
| 議題 5: `test_open` の私有の子モジュールに置き、機能 `test-support` の有無で `pub use`／`pub(crate) use` を切り替える | 組める。狭める名前（`evaluate`・`DpiSyncDecision`・`DpiSyncOutcome`・`DPI_SYNC_HOLD_MAX_FRAMES`・`HoldReason`・`BALLOON_LIMIT_CLAMP_TAG`・`PREPARE_GHOST_WINDOWS_CONTEXT`）は、本体の本番のコードからは使われていない（本番が使うのは `dpi_sync::{current_frame, apply_dpi_phase_gate, defers_window_write, DpiSyncHold, HoldSite}` で、狭めない）。crate の中の本番もそれぞれ使うので、`pub(crate) use` が使われない取り込みの警告にならない。`follow.rs:L52-L55` の `BALLOON_LIMIT_CLAMP_TAG` は `pub use` でなく私有の `use` なので、追随の持ち物を直さずに済む。`test-support` を付けるのは本体の dev-dependency（`crates/areka/Cargo.toml:L97`）だけ |
| 指摘 17: 鍵を消しても、別の書き込みで標準のずらしが書き戻されない | 正しい。`BalloonOffset` を記憶へ書くのは、バルーンのドラッグの確定（`follow/drag_follow.rs:L771`）と初期化（`reset_balloon_pos.rs:L99`）だけ。終了のときの書き込み（`persist.rs:L898`）はキャラ窓の位置だけ |
| 1,000 行の番人の見込み | おおむね妥当。ただし表に無い、行が増えるファイルがある: `areka-emo-present/src/shell_target.rs`（今 956 行・指摘 5 で入口を足す）・`areka-sylphya/src/actor.rs`（今 854 行・指摘 17 で腕を足す）。タスクの終わりの行数の確かめに、この 2 本も入れる |

## 大きな問題（3 件）

🔴 **大きな問題 1**: 指摘 1・17 で変わる振る舞いを書いた、範囲の外の文書を直す計画が無い
**中身**: 指摘 1 の後、`doc/ukadoc-coverage/ledger/assets.toml` の `seriko.alignmenttodesktop` の 8 項目（descript_ghost 4・descript_shell 4・どれも `status = "degraded"`）の「起動で窓を最初に置く既定の位置だけは `bottom` と同じに解き、記録を残す」が事実でなくなる。指摘 17 の後、`doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![execute,resetballoonpos]` の項目と `doc/COMPAT_ARCHITECTURE.md` の 8 章（L152）の「戻したずらしを位置の記憶に書く」も事実でなくなる。設計の Boundary Commitments と Modified Files には、どれも挙がっていない。
**影響**: 互換の判定の台帳と互換の決めごとの登記が、コードと逆のことを書いたまま残る。台帳の常設の検査は「縮退」の項目に証拠を求めないので、テストは緑のまま気づけない。`structure.md:L462` は「互換機能を着地させた spec は、同じ PR で台帳の該当項目の判定も更新している」と書いている。
**直し方**: U2 に台帳の 8 項目（判定と説明）の書き直し、U15 に台帳の 1 項目と `COMPAT_ARCHITECTURE.md` 8 章の 1 行の書き直しを足し、Boundary Commitments の「範囲の外の、直しに要る最小の箇所」に 3 つのファイルを足す。触る前に `git grep` でソースから読まれていないことを確かめる（`kiro-complete` の決まり）。
**つながる要件**: 4.3（直し切る）・5.1 の 3 種類目（議題 1・3）・7.4
**設計の箇所**: 「Boundary Commitments」・「File Structure Plan」の Modified Files・U2・U14・U15

🔴 **大きな問題 2**: 指摘 2（必ず直す）の直しは一部の構成で効かず、その残りを開発者の裁定なしに残している
**中身**: 軸ごとに 3px の差を許す形は、基準の拡大率が 1 で、移った先との比が 3 未満のときだけ差が 3px に収まる（`research.md` 11.2）。基準の拡大率が 1 でない構成（作者 96・主モニタ 144 の机は多い）で拡大率が上がる移りと、比が 3 以上の移り（96→288 の 300% も Windows の普通の設定）では、今と同じく置き直されない。あわせて、利用者が 3px 以内だけ動かしたバルーンを「動かしていない」と取る新しい取り違えが生まれる。`shortcut:` の注釈は前の場合だけを書き、比が 3 以上の場合を書いていない。テストは 96→144 だけを固定する。
**影響**: 要件 4.2 は「裁定で残すと決めたもののほかは直さずに残さない」。設計は「follow の持ち物に触れないので議題へは戻さない」としたが、残る不具合があるので、残すかどうかは開発者が決める事柄である。
**直し方**: 設計の討議の議題にし、次のどちらかを選んでもらう。(a) 3px の形で行き、残る構成（基準の拡大率が 1 でない上りの移り・比が 3 以上）と 3px 以内の取り違えを `shortcut:` と完了の申し送りに書き、境のテスト（差 3px は置き直す・4px は置き直さない）を足す。(b) 先の段の `audit-placement-follow` の着地の後に、相手と申し合わせて「動かした印」の形で直す。
**つながる要件**: 4.1・4.2・6.6・8.1・8.6
**設計の箇所**: U4・「設計の段で決めた事項」1

🔴 **大きな問題 3**: 指摘 4・61（必ず直す）の確かめが、全体テストで走らない
**中身**: 確かめは手本 `examples/window-placement.rs` の中の `assert_eq!` だけで、手で走らせたときにしか落ちない。`tools/test-all.ps1` の `cargo test --workspace` は手本を組むだけで、走らせない。
**影響**: 要件 6.6 の「直しが壊れたら落ちるテスト」が、毎回の確かめの中に無い。手本のスコープの取り違えがまた入っても、全体テストは緑のまま。
**直し方**: 手本の中で「スコープごとに貼る面を選ぶ」所を小さな純粋な関数にし、手本の中に `#[test]` を置いて、配置の `Cargo.toml` の `[[example]]` に `test = true` を足す（`cargo test --workspace` で走る・GPU は使わない・`async-io` の行とは別の行で、追随の `log-capture-kit` の行にも触れない）。この形をとらないなら、4・61 を要件 6.6 の例外として理由とともに設計に書き、討議で開発者の承認を取る。
**つながる要件**: 6.6・6.7
**設計の箇所**: U7・「Testing Strategy」の表の 4・61 の行

## 良い点

- 80 件を 28 の単位へ 1 件ずつ割り当て、要件の 1〜10 の各項目から設計の要素への対応表を持つ。並走の相手ごとに「何が重なり・どちらが先か・どう申し合わせたか」が実装の順の節に書いてあり、`kiro-impl` の進行役が迷わずに順を守れる。
- 公開を狭める直し（議題 5）を、3 種類の置き方の表 1 つに決めた。`pub(crate)` を `pub use` で開き直せない（E0364）点まで確かめたうえで、道と名前を変えずに済む形を選んでおり、追随の持ち物（`pt/transition_judge_verdict.rs`・`follow.rs`）を直さずに済む。コードと照らしても組める。

## 小さな注意（大きな問題には入れない・タスクの段で拾えばよい）

- 「Testing Strategy」の冒頭の「`research.md` 4 節の 25 件に 4・36 を足した 27 件」は、4 節の一覧（23 件で、4・36 を既に含む）と数が合わない。表の行そのものは 23 件と 50・61 を受け持っており、抜けは無い。
- 指摘 42 で `prepare_stages` を消すと、範囲の外の `crates/areka/examples/collision-probe.rs` の doc（`L107`・`L111`）の名前と行の番号が古くなる。要件 7.4 に従い、持ち主の spec へ申し送る。
- 指摘 1 の Seam の `warn!` の重なり（上の照らした確かめの表の 2 行目）。

## 判定

**GO**（上の 3 件を設計の討議で決めることを条件とする）

**理由**: 今の構造の中で直す形は、crate の層と依存の向きを変えず、新しい依存も足さない。80 件の割り当て・C20 の約束・実装の順に欠けは無く、議題 5 の公開を狭める形や指摘 3・17 の土台の主張もコードで確かめられた。3 件はどれも、設計の文と単位の中身に数行を足すか、開発者の 1 つの決めで閉じる。

**次の段**:
1. 設計の討議（`/kiro-design-discussion audit-placement-core`）で 3 件を議題にする。問題 1 は推奨の直し方で設計へ足す。問題 2 は (a)・(b) を開発者が選ぶ。問題 3 は推奨の形か、例外の承認かを決める。
2. 決まったことを `design.md` に反映してから、`/kiro-spec-tasks audit-placement-core` でタスクを作る。
