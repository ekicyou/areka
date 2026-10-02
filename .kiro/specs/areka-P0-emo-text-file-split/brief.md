# Brief: areka-P0-emo-text-file-split

> 2026-10-02 棚卸⑳で起票。シェル内バルーン・文字組み・早送り・印・文字の現れ方の spec 群（10 本以上）が、同じ大きいファイルをそれぞれ別の形で分割しようとしていたのを、先に 1 本で済ませる。file:line は起票時値（main `03e8d7d6`）。

## Problem

バルーンの文字まわりのソースに、1 ファイル 1,000 行の上限に張り付いたファイルが 6 本ある。これから着手する spec はどれも、そのどれかに行を足す。

| ファイル | 行数 | 足す予定の spec |
|---|---|---|
| `crates/areka-emo-text/src/actor.rs` | 975 | `shell-balloon`・`balloon-font-file`・`text-reveal-fade`・`balloon-markers`・`anchor-tag-canon` |
| `crates/areka-emo-text/src/layout.rs` | 977 | `text-typesetting`・`text-ruby`・`balloon-markers`・`text-align-shadow-canon` |
| `crates/areka-emo-text/src/viewbox_draw.rs` | 914 | `text-typesetting`・`text-reveal-fade`・`balloon-font-file`・`choice-marker-styling`・`anchor-tag-canon` |
| `crates/areka-emo-text/src/viewbox.rs` | 871 | `balloon-markers`・`text-reveal-fade`・`balloon-scroll-fade` |
| `crates/areka/src/input_events/balloon.rs` | 930 | `talk-fast-forward`・`balloon-markers`・`shell-balloon`・`anchor-tag-canon`・`balloon-lifecycle-events` |
| `crates/areka/src/emo2_boot/balloon_visibility.rs` | 923 | `shell-balloon`・`balloon-lifecycle-events` |

各 spec の brief は「分割が先」と書くだけで、誰がどう切るかを決めていない。任せると、同じファイルを別々の spec が違う形で切り、互いの移動に巻き込まれる。1 行足すだけの spec（テストの登録の 2〜3 行）でも上限に当たる。

## Current State

- 番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外の表つき）。上の 6 本は表に無い＝1,000 行を超えた時点で赤になる。
- `layout.rs` は 343〜765 行目が 1 本の関数 `layout_with_cursor_warn`（約 420 行）で、楽に外へ出せるのは `visible_window`（766 行〜）と `finish_*`／`apply_pending_*`（828〜933 行）。関数そのものを割るかどうかは設計で決める。
- `region.rs`（977 行）は本体 488 行＋内蔵テスト約 490 行。`shell-balloon` は本体を変えなくて済む見込み（棚卸⑳の再測定）なので、やるなら内蔵テストを兄弟ファイルへ出すだけで足りる。
- 前例は完了 `areka-P0-file-slimming`（1,000 行超 54 → 0）。同じ流儀（振る舞いを変えない・テストは兄弟ファイル・`#[path]` で子モジュール）で進める。

## Desired Outcome

1. 上の 6 本が、それぞれ後続の spec が足す余地（目安 700 行以下）を持つ形に分かれている。
2. 振る舞いは 1 つも変わらない。既存のテストは 1 本も書き換えずに緑のまま（`use` の付け替えと場所の移動だけ）。
3. 公開している名前（`pub`・`pub(crate)`）の道筋は変えない。変えざるを得ないものは `pub use` で今の道筋を残す。
4. 分け方の理由（どの spec がどこへ足すか）を、分けた先のモジュールの doc に 1〜2 行で残す。
5. 番人の例外の表には触らない。

## Approach

役割で切る。行数を合わせるための機械的な切り方はしない（後続の spec が「どこへ足すか」を迷わない切り方にする）。

- `actor.rs`: 実行時の状態の構造体（`TextLayerRuntime`）／1 コマの描画の流れ（`present_actor`）／指令の振り分け（`dispatch_block`）を分ける。
- `layout.rs`: 配置の本体／見える範囲の計算（`visible_window`）／仕上げ（`finish_*`・`apply_pending_*`）を分ける。本体の関数を割るかは設計で。
- `viewbox.rs`・`viewbox_draw.rs`: 計画（どこを描き直すか）／描く（装飾つきの描画）を分ける。
- `input_events/balloon.rs`: 選択肢のクリック／バルーンの中断（ダブルクリック）／ドラッグを分ける。
- `emo2_boot/balloon_visibility.rs`: 見える・隠すの判断／時間切れ／窓への反映を分ける。
- `region.rs`: 内蔵テストを兄弟ファイルへ（任意・設計で決める）。

## Scope

- **In**: 上の 6 本（と任意で `region.rs`）の振る舞いを変えない分割・`use` の付け替え・doc の 1〜2 行。
- **Out**: 振る舞いの変更すべて／キーや欄の追加／`crates/areka-emo-text/src/state.rs`・`crates/areka-parsers/src/balloon/` の作り替え（後続の spec がそれぞれ行を足す場所で、分割しても共有は消えない）／テストファイルの分割（テストは後続が新しい兄弟ファイルへ書けば太らない）／1,000 行に遠いファイル。

## Boundary Candidates

- emo-text の 4 本（`actor`・`layout`・`viewbox`・`viewbox_draw`）
- `crates/areka` の 2 本（`input_events/balloon.rs`・`emo2_boot/balloon_visibility.rs`）

## Out of Boundary

- 文字まわりの spec が共有する表（`CueCommand::Custom` の腕・`BalloonModel` の欄・`TextLayerRuntime` の欄）を並走できる形へ作り替えること。分割しても、この共有は残る。**文字まわりの spec は本 spec の後も互いに直列**である。

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-file-slimming`（流儀）。
- **Downstream**: `areka-P0-shell-balloon`（直後）・`balloon-font-file`・`text-typesetting`・`talk-fast-forward`・`text-ruby`・`balloon-markers`・`balloon-scroll-fade`・`text-reveal-fade`・`text-align-shadow-canon`・`choice-marker-styling`・`anchor-tag-canon`・`balloon-lifecycle-events`。各 brief の「分割が先」は本 spec で済む。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 完了 `areka-P0-emo-text-layer` ほか emo-text を作った spec 群（振る舞いの正本・変えない）。

## Constraints

- 触るのは上の 6 本（と `region.rs`）と、分けた先の新しいファイル、`crates/areka-emo-text/src/lib.rs`・`crates/areka/src/input_events/mod.rs`・`crates/areka/src/emo2_boot/mod.rs` のモジュールの宣言だけ。**`crates/areka/src/emo2_boot/frame/`・`crates/areka/src/placement/`・`crates/areka/src/install/`・`crates/areka/src/main.rs`・`crates/areka-sakura/`・`crates/wintf/` には触らない**（同じウェーブの他の spec が触る）。
- 全体テスト（`tools/test-all.ps1`）が、分割の前後で同じ本数・同じ結果であること。
- 実機の確かめは 1 回（emo2 を起こして会話・選択肢・中断が今日と同じ）。

## 想定

- 規模 S（5〜8 タスク＝1 ファイル 1 タスク＋仕上げ）。議題 1 件（`layout_with_cursor_warn` の 420 行の関数を割るか）。Opus で足りる。
