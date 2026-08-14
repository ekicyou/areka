use bevy_ecs::prelude::*;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, WindowPos};

use super::test_support::{
    drag_end_event_at, dragging_state, fake_handle, position_of, rect, window_pos_at,
    window_pos_sized,
};
use super::{Anchored, BalloonFollow, MonitorSnapshot, on_char_drag_end};
use crate::placement::resolver::Anchor;
use crate::placement::resolver::PointPx;
use crate::placement::resolver::SizePx;

// -------------------------------------------------------------------------
// on_balloon_drag_end: バルーン単独ドラッグ確定 offset の永続 write-through
// （task 2.3・Req2.1・8.1・design C2/C3）
//
// バルーン窓は move_window=true ゆえ wndproc が実窓位置を WindowPos.position へ
// 更新済み——DragEnd 時点の最終確定位置はこの WindowPos.position で読める
// （on_balloon_drag と同源）。on_balloon_drag_end は最終確定位置から
// offset = balloon_pos − char_pos を**再導出**（in-session BalloonFollow.offset は
// 使わない）し、その左上基準値を基準変換せずそのまま
// BalloonOffset entries として Ghost 永続スコープへ即時 write-through する。
// 実 publisher（spawn_sylphya + SharedFakeIo）で barrier→load_scope し、保存値が
// 最終確定位置由来の persist 値に一致することを固定する（Issue 1 対応・2.1/8.1）。
// -------------------------------------------------------------------------

/// Task 2.3 保存フック（Req2.1/8.1・design C3）: バルーン窓の DragEnd で、最終確定
/// 位置から**再導出**した左上基準の相対 offset が（基準変換なしで）scope の
/// BalloonOffset として Ghost 永続スコープへ write-through される。**in-session の
/// BalloonFollow.offset は SAVE に使わない**（DragEnd 最終確定位置から再導出）——
/// stale な offset を仕込んで弁別する。
#[test]
fn on_balloon_drag_end_persists_balloon_offset_for_scope() {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{
        Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };

    use super::super::persist::PersistWiring;
    use super::on_balloon_drag_end;
    use crate::placement::spawn::BalloonWindowMarker;

    // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・persist.rs と同流儀）。
    struct SharedFakeIo(Arc<FakePersistIo>);
    impl PersistIo for SharedFakeIo {
        fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo(shared.clone())),
        runtime_sink: None,
    });

    let mut world = World::new();
    // UI スレッド常駐の保存投函口を挿入（persist_entries が引く NonSend リソース）。
    world.insert_non_send_resource(PersistWiring {
        publisher: parts.publisher.clone(),
    });

    // char 窓（Bottom・emo2 実寸）と、単独ドラッグで wndproc が最終確定位置へ移した
    // balloon 窓。値はいずれも 96 の倍数を避け、隠れた dpi/96 再スケールの檻とする。
    let char_size = SizePx { w: 434, h: 687 };
    let char_pos = Point { x: 1483, y: 733 };
    let final_balloon_pos = Point { x: 1071, y: 708 }; // wndproc の最終確定位置
    let anchor = Anchor::Bottom;

    // stale な in-session offset（SAVE に誤用したら弁別で落ちる檻の値）。
    let stale_offset = PointPx { x: 999, y: 888 };

    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(final_balloon_pos.x, final_balloon_pos.y),
            BalloonWindowMarker { scope: 1 },
        ))
        .id();
    let char_w = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(char_pos.x, char_pos.y, char_size.w, char_size.h),
            Anchored(anchor),
            BalloonFollow {
                balloon,
                offset: stale_offset,
            },
        ))
        .id();

    // 期待 persist 値 = 最終確定位置から再導出した左上基準 offset そのもの
    // （保存基準＝ランタイム基準・アンカー辺基準変換なし）。
    let expected = PointPx {
        x: final_balloon_pos.x - char_pos.x, // 1071−1483 = −412
        y: final_balloon_pos.y - char_pos.y, // 708−733  = −25
    };
    assert_eq!(
        expected,
        PointPx { x: -412, y: -25 },
        "保存値は char 左上基準の生 offset（Bottom でも h/w を引かない）"
    );
    assert_ne!(
        expected, stale_offset,
        "檻の前提: 最終確定 offset は stale な in-session offset と異なる"
    );

    // DragEnd をバルーン窓へ配送（cursor 値は無関係＝最終確定位置は balloon 窓の
    // WindowPos.position を読む・move_window=true）。
    let ev = Phase::Bubble(drag_end_event_at(balloon, (0, 0)));
    assert!(!on_balloon_drag_end(&mut world, balloon, balloon, &ev));

    // キャラ窓は不動・BalloonFollow.offset（in-session 表現）も on_balloon_drag_end では
    // 変えない（保存は最終確定位置から独立に導出する）。
    assert_eq!(position_of(&world, char_w), char_pos);
    assert_eq!(
        world.get::<BalloonFollow>(char_w).unwrap().offset,
        stale_offset,
        "on_balloon_drag_end は in-session offset を変異させない（保存専用）"
    );

    // barrier 復帰＝上記 put の write-through 保存まで完了（同一送信端 FIFO）。
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // 別ハンドルの load_scope で scope1 の BalloonOffset を観測（実 IO 通過＝投函の証明）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    assert!(
        loaded.contains(&(
            PersistKey::BalloonOffset {
                scope: 1,
                axis: Axis::X
            },
            expected.x.to_string()
        )),
        "バルーン DragEnd の最終確定 offset X={} が scope1 の BalloonOffset として保存されていない: {loaded:?}",
        expected.x
    );
    assert!(
        loaded.contains(&(
            PersistKey::BalloonOffset {
                scope: 1,
                axis: Axis::Y
            },
            expected.y.to_string()
        )),
        "バルーン DragEnd の最終確定 offset Y={} が scope1 の BalloonOffset として保存されていない: {loaded:?}",
        expected.y
    );

    // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
    parts.publisher.close();
    let _ = parts.handle.join();
}

