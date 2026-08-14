//! 新旧経路のバイト等価の檻（design.md §Testing Strategy「Integration Tests（presenter 経由）」
//! 項目 2・task 6.2・要件 3.3／6.2／6.3／6.4）。
//!
//! # 何を突き合わせるのか
//!
//! 本ファイルは実 `apply_show` が画面へ載せた**表示バイトと当たり判定マスク**を、同一入力を
//! テスト側で独立に辿った**便宜経路**——`Composer::compose`（native）＋**まっさらな出力先**への
//! `resample`（k 適用）＋`AlphaMask::from_pbgra32`——と 1 バイト単位で突き合わせる
//! （`test_support::scaled_golden_with` がこの 3 段そのものである）。拡大率は恒等・整数倍・端数の
//! 3 通りを踏む。
//!
//! # 2 つの経路の唯一の差＝**出力先が使い回しかどうか**（ここが本檻の検出力の源）
//!
//! 便宜経路は毎回まっさらな `ComposedSurface` へ書く。本番経路（task 5.3 以後）は
//! **キャッシュ追い出しエントリから回収したバッファ**へ書く——そこには**直前のコマの絵が
//! 丸ごと残っている**（`ComposeCache::take_recycled` はバイト列を保ったままエントリを渡し、
//! `FrameBudget::display_buffer` はそれをそのまま返す）。ゆえに「直前の内容を持つ器へ書いても
//! 結果が変わらない」が崩れた瞬間、両者は食い違う。
//!
//! これは task 5.4（追補 §A2・リサンプル出力の冗長なゼロ埋め除去）が踏み抜き得る欠陥の形である。
//! ゼロ埋めを外す是正は「どうせ全部上書きされる」という前提の上に立っており、**前提が崩れれば
//! 前のコマが画面に出る**。本檻はその前提を presenter の実経路で固定する。
//!
//! 残留が実際に在ることは檻自身が確かめる: ⑴交互に適用する 2 面の**表示バイトとマスクが互いに
//! 異なる**こと ⑵暖機後の表示バッファの番地が回り続けている（＝いま書いている器に直前の別の面の
//! 絵が入っていた）こと——の 2 点を主張する。どちらかが崩れたら残留の観測は空虚であり、檻は
//! そこで止まる。
//!
//! # 本檻が捕まえられない残留の形（実測で判明した限界・下の層が受け持つ）
//!
//! 「出力の一部の領域を**毎回同じように**書き落とす」変異は、外形が変わらない定常状態では
//! **本檻を素通りする**。書き落とされた領域は誰にも書かれないまま 0 であり続けるため、
//! 使い回しの器の側もまっさらな器の側も同じ 0 になるからである。実測: リサンプルの出力ゼロ埋めを
//! 外し、かつ最終行を書かない変異（＝5.4 を誤って実装した形）を当てても本ファイルの 3 本は
//! 緑のままだった（`areka-emo-compose` 側は 3 本が赤）。
//!
//! この穴を塞いでいるのは `scale_prior_path_tests.rs` の
//! `resample_into_a_reused_output_is_byte_equal_to_a_fresh_output` である——器を **`0xAA` で
//! 塗ってから**書かせるので、「誰も書かない領域」が 0 ではあり得なくなる。presenter からは
//! `ComposedSurface::bytes_mut` が `pub(crate)` で届かず同じ手が使えない。**層を分けているのは
//! この理由であって、重複ではない。**
//!
//! 逆に本檻だけが捕まえる形もある: リサンプルそのものを飛ばす／転写を途中で打ち切るといった
//! **presenter 層の配線**の欠陥である（実測: `FrameBudget::resample_native_into` が「出力の外形が
//! 既に合っていればリサンプルを省く」変異を当てると、クレート全 190 件のうち**赤になるのは
//! 本ファイルの 1 本だけ**だった）。
//!
//! # この檻が 5.5 に対して何を言い、何を言わないか（正直な線引き）
//!
//! 本番経路も便宜経路も、k 適用には**同じ** `resample`／`resample_with` を通る。ゆえに
//! task 5.5（追補 §A3・リサンプル計算の是正）が内側ループを書き換えると**両辺が同時に同じだけ
//! 変わる**——本檻は緑のままである。5.5 に対する安全網は本ファイルではなく、
//! `areka-emo-compose` の `scale_prior_path_tests.rs`（是正前の意味論を凍結した参照実装との
//! バイト等価）が持つ。両者は次の鎖で繋がっている:
//!
//! ```text
//! presenter の表示バイト  ==  便宜経路（compose + fresh resample）   ← 本ファイル
//!                              ==  凍結した是正前の参照実装          ← scale_prior_path_tests.rs
//! ```
//!
//! 鎖の 2 本が両方緑であるとき、かつそのときに限り「presenter の表示バイトは是正前と等価」が
//! 言える。片方を緩めると鎖は切れる。
//!
//! # 空の実装で偽の合格を出さない
//!
//! 表示バイトに不透明画素が実在すること・マスクに立っているビットと落ちているビットが**両方**
//! 在ることを毎回確かめる。全透明や一様マスクを回す実装は「バイト等価」を空虚に満たしてしまう。
//!
//! # 実時間を合否条件に使わない（要件 6.2）／純 x64 常設（要件 6.3）
//!
//! 判定量はバイト・外形・番地・ビット数だけである。環境変数ゲートも `#[ignore]` も持たない。

