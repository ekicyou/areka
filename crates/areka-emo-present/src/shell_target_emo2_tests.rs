//! `emo2` は画面に出る画素が 1 つも変わらない——権威で読んだ実シェルの檻（要件 5.1〜5.3・5.8）。
//!
//! 見るのは 1 つだけである——**同じ [`ShellTarget`] から組んだ 2 つの面の表が、全部の面で
//! 同じ絵になること**。
//!
//! - **A**（権威経由）＝ [`ShellTarget::build_world`]。面の画像の対応を渡して組む
//!   （[`EmoWorld::build_with_images`] → `bind_atlas`）。本仕様が足した経路である。
//! - **B**（適用前と同じ）＝ 同じ `surfaces.txt` の解析結果から [`EmoWorld::build`] で組み、
//!   **同じ [`ShellTarget::atlas`]** を装着したもの。画像 0 件なので、土台の絵の決定は
//!   1 面も触らない＝本仕様の適用前の面の表そのものである。
//!
//! `emo2` は面 0・面 10 とも `element0` を持つ（要件 2.1 の表ウ）ので、A は B と 1 バイトも
//! 違ってはならない。違えば「`element0` が在れば面の画像を使わない」が壊れている。
//!
//! # 索引表を権威経由にする理由
//!
//! B に別の索引表（`surfaces.txt` の `element` だけから焼いたもの）を渡してはならない。外形を
//! 算出する `flatten_extent`（`crates/areka-emo-compose/src/plan.rs`）は**索引表で引けない層を
//! 記録なしで飛ばす**ので、誤って足された `surface10.png` の層が B の索引表では引けず、
//! A と B の両方から同じように消えて、壊れていても緑のままになる。両方に同じ（権威が焼いた）
//! 索引表を渡せば、誤って足された層は A にだけ現れて外形と画素に出る。
//!
//! # ここに置かないもの
//!
//! 権威の記録（要件 6.1・6.2・6.4）の檻は `shell_target_load_tests.rs` の
//! `emo2_shell_records_two_shadowed_images_and_no_warnings` が既に持つ——`recognized=2`・
//! `used=0`・`shadowed=2` の `info!` 1 行、使わなかった画像の `debug!` 2 行（面 0・面 10）、
//! `warn!` 0 行を判定済みである。同じ檻を二重に置かない。

use super::*;

use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::{BindSet, ComposeError, Composer, PatternState};

use super::test_support::{emo2_shell_dir, with_com_initialized};

/// 面 0 の外形（要件 5.2・`measure_tests.rs` の `SCOPE0_W`／`SCOPE0_H` と同じ値）。
const SURFACE0_EXTENT: (u32, u32) = (434, 687);
/// 面 10 の外形（同 `SCOPE1_W`／`SCOPE1_H`）。
///
/// 面 10 の `element0` は 336×400、直下の面の画像 `surface10.png` は 427×463 の**別の絵**である。
/// ゆえに「`element0` が在れば使わない」を外すと、A の面 10 だけがこの値から 427×463 へ動く
/// （要件 5.8 が面 0 の形に頼るなと言うのは、面 0 では `element0` と面の画像が同じ絵で、
/// 二重に重ねても結果のバイトが変わらないためである）。
const SURFACE10_EXTENT: (u32, u32) = (336, 400);

/// 合成結果を比べられる形（外形・行の幅・全画素）にしたもの。
type Snapshot = Result<(u32, u32, u32, Vec<u8>), ComposeError>;

/// 面 1 枚を合成して [`Snapshot`] にする（有効 bind もコマも無い素の状態）。
fn snapshot(world: &EmoWorld, atlas: &AtlasTable, id: u32) -> Snapshot {
    Composer::new()
        .compose(
            world,
            atlas,
            id,
            &BindSet::default(),
            &PatternState::default(),
        )
        .map(|s| (s.width(), s.height(), s.stride(), s.bytes().to_vec()))
}

/// 2 つの合成結果の食い違いを 1 行にして返す（同じなら `None`）。
///
/// 見つけた最初の面で止めずに**全部の面を見る**ために、panic ではなく値で返す。止めてしまうと、
/// 赤の理由が「たまたま番号の小さい面」に左右され、面 10（要件 5.8 の較正）が食い違っている
/// ことが出力に現れない。画素の列は面 1 枚で 1 MB を超えるので、丸ごと並べず先頭の差だけを示す。
fn mismatch(id: u32, a: &Snapshot, b: &Snapshot) -> Option<String> {
    match (a, b) {
        (Ok((aw, ah, astride, abytes)), Ok((bw, bh, bstride, bbytes))) => {
            if (aw, ah, astride) != (bw, bh, bstride) {
                return Some(format!(
                    "面 {id}: 外形が違う A={aw}x{ah}(stride {astride}) B={bw}x{bh}(stride {bstride})"
                ));
            }
            if abytes != bbytes {
                let at = abytes
                    .iter()
                    .zip(bbytes.iter())
                    .position(|(x, y)| x != y)
                    .unwrap_or_else(|| abytes.len().min(bbytes.len()));
                return Some(format!(
                    "面 {id}: 画素が違う（先頭の差は {at} バイト目 A={:?} B={:?}・長さ A={} B={}）",
                    abytes.get(at),
                    bbytes.get(at),
                    abytes.len(),
                    bbytes.len()
                ));
            }
            None
        }
        (Err(ae), Err(be)) if ae == be => None,
        (Err(ae), Err(be)) => Some(format!("面 {id}: 失敗の種類が違う A={ae} B={be}")),
        (a, b) => Some(format!(
            "面 {id}: 片方だけが合成できた A={:?} B={:?}",
            a.as_ref().map(|(w, h, _, _)| (w, h)),
            b.as_ref().map(|(w, h, _, _)| (w, h))
        )),
    }
}

