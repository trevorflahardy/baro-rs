# Baro UI System

A SwiftUI-inspired, declarative UI framework for embedded displays.

## Design Philosophy

The Baro UI system is designed for long-running embedded firmware with these principles:

- **Composable**: Nest containers freely to build complex layouts
- **Automatic sizing**: Elements size themselves based on content when possible
- **Type-safe**: No trait objects - compile-time guarantees
- **Resource-conscious**: Core layout uses bounded, fixed-size buffers; some advanced widgets (e.g., graphs) use controlled heap allocations
- **Clean code**: Readable, maintainable UI code

## Core Concepts

### Elements

All UI components are unified under the `Element` enum:

```rust
pub enum Element {
    Text(Box<TextComponent>),
    MultiLineText(Box<MultiLineText>),
    Button(Box<Button>),
    Container(Box<Container<MAX_CONTAINER_CHILDREN>>),  // Nestable!
    Spacer { bounds, dirty },
}
```

This enables containers to hold heterogeneous children without trait objects.

### Containers

Containers are flex-like layout primitives that:
- Own their children as `Element`s
- Calculate and set child bounds automatically
- Support vertical and horizontal directions
- Handle alignment, spacing, and flex-grow sizing

**Key insight**: You no longer need to know exact sizes upfront. Create elements with automatic sizing and let the container figure out the layout.

### Size Constraints

When adding children to containers, specify how they should size:

- `SizeConstraint::Fit` — Use the element's intrinsic/preferred size
- `SizeConstraint::Fixed(px)` — Force a specific main-axis size
- `SizeConstraint::Grow(weight)` — Flex-grow: take proportional share of remaining space

## Quick Start

### Simple Vertical Stack

```rust
use crate::ui::*;

// Create a root container
let mut container = Container::<8>::new(bounds, Direction::Vertical)
    .with_alignment(Alignment::Stretch)
    .with_gap(10);

// Add children with automatic sizing
container.add_child(
    Element::text_auto("Title", TextSize::Large),
    SizeConstraint::Fit
).ok();

container.add_child(
    Element::button_auto("Click Me", Action::Custom(1)),
    SizeConstraint::Fixed(50)
).ok();
```

### Nested Containers (The Key Feature!)

```rust
// Header container
let mut header = Container::<MAX_CONTAINER_CHILDREN>::new(
    Rectangle::new(Point::zero(), Size::new(320, 60)),
    Direction::Vertical
)
.with_alignment(Alignment::Center);

header.add_child(
    Element::text_auto("App Title", TextSize::Large),
    SizeConstraint::Fit
).ok();

// Body container
let mut body = Container::<MAX_CONTAINER_CHILDREN>::new(
    Rectangle::zero(),
    Direction::Vertical
)
.with_alignment(Alignment::Stretch)
.with_gap(10);

body.add_child(
    Element::button_auto("Action 1", action1),
    SizeConstraint::Fixed(50)
).ok();

// Root combines them
let mut root = Container::<2>::new(bounds, Direction::Vertical);
root.add_child(Element::container(header), SizeConstraint::Fixed(60)).ok();
root.add_child(Element::container(body), SizeConstraint::Grow(1)).ok();
```

## Automatic Sizing

Elements now support automatic sizing - no need to calculate pixel dimensions:

```rust
// Before: Required explicit bounds
let text = Element::text(
    Rectangle::new(Point::zero(), Size::new(200, 20)),
    "Hello",
    TextSize::Medium
);

// After: Automatic sizing
let text = Element::text_auto("Hello", TextSize::Medium);
```

Same for buttons:

```rust
// Auto-sized button (minimum 100x44 for touchability)
let btn = Element::button_auto("Settings", Action::NavigateToPage(PageId::Settings));
```

## Layout Flow

1. **Container creation**: Specify direction and initial bounds
2. **Add children**: Use `add_child()` with size constraints
3. **Layout runs automatically**: Container calculates child positions and sizes
4. **Render**: Call `draw()` on the root container

The container system handles:
- Padding/spacing
- Alignment (start/center/end/stretch)
- Main-axis distribution (start/center/end/space-between/space-around/space-evenly)
- Flex-grow proportional sizing
- Automatic bounds propagation to children

## Best Practices

### Use MAX_CONTAINER_CHILDREN for Nested Containers

When creating containers that will be nested as Elements, use `MAX_CONTAINER_CHILDREN`:

```rust
let header = Container::<MAX_CONTAINER_CHILDREN>::new(bounds, Direction::Vertical);
```

This ensures type compatibility with `Element::container()`.

### Prefer Automatic Sizing

Use `_auto()` constructors when possible:

```rust
Element::text_auto("Label", TextSize::Small)
Element::button_auto("Action", action)
```

### Keep Container Sizes Reasonable

`MAX_CONTAINER_CHILDREN` is currently 16. For deeply nested UIs, consider flattening or using smaller fixed-size containers for leaf nodes.

### Layout is Eager

Layout recalculates when:
- You add a child
- Container bounds change
- You call `set_bounds()` on the container

This is intentional - no "pending layout" state to worry about.

## Comparison to Old System

### Before: Rigid and Manual

```rust
let hint = Rectangle::new(Point::zero(), Size::new(320, 1));
let title = TextComponent::new(hint, "Title", TextSize::Large);
container.add_child(Element::Text(Box::new(title)), SizeConstraint::Fixed(30)).ok();
```

- Required "hint" bounds even though layout overwrites them
- Couldn't nest containers
- Size calculations were manual and error-prone

### After: Flexible and Declarative

```rust
let title = TextComponent::auto("Title", TextSize::Large);
header.add_child(Element::Text(Box::new(title)), SizeConstraint::Fit).ok();

// Nest the header in the root
root.add_child(Element::container(header), SizeConstraint::Fixed(60)).ok();
```

- Automatic sizing where sensible
- Containers are elements - nest freely
- Layout is automatic
- Code reads like the UI structure

## Advanced: Horizontal Layouts

```rust
let mut row = Container::<4>::hstack()  // Convenience constructor
    .with_main_axis_alignment(MainAxisAlignment::SpaceEvenly);

row.add_child(Element::button_auto("A", action_a), SizeConstraint::Fit).ok();
row.add_child(Element::button_auto("B", action_b), SizeConstraint::Fit).ok();
row.add_child(Element::button_auto("C", action_c), SizeConstraint::Fit).ok();
```

## See Also

- `EXAMPLES.md` — More complete code examples
- `src/ui/layouts/container.rs` — Full Container API documentation
- `src/ui/elements.rs` — Element enum and constructors
- `src/pages/home.rs` — Real-world usage in the home page

