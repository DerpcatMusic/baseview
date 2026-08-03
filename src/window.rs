use std::marker::PhantomData;

use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

use crate::event::{Event, EventStatus};
use crate::window_open_options::WindowOpenOptions;
use crate::{MouseCursor, Size};

/// An X11 drag could not be registered with this window's live event queue.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatariX11ScheduleError {
    /// The primary pointer gesture is no longer owned by this editor.
    AuthorityUnavailable,
    /// Another outbound drag already owns this editor's X11 queue.
    Busy,
    /// The editor did not expose a usable X11 window handle.
    MissingWindowHandle,
}

#[cfg(target_os = "linux")]
impl std::fmt::Display for MatariX11ScheduleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AuthorityUnavailable => "X11 drag requires the current primary pointer gesture",
            Self::Busy => "an X11 drag is already active for this window",
            Self::MissingWindowHandle => "the editor does not expose a valid X11 window handle",
        })
    }
}

#[cfg(target_os = "linux")]
impl std::error::Error for MatariX11ScheduleError {}

#[cfg(target_os = "macos")]
use crate::macos as platform;
#[cfg(target_os = "windows")]
use crate::win as platform;
#[cfg(target_os = "linux")]
use crate::x11 as platform;

pub struct WindowHandle {
    window_handle: platform::WindowHandle,
    // so that WindowHandle is !Send on all platforms
    phantom: PhantomData<*mut ()>,
}

impl WindowHandle {
    fn new(window_handle: platform::WindowHandle) -> Self {
        Self { window_handle, phantom: PhantomData }
    }

    /// Close the window
    pub fn close(&mut self) {
        self.window_handle.close();
    }

    /// Returns `true` if the window is still open, and returns `false`
    /// if the window was closed/dropped.
    pub fn is_open(&self) -> bool {
        self.window_handle.is_open()
    }
}

unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.window_handle.raw_window_handle()
    }
}

pub trait WindowHandler {
    fn on_frame(&mut self, window: &mut Window);
    fn on_event(&mut self, window: &mut Window, event: Event) -> EventStatus;
}

pub struct Window<'a> {
    window: platform::Window<'a>,

    // so that Window is !Send on all platforms
    phantom: PhantomData<*mut ()>,
}

impl<'a> Window<'a> {
    /// Whether this callback has the primary-pointer authority required to
    /// start an X11 outbound file drag.
    #[cfg(target_os = "linux")]
    #[must_use]
    pub fn can_schedule_matari_x11_drag(&self) -> bool {
        self.window.can_schedule_matari_x11_drag()
    }

    /// Start an outbound X11 drag from the live toolkit-owned event queue.
    ///
    /// The ticket is returned unchanged when the gesture cannot be committed;
    /// once native startup begins, lifecycle failures are reported by Matari.
    #[cfg(target_os = "linux")]
    pub fn schedule_matari_x11_drag(
        &self,
        ticket: matari_audio_drag_and_drop::StartTicket,
    ) -> Result<(), matari_audio_drag_and_drop::RejectedStart<MatariX11ScheduleError>> {
        self.window.schedule_matari_x11_drag(ticket)
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn new(window: platform::Window<'a>) -> Window<'a> {
        Window { window, phantom: PhantomData }
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn new(window: platform::Window<'a>) -> Window<'a> {
        Window { window, phantom: PhantomData }
    }

    pub fn open_parented<P, H, B>(parent: &P, options: WindowOpenOptions, build: B) -> WindowHandle
    where
        P: HasRawWindowHandle,
        H: WindowHandler + 'static,
        B: FnOnce(&mut Window) -> H,
        B: Send + 'static,
    {
        let window_handle = platform::Window::open_parented::<P, H, B>(parent, options, build);
        WindowHandle::new(window_handle)
    }

    pub fn open_blocking<H, B>(options: WindowOpenOptions, build: B)
    where
        H: WindowHandler + 'static,
        B: FnOnce(&mut Window) -> H,
        B: Send + 'static,
    {
        platform::Window::open_blocking::<H, B>(options, build)
    }

    /// Close the window
    pub fn close(&mut self) {
        self.window.close();
    }

    /// Resize the window to the given size. The size is always in logical pixels. DPI scaling will
    /// automatically be accounted for.
    pub fn resize(&mut self, size: Size) {
        self.window.resize(size);
    }

    /// Re-interpret the window at a new content-scale factor, holding the
    /// logical size constant and growing/shrinking the physical size to match.
    ///
    /// This is for embedded plug-in views whose scale becomes known only after
    /// the window is attached (the `WindowScalePolicy` chosen at open is
    /// otherwise fixed for the window's lifetime). Implemented on X11, where a
    /// host may report its content scale late via
    /// `IPlugViewContentScaleSupport`; a no-op on Windows and macOS, which
    /// drive DPI through the OS. The change is applied on the next event-loop
    /// iteration and reported to the handler via a `Resized` event.
    pub fn set_scale_factor(&mut self, scale: f64) {
        self.window.set_scale_factor(scale);
    }

    pub fn set_mouse_cursor(&mut self, cursor: MouseCursor) {
        self.window.set_mouse_cursor(cursor);
    }

    pub fn has_focus(&mut self) -> bool {
        self.window.has_focus()
    }

    pub fn focus(&mut self) {
        self.window.focus()
    }

    /// If provided, then an OpenGL context will be created for this window. You'll be able to
    /// access this context through [crate::Window::gl_context].
    #[cfg(feature = "opengl")]
    pub fn gl_context(&self) -> Option<&crate::gl::GlContext> {
        self.window.gl_context()
    }
}

unsafe impl<'a> HasRawWindowHandle for Window<'a> {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.window.raw_window_handle()
    }
}

unsafe impl<'a> HasRawDisplayHandle for Window<'a> {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.window.raw_display_handle()
    }
}