/// 要件 5.1〜5.3・5.8: `emo2` の**全部の面**が、権威経由（A）と適用前（B）で外形も全画素も同じ。
///
/// 併せて要件 5.1・5.2——面の画像 2 枚がどちらも `element0` に隠れて土台に使われないこと、
/// 面 0 の外形が 434×687・面 10 が 336×400 のままであること——も判定する。一致だけでは
/// 「A も B も同じように壊れた」形を捨てられず、また「画像を 1 枚も使わなかった」ことまでは
/// 言えない（使った結果が偶然同じ画素になる形もありうる）ためである。
///
/// 判定の順は一致が先である。一致を後ろに置くと、`element0` の判定を壊したときに赤くなるのが
/// 「面 10 の外形」だけになり、全部の面を見た結果が出力に現れない。
#[test]
fn emo2_every_surface_is_identical_with_and_without_base_images() {
    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("COM 初期化下で WIC ファクトリが作れる");
        let target = load_shell_target(&emo2_shell_dir(), &dec).expect("emo2 のシェルは読める");
        assert!(
            target.bake_errors().is_empty(),
            "前提: emo2 のシェルは 1 枚も落とさずに焼ける: {:?}",
            target.bake_errors()
        );

        // A: 権威経由（面の画像の対応を渡して組む）。
        let a = target.build_world();

        // B: 適用前と同じ（画像 0 件で組み、**同じ**索引表を装着する）。
        let mut b = EmoWorld::build(&target.shell);
        b.bind_atlas(target.atlas(), SetId(0));

        let ids: Vec<u32> = a.surface_ids().collect();
        assert_eq!(
            ids,
            b.surface_ids().collect::<Vec<u32>>(),
            "面の顔ぶれが権威経由（A）と適用前（B）で違う"
        );
        // 較正: 面が 0 個なら以下の「全部の面で一致」は恒真になる。`emo2` の面は 59 個（実測）。
        assert!(
            ids.len() >= 50,
            "前提: emo2 の面は 50 個を下回らない（2026-09-20 実測 59 個）: {} 個",
            ids.len()
        );

        let mut composed = 0usize;
        let mut mismatches: Vec<String> = Vec::new();
        for id in ids {
            let from_a = snapshot(&a, target.atlas(), id);
            let from_b = snapshot(&b, target.atlas(), id);
            if from_a.is_ok() {
                composed += 1;
            }
            mismatches.extend(mismatch(id, &from_a, &from_b));
        }
        // 較正: 全部の面が `Err` なら、比べているのは失敗の種類だけで画素を 1 つも見ていない。
        assert!(
            composed > 0,
            "合成できた面が 1 つも無い（画素を 1 つも比べていない）"
        );
        assert!(
            mismatches.is_empty(),
            "権威経由（A）と適用前（B）で違う面が {} 個ある:\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );

        // 要件 5.2: 画素を持つ 2 面の外形が、適用前の値のままである（上の一致が「A も B も
        // 同じように壊れた」形で緑になっていないことの対照でもある）。
        for (id, extent) in [(0, SURFACE0_EXTENT), (10, SURFACE10_EXTENT)] {
            let (w, h, _, bytes) = snapshot(&a, target.atlas(), id)
                .unwrap_or_else(|e| panic!("面 {id} は合成できる: {e}"));
            assert_eq!((w, h), extent, "面 {id} の外形が適用前と違う");
            assert!(!bytes.is_empty(), "面 {id} の画素が 0 バイト");
        }

        // 要件 5.1: 面の画像 2 枚はどちらも `element0` に隠れて土台に使われない。上の一致は
        // 「画像を 1 枚も使わなかった」ことまでは言わないので（使った結果が偶然同じ画素に
        // なる形もありうる）、決定そのものの結果をここで判定する。
        let report = a.base_images();
        assert!(
            report.used.is_empty(),
            "emo2 は面 0・面 10 とも `element0` を持つので使う画像は 0 件: {:?}",
            report.used
        );
        assert_eq!(
            report
                .shadowed
                .iter()
                .map(|(id, file)| (*id, file.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, "surface0.png"), (10, "surface10.png")],
            "隠れた画像は面 0・面 10 の 2 枚"
        );
    });
}
