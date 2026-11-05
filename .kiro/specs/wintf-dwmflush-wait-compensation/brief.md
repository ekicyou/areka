# Brief: wintf-dwmflush-wait-compensation

> 起票: 2026-11-04（`/kiro-discovery`・開発者の依頼「DwmFlush を待機する tick ループに『待てなかった分だけ別手段で補う』処理を入れる」）。決めごとは同じ日の discovery で開発者と詰めた。
>
> **2026-11-05 `audit-wintf-runtime` に吸収**（要件の討議・開発者承認）: この brief の直す中身は `audit-wintf-runtime` が audit の 2・8 番として、ここに書いた形のまま直す。この spec は単独では着手しない。台帳からは `audit-wintf-runtime` の完了のときに閉じる。

## Problem

- `DwmFlush()` は、S_OK を返しても**待たずにすぐ戻る**ことがある（ディスプレイ消灯・ロック画面・リモートデスクトップ・モニター構成の変更中など）。HRESULT では見分けられない。
- vblank のスレッド（`crates/wintf/src/runtime/tick_bridge.rs` の `vsync_loop`）は、`Ok` なら notify してすぐ次を呼ぶ。待たずに戻ると空回りになり、CPU が張り付く。tick も毎秒数千回回る。
- `Err` の腕は 15ms 眠るだけで、**notify を撃たない**。tick を起こすのはこの Event だけなので（昔の `try_tick_on_vsync` の経路は、カウンタを増やす側がもう無い）、`Err` が続くあいだは次のものが全部止まる。ログも出ない。
  - ECS の tick
  - 窓の書き込み指令の掃き出し
  - アニメ
  - クリック透過の見直し
- 同じファイルの `AsyncTickTask` の注記には「固定 60Hz なのは `Err` 腕のフォールバック」とあり、実際の動きと食い違っている。

## Desired Outcome

- どんな戻り方をしても、vblank のスレッドは空回りしない。消灯・ロック・RDP のあいだも CPU が張り付かない。
- ちゃんと待てた回には何も足さない（今と同じ周期で、画面の更新周期に従う）。
- 1 周に必ず 1 回 notify する（`Err` の回も）。

## Approach（discovery で決めた形）

```rust
while !stop.load(Ordering::Acquire) {
    let start = Instant::now();
    let flushed = unsafe { DwmFlush() };
    if let Err(e) = &flushed { /* debug の 1 行 */ }
    if let Some(wait) = compensation(start.elapsed()) {
        thread::sleep(wait);
    }
    event.notify(usize::MAX);
}
```

1. **待ったかどうかは時間だけで決める。** 呼ぶ前後を `Instant` で測る。HRESULT は待ちの判定に使わない（失敗した `DwmFlush` は待たずに戻るので、時間だけで補われる）。
2. **判定の線は 1ms 未満。** 空回りの戻りは数十 µs、500Hz の画面でも 1 周期は 2ms。このスレッドは戻ってから notify するだけで次を呼ぶので、ちゃんと待てた 1 回はほぼ 1 周期まるごとになる。だから「呼ぶ前後の時間」だけで見分けられる。
3. **補うときは、呼ぶ前から数えて 1 フレーム分（約 16ms・今の `Err` 腕と同じ）になるまでの不足分を待つ。** 1ms までの不足分だけを待つと、高分解能のタイマーでは毎秒数百回 tick が回る。
4. **待つ手段は `std::thread::sleep`。** タイマーはカーネルのもので DWM に縛られない。粗くなったとしても長く眠るだけなので、空回りにはならない。
5. **状態を持たない。** 連続の回数のカウンタもモードの切り替えも持たず、毎回判定して、補いはその回だけ。
6. **notify はループの最後の 1 か所。** `Err` の腕だけ notify を抜かす枝を無くす。
7. **HRESULT はログにだけ使う。** `Err` は debug の 1 行（HRESULT つき）。補った回は trace。どちらも熱い経路なので、カウンタで間引くことはしない。
8. **注記を直す。** `AsyncTickTask` の注記の「60Hz のフォールバック」を、実際の動き（待てなかった回だけ約 16ms 待って起こす）に合わせる。

アニメは実時間で進んでいる（`FrameTime` は `dola::runtime::clock::now()`）ので、補いで tick の数が減っても動きの速さは変わらない。

## Scope

- **In**:
  - `vsync_loop` の作り直し
  - 判定の純関数 `compensation(waited) -> Option<Duration>`
  - 1 回分の処理に、時間・`DwmFlush`・待ち方を差し込める形
  - 単体テスト（ちゃんと待てた回は補わない・空回りの回は約 16ms・線のちょうど上と下・`Err` の回も notify が 1 回）
  - 注記の直し
  - 実機の確かめ
- **Out**:
  - `crates/pilot/examples/wintf-winmsg-executor` の同じ形のループ
  - DXGI のフレームの統計や待ち可能なタイマーへの置き換え
  - tick の門（`tick-gate-adoption`）
  - 刻みの締め切り（`instant-to-interrupt-ticks`）

## 確かめ方

- 単体テスト: 上の Scope のとおり。実 DWM は呼ばない。
- 実機（emo2）: 今あるスレッド別 CPU の報告（役割 `vblank` と UI スレッド）と trace のログで、3 つの場面で張り付かないことを確かめる。

| 場面 | 起こし方 | 開発者の手 |
|---|---|---|
| ディスプレイ消灯 | AI が `SC_MONITORPOWER` で消す | なし |
| ロック画面 | AI が `LockWorkStation` でロックする | 解除のログインだけ |
| リモートデスクトップ | 接続して眺める | 接続だけ |

- 調べの段で 1 つ確かめる: 今のツールチェーンの `std::thread::sleep` が Windows で高分解能の待ち可能タイマーを使うか（Rust 1.75 からの見込み）。答えがどちらでも作りは変わらない（補いは約 16ms なので）。

## Boundary

- 触るファイル: `crates/wintf/src/runtime/tick_bridge.rs`（テストが大きくなるなら兄弟のテストファイル）だけ。
- 依存は足さない（`Cargo.toml` に触らない）。

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**（同じウェーブに置かない）:
  - `audit-wintf-runtime`（C17）: `src/runtime/` を受け持つ。あちらがまだ始まっていなければこちらを先に着地させ、新しいコードも見直しに入れる。
  - `tick-gate-adoption`（保留）・`zorder-chain-residue`（据え置き・`tick_bridge.rs` の vblank の期限のテスト）: 同じファイルに触る。
- `instant-to-interrupt-ticks`・`dola-interrupt-ticks` は「`DwmFlush` の待ちは変えない」としているので、重ならない。

## Constraints

- 区分 A（開発者が起票を頼んだ）・規模 S（4〜6 タスク）・版に縛らない。
- 状態を持たない（開発者の要件）。
- 時刻は正確に扱う（設計の大原則）。補いの長さは実時間で測った不足分から決める。
