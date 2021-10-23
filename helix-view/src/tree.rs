use crate::{graphics::Rect, View, ViewId};
use slotmap::HopSlotMap;

// the dimensions are recomputed on window resize/tree change.
//
#[derive(Debug)]
pub struct Tree {
    root: ViewId,
    // (container, index inside the container)
    pub focus: ViewId,
    // fullscreen: bool,
    area: Rect,

    nodes: HopSlotMap<ViewId, Node>,

    // used for traversals
    stack: Vec<(ViewId, Rect)>,
}

#[derive(Debug)]
pub struct Node {
    parent: ViewId,
    content: Content,
}

#[derive(Debug)]
pub enum Content {
    View(Box<View>),
    Container(Box<Container>),
}

impl Node {
    pub fn container(layout: Layout) -> Self {
        Self {
            parent: ViewId::default(),
            content: Content::Container(Box::new(Container::new(layout))),
        }
    }

    pub fn view(view: View) -> Self {
        Self {
            parent: ViewId::default(),
            content: Content::View(Box::new(view)),
        }
    }
}

// TODO: screen coord to container + container coordinate helpers

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Horizontal,
    Vertical,
    // could explore stacked/tabbed
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug)]
pub struct Container {
    layout: Layout,
    children: Vec<ViewId>,
    area: Rect,
}

impl Container {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            children: Vec::new(),
            area: Rect::default(),
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new(Layout::Vertical)
    }
}

impl Tree {
    pub fn new(area: Rect) -> Self {
        let root = Node::container(Layout::Vertical);

        let mut nodes = HopSlotMap::with_key();
        let root = nodes.insert(root);

        // root is it's own parent
        nodes[root].parent = root;

        Self {
            root,
            focus: root,
            // fullscreen: false,
            area,
            nodes,
            stack: Vec::new(),
        }
    }

    pub fn insert(&mut self, view: View) -> ViewId {
        let focus = self.focus;
        let parent = self.nodes[focus].parent;
        let mut node = Node::view(view);
        node.parent = parent;
        let node = self.nodes.insert(node);
        self.get_mut(node).id = node;

        let container = match &mut self.nodes[parent] {
            Node {
                content: Content::Container(container),
                ..
            } => container,
            _ => unreachable!(),
        };

        // insert node after the current item if there is children already
        let pos = if container.children.is_empty() {
            0
        } else {
            let pos = container
                .children
                .iter()
                .position(|&child| child == focus)
                .unwrap();
            pos + 1
        };

        container.children.insert(pos, node);
        // focus the new node
        self.focus = node;

        // recalculate all the sizes
        self.recalculate();

        node
    }

    pub fn split(&mut self, view: View, layout: Layout) -> ViewId {
        let focus = self.focus;
        let parent = self.nodes[focus].parent;

        let node = Node::view(view);
        let node = self.nodes.insert(node);
        self.get_mut(node).id = node;

        let container = match &mut self.nodes[parent] {
            Node {
                content: Content::Container(container),
                ..
            } => container,
            _ => unreachable!(),
        };
        if container.layout == layout {
            // insert node after the current item if there is children already
            let pos = if container.children.is_empty() {
                0
            } else {
                let pos = container
                    .children
                    .iter()
                    .position(|&child| child == focus)
                    .unwrap();
                pos + 1
            };
            container.children.insert(pos, node);
            self.nodes[node].parent = parent;
        } else {
            let mut split = Node::container(layout);
            split.parent = parent;
            let split = self.nodes.insert(split);

            let container = match &mut self.nodes[split] {
                Node {
                    content: Content::Container(container),
                    ..
                } => container,
                _ => unreachable!(),
            };
            container.children.push(focus);
            container.children.push(node);
            self.nodes[focus].parent = split;
            self.nodes[node].parent = split;

            let container = match &mut self.nodes[parent] {
                Node {
                    content: Content::Container(container),
                    ..
                } => container,
                _ => unreachable!(),
            };

            let pos = container
                .children
                .iter()
                .position(|&child| child == focus)
                .unwrap();

            // replace focus on parent with split
            container.children[pos] = split;
        }

        // focus the new node
        self.focus = node;

        // recalculate all the sizes
        self.recalculate();

        node
    }

