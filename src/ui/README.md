# Baro UI System

A modular, efficient UI framework for embedded displays built on top of `embedded-graphics`.

## Overview

The Baro UI System provides a comprehensive set of tools for building responsive, efficient user interfaces on resource-constrained embedded devices. It features:

- **Dirty Region Tracking** - Only redraw what changed to minimize flickering and improve performance
- **Event System** - Pages can subscribe to sensor updates, storage events, and system events
- **Flexible Layouts** - Containers with horizontal/vertical alignment and flexible sizing
- **Scrollable Content** - Handle content that exceeds viewport bounds
- **Styled Components** - Easy-to-use styling system with themes and variants
- **Touch Handling** - Full touch event support for interactive elements

## Architecture

The UI system is organized into several modules:

```
ui/
├── core.rs           # Core traits and types
├── styling.rs        # Styling system (themes, colors, padding)
├── components/       # Reusable UI components
│   ├── button.rs
│   └── text.rs
└── layouts/          # Layout containers
    ├── container.rs
    └── scrollable.rs
```

## Core Concepts

### Drawable Trait

All UI elements implement the `Drawable` trait:

```rust
pub trait Drawable {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error>;
    fn bounds(&self) -> Rectangle;
    fn is_dirty(&self) -> bool;
    fn mark_clean(&mut self);
    fn mark_dirty(&mut self);
    fn dirty_region(&self) -> Option<DirtyRegion>;
}
```

### Touchable Trait

Interactive elements implement the `Touchable` trait:

```rust
pub trait Touchable {
    fn contains_point(&self, point: TouchPoint) -> bool;
    fn handle_touch(&mut self, event: TouchEvent) -> TouchResult;
}
```

### Touch Events

Touch events flow through the UI hierarchy:

```rust
pub enum TouchEvent {
    Press(TouchPoint),    // Initial touch
    Drag(TouchPoint),     // Touch moved
}

pub enum TouchResult {
    Handled,              // Event was handled
    NotHandled,           // Pass to next element
    Action(Action),       // Triggered an action
}
```

## Styling System

### Themes and Color Palettes

Create consistent visual styles with themes:

```rust
use crate::ui::{Theme, ColorPalette};

// Use default dark theme
let theme = Theme::dark();

// Or create custom palette
let palette = ColorPalette {
    primary: Rgb565::CSS_DODGER_BLUE,
    background: Rgb565::BLACK,
    text_primary: Rgb565::WHITE,
    // ... other colors
};
```

### Padding and Spacing

Apply consistent spacing throughout your UI:

```rust
use crate::ui::{Padding, Spacing};

// All sides equal
let padding = Padding::all(8);

// Vertical and horizontal
let padding = Padding::symmetric(12, 16); // vertical, horizontal

// Individual sides
let padding = Padding::new(8, 16, 8, 16); // top, right, bottom, left

// Use theme spacing
let theme = Theme::default();
let padding = Padding::all(theme.spacing.medium);
```

### Style Objects

Apply styles to components:

```rust
use crate::ui::{Style, Padding};

let style = Style::new()
    .with_background(Rgb565::CSS_DODGER_BLUE)
    .with_foreground(Rgb565::WHITE)
    .with_border(Rgb565::CSS_GRAY, 2)
    .with_padding(Padding::all(8));
```

## Components

### Button

Interactive buttons with multiple variants:

```rust
use crate::ui::{Button, ButtonVariant, ColorPalette, Action, PageId};

let button = Button::new(
    Rectangle::new(Point::new(20, 50), Size::new(280, 50)),
    "Settings",
    Action::NavigateToPage(PageId::Settings),
)
.with_variant(ButtonVariant::Primary)
.with_palette(ColorPalette::default())
.with_border_radius(8);

// Draw the button
button.draw(&mut display)?;

// Handle touch events
match button.handle_touch(event) {
    TouchResult::Action(action) => {
        // Button was clicked, handle action
    }
    _ => {}
}
```

