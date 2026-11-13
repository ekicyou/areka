//! `parent_window.rs` のテスト（本体の行数を 900 行の内に保つため兄弟のファイルへ置く）。

mod classify_tests {
    use super::super::*;

    // 要件 3.2: HELLO（helper HWND u32 LE 4 バイト）は RecordHello へ分類される。
    #[test]
    fn hello_classifies_to_record_with_decoded_hwnd() {
        for value in [0x1u32, 0x1234_5678, 0xDEAD_BEEF, u32::MAX] {
            let payload = value.to_le_bytes();
            let raw = MsgTag::Hello.as_u32() as usize;
            let action = classify_inbound(raw, payload.len(), &payload);
            assert_eq!(
                action,
                InboundAction::RecordHello(value),
                "HELLO payload {value:#x} を decode_hwnd_le で復号して記録する"
            );
        }
    }

    // 要件 3.2: HELLO 復号は copydata_payload→decode_hwnd_le の等価性を満たす。
    #[test]
    fn hello_decode_matches_ipc_primitives() {
        let value = 0xCAFE_F00Du32;
        let payload = encode_hwnd_le(hwnd_from_u32(value));
        let raw = MsgTag::Hello.as_u32() as usize;
        assert_eq!(
            classify_inbound(raw, payload.len(), &payload),
            InboundAction::RecordHello(value)
        );
    }

    // 要件 2.5: HELLO だが payload 長が 4 でない → 記録せず不正扱い。
    #[test]
    fn malformed_hello_is_ignored() {
        let raw = MsgTag::Hello.as_u32() as usize;
        // 3 バイト（短い）
        assert_eq!(
            classify_inbound(raw, 3, b"abc"),
            InboundAction::IgnoreMalformedHello
        );
        // 5 バイト（長い）
        assert_eq!(
            classify_inbound(raw, 5, b"abcde"),
            InboundAction::IgnoreMalformedHello
        );
        // 空
        assert_eq!(
            classify_inbound(raw, 0, b""),
            InboundAction::IgnoreMalformedHello
        );
    }

    // 要件 2.5: 親が能動処理しない既知タグ（Load/Request/Unload）は IgnoreKnown（記録のみ・crash なし）。
    #[test]
    fn known_nonhello_tags_are_ignored_known() {
        for tag in [MsgTag::Load, MsgTag::Request, MsgTag::Unload] {
            let raw = tag.as_u32() as usize;
            assert_eq!(
                classify_inbound(raw, 0, b""),
                InboundAction::IgnoreKnown(tag)
            );
        }
    }

    // 要件 4.2: RESPONSE は payload を StoreResponse へ分類する（再入受領で slot へ store）。
    #[test]
    fn response_classifies_to_store_with_payload() {
        let raw = MsgTag::Response.as_u32() as usize;
        let payload: &[u8] = b"echo-response";
        assert_eq!(
            classify_inbound(raw, payload.len(), payload),
            InboundAction::StoreResponse(payload.to_vec()),
            "RESPONSE payload を store 用に持ち上げる（要件 4.2）"
        );
        // 空 payload の RESPONSE も store 対象（0 バイト応答）。
        assert_eq!(
            classify_inbound(raw, 0, b""),
            InboundAction::StoreResponse(Vec::new())
        );
    }

    // 要件 2.5: 未知タグは crash させず IgnoreBad（記録のみ）。
    #[test]
    fn unknown_tag_is_ignored_as_bad() {
        let action = classify_inbound(0xFFusize, 0, b"");
        assert!(matches!(action, InboundAction::IgnoreBad(_)));
    }

    // 要件 2.5: cbData と実長の不整合は crash させず IgnoreBad。
    #[test]
    fn length_mismatch_is_ignored_as_bad() {
        let raw = MsgTag::Hello.as_u32() as usize;
        let action = classify_inbound(raw, 10, b"abc");
        assert!(matches!(action, InboundAction::IgnoreBad(_)));
    }
}

mod gate_tests {
    use super::super::*;

    // 要件 3.3: helper HWND 未確定なら送信ゲートが Incomplete で弾く（窓なしの純ロジック）。
    #[test]
    fn gate_rejects_when_helper_hwnd_unset() {
        let err = resolve_send_target(None).expect_err("未確定は Err で弾く");
        assert!(
            matches!(err, SendError::Handshake(HandshakeError::Incomplete)),
            "未ハンドシェイク送信は Incomplete（要件 3.3）"
        );
    }