    pub fn remove(&mut self, index: ViewId) {
        let mut stack = Vec::new();

        if self.focus == index {
            // focus on something else
            self.focus_next();
        }

        stack.push(index);

        while let Some(index) = stack.pop() {
            let parent_id = self.nodes[index].parent;
            if let Node {
                content: Content::Container(container),
                ..
            } = &mut self.nodes[parent_id]
            {
                if let Some(pos) = container.children.iter().position(|&child| child == index) {
                    container.children.remove(pos);

                    // TODO: if container now only has one child, remove it and place child in parent
                    if container.children.is_empty() && parent_id != self.root {
                        // if container now empty, remove it
                        stack.push(parent_id);
                    }
                }
            }
            self.nodes.remove(index);
        }

        self.recalculate()
    }

    pub fn views(&self) -> impl Iterator<Item = (&View, bool)> {
        let focus = self.focus;
        self.nodes.iter().filter_map(move |(key, node)| match node {
            Node {
                content: Content::View(view),
                ..
            } => Some((view.as_ref(), focus == key)),
            _ => None,
        })
    }

    pub fn views_mut(&mut self) -> impl Iterator<Item = (&mut View, bool)> {
        let focus = self.focus;
        self.nodes
            .iter_mut()
            .filter_map(move |(key, node)| match node {
                Node {
                    content: Content::View(view),
                    ..
                } => Some((view.as_mut(), focus == key)),
                _ => None,
            })
    }

    pub fn get(&self, index: ViewId) -> &View {
        match &self.nodes[index] {
            Node {
                content: Content::View(view),
                ..
            } => view,
            _ => unreachable!(),
        }
    }

