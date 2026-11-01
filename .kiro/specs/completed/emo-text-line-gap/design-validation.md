# 設計の検証: emo-text-line-gap

- 日付: 2026-11-01
- 対象: `design.md`（`0de38a80`）・`requirements.md`・`research.md`・steering（`roadmap.md` の 2026-10-31 C15 の行。`structure.md` などの決まりは design.md が引いた分だけを照らした）
- 進め方: `kiro-validate-design`（分析 → 重い指摘 → 良い所 → GO／NO-GO）。対話なしで判定まで出す。

## 設計の検証のまとめ

案 2（横書きの帯も DirectWrite が測った基線から置く）を、縦書きの前例（`vertical_em_box_left`＝生成時に 1 度だけ測った比 × em）と同じ形で写した設計で、直す所は `choice.rs::line_bands` の横書きの腕 1 か所に集まり、塗り・当たり・ダーティ帯は既存の単一の源のまま値だけが動く。コードとの突き合わせでは、行数・可視性・`None` の枝の不到達・層の檻の主張はすべて事実に合う。ただし「書体の行間 0 の書体は画素が動かない」（要件 5.1）の本番側の確かめの門が緩く、要件 2.3 の読み替えが要件の文に反映されていないので、この 2 点を設計の討議で決めてから実装へ進めるのがよい。

## コードとの突き合わせ（file:line はブランチの先頭 `0de38a80`）

| 設計の主張 | 確かめた所 | 判定 |
|---|---|---|
| 行数 `layout.rs` 551・`choice.rs` 790・`draw_metrics.rs` 462・`viewbox_draw.rs` 388 | `wc -l` | 合う |
| 横書きの帯は `line_bands` の横書きの腕だけが決める | `choice.rs:211-243`（横書き 232-238・縦書き 228-231）。本番の呼び手は `actor_present.rs:591` の `choice_ranges` だけ | 合う |
| `highlight_band_offset = round(max(0, B − E) / 2)` | `choice.rs:175-177` | 合う |
| `line_box_height` の説明が「DirectWrite は行を ascent＋descent で組む」 | `layout.rs:127-140`（130 行） | 合う（直す対象として正しい） |
| `measure_line_box_ratio` が `(ascent + descent) ÷ upem` だけを返す・私的 | `draw_metrics.rs:385-424` | 合う（`measure_font_metrics` への改めは中身の移しで足りる） |
| 縦書きの比の測りが `create_line_layout` 経由・`Err(&'static str)` 型 | `draw_metrics.rs:433-458` | 合う（横書きの測りを同じ型で足せる） |
| `create_line_layout` は `pub(crate)` | `draw.rs:491` | 合う |
| 借りる助けの関数（`Rig`・`live_diff_model_font`・`glyph_items`・`block_axis_ink_span`）が `pub(super)` | `viewbox_draw_test_support.rs:48・121・138・195` | 合う。`bgra_y_span` は私的（`viewbox_draw_vertical_band_readback_tests.rs:45`）＝設計のとおり `pub(super)` へ上げる 1 語が要る |
| 新しいテストのファイルは `viewbox_draw.rs` の子で、`lib.rs` の一覧に 1 行 | `viewbox_draw.rs:350-388`（同じ形の子の宣言が並ぶ）・`lib.rs:427` `SOURCES_OUTSIDE_THE_PURE_SCAN`（`viewbox_draw_vertical_band_readback_tests.rs` が 488 行に在る） | 合う |
| 縦書きの `None` の枝（横書きに束縛した物へ縦書きを問う）は本番で通らない | `draw_metrics.rs:359-368`。本番の生成は `actor_decoration.rs:66` の `new_shared` だけで、`actor_present.rs:250-254` が `resolved.mode` を渡し、同じ `resolved.mode` を `choice_ranges`（327-332 行）へ渡す。`vertical_em_box_left` を本番で呼ぶのは `choice.rs:230` だけ。`DWriteMetrics::new` の呼び手はテストと `examples/` だけ | 合う（research.md §2.2 の結論を支持） |
| t ＝ 0 のとき今の値とビットまで同じ | `0.0 + x`（x ＝ `(B − E).max(0.0) / 2` は 0 以上で有限）は x と同じビット。`B − E` が丁度 0 のときも `+0.0` で符号は変わらない | `FixedMetrics` と既存の偽の計測器（既定 0）では合う。**本番の `DWriteMetrics` は t を測るので、厳密な 0 にならない**（重い指摘 1） |
| 同じウェーブの約束 | C15 の行（`roadmap.md:216`）の②の触る一覧は `draw_metrics.rs`・`choice.rs`・`layout.rs`・`tests/line_pitch_readback_test.rs`・§8。設計は `viewbox_draw.rs`（子の宣言 3 行）・`lib.rs`（1 行・C15 で本 spec だけが使える席＝`roadmap.md:211`）・場合により `viewbox_draw_vertical_band_readback_tests.rs`（1 語）を足し、`tests/line_pitch_readback_test.rs` を外す。ほかの 5 本は emo-text に触らない | 重なり 0（増えた分は設計に理由つきで書いてある） |

## 重い指摘

