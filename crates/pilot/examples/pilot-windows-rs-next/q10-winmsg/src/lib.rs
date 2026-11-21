pub(crate) mod bindings {
    #![allow(non_camel_case_types, non_snake_case, dead_code, clippy::all)]
    pub type PCSTR = *const u8;
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod util;

use std::{
    any::Any,
    cell::Cell,
    future::Future,
    mem::{ManuallyDrop, MaybeUninit},
    panic::{self, AssertUnwindSafe},
    pin::{pin, Pin},
    ptr::{self, NonNull},
    task::{Context, Poll, Waker},
};

use crate::bindings::*;
use async_task::Runnable;
use util::{Window, WindowType};

use crate::util::MsgFilterHook;

const MSG_ID_WAKE: u32 = WM_USER as u32;

thread_local! {
    static PANIC_PAYLOAD: Cell<Option<Box<dyn Any + Send + 'static>>> = const { Cell::new(None) };
    static EXECUTOR_WINDOW: Window<()> = Window::new(WindowType::MessageOnly, (), |_, msg| {
        if msg.msg == MSG_ID_WAKE {
            let runnable = unsafe {
                let runnable_ptr = NonNull::new_unchecked(msg.lparam as *mut _);
                Runnable::<()>::from_raw(runnable_ptr)
            };
            if let Err(panic_payload) = panic::catch_unwind(|| runnable.run()) {
                PANIC_PAYLOAD.set(Some(panic_payload));
            }
            Some(0)
        } else {
            None
        }
    })
    .unwrap();
}

/// An owned permission to join on a task (await its termination).
///
/// If a `JoinHandle` is dropped, the task continues running in the
/// background and its return value is lost.
pub struct JoinHandle<T> {
    task: ManuallyDrop<async_task::Task<T>>,
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        let task = unsafe { ManuallyDrop::take(&mut self.task) };
        task.detach();
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        pin!(&mut *self.task).poll(cx)
    }
}

unsafe fn spawn_unchecked_lifetime<T>(future: impl Future<Output = T>) -> JoinHandle<T> {
    let hwnd = EXECUTOR_WINDOW.with(|w| w.hwnd());

    // SAFETY: The `future` does not need to be `Send` because the thread that
    // receives the runnable is our own, meaning the runnable is also dropped
    // on the original thread.
    let (runnable, task) = unsafe {
        async_task::spawn_unchecked(future, move |runnable: Runnable| {
            let _ = PostMessageW(hwnd, MSG_ID_WAKE, 0, runnable.into_raw().as_ptr() as _);
        })
    };

    runnable.schedule();

    JoinHandle {
        task: ManuallyDrop::new(task),
    }
}

/// Spawns a new future on the current thread.
///
/// This function may be used to spawn tasks when the message loop is not running.
/// The provided future starts running once the message loop is entered with
/// [`block_on()`] or [`MessageLoop::run()`].
///
/// # Examples
///
/// This example is a compile-time test to ensure that we only accept `'static`
/// return types to prevent <https://github.com/rust-lang/rust/issues/84366>.
///
/// ```compile_fail
/// fn test_fn<'a>() {
///     let closure = || -> &'a str { "" };
///     wintf_winmsg_executor::spawn_local(async move { closure() });
/// }
/// ```
pub fn spawn_local<T: 'static>(future: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    // SAFETY: future is `'static`
    unsafe { spawn_unchecked_lifetime(future) }
}

/// Runs a future to completion on the calling thread's message loop.
///
/// Runs the provided future on the current thread, blocking until it completes.
/// Any tasks spawned from the same thread using [`spawn_local()`] also run concurrently.
/// Note that all spawned tasks are suspended after [`block_on()`] returns.
/// Calling [`block_on()`] again resumes the spawned tasks.
///
/// # Panics
///
/// Panics if the message loop quits before the future completes.
/// This can happen when the future or any spawned task calls the
/// `PostQuitMessage()` WinAPI function.
pub fn block_on<'a, T: 'a>(future: impl Future<Output = T> + 'a) -> T {
    let msg_loop = &MessageLoop::new();

    // SAFETY: All borrowed variables outlive the task itself because we only
    // return from this function after the task has finished.
    let task = unsafe {
        spawn_unchecked_lifetime(async move {
            let result = future.await;
            msg_loop.quit();
            result
        })
    };

    msg_loop.run_loop(|_| FilterResult::Forward);

    poll_ready(task).expect("received unexpected quit message")
}

fn poll_ready<T>(future: impl Future<Output = T>) -> Result<T, ()> {
    let future = pin!(future);
    match future.poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(result) => Ok(result),
        Poll::Pending => Err(()),
    }
}

/// Return value of the filter closure passed to [`MessageLoop::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// The message is forwarded to the window procedure.
    Forward,

    /// The message is dropped and not forwarded to the window procedure.
    Drop,
}