use super::*;

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::BindSet;

use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::test_support::{
    elem, make_world_with_gpu, scaled_golden_with, shell_of, show_ok, spawn_window_with_dpi,
    surface,
};

// ── fixture ────────────────────────────────────────────────────────────────────────────

/// native 外形。恒等・k=2/1・k=5/4 のいずれでも端数と非端数が混ざる大きさを選ぶ
/// （`6×5` に 5/4 を掛けると 7.5→8・6.25→6 で両軸とも丸めが発火する）。
const NATIVE_W: u32 = 14;
const NATIVE_H: u32 = 10;

/// 暖機の適用回数（交代する席・輪番のマスクが立ち上がりきる回数。6.1 の檻と同じ 4）。
const WARMUP: usize = 4;

/// 定常状態の観測反復数。
const STEADY: usize = 6;

/// 交互に適用する 2 面の id。
const FACE_A: u32 = 1000;
const FACE_B: u32 = 3000;

/// n 回目に適用する面 id。
fn face_at(i: usize) -> u32 {
    if i.is_multiple_of(2) { FACE_A } else { FACE_B }
}

/// 面 1000／3000 ＝ 同じ `w×h`・**α が画素ごとに変わる**・互いに別バイトの 2 面。
///
/// - α は `0xFF`（マスク hit）と `0x20`（閾値 128 未満＝非 hit）の 2 値で、**α=0 を含まない**。
///   ゆえに atlas の α=0 除外トリムは全域を残し、合成外形は両面とも正確に `w×h` である
///   （＝供給面のリサイズ経路を踏まず、表示バッファの回収がそのまま成立する）。
/// - 2 面で α の市松の**位相**を反転させてあるので、当たり判定マスクも互いに別物になる。
///   全不透明の fixture ではマスクが全ビット 1 の一様マスクになり、マスクの等価主張が
///   寸法しか弁別できなくなる。
/// - 色は α を掛けた premultiplied 値で焼く（`B,G,R ≤ A` の不変条件を崩さない）。
fn build_two_alpha_varying_faces(w: u32, h: u32) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![
        surface(FACE_A, vec![elem("p.png", 0, 0)]),
        surface(FACE_B, vec![elem("q.png", 0, 0)]),
    ];

    let stride = w * 4;
    let image = |salt: u8, phase: u32| -> Vec<u8> {
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let a: u8 = if (x + y + phase).is_multiple_of(2) { 0xFF } else { 0x20 };
                let pm = |c: u8| ((c as u16 * a as u16) / 255) as u8;
                img.push(pm((x as u8).wrapping_mul(3).wrapping_add(salt)));
                img.push(pm((y as u8).wrapping_mul(5).wrapping_add(salt)));
                img.push(pm(((x + y) as u8).wrapping_mul(7).wrapping_add(salt)));
                img.push(a);
            }
        }
        img
    };

    let mut dec = MemoryDecoder::new();
    dec.insert(base.join("p.png"), w, h, stride, image(0x13, 0), true);
    dec.insert(base.join("q.png"), w, h, stride, image(0x8B, 1), true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

// ── 便宜経路（是正前と同じ「まっさらな出力先」で辿る対照）─────────────────────────────────

/// 1 面ぶんの便宜経路の期待値。
struct Expected {
    /// k 適用後の表示バイト（`compose` → 新規 out への `resample`）。
    bytes: Vec<u8>,
    /// k 適用後の外形。
    extent: (u32, u32),
    /// 上の bytes から独立に組んだ当たり判定マスク。
    mask: AlphaMask,
}

/// 便宜経路を辿って期待値を作る（presenter の内部値の追認ではなく独立再現）。
fn expected_of(emo_world: &EmoWorld, atlas: &AtlasTable, id: u32, scale: ScaleRatio) -> Expected {
    let g = scaled_golden_with(
        emo_world,
        atlas,
        id,
        &BindSet::default(),
        &PatternState::default(),
        scale,
    );
    let mask = AlphaMask::from_pbgra32(&g.scaled, g.scaled_size.0, g.scaled_size.1, g.scaled_size.0 * 4);
    Expected {
        bytes: g.scaled,
        extent: g.scaled_size,
        mask,
    }
}

/// マスクの立っているビット数・落ちているビット数。
fn mask_bit_counts(mask: &AlphaMask) -> (usize, usize) {
    let mut hits = 0;
    let mut misses = 0;
    for y in 0..mask.height() {
        for x in 0..mask.width() {
            if mask.is_hit(x, y) {
                hits += 1;
            } else {
                misses += 1;
            }
        }
    }
    (hits, misses)
}

/// 期待値そのものが「中身のある絵」であることを主張する（等価主張の非空虚性）。
fn assert_expected_is_not_empty(e: &Expected, at: &str) {
    assert!(
        e.bytes.chunks_exact(4).any(|px| px[3] == 0xFF),
        "{at}: 期待バイトに不透明画素が 1 つも無い（全透明を突き合わせても等価は自明に成立する）"
    );
    let (hits, misses) = mask_bit_counts(&e.mask);
    assert!(
        hits > 0 && misses > 0,
        "{at}: 期待マスクが一様（hit={hits} miss={misses}）＝マスクの等価主張が空虚"
    );
}

// ── 檻 ────────────────────────────────────────────────────────────────────────────────

/// 要件 3.3／6.4 観測完了: **使い回しバッファの上を走る本番経路**の表示バイトと当たり判定マスクが、
/// まっさらな出力先を使う便宜経路と 1 バイトも違わない（k 恒等・整数倍・端数の 3 通り）。
///
/// 2 面を交互に適用する。合成キャッシュは容量 1 ゆえ毎回ミスし、直前の面のエントリが追い出されて
/// その表示バッファが回収される——**いま書いている器には直前の別の面の絵が入っている**。
/// 便宜経路との差はこの一点だけであり、ここが 5.4（ゼロ埋め除去）の安全網になる。
///
/// # 主張の内訳
///
/// 1. 表示バッファの外形・全バイトが便宜経路と一致（毎適用）
/// 2. キャッシュスロットのマスクが、表示バイトから独立に組んだマスクと一致（毎適用）
/// 3. 当たり判定へ供給されたマスク（`AlphaMaskResource`）も同じ内容（下流まで同一）
/// 4. 供給面の readback も同じバイト（GPU まで実際に載っている・各 k で 1 度）
/// 5. 非空虚性: 不透明画素が在る・マスクに hit と非 hit の両方が在る
/// 6. 残留の実在: 2 面の期待バイトとマスクが互いに異なり、暖機後の表示バッファの番地が
///    既知の集合の中で回り続けている（＝使い回しが成立している）
#[test]
fn the_budget_path_produces_the_same_display_bytes_and_mask_as_a_fresh_buffer_path() {
    // (窓 DPI, num, den) — 恒等・整数倍・端数。author_dpi は 96 固定。
    for (window_dpi, num, den) in [(96_u16, 1_u32, 1_u32), (192, 2, 1), (120, 5, 4)] {
        let scale = ScaleRatio::new(num, den).expect("非ゼロの比は構築できる");
        let k = format!("k={num}/{den}");

        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, window_dpi);
        let (emo_world, atlas) = build_two_alpha_varying_faces(NATIVE_W, NATIVE_H);
        // 便宜経路は presenter が握るのと別の実体で辿る（内部値の追認にしない）。
        let (probe_world, probe_atlas) = build_two_alpha_varying_faces(NATIVE_W, NATIVE_H);
        let want_a = expected_of(&probe_world, &probe_atlas, FACE_A, scale);
        let want_b = expected_of(&probe_world, &probe_atlas, FACE_B, scale);

        // fixture 前提: 2 面が別の絵・別のマスクであること（同じなら残留を弁別できない）。
        assert_eq!(
            want_a.extent, want_b.extent,
            "{k}: 2 面の外形が違う（表示バッファの回収経路を踏めない）"
        );
        assert_ne!(
            want_a.bytes, want_b.bytes,
            "{k}: 2 面の表示バイトが同一＝残留が混ざっても気付けない"
        );
        assert_ne!(
            want_a.mask, want_b.mask,
            "{k}: 2 面のマスクが同一＝マスクへの残留が混ざっても気付けない"
        );
        assert_expected_is_not_empty(&want_a, &format!("{k} 面 {FACE_A}"));
        assert_expected_is_not_empty(&want_b, &format!("{k} 面 {FACE_B}"));

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 暖機後の表示バッファの番地（使い回しが成立していることの証拠として集める）。
        let mut steady_display_ptrs: Vec<*const u8> = Vec::new();

        for i in 0..WARMUP + STEADY {
            let id = face_at(i);
            let want = if id == FACE_A { &want_a } else { &want_b };
            let at = format!("{k} の {i} 回目（面 {id}）");
            show_ok(&mut presenter, &mut world, TargetId(0), id);

            let target = presenter
                .targets
                .get(&TargetId(0))
                .expect("装着済み target");
            let entry = target
                .cache
                .get(id, &BindSet::default(), &PatternState::default(), scale)
                .expect("直前の適用がスロットを埋めている");

            // ⑴ 表示バイト。
            assert_eq!(
                (entry.composed.width(), entry.composed.height()),
                want.extent,
                "{at}: 表示バッファの外形が便宜経路と違う"
            );
            assert_eq!(
                entry.composed.bytes(),
                want.bytes.as_slice(),
                "{at}: 表示バイトが便宜経路と 1 バイト以上違う（使い回しバッファの残留が\
                 混ざっている疑い）"
            );
            // ⑸ 非空虚性（本番側の実バイトについても毎回見る）。
            assert!(
                entry.composed.bytes().chunks_exact(4).any(|px| px[3] == 0xFF),
                "{at}: 表示バッファに不透明画素が 1 つも無い（空を回している）"
            );

            // ⑵ キャッシュスロットのマスク。
            assert_eq!(
                *entry.mask, want.mask,
                "{at}: スロットのマスクが表示バイト由来の独立再現と違う"
            );

            // ⑶ 当たり判定へ供給されたマスク。
            let surface_entity = target
                .mount
                .as_ref()
                .expect("表示成立後は mount が生成済み")
                .surface_entity();
            let supplied = world
                .get::<AlphaMaskResource>(surface_entity)
                .expect("表示成立後は surface entity に AlphaMaskResource が載る")
                .mask()
                .expect("表示成立後はマスクが供給済み");
            assert_eq!(
                *supplied, want.mask,
                "{at}: 当たり判定へ供給されたマスクが便宜経路と違う"
            );

            if i >= WARMUP {
                steady_display_ptrs.push(entry.composed.bytes().as_ptr());
            }
        }

        // ⑷ 供給面の readback（GPU まで同じ絵が載っている・各 k で 1 度）。
        let last = face_at(WARMUP + STEADY - 1);
        let want_last = if last == FACE_A { &want_a } else { &want_b };
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            want_last.bytes,
            "{k}: 供給面の readback が便宜経路と違う（表示面まで届いていない）"
        );

        // ⑹ 残留の実在: 暖機後の表示バッファは**高々 2 本**の器を回している。
        //
        // 器が毎回新品なら定常の番地は毎回別物になり、ここで止まる——そのとき「使い回しの上でも
        // バイトが等しい」という上の主張は何も観測していなかったことになる。番地は**残留経路が
        // 実在することの証拠**として読むのであって、バイト等価の合否そのものではない
        // （合否はあくまで上のバイト比較である）。
        let mut distinct = steady_display_ptrs.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(
            distinct.len() <= 2,
            "{k}: 定常状態の表示バッファが {} 種類の器に散っている（回収が成立しておらず、\
             使い回しの上での等価を観測できていない）",
            distinct.len()
        );
        assert!(
            steady_display_ptrs.len() > distinct.len(),
            "{k}: 定常状態で同じ器が一度も再登場しない（直前の面の絵が残った器へ書く経路を\
             踏めていない）"
        );
    }
}