**Button Variants:**
- `Primary` - Filled button with primary color
- `Secondary` - Filled button with secondary color
- `Outline` - Border-only button
- `Text` - Text-only button with no background

### Text Component

Display single-line text:

```rust
use crate::ui::{TextComponent, TextSize};
use embedded_graphics::text::Alignment;

let text = TextComponent::new(
    Rectangle::new(Point::new(20, 50), Size::new(280, 20)),
    "Temperature: 23.5°C",
    TextSize::Medium,
)
.with_alignment(Alignment::Left)
.with_style(Style::new().with_foreground(Rgb565::WHITE));

// Update text dynamically
text.set_text("Temperature: 24.1°C");
```

**Text Sizes:**
- `TextSize::Small` - 5x8 font
- `TextSize::Medium` - 6x10 font (default)
- `TextSize::Large` - 10x20 font

### Multi-line Text

Display text with automatic word wrapping:

```rust
use crate::ui::MultiLineText;

let text = MultiLineText::new(
    Rectangle::new(Point::new(10, 50), Size::new(300, 100)),
    "This is a longer text that will automatically wrap to multiple lines based on the available width.",
    TextSize::Medium,
)
.with_line_spacing(2)
.with_style(Style::new().with_foreground(Rgb565::WHITE));

// Update text
text.set_text("New multi-line content...");
```

## Layouts

### Container

Arrange children in horizontal or vertical layouts with flexible sizing:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint};

let mut container = Container::<4>::new(
    Rectangle::new(Point::zero(), Size::new(320, 240)),
    Direction::Vertical,
)
.with_alignment(Alignment::Start)
.with_spacing(8)
.with_style(Style::new().with_background(Rgb565::BLACK));

// Add children
container.add_child(Size::new(300, 50), SizeConstraint::Fit)?;
container.add_child(Size::new(300, 0), SizeConstraint::Expand)?;
container.add_child(Size::new(300, 40), SizeConstraint::Fixed(40))?;

// Get bounds for each child to render them
if let Some(bounds) = container.child_bounds(0) {
    // Draw first child within these bounds
}
```

**Direction:**
- `Horizontal` - Arrange children left to right
- `Vertical` - Arrange children top to bottom

**Alignment:**
- `Start` - Align to start (left/top)
- `Center` - Center alignment
- `End` - Align to end (right/bottom)
- `Stretch` - Stretch to fill cross-axis

**Size Constraints:**
- `Fit` - Use child's natural size
- `Expand` - Fill available space (distributes evenly among all Expand children)
- `Fixed(u32)` - Use specific size in pixels

### Scrollable Container

Handle content larger than the viewport:

```rust
use crate::ui::{ScrollableContainer, ScrollDirection};

let mut scrollable = ScrollableContainer::new(
    Rectangle::new(Point::new(10, 50), Size::new(300, 180)),  // viewport
    Size::new(300, 500),  // content size
    ScrollDirection::Vertical,
)
.with_style(Style::new().with_border(Rgb565::CSS_GRAY, 1));

// Scroll programmatically
scrollable.scroll_by(Point::new(0, -20)); // scroll up by 20 pixels
scrollable.scroll_to(Point::new(0, 100)); // scroll to position

// Handle touch for drag scrolling
scrollable.handle_touch(event);

// Transform viewport coordinates to content coordinates
if let Some(content_point) = scrollable.viewport_to_content(touch_point) {
    // This point is in content space
}
```

**Scroll Direction:**
- `Vertical` - Vertical scrolling only
- `Horizontal` - Horizontal scrolling only
- `Both` - Scroll in both directions

## Page System

### Creating a Page

Pages implement the `Page` trait:

```rust
use crate::pages::page_manager::Page;
use crate::ui::{Drawable, PageId, PageEvent, Action, TouchEvent};

pub struct MyPage {
    bounds: Rectangle,
    dirty: bool,
    // ... your components
}

impl Page for MyPage {
    fn id(&self) -> PageId {
        PageId::Custom(1)
    }

    fn title(&self) -> &str {
        "My Page"
    }