    // 要件 3.3: helper HWND 確定なら target HWND を返し送信を許可する。
    #[test]
    fn gate_allows_when_helper_hwnd_set() {
        let wire: u32 = 0x1234_5678;
        let target = resolve_send_target(Some(wire)).expect("確定なら Ok");
        assert_eq!(
            target,
            hwnd_from_u32(wire),
            "確定 helper HWND を target とする"
        );
    }

    // SendError の Display/Debug（要件 3.3 / 5.2 の一様報告）。
    #[test]
    fn send_error_variants_render() {
        let hs = SendError::Handshake(HandshakeError::Incomplete);
        let ipc = SendError::Ipc(shiori_host32_ipc::IpcError::Timeout);
        assert!(!format!("{hs}").is_empty());
        assert!(!format!("{ipc}").is_empty());
        assert!(!format!("{hs:?}").is_empty());
    }
}

mod window_tests {
    use super::super::*;
    use windows::Win32::System::DataExchange::COPYDATASTRUCT;

    /// 親窓 1 つへの loopback テスト（deterministic）。以下を順に検証する:
    ///
    /// - (a) HELLO 未受領で短い timeout の `pump_until_hello_or` が `None`（要件 3.4・Timeout 経路）。
    /// - (b) 同じ親窓へ自分の窓を名乗る HELLO を loopback 送出 → `pump_until_hello_or` が `Some(helper_hwnd)` を返し helper_hwnd が確定する（要件 3.2）。
    /// - (c) 無視したメッセージの記録は [`ignored_frames_are_logged`] が確かめる。
    /// - (d) RESPONSE を loopback 送出 → WndProc の Response アームが `response_slot` へ store する（要件 4.2）。
    /// - (e) 無応答相手（親自身）へ `send_request` → 上限時間で `SendError::Ipc(Timeout)`・親はハングしない（要件 5.1/5.2/5.3）。
    ///
    /// bounded: いずれの pump / send_request も timeout / 受領 / quit で必ず抜ける（無限ループ禁止）。
    #[test]
    fn pump_times_out_then_hello_response_and_send_timeout() {
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");

        // --- (a) HELLO 未受領 → 短い timeout で None（要件 3.4）---
        // 的は None で返ること（返れば bounded）。経過時間は確かめない（指摘 48）。
        let none = parent.pump_until_hello_or(Duration::from_millis(120));
        assert_eq!(
            none, None,
            "HELLO 未受領なら pump は None を返す（要件 3.4）"
        );
        assert_eq!(parent.shared().helper_hwnd.get(), None);

        // --- (b) HELLO を loopback 送出 → pump が Some(helper_hwnd) を返す（要件 3.2）---
        // 親窓宛に HELLO（helper HWND u32 LE）を送る。送り手と中身の窓が同じでないと捨てられる
        // （指摘 38）ので、親自身を helper として名乗る。send_copydata は同期送出ゆえ、
        // WndProc が呼ばれ helper_hwnd が確定した状態で戻る。
        let helper_hwnd_wire: u32 = parent.hwnd_u32();
        let hello_payload = helper_hwnd_wire.to_le_bytes();
        shiori_host32_ipc::send_copydata(
            parent.hwnd(),
            parent.hwnd(),
            MsgTag::Hello,
            &hello_payload,
            Duration::from_secs(5),
        )
        .expect("HELLO loopback 送出に失敗");

        // 送出直後に helper_hwnd は確定しているはず（send_copydata は同期）。
        assert_eq!(
            parent.shared().helper_hwnd.get(),
            Some(helper_hwnd_wire),
            "HELLO 受領で helper HWND が確定する（要件 3.2）"
        );

        // 確定後の pump は即 Some を返す（bounded・要件 3.2）。
        let some = parent.pump_until_hello_or(Duration::from_secs(5));
        assert_eq!(
            some,
            Some(helper_hwnd_wire),
            "HELLO 受領後 pump は Some(helper_hwnd) を返す（要件 3.2）"
        );

        // --- (d) RESPONSE を親窓へ loopback → ResponseSlot に payload が store される（要件 4.2）---
        // WndProc の Response アームが store することを観測する。send_copydata は同期ゆえ、
        // 復帰時点で slot に格納済み。
        {
            parent.shared().response_slot.clear();
            let resp_payload: &[u8] = b"echoed-bytes";
            shiori_host32_ipc::send_copydata(
                parent.hwnd(),
                parent.hwnd(),
                MsgTag::Response,
                resp_payload,
                Duration::from_secs(5),
            )
            .expect("RESPONSE loopback 送出に失敗");
            assert_eq!(
                parent.shared().response_slot.take(),
                Some(resp_payload.to_vec()),
                "RESPONSE の WndProc アームが payload を ResponseSlot へ store する（要件 4.2）"
            );
        }

        // --- (e) 無応答相手への send_request は上限時間で Timeout（要件 5.1/5.2/5.3）---
        // helper_hwnd は (b) で親自身。REQUEST を親 WndProc が受けても RESPONSE を
        // store しない（Request アームは IgnoreKnown）→ slot は空のまま → Timeout。
        // SMTO_ABORTIFHUNG ＋短 timeout で有限復帰し親はハングしない。
        {
            let before = Instant::now();
            let result = parent.send_request(MsgTag::Request, b"ping", Duration::from_millis(150));
            assert!(
                matches!(
                    result,
                    Err(SendError::Ipc(shiori_host32_ipc::IpcError::Timeout))
                ),
                "無応答時は Timeout で復帰する（要件 5.2）: got {result:?}"
            );
            // 的は「止まらずに返る」。混んだ PC でも揺れないよう上限は広く取る（指摘 48）。
            assert!(
                before.elapsed() < Duration::from_secs(30),
                "send_request は SMTO_ABORTIFHUNG ＋上限時間で有限復帰する（要件 5.3）"
            );
        }

        drop(parent);
    }