/// Task 8.2 保存→復元 往復値等価の END-TO-END 統合檻（Req8.1・Req7.2・design
/// Testing Strategy Integration §1）。
///
/// 実 `FsPersistIo`＋temp dir に置いた**最小解決可能ゴースト**へ、save 側（DragEnd 観測点
/// →`PersistWiring`→実アクター→`sylphya.toml`）と restore 側（`load_restored_state`
/// →`apply_restored_placements`）を実ファイルシステム越しに結線し、キャラ位置・バルーン
/// オフセットが値等価で往復すること、および同居する無関係 key（`BootCount`）が save で
/// 破壊されないことを決定論固定する。
///
/// これまでの follow.rs 檻は `FakePersistIo`（インメモリ）で「投函→load_scope」の
/// 送信端 FIFO を証明したが、本檻は**実 FS 書込＋mount 解決経由の実読出**で往復全体
/// （save→file→resolve→load→merge）を一本の檻に収める（design §1 が実 `FsPersistIo`
/// を要求する所以）。
///
/// 檻の噛み方（往復が壊れたら落ちる）:
/// - char: save 側の bottom 吸着確定位置（1427, 513）が実ファイルへ書かれ、restore 側で
///   同一 work area の `project_restore` が恒等（既に下端一致・x 域内）ゆえ merge 後の
///   `char_pos` が確定位置と値等価に戻る。既定 char_pos(100,100) が漏れれば落ちる。
/// - balloon: DragEnd 最終確定位置から再導出した左上基準 offset(-412,-43) が**基準変換
///   なしで**そのままファイルへ書かれ（保存基準＝ランタイム基準＝char 左上・2026-07-31
///   実機裁定）、restore 側でもそのまま採用され `balloon_pos` が balloon 最終確定位置
///   (1015, 470)へ戻る。
/// - 7.2: 事前に `persist_put` した無関係 key `BootCount="1"` が、char/balloon の DragEnd
///   save 後も `load_restored_state` に不変で残る（read-modify-write の無関係 key 温存）。
///
/// 座標は 96 の非倍数（1427・1015・470…）で隠れた dpi/96 再スケールの檻を兼ね、既定値
/// （100・0・96 系）と重ならない。
#[test]
fn round_trip_save_restore_value_equivalence_over_real_fs() {
    use std::path::PathBuf;

    use areka_ghost::sylphya_wiring::profile_areka_root;
    use areka_parsers::charset::DefaultEncoding;
    use areka_sylphya::persist::FsPersistIo;
    use areka_sylphya::{
        Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };

    use super::super::persist::{
        PersistWiring, apply_restored_placements, load_restored_state,
    };
    use super::on_balloon_drag_end;
    use crate::placement::resolver::ScopePlacement;
    use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

    // panic をまたいで temp dir を確実に片付ける Drop ガード。
    struct TempGhostDir(PathBuf);
    impl Drop for TempGhostDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // --- fixture: 最小解決可能ゴースト（persist.rs plant_minimal_ghost 同型）---------
    let mut root = std::env::temp_dir();
    root.push("areka_follow_round_trip_e2e_8_2");
    let _ = std::fs::remove_dir_all(&root);
    let _guard = TempGhostDir(root.clone());
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
    )
    .expect("write ghost descript");
    std::fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");

    // 永続ファイルは load_restored_state が読む場所と同一＝profile_areka_root(shiori.dir)。
    // shiori.dir は resolve が root/ghost/master へ解決する（persist.rs load 檻が証明）。
    // FsPersistIo::commit は親ディレクトリを作らないため、書込先を先に用意する
    // （本番 boot 経路は profile/areka を別途用意する・ここでは檻の前提を満たす）。
    let profile_root = profile_areka_root(&ghost_master);
    std::fs::create_dir_all(&profile_root).expect("create profile/areka");

    // --- save 側 sylphya（実 FsPersistIo・実 FS 往復）------------------------------
    let roots = ScopeRoots {
        ghost: Some(profile_root.clone()),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(FsPersistIo),
        runtime_sink: None,
    });

    // 7.2 の無関係 key を DragEnd save に**先立って**植える（read-modify-write の温存対象）。
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "1".to_string())],
    );

    // --- headless World（char + balloon + PersistWiring）--------------------------
    let char_size = SizePx { w: 434, h: 687 };
    // work area 下端 1200 → bottom 吸着 y = 1200 − 687 = 513（save/restore 双方で同一）。
    let snapshot = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1920, 1200)],
    };

    let mut world = World::new();
    world.insert_non_send_resource(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    world.insert_resource(MonitorSnapshot {
        work_areas: snapshot.work_areas.clone(),
    });

    // balloon 窓（scope1）: 単独ドラッグ確定後の最終位置は後段で明示設定する。
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(500, 500),
            BalloonWindowMarker { scope: 1 },
        ))
        .id();
    // char 窓（scope1・Bottom）: DraggingState + cursor から mapped=(1427, 513) が確定。
    let stale_offset = PointPx { x: 999, y: 888 };
    let char_w = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1200, 600, char_size.w, char_size.h),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 1 },
            BalloonFollow {
                balloon,
                offset: stale_offset,
            },
            dragging_state((1250, 500), (1300, 550)),
        ))
        .id();

    // --- char DragEnd（保存）: cursor(1477,313) → raw(1427,263) → Bottom mapped(1427,513) ---
    let char_ev = Phase::Bubble(drag_end_event_at(char_w, (1477, 313)));
    assert!(!on_char_drag_end(&mut world, char_w, char_w, &char_ev));
    let char_final = position_of(&world, char_w); // Point（WindowPos 通貨）
    assert_eq!(
        char_final,
        Point { x: 1427, y: 513 },
        "char DragEnd 確定位置（bottom 吸着・96 非倍数）"
    );
    // ScopePlacement は PointPx 通貨ゆえ比較用に写す（値は同一）。
    let char_final_px = PointPx {
        x: char_final.x,
        y: char_final.y,
    };

    // --- balloon 単独ドラッグ確定位置を wndproc が置いたものとして明示設定 -----------
    // （on_char_drag_end の follow_balloon が stale offset で動かした後の、ユーザーの
    //   独立バルーンドラッグの最終確定位置。on_balloon_drag_end は WindowPos.position を読む。）
    let balloon_final = Point { x: 1015, y: 470 };
    world.get_mut::<WindowPos>(balloon).unwrap().position = Some(balloon_final);
    let balloon_final_px = PointPx {
        x: balloon_final.x,
        y: balloon_final.y,
    };

    // --- balloon DragEnd（保存）: 最終確定位置から左上基準 offset を再導出しそのまま保存 ---
    let balloon_ev = Phase::Bubble(drag_end_event_at(balloon, (0, 0)));
    assert!(!on_balloon_drag_end(&mut world, balloon, balloon, &balloon_ev));

    // 期待 persist＝左上基準 offset そのもの（保存基準＝ランタイム基準・変換なし）。
    let expected_offset_tl = PointPx {
        x: balloon_final.x - char_final.x, // 1015−1427 = −412
        y: balloon_final.y - char_final.y, // 470−513  = −43
    };
    let expected_persist = expected_offset_tl;
    assert_eq!(
        expected_persist,
        PointPx { x: -412, y: -43 },
        "保存値は char 左上基準の生 offset（Bottom でも char_size を混ぜない）"
    );

    // --- barrier: 上記 3 件の put（BootCount／WindowPos／BalloonOffset）が実 FS へ確定 ---
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // 実アクターと同一 roots・実 FsPersistIo で読み戻し、保存 entries を直接確認（往復の中間証拠）。
    // 保存 x は**原点＝下端中央**基準（左上 1427 ＋ w/2=217 → 1644）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &FsPersistIo);
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 1,
                axis: Axis::X
            },
            "1644".to_string()
        )) && loaded.contains(&(
            PersistKey::WindowPos {
                scope: 1,
                axis: Axis::Y
            },
            "513".to_string()
        )),
        "char 確定位置が実 FS へ書かれていない: {loaded:?}"
    );
    assert!(
        loaded.contains(&(
            PersistKey::BalloonOffset {
                scope: 1,
                axis: Axis::X
            },
            expected_persist.x.to_string()
        )) && loaded.contains(&(
            PersistKey::BalloonOffset {
                scope: 1,
                axis: Axis::Y
            },
            expected_persist.y.to_string()
        )),
        "balloon 左上基準 offset が実 FS へ書かれていない: {loaded:?}"
    );

    // --- restore 側: mount 解決経由で実ファイルを読み、merge へ流す ------------------
    let entries = load_restored_state(&root, DefaultEncoding::Ansi);

    // 7.2: 無関係 key BootCount が DragEnd save 後も不変で残る（read-modify-write 温存）。
    assert!(
        entries.contains(&(PersistKey::BootCount, "1".to_string())),
        "同居する無関係 key BootCount が DragEnd save で破壊された（7.2）: {entries:?}"
    );

    // resolver 出力を模す合成 placement（既定は saved と別位置＝復元優先の証明）。
    let default_char_pos = PointPx { x: 100, y: 100 };
    let default_balloon_offset = PointPx { x: 7, y: 7 };
    let synthetic = ScopePlacement {
        scope: 1,
        char_pos: default_char_pos,
        char_size,
        balloon_pos: PointPx {
            x: default_char_pos.x + default_balloon_offset.x,
            y: default_char_pos.y + default_balloon_offset.y,
        },
        balloon_size: SizePx { w: 200, h: 300 },
        balloon_offset: default_balloon_offset,
        // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    };
    // saved 位置を覆う work area ゆえ project_restore は恒等（既に下端一致・x 域内）。
    let out = apply_restored_placements(vec![synthetic], &entries, &snapshot);

    assert_eq!(out.len(), 1);
    // (8.1) 復元 char_pos が DragEnd 確定位置と値等価（既定を上書き）。
    assert_eq!(
        out[0].char_pos, char_final_px,
        "復元 char_pos が DragEnd 確定位置と値等価でない（1.4/8.1）"
    );
    assert_ne!(
        out[0].char_pos, default_char_pos,
        "復元が既定位置を漏らしている"
    );
    // (8.1) 復元 balloon offset（左上基準）が DragEnd 由来 offset と値等価。
    assert_eq!(
        out[0].balloon_offset, expected_offset_tl,
        "復元 balloon offset が DragEnd 由来 offset と値等価でない（2.2/2.3/8.1）"
    );
    // (8.1) 復元 balloon_pos が balloon DragEnd 最終確定位置と値等価。
    assert_eq!(
        out[0].balloon_pos, balloon_final_px,
        "復元 balloon_pos が balloon DragEnd 最終確定位置と値等価でない（2.3/8.1）"
    );
    // 事後条件（design C1）: 寸法・anchor は不変。
    assert_eq!(out[0].char_size, char_size);
    assert_eq!(out[0].anchor, Anchor::Bottom);

    // 正典終了（アクター join）——temp dir は _guard の Drop が片付ける。
    parts.publisher.close();
    let _ = parts.handle.join();
}