/// 要件 3.3／6.4 観測完了（**同一入力の反復＝ヒット経路を含む**）: 同じ面を続けて適用しても、
/// 表示バイトとマスクは便宜経路と一致し続ける。
///
/// 上の檻は設計上すべての適用がミスする（面を毎回変える）。`show.rs` 手順 (3)——供給面
/// アップロード＋マスク供給——は合成の有無に依らず走るため、**ヒット経路にも乗っている**。
/// ミスしか踏まない檻だけでは、そこが 1 度も動かないまま「毎コマ経路は全て覆った」と言えてしまう
/// （task 2.3 の偽合格が同じ形だった）。
///
/// ヒットが実際に成立したことは**マスクの実体が動かないこと**で示す（ミスなら輪番が 1 つ進んで
/// もう 1 本の `Arc` へ移る）。
#[test]
fn repeating_the_same_surface_keeps_the_display_bytes_and_mask_equivalent() {
    for (window_dpi, num, den) in [(192_u16, 2_u32, 1_u32), (120, 5, 4)] {
        let scale = ScaleRatio::new(num, den).expect("非ゼロの比は構築できる");
        let k = format!("k={num}/{den}");

        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, window_dpi);
        let (emo_world, atlas) = build_two_alpha_varying_faces(NATIVE_W, NATIVE_H);
        let (probe_world, probe_atlas) = build_two_alpha_varying_faces(NATIVE_W, NATIVE_H);
        let want = expected_of(&probe_world, &probe_atlas, FACE_A, scale);
        assert_expected_is_not_empty(&want, &format!("{k} 面 {FACE_A}"));

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 交互適用で暖機してから、同じ面を 2 回続けて適用する（2 回目がヒット）。
        for i in 0..WARMUP {
            show_ok(&mut presenter, &mut world, TargetId(0), face_at(i));
        }
        show_ok(&mut presenter, &mut world, TargetId(0), FACE_A);
        let missed_mask = {
            let target = presenter.targets.get(&TargetId(0)).expect("装着済み target");
            let entry = target
                .cache
                .get(FACE_A, &BindSet::default(), &PatternState::default(), scale)
                .expect("直前の適用がスロットを埋めている");
            std::sync::Arc::as_ptr(&entry.mask)
        };

        show_ok(&mut presenter, &mut world, TargetId(0), FACE_A);
        let target = presenter.targets.get(&TargetId(0)).expect("装着済み target");
        let entry = target
            .cache
            .get(FACE_A, &BindSet::default(), &PatternState::default(), scale)
            .expect("直前の適用がスロットを埋めている");
        assert_eq!(
            std::sync::Arc::as_ptr(&entry.mask),
            missed_mask,
            "{k}: マスクの実体が動いた＝引き当てがミスしている（ヒット経路を観測できていない）"
        );
        assert_eq!(
            (entry.composed.width(), entry.composed.height()),
            want.extent,
            "{k}: ヒット経路の表示外形が便宜経路と違う"
        );
        assert_eq!(
            entry.composed.bytes(),
            want.bytes.as_slice(),
            "{k}: ヒット経路の表示バイトが便宜経路と違う"
        );
        assert_eq!(
            *entry.mask, want.mask,
            "{k}: ヒット経路のマスクが便宜経路と違う"
        );
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            want.bytes,
            "{k}: ヒット経路の readback が便宜経路と違う"
        );
    }
}