🔴 **重い指摘 1**: 書体の行間 0 の書体で「画素が動かない」ことの本番側の門が緩い
**懸念**: 本番の `DWriteMetrics` は Yu Gothic UI・ＭＳ ゴシックでも t ＝（基線 − ascent ÷ upem × h）を浮動小数で測るので、厳密な 0 にはならない。設計の確かめは「|比 × h| ＜ 0.5」だが、寄せは `round(t + x)` なので、x の端数が .5 に近ければ 0.5 未満の t でも丸めが変わる。例: Yu Gothic UI 20 は B 26.6・E 22・x 2.3 で、t が 0.2 を超えると寄せが 2 から 3 になる。今の檻が見ているのは 28（x 3.62・余裕 0.12〜0.38）と既定のＭＳ ゴシックだけ（`actor_choice_contract_tests.rs:845-859`・`tests/line_pitch_readback_test.rs:115` の `BAND_OFFSET_28`）。
**影響**: 要件 5.1（1 バイトも動かさない）を、今の檻が無い大きさ（20・k ＝ 2 など）で黙って破れる。
**提案**: 実物の読み戻し（LineGapBandTests の 3）と記録だけの測り（1）で、行間 0 の書体の各組（20・28 × k ＝ 1・2）について、本番の `LineBand::offset` が `highlight_band_offset(B, E)`（t ＝ 0 の式）と**等しい**ことを固定し、t そのものも記録する。門の数は「0.5 未満」でなく「丸めを変えない」に改める。t が浮動小数の誤差（例えば 1e−3 画素）を超えて 0 でないと測れたら、それは要件 5.1 と 2.1 の衝突なので要件 2.7 に従って止まる。
**追跡**: 要件 5.1・5.4・6.2
**根拠**: design.md「DWriteMetrics の測り」Implementation Notes の「確かめ」・「BandOffset」事後条件「t ＝ 0 のとき…」・Testing Strategy「実物の読み戻し」

🔴 **重い指摘 2**: 要件 2.3 の読み替え（同じ大きさの行どうしだけ）が要件の文に反映されていない
**懸念**: 要件 2.3 は「横書きでは隣の行の帯と重ねない」と書体も大きさも限らない。設計は大きさの違う行が隣り合うと Yu Gothic UI でも今から 4 image px 重なる（research.md §2.8）と見つけ、2.3 を「同じ大きさの行どうし」と読む推奨を「開発者の決め」に置いた。読み替えは妥当（要件 2.4・5.1 と両立する唯一の形）だが、要件の文は今のままで、要件 5.1 とは今の形の時点で両立しない。
**影響**: 実装とレビューは要件の文で判じるので、大きさの違う行の重なりが要件の違反として赤や差し戻しの源になる。決めが記録に残らない。
**提案**: 設計の討議でこの推奨を確定し、通ったら要件 2.3 の文を「同じ大きさの行どうしは重ねない。大きさの違う行が隣り合うときの重なりは今の形のまま（本 spec の外）」へ改め、境界の節にも書く（裁定で要件を改めたら設計と境界まで合わせる）。重なりそのものを直したいなら別の spec として `/kiro-discovery` で起票する。
**追跡**: 要件 2.3・2.4・2.7・5.1
**根拠**: design.md「開発者の決め（設計の討議へ・要件 2.7）」・Non-Goals の 4 つ目

## 設計の良い所

- **測りに頼る形で、DirectWrite が書体の行間をどこへ置いても正しい**: t を「測った基線 − ascent」で出すので、行間を上・上下に半分・下のどれに置いても帯は字の箱の中央へ行く（research.md §1.4 の 3 つの仮説）。DirectWrite の資料に書いていない点を設計の前提にしていない。縦書きの前例と同じ持ち方（生成時に 1 度だけ・比 × em・測れなければ `warn!`＋0）で、新しい形を作っていない。
- **変える所が 1 か所で、単一の源を崩さない**: 寄せの式の定義点を私的な関数 1 つに置き、公開の `highlight_band_offset` の引数の形を保つので、上限に近い `choice_tests.rs`・`draw_format_metrics_tests.rs` に触らずに済む。塗り・当たり・ダーティ帯は `LineBand` を読むだけなので一緒に動き（要件 3.1）、`max(0, t)` で寄せが 0 以上のまま残るため `expand_overhang_for_band` の近端も広げずに済む。直す前の測りで 3 つの出口（1.4 で閉じる・2.7 で止まる・直す）を分けてあり、裁定の確かめの道がはっきりしている。

## 最終の判定

- **判定: GO**（条件つき）
- **理由**: 既存の層と単一の源にそのまま乗る小さな直しで、コードとの突き合わせで事実の誤りは見つからなかった。残る 2 点は設計の骨組みを変えず、検査の門の改め（指摘 1）と要件の文の改め（指摘 2）で片づく。
- **次の手順**:
  1. 設計の討議（`kiro-design-discussion`）で、指摘 1 の門の改めと、指摘 2 の推奨（行ごとの帯のまま・要件 2.3 を同じ大きさの行どうしと読む）を確定し、design.md・requirements.md に反映する。
  2. その後 `/kiro-spec-tasks emo-text-line-gap` でタスクを作る（最初のタスクは測りと `measure_font_metrics` への置き換え）。

## 討議への申し送り

- 指摘 1 の門を「本番の帯 ＝ t ＝ 0 の式の帯」に改めると、行間 0 の書体で t が誤差を超えて 0 でないと測れたときに要件 2.7 の止まりへ入る。最初の測り（要件 1.2）で Yu Gothic UI・ＭＳ ゴシックの t を記録すれば、止まるかどうかは測る前に分かる。
- 実物の読み戻しの (d)（帯の中心と字の箱の中心の差）は、帯も字の箱も同じ基線と ascent から出すので、ほぼ式の確かめになる。独立の確かめはインクの被覆の (c) が担う。検査の重さは今の設計のままでよい。