    fn on_activate(&mut self) {
        // Called when page becomes active
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        // Handle touch events
        // Return Some(Action::...) to trigger navigation, etc.
        None
    }

    fn update(&mut self) {
        // Called every frame
    }

    fn on_event(&mut self, event: &PageEvent) -> bool {
        // Handle page events (sensor updates, etc.)
        // Return true if page needs redraw
        match event {
            PageEvent::SensorUpdate(data) => {
                // Update your UI with sensor data
                true // needs redraw
            }
            _ => false
        }
    }
}

impl Drawable for MyPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Draw your page
        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty // || check child components
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        // mark child components clean
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
```

### Page Manager

The page manager handles navigation and event dispatching:

```rust
use crate::pages::{PageManager, PageWrapper, HomePage, SettingsPage};
use crate::ui::{PageId, PageEvent, SensorData};

// Create page manager
let mut page_manager = PageManager::new(
    PageId::Home,
    Rectangle::new(Point::zero(), Size::new(320, 240)),
);

// Register pages
page_manager.register_page(PageWrapper::Home(HomePage::new(bounds)));
page_manager.register_page(PageWrapper::Settings(SettingsPage::new(bounds)));

// Navigate
page_manager.navigate_to(PageId::Settings);
page_manager.go_back();

// Handle touch
if let Some(action) = page_manager.handle_touch(touch_event) {
    match action {
        Action::NavigateToPage(page_id) => page_manager.navigate_to(page_id),
        Action::GoBack => { page_manager.go_back(); }
        _ => {}
    }
}

// Dispatch events to active page
let sensor_data = SensorData {
    temperature: Some(23.5),
    humidity: Some(45.0),
    pressure: None,
    timestamp: 12345,
};
let needs_redraw = page_manager.dispatch_event(&PageEvent::SensorUpdate(sensor_data));

// Draw (only if dirty)
if page_manager.is_dirty() {
    page_manager.draw(&mut display)?;
}
```

### Event System

Pages can subscribe to events by implementing `on_event`:

```rust
fn on_event(&mut self, event: &PageEvent) -> bool {
    match event {
        PageEvent::SensorUpdate(data) => {
            // Update sensor displays
            if let Some(temp) = data.temperature {
                self.temp_display.set_text(&format!("{:.1}°C", temp));
            }
            true // request redraw
        }
        PageEvent::StorageEvent(StorageEvent::RawSample { sensor, value, timestamp }) => {
            // Log raw sample
            self.add_log_entry(&format!("[{}] {}: {}", timestamp, sensor, value));
            true
        }
        PageEvent::StorageEvent(StorageEvent::Rollup { interval, count, .. }) => {
            // Log rollup
            self.add_log_entry(&format!("Rollup {}: {} samples", interval, count));
            true
        }
        PageEvent::SystemEvent(_) => false,
    }
}
```

## Dirty Region Tracking

The UI system uses dirty regions to optimize rendering:

1. **Mark dirty when state changes:**
   ```rust
   fn set_value(&mut self, value: f32) {
       self.value = value;
       self.dirty = true; // Mark for redraw
   }
   ```

2. **Check dirty state before drawing:**
   ```rust
   if page_manager.is_dirty() {
       page_manager.draw(&mut display)?;
   }
   ```

3. **Clean after drawing:**
   ```rust
   component.draw(&mut display)?;
   component.mark_clean(); // Mark as drawn
   ```

## Best Practices

### Performance

1. **Use dirty tracking** - Only redraw when state changes
2. **Batch updates** - Update multiple components, then draw once
3. **Limit allocations** - Use `heapless` collections with fixed capacity
4. **Cache calculated layouts** - Containers cache child bounds

### Memory Management

1. **Choose appropriate capacities** - `Container::<N>` where N is max children
2. **Use static allocation** - Avoid heap allocations where possible
3. **Limit text lengths** - Use `HeaplessString<N>` with reasonable sizes

### Touch Handling

1. **Propagate events** - Return `NotHandled` when appropriate
2. **Handle drag properly** - Track state for press/drag/release sequence
3. **Check bounds first** - Use `contains_point` before processing

### Styling

1. **Use themes** - Create a consistent theme and reuse it
2. **Define constants** - Use const values for common sizes and colors
3. **Variants for components** - Use button variants instead of custom styles

## Example: Complete Page

```rust
use crate::pages::page_manager::Page;
use crate::ui::*;
use embedded_graphics::prelude::*;
use heapless::Vec;

