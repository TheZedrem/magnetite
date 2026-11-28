//! The keyboard finds its way back to the page under Wayland.
//!
//! There a page is a subsurface of the frame (ADR-0012), and Chromium alone
//! decides which of the two has the keyboard. It forgets the page: when the
//! window gets the keyboard back from another program, the frame has it,
//! and with it the sidebar or the top bar. A page's keys (space to play a
//! video) then do nothing until a click into the page.
//!
//! - A page that loses the keyboard while the frame does not get it in the
//!   same turn lost it to another program (or to a window of ours above it,
//!   such as the command bar): it is remembered (`page_lost_keyboard`), and
//!   gets the keyboard back when the frame is next activated
//!   (`frame_got_keyboard`). A click into the sidebar moves the keyboard to
//!   the frame in one turn, so the sidebar keeps what the user gave it.
//! - The top bar has no field to type in: it hands the keyboard back at once
//!   (`top_bar_took_keyboard`).
//!
//! Under X11 a page is a window of its own, and the window manager gives
//! the keyboard back to the one that had it.

use cef::{ImplView, ImplWindow};

use crate::cef_app::runs_on_x11;
use crate::engine::{Engine, WindowId};
use crate::events::ContentId;

impl Engine {
    /// The keyboard goes to page or panel `had` if it is on screen, else to
    /// the one that had it last, else to the page in front.
    pub(crate) fn keyboard_to_page(&self, had: Option<ContentId>) {
        let last = self.lock().win.last_focused;
        let page = [had, last, self.top_page_id()]
            .into_iter()
            .flatten()
            .find_map(|id| {
                self.content_handles(id)
                    .filter(|(w, _)| w.is_visible() != 0)
            });
        if let Some((_, view)) = page {
            view.request_focus();
        }
    }

    /// Page `id`'s window is no longer the active one: remembered, unless
    /// the frame took the keyboard. Call it once the turn that said so is
    /// over (posted): by then the frame is active if the keyboard only moved
    /// to the sidebar.
    pub(crate) fn page_lost_keyboard(&self, id: ContentId) {
        if runs_on_x11() {
            return;
        }
        let home = self.lock().home_of(id);
        self.in_window(home, || {
            let frame_active = self.frame_window().is_some_and(|w| w.is_active() != 0);
            self.lock().win.keyboard_left = (!frame_active).then_some(id);
        });
    }

    /// Window `win`'s frame was activated: the page that lost the keyboard
    /// to another program gets it back. Posted: the keyboard moves again,
    /// which is not for inside the activation, and a page that lost the
    /// keyboard in this turn has said so by then (`page_lost_keyboard`).
    pub(crate) fn frame_got_keyboard(win: WindowId) {
        if runs_on_x11() {
            return;
        }
        Engine::post(Box::new(move || {
            let engine = Engine::global();
            engine.in_window(win, || {
                let left = engine.lock().win.keyboard_left.take();
                if left.is_some() {
                    engine.keyboard_to_page(left);
                }
            });
        }));
    }

    /// The top bar of window `win` got the keyboard (a click on one of its
    /// buttons): back to the page. Posted, as in `frame_got_keyboard`.
    pub(crate) fn top_bar_took_keyboard(win: WindowId) {
        if runs_on_x11() {
            return;
        }
        Engine::post(Box::new(move || {
            let engine = Engine::global();
            engine.in_window(win, || engine.keyboard_to_page(None));
        }));
    }
}