    pub fn get_mut(&mut self, index: ViewId) -> &mut View {
        match &mut self.nodes[index] {
            Node {
                content: Content::View(view),
                ..
            } => view,
            _ => unreachable!(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.nodes[self.root] {
            Node {
                content: Content::Container(container),
                ..
            } => container.children.is_empty(),
            _ => unreachable!(),
        }
    }

    pub fn resize(&mut self, area: Rect) -> bool {
        if self.area != area {
            self.area = area;
            self.recalculate();
            return true;
        }
        false
    }

    pub fn recalculate(&mut self) {
        if self.is_empty() {
            return;
        }

        self.stack.push((self.root, self.area));

        // take the area
        // fetch the node
        // a) node is view, give it whole area
        // b) node is container, calculate areas for each child and push them on the stack

        while let Some((key, area)) = self.stack.pop() {
            let node = &mut self.nodes[key];

            match &mut node.content {
                Content::View(view) => {
                    // debug!!("setting view area {:?}", area);
                    view.area = area;
                } // TODO: call f()
                Content::Container(container) => {
                    // debug!!("setting container area {:?}", area);
                    container.area = area;

                    match container.layout {
                        Layout::Horizontal => {
                            let len = container.children.len();

                            let height = area.height / len as u16;

                            let mut child_y = area.y;

                            for (i, child) in container.children.iter().enumerate() {
                                let mut area = Rect::new(
                                    container.area.x,
                                    child_y,
                                    container.area.width,
                                    height,
                                );
                                child_y += height;

                                // last child takes the remaining width because we can get uneven
                                // space from rounding
                                if i == len - 1 {
                                    area.height = container.area.y + container.area.height - area.y;
                                }

                                self.stack.push((*child, area));
                            }
                        }
                        Layout::Vertical => {
                            let len = container.children.len();

                            let width = area.width / len as u16;

                            let inner_gap = 1u16;
                            // let total_gap = inner_gap * (len as u16 - 1);

                            let mut child_x = area.x;

                            for (i, child) in container.children.iter().enumerate() {
                                let mut area = Rect::new(
                                    child_x,
                                    container.area.y,
                                    width,
                                    container.area.height,
                                );
                                child_x += width + inner_gap;

                                // last child takes the remaining width because we can get uneven
                                // space from rounding
                                if i == len - 1 {
                                    area.width = container.area.x + container.area.width - area.x;
                                }

                                self.stack.push((*child, area));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn traverse(&self) -> Traverse {
        Traverse::new(self)
    }

    // Finds the split in the given direction if it exists
    pub fn find_split_in_direction(&self, id: ViewId, direction: Direction) -> Option<ViewId> {
        let parent = self.nodes[id].parent;
        // Base case, we found the root of the tree
        if parent == id {
            return None;
        }
        // Parent must always be a container
        let parent_container = match &self.nodes[parent].content {
            Content::Container(container) => container,
            Content::View(_) => unreachable!(),
        };

        match (direction, parent_container.layout) {
            (Direction::Up, Layout::Vertical)
            | (Direction::Left, Layout::Horizontal)
            | (Direction::Right, Layout::Horizontal)
            | (Direction::Down, Layout::Vertical) => {
                // The desired direction of movement is not possible within
                // the parent container so the search must continue closer to
                // the root of the split tree.
                self.find_split_in_direction(parent, direction)
            }
            (Direction::Up, Layout::Horizontal)
            | (Direction::Down, Layout::Horizontal)
            | (Direction::Left, Layout::Vertical)
            | (Direction::Right, Layout::Vertical) => {
                // It's possible to move in the desired direction within
                // the parent container so an attempt is made to find the
                // correct child.
                match self.find_child(id, &parent_container.children, direction) {
                    // Child is found, search is ended
                    Some(id) => Some(id),
                    // A child is not found. This could be because of either two scenarios
                    // 1. Its not possible to move in the desired direction, and search should end
                    // 2. A layout like the following with focus at X and desired direction Right
                    // | _ | x |   |
                    // | _ _ _ |   |
                    // | _ _ _ |   |
                    // The container containing X ends at X so no rightward movement is possible
                    // however there still exists another view/container to the right that hasn't
                    // been explored. Thus another search is done here in the parent container
                    // before concluding it's not possible to move in the desired direction.
                    None => self.find_split_in_direction(parent, direction),
                }
            }
        }
    }

    fn find_child(&self, id: ViewId, children: &[ViewId], direction: Direction) -> Option<ViewId> {
        let mut child_id = match direction {
            // index wise in the child list the Up and Left represents a -1
            // thus reversed iterator.
            Direction::Up | Direction::Left => children
                .iter()
                .rev()
                .skip_while(|i| **i != id)
                .copied()
                .nth(1)?,
            // Down and Right => +1 index wise in the child list
            Direction::Down | Direction::Right => {
                children.iter().skip_while(|i| **i != id).copied().nth(1)?
            }
        };
        let (current_x, current_y) = match &self.nodes[self.focus].content {
            Content::View(current_view) => (current_view.area.left(), current_view.area.top()),
            Content::Container(_) => unreachable!(),
        };

        // If the child is a container the search finds the closest container child
        // visually based on screen location.
        while let Content::Container(container) = &self.nodes[child_id].content {
            match (direction, container.layout) {
                (_, Layout::Vertical) => {
                    // find closest split based on x because y is irrelevant
                    // in a vertical container (and already correct based on previous search)
                    child_id = *container.children.iter().min_by_key(|id| {
                        let x = match &self.nodes[**id].content {
                            Content::View(view) => view.inner_area().left(),
                            Content::Container(container) => container.area.left(),
                        };
                        (current_x as i16 - x as i16).abs()
                    })?;
                }
                (_, Layout::Horizontal) => {
                    // find closest split based on y because x is irrelevant
                    // in a horizontal container (and already correct based on previous search)
                    child_id = *container.children.iter().min_by_key(|id| {
                        let y = match &self.nodes[**id].content {
                            Content::View(view) => view.inner_area().top(),
                            Content::Container(container) => container.area.top(),
                        };
                        (current_y as i16 - y as i16).abs()
                    })?;
                }
            }
        }
        Some(child_id)
    }

    pub fn focus_direction(&mut self, direction: Direction) {
        if let Some(id) = self.find_split_in_direction(self.focus, direction) {
            self.focus = id;
        }
    }

    pub fn focus_next(&mut self) {
        // This function is very dumb, but that's because we don't store any parent links.
        // (we'd be able to go parent.next_sibling() recursively until we find something)
        // For now that's okay though, since it's unlikely you'll be able to open a large enough
        // number of splits to notice.

        // current = focus
        // let found = loop do {
        //   node = focus.parent;
        //   let found = node.next_sibling_of(current)
        //   if some {
        //       break found;
        //   }
        //   // else
        //   if node == root {
        //       return first child of root;
        //   };
        //   current = parent;
        //  }
        // }
        //
        // use found next sibling
        // loop do {
        //   if found = view -> focus = found, return
        //   if found = container -> found = first child
        // }

        let mut views = self
            .traverse()
            .skip_while(|&(id, _view)| id != self.focus)
            .skip(1); // Skip focused value
        if let Some((id, _)) = views.next() {
            self.focus = id;
        } else {
            // extremely crude, take the first item again
            let (key, _) = self.traverse().next().unwrap();
            self.focus = key;
        }
    }

    pub fn area(&self) -> Rect {
        self.area
    }
}

#[derive(Debug)]
pub struct Traverse<'a> {
    tree: &'a Tree,
    stack: Vec<ViewId>, // TODO: reuse the one we use on update
}

impl<'a> Traverse<'a> {
    fn new(tree: &'a Tree) -> Self {
        Self {
            tree,
            stack: vec![tree.root],
        }
    }
}

impl<'a> Iterator for Traverse<'a> {
    type Item = (ViewId, &'a View);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let key = self.stack.pop()?;

            let node = &self.tree.nodes[key];

            match &node.content {
                Content::View(view) => return Some((key, view)),
                Content::Container(container) => {
                    self.stack.extend(container.children.iter().rev());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::DocumentId;

    #[test]
    fn find_split_in_direction() {
        let mut tree = Tree::new(Rect {
            x: 0,
            y: 0,
            width: 180,
            height: 80,
        });
        let mut view = View::new(DocumentId::default());
        view.area = Rect::new(0, 0, 180, 80);
        tree.insert(view);

        let l0 = tree.focus;
        let view = View::new(DocumentId::default());
        tree.split(view, Layout::Vertical);
        let r0 = tree.focus;

        tree.focus = l0;
        let view = View::new(DocumentId::default());
        tree.split(view, Layout::Horizontal);
        let l1 = tree.focus;

        tree.focus = l0;
        let view = View::new(DocumentId::default());
        tree.split(view, Layout::Vertical);
        let l2 = tree.focus;

        // Tree in test
        // | L0  | L2 |    |
        // |    L1    | R0 |
        tree.focus = l2;
        assert_eq!(Some(l0), tree.find_split_in_direction(l2, Direction::Left));
        assert_eq!(Some(l1), tree.find_split_in_direction(l2, Direction::Down));
        assert_eq!(Some(r0), tree.find_split_in_direction(l2, Direction::Right));
        assert_eq!(None, tree.find_split_in_direction(l2, Direction::Up));

        tree.focus = l1;
        assert_eq!(None, tree.find_split_in_direction(l1, Direction::Left));
        assert_eq!(None, tree.find_split_in_direction(l1, Direction::Down));
        assert_eq!(Some(r0), tree.find_split_in_direction(l1, Direction::Right));
        assert_eq!(Some(l0), tree.find_split_in_direction(l1, Direction::Up));

        tree.focus = l0;
        assert_eq!(None, tree.find_split_in_direction(l0, Direction::Left));
        assert_eq!(Some(l1), tree.find_split_in_direction(l0, Direction::Down));
        assert_eq!(Some(l2), tree.find_split_in_direction(l0, Direction::Right));
        assert_eq!(None, tree.find_split_in_direction(l0, Direction::Up));

        tree.focus = r0;
        assert_eq!(Some(l2), tree.find_split_in_direction(r0, Direction::Left));
        assert_eq!(None, tree.find_split_in_direction(r0, Direction::Down));
        assert_eq!(None, tree.find_split_in_direction(r0, Direction::Right));
        assert_eq!(None, tree.find_split_in_direction(r0, Direction::Up));
    }
}