pub struct DashboardPage {
    bounds: Rectangle,
    title: TextComponent,
    temp_display: TextComponent,
    buttons: Vec<Button, 3>,
    dirty: bool,
}

impl DashboardPage {
    pub fn new(bounds: Rectangle) -> Self {
        let palette = ColorPalette::default();

        let title = TextComponent::new(
            Rectangle::new(Point::new(20, 20), Size::new(280, 30)),
            "Dashboard",
            TextSize::Large,
        );

        let temp_display = TextComponent::new(
            Rectangle::new(Point::new(20, 60), Size::new(280, 20)),
            "Temperature: --",
            TextSize::Medium,
        );

        let mut buttons = Vec::new();
        buttons.push(Button::new(
            Rectangle::new(Point::new(20, 100), Size::new(130, 50)),
            "Settings",
            Action::NavigateToPage(PageId::Settings),
        ).with_variant(ButtonVariant::Primary)).ok();

        Self {
            bounds,
            title,
            temp_display,
            buttons,
            dirty: true,
        }
    }
}

impl Page for DashboardPage {
    fn id(&self) -> PageId {
        PageId::Home
    }

    fn title(&self) -> &str {
        "Dashboard"
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        for button in &mut self.buttons {
            match button.handle_touch(event) {
                TouchResult::Action(action) => return Some(action),
                TouchResult::Handled => return None,
                _ => continue,
            }
        }
        None
    }

    fn update(&mut self) {}

    fn on_event(&mut self, event: &PageEvent) -> bool {
        if let PageEvent::SensorUpdate(data) = event {
            if let Some(temp) = data.temperature {
                let mut text = heapless::String::<32>::new();
                use core::fmt::Write;
                write!(&mut text, "Temperature: {:.1}°C", temp).ok();
                self.temp_display.set_text(&text);
                return true;
            }
        }
        false
    }
}

impl Drawable for DashboardPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Clear background
        self.bounds.into_styled(
            PrimitiveStyle::with_fill(Rgb565::BLACK)
        ).draw(display)?;

        // Draw components
        self.title.draw(display)?;
        self.temp_display.draw(display)?;
        for button in &self.buttons {
            button.draw(display)?;
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty ||
        self.title.is_dirty() ||
        self.temp_display.is_dirty() ||
        self.buttons.iter().any(|b| b.is_dirty())
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.title.mark_clean();
        self.temp_display.mark_clean();
        for button in &mut self.buttons {
            button.mark_clean();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
```

## Troubleshooting

### Flickering Display

- Ensure dirty tracking is working correctly
- Only redraw when `is_dirty()` returns true
- Use partial updates when possible

### Touch Not Working

- Check `contains_point` implementation
- Verify bounds are correct
- Ensure event propagation with `TouchResult`

### Layout Issues

- Verify container bounds
- Check size constraints (Fit/Expand/Fixed)
- Ensure spacing is accounted for

### Memory Issues

- Reduce `heapless` collection capacities
- Limit text buffer sizes
- Profile with `#[global_allocator]` stats

## Future Enhancements

Potential additions to the UI system:

- **Advanced text rendering** - RTL support, custom fonts
- **Animation system** - Smooth transitions and effects
- **Image support** - Display bitmaps and icons
- **Virtual displays** - Off-screen rendering
- **Accessibility** - Screen reader support, high contrast modes
- **Input widgets** - Text input, sliders, checkboxes
- **Charts & graphs** - Data visualization components

---

For more examples, see the `pages/` module implementations.
