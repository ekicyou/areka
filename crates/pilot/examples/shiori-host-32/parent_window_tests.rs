//! `parent_window.rs` の受け皿セルと 1 往復の送りのテスト（兄弟のファイル）。

use super::*;

/// 受け皿セル: 空にして取ると無し・入れて取ると 1 回で消える・空にすると残りが消える。
#[test]
fn response_slot_clear_store_take_semantics() {
    let slot = ResponseSlot::default();
    assert_eq!(slot.take(), None);
    slot.clear();
    assert_eq!(slot.take(), None);

    slot.store(b"Value: hello".to_vec());
    assert_eq!(slot.take().as_deref(), Some(&b"Value: hello"[..]));
    assert_eq!(slot.take(), None);

    slot.store(b"stale".to_vec());
    slot.clear();
    assert_eq!(slot.take(), None);
}

/// `wparam` の送り手へ RESPONSE を返すかどうかだけ選べる、同じスレッドの相手の窓。
fn peer_window(reply: Option<&'static [u8]>) -> Window<()> {
    Window::new_checked(
        WindowType::MessageOnly,
        (),
        move |_s: Pin<&()>, m: WindowMessage| -> Option<LRESULT> {
            if m.msg != WM_COPYDATA {
                return None;
            }
            if let Some(body) = reply {
                let parent = HWND(m.wparam.0 as *mut core::ffi::c_void);
                ipc::send_copydata(
                    parent,
                    m.hwnd,
                    MsgTag::Response,
                    body,
                    Duration::from_secs(1),
                )
                .expect("RESPONSE を送れる");
            }
            Some(LRESULT(0))
        },
    )
    .expect("message-only 窓を作れる")
}

/// 送れたが RESPONSE が来なかったことは `Ok(None)`（時間切れと呼ばない）。
#[test]
fn request_delivered_without_response_is_none() {
    let parent = ParentMessageWindow::create().expect("親の窓を作れる");
    let peer = peer_window(None);
    let got = parent.request(peer.hwnd(), b"GET", Duration::from_secs(1));
    assert_eq!(got, Ok(None));
}

/// 送りの間に入れ子で届いた RESPONSE を取り、前の往復の残りは返さない。
#[test]
fn request_returns_nested_response_and_drops_stale() {
    let parent = ParentMessageWindow::create().expect("親の窓を作れる");
    parent
        .window
        .state()
        .get_ref()
        .response_slot
        .store(b"stale".to_vec());
    let peer = peer_window(Some(b"Value: hi"));
    let got = parent.request(peer.hwnd(), b"GET", Duration::from_secs(1));
    assert_eq!(got, Ok(Some(b"Value: hi".to_vec())));

    let silent = peer_window(None);
    assert_eq!(
        parent.request(silent.hwnd(), b"GET", Duration::from_secs(1)),
        Ok(None),
        "前の往復の応答が残らない"
    );
}