/// fixture 自身の健全性: 2 面が **同じ外形・別のバイト・別のマスク**であり、
/// マスクが一様でないこと。
///
/// 檻の前提が壊れていれば等価主張は全て空虚になる。本 spec は fixture 側の破綻で 3 度
/// 偽の緑を踏んでいるため、生成器そのものを 1 本の檻で押さえる。
#[test]
fn the_two_face_fixture_differs_in_bytes_and_in_mask() {
    let (probe_world, probe_atlas) = build_two_alpha_varying_faces(NATIVE_W, NATIVE_H);
    for (num, den) in [(1_u32, 1_u32), (2, 1), (5, 4)] {
        let scale = ScaleRatio::new(num, den).expect("非ゼロの比は構築できる");
        let a = expected_of(&probe_world, &probe_atlas, FACE_A, scale);
        let b = expected_of(&probe_world, &probe_atlas, FACE_B, scale);
        let at = format!("k={num}/{den}");

        assert_eq!(a.extent, b.extent, "{at}: 2 面の外形が違う");
        assert_eq!(
            a.extent,
            scale.scaled_extent(NATIVE_W, NATIVE_H),
            "{at}: α≠0 ゆえトリムは全域を残し、外形は scaled_extent と厳密一致するはず"
        );
        assert_ne!(a.bytes, b.bytes, "{at}: 2 面の表示バイトが同一");
        assert_ne!(a.mask, b.mask, "{at}: 2 面のマスクが同一");
        assert_expected_is_not_empty(&a, &format!("{at} 面 {FACE_A}"));
        assert_expected_is_not_empty(&b, &format!("{at} 面 {FACE_B}"));
    }
}
