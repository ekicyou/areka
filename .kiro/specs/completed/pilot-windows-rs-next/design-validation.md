# 設計の検証: pilot-windows-rs-next

- 検証した日: 2026-11-16
- 読んだもの: `spec.json`（language `ja`・phase `design-generated`）・`requirements.md`（要件 1〜15・確定 1〜10）・`design.md`・`research.md`・`brief.md`・steering（`two-tunnel.md` ほか）
- コードで確かめた所: `crates/log-capture-kit/tests/workspace_scan/mod.rs`（`EXCLUDED_DIRS` は `target`・`vendors`・`.git` だけ）・`crates/areka/src/session_end_sync_send_tests.rs`（読むのは `crates/*/src/**` だけ）・`tools/test-all.ps1`（fmt・test・deny・about はどれもワークスペースの一員だけ）・`crates/pilot/` の例の自動の発見（深さ 1 だけ）・根の `Cargo.toml` の `[patch.crates-io]`・`crates/areka-emo-text/tests/support/`（今の側の GPU の World は MTA で組む）・上流 `f37e2f90` の `crates/libs/bindgen/src/lib.rs`（既定の形が暗に参照する兄弟のクレート）と `crates/libs/` の一覧

## 設計の検証の要約

設計は要件 1〜15 の全部の受け入れ基準を部品と手順へ対応させており、置き場の決まりは本線の見張りの試験の実物どおりで、本線を汚さない形として成り立つ。上流の事実（`--reference` が無い・`SetWindowLongPtrW` の別名・マクロの `_Impl`・`==` の導出）も固定のコミットのソースに当たって書かれている。直すべき所は、共有の束縛 `bind-next` の依存の一覧の抜け・問い 10 の隔てが指示だけであること・問い 7 の比べの条件が決まっていないことの 3 つで、どれも設計の討議の中で直せる。

## 重要な指摘（3 つ）

🔴 **指摘 1: `bind-next`（既定の形の WinRT）が要る上流のクレートが「許す依存」と図に無い**
**Concern**: 上流 `f37e2f90` の `bindgen/src/lib.rs`（`prepend_default_refs` の呼び手・530〜570 行あたり）は、既定の形のとき `Windows.Foundation` の型を `windows_future`（`IAsyncAction` ほか）・`windows_collections`・`windows_reference`（`IReference`）・`windows_time`（`TimeSpan`・`DateTime`）・`windows_numerics` へ回す。Composition と DispatcherQueue の束縛はこのうち少なくとも `windows_future`（`ShutdownQueueAsync`・`RequestCommitAsync`）と `windows_time`（アニメーションの `Duration`）を名指しする見込みで、どれも `crates/libs/` に実在する。設計の「Allowed Dependencies」・技術の表・依存の図には `windows-numerics` しか無い。
**Impact**: 図は「図に無い矢印は誤り」と強く決めているので、`bind-next` が最初のビルドで止まったときに、実装係が足すとレビューで差し戻され、足さないと組めない。
**Suggestion**: 「Allowed Dependencies」と技術の表に `windows-future`・`windows-collections`・`windows-reference`・`windows-time`（同じ rev）を足し、図に `BindNext --> Future/Collections/Reference/Time` を描く。実際に要った物だけを残すと書き、要った物を一次記録の問い 4 の節に書く（本坑の 1 本目の材料になる）。
**Traceability**: 2.3・2.4・5.1
**Evidence**: design.md「Boundary Commitments > Allowed Dependencies」・「Architecture Pattern & Boundary Map」の図と「依存の向き（強制）」・「Technology Stack」の上流の行