    /// 窓 2 組が並んで生きる（指摘 40）: 2 つのスレッドで 1 つずつ親窓を作り、両方が生きている
    /// あいだにそれぞれへ自分あての HELLO を送って受けられる。1 プロセスに親窓が 1 つだけという制限は無い
    /// （本番で 32bit のゴーストを 2 体同時に動かすと、各 SHIORI のスレッドが自分の窓を作る）。
    #[test]
    fn two_parent_windows_live_side_by_side_on_two_threads() {
        // 生成も同じ瞬間に重ねる（`ready`）。
        let ready = std::sync::Barrier::new(2);
        let both_created = std::sync::Barrier::new(2);
        let both_checked = std::sync::Barrier::new(2);
        let wires: Vec<u32> = std::thread::scope(|s| {
            let workers: Vec<_> = (0..2)
                .map(|_| {
                    s.spawn(|| {
                        ready.wait();
                        let parent =
                            ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
                        both_created.wait();
                        let own = parent.hwnd_u32();
                        send_raw(&parent, MsgTag::Hello.as_u32() as usize, &own.to_le_bytes());
                        assert_eq!(parent.shared().helper_hwnd.get(), Some(own));
                        // 相手の窓も確かめ終わるまで自分の窓を生かしておく。
                        both_checked.wait();
                        own
                    })
                })
                .collect();
            workers.into_iter().map(|w| w.join().unwrap()).collect()
        });
        assert_ne!(wires[0], wires[1], "2 組は別の窓");
    }

    /// 生の WM_COPYDATA を親窓へ同期送出する（`dwData`・中身をそのまま渡す・送り手は親自身）。
    fn send_raw(parent: &ParentMessageWindow, dw_data: usize, payload: &[u8]) {
        send_raw_from(parent, parent.hwnd_u32(), dw_data, payload);
    }