/// 実機サインオフ再現檻（多窓・2 スコープ・Bottom）: DragEnd 時に `DraggingState` を
/// 失った char が位置を保存できず、相対追従のバルーンが復元でずれる欠陥を決定論再現する。
///
/// # 背景（実機 emo2 `sylphya.toml` の観測異常）
///
/// 4 窓（scope0/1 の char+balloon）を各々ドラッグしたにもかかわらず `[window.1]` のみ保存され
/// `[window.0]` が欠落（一方 `[balloon-offset.0/1]` は両方保存）。復元時 scope0(むらさき) char が
/// resolver 既定へスナップし、相対追従のバルーン（Req1.6: 位置の単一真実源はキャラ窓）が既定
/// char へ引きずられて位置がずれた。
///
/// # 再現する根本経路（root cause）
///
/// `on_char_drag_end` は保存位置を `policy_mapped_position`（＝`DraggingState` からの生座標
/// 再導出）に依存させており、`DraggingState` 不在なら `None` で**早期 return＝保存 skip** する。
/// しかし非 Free char は連続 `on_char_drag` が既に最終位置へ動かし済みで `WindowPos.position` が
/// 最終確定位置を保持している。dispatch が DragEnd 前に `DraggingState` を落とすと（多窓時に
/// observed・実 flow の穴）、char は動いたのに位置が保存されない——一方 `on_balloon_drag_end` は
/// char の `WindowPos.position` を読んで offset を保存するため balloon-offset だけが残り、実機の
/// 観測状態（`[window.0]` 欠落・`[balloon-offset.0]` 残存）に一致する。
///
/// # 檻の噛み方
///
/// - scope0 char: `DraggingState` **無し**・`WindowPos.position` は連続ドラッグが置いた最終位置。
///   修正前は `on_char_drag_end` が保存 skip → `[window.0]` 欠落 → 復元で char が既定へ落ち、
///   balloon が既定 char へ追従してずれる（RED）。修正後は `WindowPos.position` を最終位置として
///   保存 → 復元で char/balloon とも最終確定位置へ戻る（GREEN）。
/// - scope1 char: `DraggingState` **有り**（正常経路の対照）。修正前後で常に保存・復元される。
///
/// 座標は 96 の非倍数を用い（隠れた dpi/96 再スケールの副次檻）、scope 間・既定値と重ねない。
#[test]
fn dragged_char_persists_even_without_dragging_state_at_dragend() {
    use std::path::PathBuf;

    use areka_ghost::sylphya_wiring::profile_areka_root;
    use areka_parsers::charset::DefaultEncoding;
    use areka_sylphya::persist::FsPersistIo;
    use areka_sylphya::{
        Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };

    use super::super::persist::{
        PersistWiring, apply_restored_placements, load_restored_state,
    };
    use super::on_balloon_drag_end;
    use crate::placement::resolver::ScopePlacement;
    use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

    struct TempGhostDir(PathBuf);
    impl Drop for TempGhostDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // --- fixture: 最小解決可能ゴースト（round_trip 8.2 と同型）------------------------
    let mut root = std::env::temp_dir();
    root.push("areka_follow_dragend_no_dragging_state_repro");
    let _ = std::fs::remove_dir_all(&root);
    let _guard = TempGhostDir(root.clone());
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
    )
    .expect("write ghost descript");
    std::fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");
    let profile_root = profile_areka_root(&ghost_master);
    std::fs::create_dir_all(&profile_root).expect("create profile/areka");

    // --- save 側 sylphya（実 FsPersistIo・実 FS 往復）------------------------------
    let roots = ScopeRoots {
        ghost: Some(profile_root.clone()),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(FsPersistIo),
        runtime_sink: None,
    });

    // work area 下端 1200・単一モニタ。両スコープの Bottom 吸着 y を確定する。
    let snapshot = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1920, 1200)],
    };
    let mut world = World::new();
    world.insert_non_send_resource(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    world.insert_resource(MonitorSnapshot {
        work_areas: snapshot.work_areas.clone(),
    });

    // scope0（むらさき）: char_size(434,687)→ bottom 吸着 y = 1200−687 = 513。
    let s0_size = SizePx { w: 434, h: 687 };
    let s0_char_final = Point { x: 1427, y: 513 };
    let s0_balloon_final = Point { x: 1289, y: 529 };
    // scope1（エモ）: char_size(400,600)→ bottom 吸着 y = 1200−600 = 600。
    let s1_size = SizePx { w: 400, h: 600 };
    let s1_char_final = Point { x: 811, y: 600 };
    let s1_balloon_final = Point { x: 985, y: 727 };

    // scope0 char: DraggingState **無し**（DragEnd 前に dispatch が落とした穴）。連続ドラッグが
    // 既に最終位置へ動かし済みとして WindowPos.position を最終確定位置で spawn する。
    let s0_balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(500, 500),
            BalloonWindowMarker { scope: 0 },
        ))
        .id();
    let s0_char = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(s0_char_final.x, s0_char_final.y, s0_size.w, s0_size.h),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 0 },
            BalloonFollow {
                balloon: s0_balloon,
                offset: PointPx { x: 111, y: 222 },
            },
            // ここに dragging_state を**付けない**のが本檻の肝。
        ))
        .id();
    // scope1 char: DraggingState **有り**（正常経路の対照）。raw.x=811（cursor==drag_start）。
    let s1_balloon = world
        .spawn((
            fake_handle(0x4000),
            window_pos_at(700, 700),
            BalloonWindowMarker { scope: 1 },
        ))
        .id();
    let s1_char = world
        .spawn((
            fake_handle(0x3000),
            window_pos_sized(800, 650, s1_size.w, s1_size.h),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 1 },
            BalloonFollow {
                balloon: s1_balloon,
                offset: PointPx { x: 333, y: 444 },
            },
            dragging_state((s1_char_final.x, 650), (1000, 1000)),
        ))
        .id();

    // --- 実機と同じ順序で 4 回ドラッグ確定（char0 → char1 → balloon0 → balloon1）---
    // char0 DragEnd: DraggingState 不在。修正後は WindowPos.position(1427,513) を最終位置に採る。
    assert!(!on_char_drag_end(
        &mut world,
        s0_char,
        s0_char,
        &Phase::Bubble(drag_end_event_at(s0_char, (1000, 1000))),
    ));
    assert_eq!(
        position_of(&world, s0_char),
        s0_char_final,
        "scope0 char は連続ドラッグ最終位置を保持（DragEnd は位置を変えない）"
    );
    // char1 DragEnd: raw(811,650)→Bottom mapped(811,600)。
    assert!(!on_char_drag_end(
        &mut world,
        s1_char,
        s1_char,
        &Phase::Bubble(drag_end_event_at(s1_char, (1000, 1000))),
    ));
    assert_eq!(
        position_of(&world, s1_char),
        s1_char_final,
        "scope1 char DragEnd 確定位置（bottom 吸着・DraggingState 経路）"
    );

    // balloon の最終確定位置を wndproc が置いたものとして明示設定（move_window=true 相当）。
    world.get_mut::<WindowPos>(s0_balloon).unwrap().position = Some(s0_balloon_final);
    world.get_mut::<WindowPos>(s1_balloon).unwrap().position = Some(s1_balloon_final);
    // balloon0 DragEnd（保存）: char0 の WindowPos.position(1427,513) 基準に offset を保存。
    assert!(!on_balloon_drag_end(
        &mut world,
        s0_balloon,
        s0_balloon,
        &Phase::Bubble(drag_end_event_at(s0_balloon, (0, 0))),
    ));
    // balloon1 DragEnd（保存）。
    assert!(!on_balloon_drag_end(
        &mut world,
        s1_balloon,
        s1_balloon,
        &Phase::Bubble(drag_end_event_at(s1_balloon, (0, 0))),
    ));

    // --- barrier: put が実 FS へ確定 ---------------------------------------------
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // 実 FS を直接読み、両スコープの WindowPos が保存されていることを中間確認する
    // （実機で欠落した [window.0] がここで存在すべき＝修正前はここで RED）。
    // 保存 x は**原点＝下端中央**基準（左上 ＋ char_w/2）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &FsPersistIo);
    for (scope, cf, cw) in [(0u32, s0_char_final, 434), (1u32, s1_char_final, 400)] {
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope,
                    axis: Axis::X
                },
                (cf.x + cw / 2).to_string()
            )) && loaded.contains(&(
                PersistKey::WindowPos {
                    scope,
                    axis: Axis::Y
                },
                cf.y.to_string()
            )),
            "scope{scope} の char 位置 ({},{}) が実 FS へ保存されていない（実機 [window.{scope}] 欠落再現）: {loaded:?}",
            cf.x,
            cf.y
        );
    }

    // --- restore: mount 解決経由で読み、両スコープの合成 placement を merge ----------
    let entries = load_restored_state(&root, DefaultEncoding::Ansi);
    let synth = |scope: usize, size: SizePx| ScopePlacement {
        scope,
        char_pos: PointPx { x: 100, y: 100 }, // 既定（saved と別位置＝復元優先の証明）
        char_size: size,
        balloon_pos: PointPx { x: 107, y: 107 },
        balloon_size: SizePx { w: 200, h: 300 },
        balloon_offset: PointPx { x: 7, y: 7 },
        // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    };
    let out = apply_restored_placements(
        vec![synth(0, s0_size), synth(1, s1_size)],
        &entries,
        &snapshot,
    );
    assert_eq!(out.len(), 2);

    // 期待復元値（両スコープとも saved char + balloon 最終確定位置へ戻る）。
    for (p, cf, bf, size) in [
        (&out[0], s0_char_final, s0_balloon_final, s0_size),
        (&out[1], s1_char_final, s1_balloon_final, s1_size),
    ] {
        let cf_px = PointPx { x: cf.x, y: cf.y };
        let bf_px = PointPx { x: bf.x, y: bf.y };
        assert_eq!(
            p.char_pos, cf_px,
            "復元 char_pos が DragEnd 確定位置と値等価でない（scope{}）",
            p.scope
        );
        assert_ne!(
            p.char_pos,
            PointPx { x: 100, y: 100 },
            "復元 char_pos が既定へ落ちている（scope{} の window 保存欠落）",
            p.scope
        );
        // balloon 往復健全性（純関数側は persist.rs 8.5 群で証明済み・ここは結線の確認）。
        // 保存基準＝ランタイム基準（char 左上）ゆえ、保存値は生 offset そのもの。
        let offset_tl = PointPx {
            x: bf.x - cf.x,
            y: bf.y - cf.y,
        };
        assert!(
            loaded.contains(&(
                PersistKey::BalloonOffset {
                    scope: p.scope as u32,
                    axis: Axis::X
                },
                offset_tl.x.to_string()
            )) && loaded.contains(&(
                PersistKey::BalloonOffset {
                    scope: p.scope as u32,
                    axis: Axis::Y
                },
                offset_tl.y.to_string()
            )),
            "scope{} の balloon offset が char 左上基準の生値で保存されていない（size={size:?}）: {loaded:?}",
            p.scope
        );
        assert_eq!(
            p.balloon_offset, offset_tl,
            "復元 balloon offset が左上基準の生 offset と値等価でない（scope{}）",
            p.scope
        );
        assert_eq!(
            p.balloon_pos, bf_px,
            "復元 balloon_pos が balloon DragEnd 最終確定位置と値等価でない（scope{}）",
            p.scope
        );
    }

    parts.publisher.close();
    let _ = parts.handle.join();
}