🔴 **指摘 2: 問い 10 の「対応表だけを渡す」が指示だけで、答えが読める**
**Concern**: Sonnet の実装係は同じ作業木で走り、移し終えた問い 1 の写し（`q1-winmsg/src/`・`build.rs`・`filters.txt`）がそのまま読める。しかも `q10-winmsg` の例は `../q1-winmsg/examples` を path で指すので、隣のフォルダへ目が向く。設計の手当ては「見せない」という指示だけである。
**Impact**: 問い 10 の答えは移行の本体 6〜8 本の実装係のモデル選びとレビューの見る点を決める。実装係が答えを読んだかを後から示せないと、「対応表だけで移せた」の結果が証拠にならない。
**Suggestion**: 次のどちらかを設計に書く。⑴ 作業木の `target\pilot-windows-rs-next\q10-sandbox\` に `q10-winmsg/`・`REWRITE_TABLE.md`・例 2 本の写し・最小の `[workspace]` だけを置いて、そこで走らせる（要件 15.2 の内側）。⑵ 同じ作業木で走らせる代わりに、下請けの道具の呼び出しの記録から `q1-winmsg/src`・`build.rs`・`filters.txt` を読んでいないことを確かめ、その旨を一次記録に書く。手間が小さいのは ⑵、強いのは ⑴。
**Traceability**: 11.1・11.3・13.2
**Evidence**: design.md「q10-winmsg（問い 10）> Responsibilities & Constraints」・File Structure Plan の `q10-winmsg/Cargo.toml` の行

🔴 **指摘 3: 問い 7 の画素の比べで、新しい側の描く条件を今の側に揃える決めが無い**
**Concern**: `vt-compare` の流れは「CoInitializeEx via bind-next」とだけ書き、アパートメントを決めていない（今の側の GPU の World は `tests/support/fixture.rs` のとおり MTA で組み、本番の UI スレッドも MTA）。また新しい側の `DrawTextLayout` の条件は「カラーフォントの指定つき」だけで、文字のアンチエイリアスの形・描画の設定（rendering params）・DPI・面の画素の形式・筆の色を今の側（`areka-emo-text` の `draw.rs` の経路）に揃えるとは書いていない。
**Impact**: 条件がずれると、`windows-text` の違いと描く設定の違いが同じ差の画素に混ざる。±8 px の平行移動で取れるのは位置のずれだけで、設定の違いは残る。閾値を置かず開発者が目で見て決める（確定 4）ので、差の出どころが読めないと判定の材料にならない。
**Suggestion**: `vt-compare` を MTA で始めると書き、`new_side.rs` が揃える条件の一覧（`D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT`・文字のアンチエイリアスの形・DPI 96・B8G8R8A8 の premultiplied・筆の色）を、今の側の該当の行を名指しして設計に書く。揃えられない条件があれば、その条件を一次記録の問い 7 の節に書くと決める。
**Traceability**: 8.1・8.2・8.3・13.5
**Evidence**: design.md「System Flows > 問い 7 の比べ（vt-compare）」・「q7-vertical > Responsibilities & Constraints」の `new_side.rs` の項

## 設計の強み

1. **置き場の決まりが本線の見張りの実物に基づいている**: 走査が外すフォルダ名（`target`・`vendors`・`.git`）・`session_end_sync_send_tests.rs` が `crates/*/src/**` しか読まないこと・全体テストの段がワークスペースの一員しか見ないこと・`crates/pilot` の例の発見が深さ 1 だけであることを、本書でもコードで確かめた。`.cargo/config.toml` の `../../../../target/...` も作業木の根を正しく指す。表の形で「レビューで見る」と決めてあり、本線を赤にしない作りとして通る。
2. **上流の事実を固定のコミットで当たり、問いの答えの形を先に絞っている**: `--reference` の旗が無いこと・`SetWindowLongPtrW` の 32bit の別名・マクロの `_Impl` が宣言の戻り値のままであること・`.sys()` に COM の型と `==` が無いことを `f37e2f90` のソースの箇所つきで書き、問い 2・5・6・8・12.4 の確かめを「見込みを実物で確かめる」形に絞った。要件の対応表も 15 の要件の全部の基準に部品と手順が付いている。

## 小さな気付き（重要な指摘には入れない）

- 根の `Cargo.toml` の `[patch.crates-io] image-webp`（git）は置き場のワークスペースに効かない。借りる `areka-emo-text` → `areka-emo-compose` → `areka-emo-atlas` → `image` の木で `image-webp` だけは本線の `Cargo.lock` の種どおりにならず、cargo が crates.io の版へ黙って解き直す見込み。縦書きの画素には効かないが、「Open Questions」の 3（`Cargo.lock` の種）の討議で、同じ `[patch]` を置き場の根に写すか（要件 2.2 の禁止は上流の windows-rs だけ）、ずれを一次記録に書くだけにするかを決めるとよい。
- RunCommands のコマンドは `C:\home\maz\git\areka` を根に書いている。実装はハーネスの作業木で行うので、一次記録には作業木の根で書くか、根の差し替えを先頭に書くとよい。
- 問い 9 の数えは `crates/pilot` だけを外し、試験の道具のクレートも数える。要件 10.1 の「出荷するクレート」と範囲が少し違うので、クレートごとの表に出荷するかの欄を足すと、本体の規模の見立てに直に使える。

## 最終判定

- **判定: GO**
- **理由**: 本線との境界・依存の向き・見張りの決まり・上流の固定がどれも実物に合っており、要件の全部の基準に実装の道筋がある。3 つの指摘は設計の文の追記で直せる範囲で、方向の誤りや組み立ての作り直しを要する物ではない。
- **次の段**: 設計の討議（`/kiro-design-discussion pilot-windows-rs-next`）で、上の指摘 1〜3 と「Open Questions / Risks」の 14 項を扱い、design.md へ反映する。その後 `/kiro-spec-tasks pilot-windows-rs-next` でタスクを作る。