/// Abstract representation of a message loop.
///
/// Not directly constructible. Use [`MessageLoop::run`] to create a message
/// loop. The message loop struct is used to control message loop behavior
/// by passing it as an argument to the filter closure of [`MessageLoop::run`].
pub struct MessageLoop {
    quit: Cell<bool>,
}

impl MessageLoop {
    fn new() -> Self {
        Self {
            quit: Cell::new(false),
        }
    }

    fn run_loop(&self, filter: impl Fn(&MSG) -> FilterResult) {
        let executor_hwnd = EXECUTOR_WINDOW.with(|ew| ew.hwnd());

        while !self.quit.get() {
            unsafe {
                let mut msg = MaybeUninit::uninit();
                if GetMessageW(msg.as_mut_ptr(), ptr::null_mut(), 0, 0) == 0 {
                    return;
                }
                let msg = msg.assume_init();

                // Do not allow the filter to drop our wake messages.
                let is_wake_message = msg.hwnd == executor_hwnd && msg.message == MSG_ID_WAKE;
                if is_wake_message || filter(&msg) == FilterResult::Forward {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                if let Some(panic_payload) = PANIC_PAYLOAD.take() {
                    panic::resume_unwind(panic_payload)
                }
            }
        }
    }

    /// Runs the message loop with a filter closure to inspect and drop messages before
    /// they are dispatched to their respective window procedures.
    ///
    /// Use the [`FilterResult`] return value to control how the message is handled.
    /// The first argument to the filter closure is the [`MessageLoop`] struct itself,
    /// which can be used to quit the message loop.
    ///
    /// Like [`block_on()`], this function runs any tasks spawned from the same thread
    /// using [`spawn_local()`]. All tasks are suspended when this function returns.
    ///
    /// Installs a [`WH_MSGFILTER`] hook to allow inspection of messages while modal
    /// windows are open.
    ///
    /// # Panics and Reentrancy
    ///
    /// Panics if called from within another [`MessageLoop::run()`] filter closure.
    ///
    /// A call to [`block_on()`] from within the filter closure creates a nested message
    /// loop, which causes the filter closure to be re-entered when a modal window is open.
    ///
    /// [`WH_MSGFILTER`]: (https://learn.microsoft.com/en-us/windows/win32/winmsg/about-hooks#wh_msgfilter-and-wh_sysmsgfilter)
    pub fn run(filter: impl Fn(&MessageLoop, &MSG) -> FilterResult) {
        let msg_loop = MessageLoop::new();

        // Any modal window (i.e. a right-click menu) blocks the main message loop
        // and dispatches messages internally. To keep the executor running use a
        // hook to get access to modal windows' internal message loop.
        // SAFETY: The Drop implementation of MsgFilterHook unregisters the hook,
        // ensuring that dispatchers will not be called after the end of the scope.
        let _hook = unsafe {
            MsgFilterHook::register(|msg| {
                panic::catch_unwind(AssertUnwindSafe(|| {
                    let filter_result = filter(&msg_loop, msg);
                    // When `quit()` is called, it has no real effect because we
                    // are running in a modal loop. Post a quit message to exit
                    // the modal message loop to store the panic payload.
                    if msg_loop.quit.get() {
                        let _ = PostMessageW(msg.hwnd, WM_QUIT as u32, 0, 0);
                    }
                    filter_result == FilterResult::Drop
                }))
                .unwrap_or_else(|payload| {
                    PANIC_PAYLOAD.with(|panic_payload| {
                        panic_payload.set(Some(payload));
                    });
                    // Also exit the modal loop ASAP when a panic occurs.
                    let _ = PostMessageW(msg.hwnd, WM_QUIT as u32, 0, 0);
                    false
                })
            })
        };
        msg_loop.run_loop(|msg| filter(&msg_loop, msg));
    }

    /// Quits the message loop as soon as possible.
    pub fn quit(&self) {
        self.quit.set(true);
    }

    /// Quits the message loop when there are no more messages to process.
    pub fn quit_when_idle(&self) {
        unsafe { PostQuitMessage(0) };
    }
}