    /// 送り手の窓（`wParam`）を `sender` と名乗って生の WM_COPYDATA を親窓へ同期送出する。
    fn send_raw_from(parent: &ParentMessageWindow, sender: u32, dw_data: usize, payload: &[u8]) {
        let cds = COPYDATASTRUCT {
            dwData: dw_data,
            cbData: payload.len() as u32,
            lpData: payload.as_ptr() as *mut core::ffi::c_void,
        };
        // SAFETY: parent.hwnd() は有効。&cds と payload は本呼び出し中生存。
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                parent.hwnd(),
                WM_COPYDATA,
                Some(WPARAM(sender as usize)),
                Some(LPARAM(&cds as *const COPYDATASTRUCT as isize)),
            );
        }
    }

    /// 無視したメッセージ（未知のタグ・形の崩れた HELLO・親が受けない既知の種類・lparam が空）は crash させず、
    /// それぞれ `parent_frame_ignored` の WARN を 1 行ずつ残す（指摘 11・要件 3.1 / 4.1）。
    /// 無視したメッセージは helper HWND を書き換えない。
    #[test]
    fn ignored_frames_are_logged() {
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");

        let ((), events) = log_capture_kit::capture(|| {
            send_raw(&parent, 0xFF, b"");
            send_raw(&parent, MsgTag::Hello.as_u32() as usize, b"abc");
            send_raw(&parent, MsgTag::Load.as_u32() as usize, b"");
            // SAFETY: parent.hwnd() は有効。lparam が空の WM_COPYDATA を同期送出するだけ。
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    parent.hwnd(),
                    WM_COPYDATA,
                    Some(WPARAM(parent.hwnd_u32() as usize)),
                    Some(LPARAM(0)),
                );
            }
        });
        let reasons: Vec<_> = events
            .iter()
            .filter(|e| {
                e.target == "shiori-host32" && e.field_str("event") == Some("parent_frame_ignored")
            })
            .map(|e| {
                assert_eq!(e.level, tracing::Level::WARN, "{e:?}");
                e.field_str("reason").unwrap_or_default().to_owned()
            })
            .collect();
        assert_eq!(
            reasons,
            [
                "unknown_tag",
                "malformed_hello",
                "not_for_parent",
                "null_lparam"
            ],
            "{events:?}"
        );
        assert_eq!(parent.shared().helper_hwnd.get(), None);
    }

    /// `parent_frame_ignored` の WARN の `reason` を順に取り出す。
    fn ignored_reasons(events: &[log_capture_kit::CapturedEvent]) -> Vec<String> {
        events
            .iter()
            .filter(|e| {
                e.target == "shiori-host32" && e.field_str("event") == Some("parent_frame_ignored")
            })
            .map(|e| {
                assert_eq!(e.level, tracing::Level::WARN, "{e:?}");
                e.field_str("reason").unwrap_or_default().to_owned()
            })
            .collect()
    }

    /// よその送り手の HELLO を捨てる（指摘 38）: 送り手の窓と中身の窓が違う HELLO と、
    /// `expect_helper_pid` の番号でないプロセスの窓からの HELLO は記録して捨てる。
    /// 伝えた番号のプロセス（ここでは自分）の窓から名乗った HELLO は受ける。
    #[test]
    fn hello_from_a_foreign_sender_is_dropped() {
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
        let own = parent.hwnd_u32();
        let hello = MsgTag::Hello.as_u32() as usize;

        let ((), events) = log_capture_kit::capture(|| {
            // 送り手は親の窓・中身は別の窓。
            send_raw_from(&parent, own, hello, &0x0BAD_F00Du32.to_le_bytes());
            // 送り手と中身は同じだが、伝えた番号のプロセスの窓でない。
            parent.expect_helper_pid(std::process::id().wrapping_add(1));
            send_raw_from(&parent, own, hello, &own.to_le_bytes());
        });
        assert_eq!(
            ignored_reasons(&events),
            ["hello_sender_mismatch", "hello_from_other_process"],
            "{events:?}"
        );
        assert_eq!(parent.shared().helper_hwnd.get(), None);

        parent.expect_helper_pid(std::process::id());
        send_raw_from(&parent, own, hello, &own.to_le_bytes());
        assert_eq!(parent.shared().helper_hwnd.get(), Some(own));
    }

    /// 2 度目の HELLO を捨てる（指摘 38）: helper の窓が決まった後の HELLO は記録して捨て、
    /// 決まった窓は変わらない。
    #[test]
    fn a_second_hello_is_dropped() {
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
        let own = parent.hwnd_u32();
        let hello = MsgTag::Hello.as_u32() as usize;

        send_raw_from(&parent, own, hello, &own.to_le_bytes());
        assert_eq!(parent.shared().helper_hwnd.get(), Some(own));

        let ((), events) =
            log_capture_kit::capture(|| send_raw_from(&parent, own, hello, &own.to_le_bytes()));
        assert_eq!(ignored_reasons(&events), ["second_hello"], "{events:?}");
        assert_eq!(parent.shared().helper_hwnd.get(), Some(own));
    }

    /// helper でない送り手の RESPONSE を捨てる（指摘 38）: 握手の前の RESPONSE と、
    /// 送り手の窓が helper の窓でない RESPONSE は受け皿に入れず記録して捨てる。
    #[test]
    fn a_response_from_a_non_helper_sender_is_dropped() {
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
        let own = parent.hwnd_u32();
        let response = MsgTag::Response.as_u32() as usize;

        let ((), events) = log_capture_kit::capture(|| {
            // 握手の前（helper の窓が決まっていない）。
            send_raw_from(&parent, own, response, b"before-hello");
            send_raw_from(
                &parent,
                own,
                MsgTag::Hello.as_u32() as usize,
                &own.to_le_bytes(),
            );
            // helper の窓（親自身）でない送り手。
            send_raw_from(&parent, 0x0BAD_F00D, response, b"foreign");
        });
        assert_eq!(
            ignored_reasons(&events),
            ["response_not_from_helper", "response_not_from_helper"],
            "{events:?}"
        );
        assert_eq!(parent.shared().response_slot.take(), None);

        send_raw_from(&parent, own, response, b"from-helper");
        assert_eq!(
            parent.shared().response_slot.take(),
            Some(b"from-helper".to_vec())
        );
    }
}
