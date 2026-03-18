// src/pages/page.rs
//! Core page abstraction and type-erased wrapper for the UI page system.
//!
//! This module defines the [`Page`] trait that all UI pages must implement,
//! along with [`PageWrapper`], an enum-based wrapper that enables heterogeneous
//! storage of concrete page types without dynamic dispatch (`dyn`).
//!
//! # Page Trait
//!
//! [`Page`] defines the lifecycle, rendering, and interaction contract for every
//! screen in the application. Implementors handle their own layout, touch input,
//! dirty-region tracking, and drawing.
//!
//! # PageWrapper
//!
//! Because embedded targets often avoid trait objects, [`PageWrapper`] provides
//! a concrete enum that delegates every [`Page`] method to the inner page type.
//! The [`PageManager`](super::page_manager::PageManager) stores a
//! `heapless::Vec<PageWrapper, N>` and routes calls through this wrapper.

use crate::ui::core::{Action, DirtyRegion, PageId, TouchEvent};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use heapless::Vec;

extern crate alloc;
use alloc::boxed::Box;

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

/// Trait that all navigable UI pages must implement.
///
/// A `Page` owns its layout, state, and dirty-tracking. The
/// [`PageManager`](super::page_manager::PageManager) calls these methods in
/// a well-defined order each frame:
///
/// 1. **`on_activate`** — once, when the page becomes the active page.
/// 2. **`on_event`** — zero or more times per frame for incoming events.
/// 3. **`update`** — once per frame to advance internal state.
/// 4. **`handle_touch`** — when a touch event targets this page.
/// 5. **`draw_page`** — when `is_dirty()` is true.
/// 6. **`on_deactivate`** — once, when navigating away from the page.
pub trait Page {
    /// Unique identifier used for navigation and lookup.
    fn id(&self) -> PageId;

    /// Human-readable title (may appear in headers or debug logs).
    fn title(&self) -> &str;

    /// Called once when this page becomes the active page.
    fn on_activate(&mut self) {}

    /// Called once when this page is no longer the active page.
    fn on_deactivate(&mut self) {}

    /// Process a touch event and optionally return a navigation [`Action`].
    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action>;

    /// Advance per-frame state (animations, timers, etc.).
    fn update(&mut self);

    /// Handle an incoming [`PageEvent`](crate::ui::core::PageEvent).
    ///
    /// Returns `true` if the event was consumed and the page needs a redraw.
    fn on_event(&mut self, _event: &crate::ui::core::PageEvent) -> bool {
        false
    }

    /// Render the entire page to the given display target.
    fn draw_page<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error>;

    /// Bounding rectangle of this page (typically the full screen).
    fn bounds(&self) -> Rectangle;

    /// Whether the page has regions that need redrawing.
    fn is_dirty(&self) -> bool;

    /// Clear the dirty flag after a successful draw.
    fn mark_clean(&mut self);

    /// Force the page to be redrawn on the next frame.
    fn mark_dirty(&mut self);

    /// Return the set of dirty sub-regions for partial-update displays.
    ///
    /// The default implementation returns the full page bounds when dirty.
    fn dirty_regions(&self) -> Vec<DirtyRegion, 8> {
        if self.is_dirty() {
            let mut regions = Vec::new();
            regions.push(DirtyRegion::new(self.bounds())).ok();
            regions
        } else {
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Blanket impl: Box<T> where T: Page
// ---------------------------------------------------------------------------

/// Allows a `Box<T>` to be used anywhere a `Page` is expected, forwarding
/// every call through to the inner value.
impl<T: Page> Page for Box<T> {
    fn id(&self) -> PageId {
        (**self).id()
    }

    fn title(&self) -> &str {
        (**self).title()
    }

    fn on_activate(&mut self) {
        (**self).on_activate()
    }

    fn on_deactivate(&mut self) {
        (**self).on_deactivate()
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        (**self).handle_touch(event)
    }

    fn update(&mut self) {
        (**self).update()
    }

    fn on_event(&mut self, event: &crate::ui::core::PageEvent) -> bool {
        (**self).on_event(event)
    }

    fn draw_page<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        (**self).draw_page(display)
    }

    fn bounds(&self) -> Rectangle {
        (**self).bounds()
    }

    fn is_dirty(&self) -> bool {
        (**self).is_dirty()
    }

    fn mark_clean(&mut self) {
        (**self).mark_clean()
    }

    fn mark_dirty(&mut self) {
        (**self).mark_dirty()
    }
}

// ---------------------------------------------------------------------------
// PageWrapper
// ---------------------------------------------------------------------------

/// Enum-based wrapper that stores one of the concrete page types.
///
/// Using an enum instead of `dyn Page` avoids the overhead (and `alloc`
/// requirements) of trait objects while still allowing the
/// [`PageManager`](super::page_manager::PageManager) to hold a heterogeneous
/// collection of pages.
///
/// Each variant wraps its page in a [`Box`] to keep the enum size uniform
/// regardless of the underlying page's stack footprint.
///
/// When adding a new page to the application, add a variant here and
/// implement the delegation in the [`Page`] impl below.
pub enum PageWrapper {
    Home(Box<crate::pages::home::outdoor::HomePage>),
    HomeGrid(Box<crate::pages::home::grid::HomeGridPage>),
    Settings(Box<crate::pages::settings::SettingsPage>),
    DisplaySettings(Box<crate::pages::settings::DisplaySettingsPage>),
    Monitor(Box<crate::pages::monitor::MonitorPage>),
    TrendPage(Box<crate::pages::trend::TrendPage>),
    WifiStatus(Box<crate::pages::wifi_status::WifiStatusPage>),
}

/// Helper macro to delegate a `Page` method call through every `PageWrapper` variant.
macro_rules! delegate_page {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            PageWrapper::Home(page) => page.$method($($arg),*),
            PageWrapper::HomeGrid(page) => page.$method($($arg),*),
            PageWrapper::Settings(page) => page.$method($($arg),*),
            PageWrapper::DisplaySettings(page) => page.$method($($arg),*),
            PageWrapper::Monitor(page) => page.$method($($arg),*),
            PageWrapper::TrendPage(page) => page.$method($($arg),*),
            PageWrapper::WifiStatus(page) => page.$method($($arg),*),
        }
    };
}

impl Page for PageWrapper {
    fn id(&self) -> PageId {
        delegate_page!(self, id)
    }

    fn title(&self) -> &str {
        delegate_page!(self, title)
    }

    fn on_activate(&mut self) {
        delegate_page!(self, on_activate)
    }

    fn on_deactivate(&mut self) {
        delegate_page!(self, on_deactivate)
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        delegate_page!(self, handle_touch, event)
    }

    fn update(&mut self) {
        delegate_page!(self, update)
    }

    fn on_event(&mut self, event: &crate::ui::core::PageEvent) -> bool {
        delegate_page!(self, on_event, event)
    }

    fn draw_page<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        delegate_page!(self, draw_page, display)
    }

    fn bounds(&self) -> Rectangle {
        delegate_page!(self, bounds)
    }

    fn is_dirty(&self) -> bool {
        delegate_page!(self, is_dirty)
    }

    fn mark_clean(&mut self) {
        delegate_page!(self, mark_clean)
    }

    fn mark_dirty(&mut self) {
        delegate_page!(self, mark_dirty)
    }
}
