use crate::{
  core::Guard,
  layout::{
    Alignment, StackAlignment,
    layout_kind::{FrameConstraints, ScrollDirection, ScrollState},
    text_style::TextStyle,
  },
  node::{
    color::Color,
    dimension::Dimension,
    node::{EventHandlers, Node},
    padding::Padding,
  },
};

pub fn row() -> Node {
  Node::row(0.0, Alignment::Start, vec![])
}

pub fn column() -> Node {
  Node::column(0.0, Alignment::Start, vec![])
}

pub fn stack() -> Node {
  Node::stack(StackAlignment::TopStart, vec![])
}

pub fn text(content: &str) -> Node {
  Node::text(content)
}

pub fn styled_text(content: &str, style: TextStyle) -> Node {
  Node::text_styled(content, style)
}

pub fn rect(width: f32, height: f32) -> Node {
  Node::new().frame(FrameConstraints {
    width: Some(width),
    height: Some(height),
    ..Default::default()
  })
}

pub fn spacer() -> Node {
  Node::new()
}

fn make_scroll(child: Node, direction: ScrollDirection) -> Node {
  Node {
    kind: crate::layout::layout_kind::LayoutKind::ScrollModifier {
      state: ScrollState::new(),
      direction,
    },
    color: Guard::new(None),
    border_radius: Guard::new(None),
    border: Guard::new(None),
    scrollbar_style: Guard::new(None),
    node_ref: None,
    interaction: None,
    layout_cache: Default::default(),
    children: vec![child],
    events: EventHandlers::default(),
  }
}

pub fn scroll_vertical(child: Node) -> Node {
  make_scroll(child, ScrollDirection::Vertical)
}

pub fn scroll_horizontal(child: Node) -> Node {
  make_scroll(child, ScrollDirection::Horizontal)
}

pub fn scroll_both(child: Node) -> Node {
  make_scroll(child, ScrollDirection::Both)
}

impl Node {
  pub fn child(mut self, child: Node) -> Self {
    self.children.push(child);
    self
  }

  pub fn with_children(mut self, children: impl IntoIterator<Item = Node>) -> Self {
    self.children.extend(children);
    self
  }

  pub fn spacing(mut self, spacing: f32) -> Self {
    match &mut self.kind {
      crate::layout::layout_kind::LayoutKind::Row { spacing: s, .. } => *s = spacing,
      crate::layout::layout_kind::LayoutKind::Column { spacing: s, .. } => *s = spacing,
      _ => {}
    }
    self
  }

  pub fn align_items(mut self, align: Alignment) -> Self {
    match &mut self.kind {
      crate::layout::layout_kind::LayoutKind::Row { align: a, .. } => *a = align,
      crate::layout::layout_kind::LayoutKind::Column { align: a, .. } => *a = align,
      _ => {}
    }
    self
  }

  pub fn stack_align(mut self, align: StackAlignment) -> Self {
    if let crate::layout::layout_kind::LayoutKind::Stack { align: a } = &mut self.kind {
      *a = align;
    }
    self
  }

  pub fn fill(self, hex: &str) -> Self {
    self.background(Color::from_hex(hex))
  }

  pub fn size(self, width: f32, height: f32) -> Self {
    self.frame(FrameConstraints {
      width: Some(width),
      height: Some(height),
      ..Default::default()
    })
  }

  pub fn width(self, width: f32) -> Self {
    self.frame(FrameConstraints {
      width: Some(width),
      ..Default::default()
    })
  }

  pub fn height(self, height: f32) -> Self {
    self.frame(FrameConstraints {
      height: Some(height),
      ..Default::default()
    })
  }

  pub fn pad(self, all: f32) -> Self {
    self.padding(Padding::all(Dimension::Px(all)))
  }

  pub fn pad_xy(self, horizontal: f32, vertical: f32) -> Self {
    self.padding(Padding::symmetric(Dimension::Px(horizontal), Dimension::Px(vertical)))
  }

  pub fn pad_left(self, val: f32) -> Self {
    self.padding(Padding::new().left(Dimension::Px(val)))
  }

  pub fn pad_right(self, val: f32) -> Self {
    self.padding(Padding::new().right(Dimension::Px(val)))
  }

  pub fn pad_top(self, val: f32) -> Self {
    self.padding(Padding::new().top(Dimension::Px(val)))
  }

  pub fn pad_bottom(self, val: f32) -> Self {
    self.padding(Padding::new().bottom(Dimension::Px(val)))
  }
}

macro_rules! child_method {
  ($ty:ident, $child:ident) => {
    ::paste::paste! {
      impl $ty {
        pub fn [<child_ $child>](self) -> Self {
          self.child($child())
        }
      }
    }
  };
}

child_method!(Node, column);
child_method!(Node, row);
child_method!(Node, spacer);
child_method!(Node, stack);
