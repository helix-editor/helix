use crate::{editor::ViewMap, graphics::Rect, View, ViewId};

// the dimensions are recomputed on window resize/tree change.
//
#[derive(Debug)]
pub struct Tree {
    pub root: ViewId,
    // (container, index inside the container)
    pub focus: ViewId,
    // fullscreen: bool,
    area: Rect,

    // used for traversals
    stack: Vec<(ViewId, Rect)>,
}

#[derive(Debug)]
pub struct Node {
    pub(crate) parent: ViewId,
    pub(crate) content: Content,
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
    pub fn new(area: Rect, nodes: &mut ViewMap) -> Self {
        let root = Node::container(Layout::Vertical);

        let root = nodes.map.insert(root);

        // root is it's own parent
        nodes.map[root].parent = root;

        Self {
            root,
            focus: root,
            // fullscreen: false,
            area,
            stack: Vec::new(),
        }
    }

    pub fn insert(&mut self, views: &mut ViewMap, view: View) -> ViewId {
        let focus = self.focus;
        let parent = views.map[focus].parent;
        let mut node = Node::view(view);
        node.parent = parent;
        let node = views.map.insert(node);
        views.get_mut(node).id = node;

        let container = match &mut views.map[parent] {
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
        self.recalculate(views);

        node
    }

    pub fn split(&mut self, views: &mut ViewMap, view: View, layout: Layout) -> ViewId {
        let focus = self.focus;
        let parent = views.map[focus].parent;

        let node = Node::view(view);
        let node = views.map.insert(node);
        views.get_mut(node).id = node;

        let container = match &mut views.map[parent] {
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
            views.map[node].parent = parent;
        } else {
            let mut split = Node::container(layout);
            split.parent = parent;
            let split = views.map.insert(split);

            let container = match &mut views.map[split] {
                Node {
                    content: Content::Container(container),
                    ..
                } => container,
                _ => unreachable!(),
            };
            container.children.push(focus);
            container.children.push(node);
            views.map[focus].parent = split;
            views.map[node].parent = split;

            let container = match &mut views.map[parent] {
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
        self.recalculate(views);

        node
    }

    /// Get a mutable reference to a [Container] by index.
    /// # Panics
    /// Panics if `index` is not in views, or if the node's content is not a [Content::Container].
    fn container_mut<'a>(&'a mut self, views: &'a mut ViewMap, index: ViewId) -> &'a mut Container {
        match &mut views.map[index] {
            Node {
                content: Content::Container(container),
                ..
            } => container,
            _ => unreachable!(),
        }
    }

    fn remove_or_replace(
        &mut self,
        views: &mut ViewMap,
        child: ViewId,
        replacement: Option<ViewId>,
    ) {
        let parent = views.map[child].parent;

        views.map.remove(child);

        let container = self.container_mut(views, parent);
        let pos = container
            .children
            .iter()
            .position(|&item| item == child)
            .unwrap();

        if let Some(new) = replacement {
            container.children[pos] = new;
            views.map[new].parent = parent;
        } else {
            container.children.remove(pos);
        }
    }

    pub fn remove(&mut self, views: &mut ViewMap, index: ViewId) {
        if self.focus == index {
            // focus on something else
            self.focus = self.prev(views);
        }

        let parent = views.map[index].parent;
        let parent_is_root = parent == self.root;

        self.remove_or_replace(views, index, None);

        let parent_container = self.container_mut(views, parent);
        if parent_container.children.len() == 1 && !parent_is_root {
            // Lets merge the only child back to its grandparent so that Views
            // are equally spaced.
            let sibling = parent_container.children.pop().unwrap();
            self.remove_or_replace(views, parent, Some(sibling));
        }

        self.recalculate(views)
    }

    pub fn views<'a>(&'a self, views: &'a ViewMap) -> impl Iterator<Item = (&'a View, bool)> {
        let focus = self.focus;
        views.map.iter().filter_map(move |(key, node)| match node {
            Node {
                content: Content::View(view),
                ..
            } if views.view_root(view.id) == self.root => Some((view.as_ref(), focus == key)),
            _ => None,
        })
    }

    /// Check if tree contains a [Node] with a given index.
    pub fn contains(&self, views: &ViewMap, index: ViewId) -> bool {
        views.map.contains_key(index)
    }

    pub fn is_empty(&self, views: &ViewMap) -> bool {
        match &views.map[self.root] {
            Node {
                content: Content::Container(container),
                ..
            } => container.children.is_empty(),
            _ => unreachable!(),
        }
    }

    pub fn resize(&mut self, views: &mut ViewMap, area: Rect) -> bool {
        if self.area != area {
            self.area = area;
            self.recalculate(views);
            return true;
        }
        false
    }

    pub fn recalculate(&mut self, views: &mut ViewMap) {
        if self.is_empty(views) {
            // There are no more views, so the tree should focus itself again.
            self.focus = self.root;

            return;
        }

        self.stack.push((self.root, self.area));

        // take the area
        // fetch the node
        // a) node is view, give it whole area
        // b) node is container, calculate areas for each child and push them on the stack

        while let Some((key, area)) = self.stack.pop() {
            let node = &mut views.map[key];

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
                            let len_u16 = len as u16;

                            let inner_gap = 1u16;
                            let total_gap = inner_gap * len_u16.saturating_sub(2);

                            let used_area = area.width.saturating_sub(total_gap);
                            let width = used_area / len_u16;

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

    pub fn traverse<'a>(&'a self, views: &'a ViewMap) -> Traverse<'a> {
        Traverse::new(self, views)
    }

    // Finds the split in the given direction if it exists
    pub fn find_split_in_direction(
        &self,
        views: &ViewMap,
        id: ViewId,
        direction: Direction,
    ) -> Option<ViewId> {
        let parent = views.map[id].parent;
        // Base case, we found the root of the tree
        if parent == id {
            return None;
        }
        // Parent must always be a container
        let parent_container = match &views.map[parent].content {
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
                self.find_split_in_direction(views, parent, direction)
            }
            (Direction::Up, Layout::Horizontal)
            | (Direction::Down, Layout::Horizontal)
            | (Direction::Left, Layout::Vertical)
            | (Direction::Right, Layout::Vertical) => {
                // It's possible to move in the desired direction within
                // the parent container so an attempt is made to find the
                // correct child.
                match self.find_child(views, id, &parent_container.children, direction) {
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
                    None => self.find_split_in_direction(views, parent, direction),
                }
            }
        }
    }

    fn find_child(
        &self,
        views: &ViewMap,
        id: ViewId,
        children: &[ViewId],
        direction: Direction,
    ) -> Option<ViewId> {
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
        let (current_x, current_y) = match &views.map[self.focus].content {
            Content::View(current_view) => (current_view.area.left(), current_view.area.top()),
            Content::Container(_) => unreachable!(),
        };

        // If the child is a container the search finds the closest container child
        // visually based on screen location.
        while let Content::Container(container) = &views.map[child_id].content {
            match (direction, container.layout) {
                (_, Layout::Vertical) => {
                    // find closest split based on x because y is irrelevant
                    // in a vertical container (and already correct based on previous search)
                    child_id = *container.children.iter().min_by_key(|id| {
                        let x = match &views.map[**id].content {
                            Content::View(view) => view.area.left(),
                            Content::Container(container) => container.area.left(),
                        };
                        (current_x as i16 - x as i16).abs()
                    })?;
                }
                (_, Layout::Horizontal) => {
                    // find closest split based on y because x is irrelevant
                    // in a horizontal container (and already correct based on previous search)
                    child_id = *container.children.iter().min_by_key(|id| {
                        let y = match &views.map[**id].content {
                            Content::View(view) => view.area.top(),
                            Content::Container(container) => container.area.top(),
                        };
                        (current_y as i16 - y as i16).abs()
                    })?;
                }
            }
        }
        Some(child_id)
    }

    pub fn prev(&self, views: &ViewMap) -> ViewId {
        // This function is very dumb, but that's because we don't store any parent links.
        // (we'd be able to go parent.prev_sibling() recursively until we find something)
        // For now that's okay though, since it's unlikely you'll be able to open a large enough
        // number of splits to notice.

        let mut view_iter = self
            .traverse(views)
            .rev()
            .skip_while(|&(id, _view)| id != self.focus)
            .skip(1); // Skip focused value
        if let Some((id, _)) = view_iter.next() {
            id
        } else {
            // extremely crude, take the last item
            let (key, _) = self.traverse(views).next_back().unwrap();
            key
        }
    }

    pub fn next(&self, views: &ViewMap) -> ViewId {
        // This function is very dumb, but that's because we don't store any parent links.
        // (we'd be able to go parent.next_sibling() recursively until we find something)
        // For now that's okay though, since it's unlikely you'll be able to open a large enough
        // number of splits to notice.

        let mut view_iter = self
            .traverse(views)
            .skip_while(|&(id, _view)| id != self.focus)
            .skip(1); // Skip focused value
        if let Some((id, _)) = view_iter.next() {
            id
        } else {
            // extremely crude, take the first item again
            let (key, _) = self.traverse(views).next().unwrap();
            key
        }
    }

    pub fn transpose(&mut self, views: &mut ViewMap) {
        let focus = self.focus;
        let parent = views.map[focus].parent;
        if let Content::Container(container) = &mut views.map[parent].content {
            container.layout = match container.layout {
                Layout::Vertical => Layout::Horizontal,
                Layout::Horizontal => Layout::Vertical,
            };
            self.recalculate(views);
        }
    }

    pub fn swap_split_in_direction(
        &mut self,
        views: &mut ViewMap,
        direction: Direction,
    ) -> Option<()> {
        let focus = self.focus;
        let target = self.find_split_in_direction(views, focus, direction)?;
        let focus_parent = views.map[focus].parent;
        let target_parent = views.map[target].parent;

        if focus_parent == target_parent {
            let parent = focus_parent;
            let [parent, focus, target] = views.map.get_disjoint_mut([parent, focus, target])?;
            match (&mut parent.content, &mut focus.content, &mut target.content) {
                (
                    Content::Container(parent),
                    Content::View(focus_view),
                    Content::View(target_view),
                ) => {
                    let focus_pos = parent.children.iter().position(|id| focus_view.id == *id)?;
                    let target_pos = parent
                        .children
                        .iter()
                        .position(|id| target_view.id == *id)?;
                    // swap node positions so that traversal order is kept
                    parent.children[focus_pos] = target_view.id;
                    parent.children[target_pos] = focus_view.id;
                    // swap area so that views rendered at the correct location
                    std::mem::swap(&mut focus_view.area, &mut target_view.area);

                    Some(())
                }
                _ => unreachable!(),
            }
        } else {
            let [focus_parent, target_parent, focus, target] =
                views
                    .map
                    .get_disjoint_mut([focus_parent, target_parent, focus, target])?;
            match (
                &mut focus_parent.content,
                &mut target_parent.content,
                &mut focus.content,
                &mut target.content,
            ) {
                (
                    Content::Container(focus_parent),
                    Content::Container(target_parent),
                    Content::View(focus_view),
                    Content::View(target_view),
                ) => {
                    let focus_pos = focus_parent
                        .children
                        .iter()
                        .position(|id| focus_view.id == *id)?;
                    let target_pos = target_parent
                        .children
                        .iter()
                        .position(|id| target_view.id == *id)?;
                    // re-parent target and focus nodes
                    std::mem::swap(
                        &mut focus_parent.children[focus_pos],
                        &mut target_parent.children[target_pos],
                    );
                    std::mem::swap(&mut focus.parent, &mut target.parent);
                    // swap area so that views rendered at the correct location
                    std::mem::swap(&mut focus_view.area, &mut target_view.area);

                    Some(())
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn area(&self) -> Rect {
        self.area
    }
}

#[derive(Debug)]
pub struct Traverse<'a> {
    views: &'a ViewMap,
    stack: Vec<ViewId>, // TODO: reuse the one we use on update
}

impl<'a> Traverse<'a> {
    fn new(tree: &'a Tree, views: &'a ViewMap) -> Self {
        Self {
            views,
            stack: vec![tree.root],
        }
    }
}

impl<'a> Iterator for Traverse<'a> {
    type Item = (ViewId, &'a View);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let key = self.stack.pop()?;

            let node = &self.views.map[key];

            match &node.content {
                Content::View(view) => return Some((key, view)),
                Content::Container(container) => {
                    self.stack.extend(container.children.iter().rev());
                }
            }
        }
    }
}

impl DoubleEndedIterator for Traverse<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let key = self.stack.pop()?;

            let node = &self.views.map[key];

            match &node.content {
                Content::View(view) => return Some((key, view)),
                Content::Container(container) => {
                    self.stack.extend(container.children.iter());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::editor::GutterConfig;
    use crate::DocumentId;

    #[test]
    fn find_split_in_direction() {
        let mut views = ViewMap::default();
        let mut tree = Tree::new(
            Rect {
                x: 0,
                y: 0,
                width: 180,
                height: 80,
            },
            &mut views,
        );
        let mut view = View::new(DocumentId::default(), GutterConfig::default());
        view.area = Rect::new(0, 0, 180, 80);
        tree.insert(&mut views, view);

        let l0 = tree.focus;
        let view = View::new(DocumentId::default(), GutterConfig::default());
        tree.split(&mut views, view, Layout::Vertical);
        let r0 = tree.focus;

        tree.focus = l0;
        let view = View::new(DocumentId::default(), GutterConfig::default());
        tree.split(&mut views, view, Layout::Horizontal);
        let l1 = tree.focus;

        tree.focus = l0;
        let view = View::new(DocumentId::default(), GutterConfig::default());
        tree.split(&mut views, view, Layout::Vertical);

        // Tree in test
        // | L0  | L2 |    |
        // |    L1    | R0 |
        let l2 = tree.focus;
        assert_eq!(
            Some(l0),
            tree.find_split_in_direction(&views, l2, Direction::Left)
        );
        assert_eq!(
            Some(l1),
            tree.find_split_in_direction(&views, l2, Direction::Down)
        );
        assert_eq!(
            Some(r0),
            tree.find_split_in_direction(&views, l2, Direction::Right)
        );
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, l2, Direction::Up)
        );

        tree.focus = l1;
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, l1, Direction::Left)
        );
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, l1, Direction::Down)
        );
        assert_eq!(
            Some(r0),
            tree.find_split_in_direction(&views, l1, Direction::Right)
        );
        assert_eq!(
            Some(l0),
            tree.find_split_in_direction(&views, l1, Direction::Up)
        );

        tree.focus = l0;
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, l0, Direction::Left)
        );
        assert_eq!(
            Some(l1),
            tree.find_split_in_direction(&views, l0, Direction::Down)
        );
        assert_eq!(
            Some(l2),
            tree.find_split_in_direction(&views, l0, Direction::Right)
        );
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, l0, Direction::Up)
        );

        tree.focus = r0;
        assert_eq!(
            Some(l2),
            tree.find_split_in_direction(&views, r0, Direction::Left)
        );
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, r0, Direction::Down)
        );
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, r0, Direction::Right)
        );
        assert_eq!(
            None,
            tree.find_split_in_direction(&views, r0, Direction::Up)
        );
    }

    #[test]
    fn swap_split_in_direction() {
        let mut views = ViewMap::default();
        let mut tree = Tree::new(
            Rect {
                x: 0,
                y: 0,
                width: 180,
                height: 80,
            },
            &mut views,
        );

        let doc_l0 = DocumentId::default();
        let mut view = View::new(doc_l0, GutterConfig::default());
        view.area = Rect::new(0, 0, 180, 80);
        tree.insert(&mut views, view);

        let l0 = tree.focus;

        let doc_r0 = DocumentId::default();
        let view = View::new(doc_r0, GutterConfig::default());
        tree.split(&mut views, view, Layout::Vertical);
        let r0 = tree.focus;

        tree.focus = l0;

        let doc_l1 = DocumentId::default();
        let view = View::new(doc_l1, GutterConfig::default());
        tree.split(&mut views, view, Layout::Horizontal);
        let l1 = tree.focus;

        tree.focus = l0;

        let doc_l2 = DocumentId::default();
        let view = View::new(doc_l2, GutterConfig::default());
        tree.split(&mut views, view, Layout::Vertical);
        let l2 = tree.focus;

        // Views in test
        // | L0  | L2 |    |
        // |    L1    | R0 |

        // Document IDs in test
        // | l0  | l2 |    |
        // |    l1    | r0 |

        fn doc_id(views: &ViewMap, view_id: ViewId) -> Option<DocumentId> {
            if let Content::View(view) = &views.map[view_id].content {
                Some(view.doc)
            } else {
                None
            }
        }

        tree.focus = l0;
        // `*` marks the view in focus from view table (here L0)
        // | l0*  | l2 |    |
        // |    l1     | r0 |
        tree.swap_split_in_direction(&mut views, Direction::Down);
        // | l1   | l2 |    |
        // |    l0*    | r0 |
        assert_eq!(tree.focus, l0);
        assert_eq!(doc_id(&views, l0), Some(doc_l1));
        assert_eq!(doc_id(&views, l1), Some(doc_l0));
        assert_eq!(doc_id(&views, l2), Some(doc_l2));
        assert_eq!(doc_id(&views, r0), Some(doc_r0));

        tree.swap_split_in_direction(&mut views, Direction::Right);

        // | l1  | l2 |     |
        // |    r0    | l0* |
        assert_eq!(tree.focus, l0);
        assert_eq!(doc_id(&views, l0), Some(doc_l1));
        assert_eq!(doc_id(&views, l1), Some(doc_r0));
        assert_eq!(doc_id(&views, l2), Some(doc_l2));
        assert_eq!(doc_id(&views, r0), Some(doc_l0));

        // cannot swap, nothing changes
        tree.swap_split_in_direction(&mut views, Direction::Up);
        // | l1  | l2 |     |
        // |    r0    | l0* |
        assert_eq!(tree.focus, l0);
        assert_eq!(doc_id(&views, l0), Some(doc_l1));
        assert_eq!(doc_id(&views, l1), Some(doc_r0));
        assert_eq!(doc_id(&views, l2), Some(doc_l2));
        assert_eq!(doc_id(&views, r0), Some(doc_l0));

        // cannot swap, nothing changes
        tree.swap_split_in_direction(&mut views, Direction::Down);
        // | l1  | l2 |     |
        // |    r0    | l0* |
        assert_eq!(tree.focus, l0);
        assert_eq!(doc_id(&views, l0), Some(doc_l1));
        assert_eq!(doc_id(&views, l1), Some(doc_r0));
        assert_eq!(doc_id(&views, l2), Some(doc_l2));
        assert_eq!(doc_id(&views, r0), Some(doc_l0));

        tree.focus = l2;
        // | l1  | l2* |    |
        // |    r0     | l0 |

        tree.swap_split_in_direction(&mut views, Direction::Down);
        // | l1  | r0  |    |
        // |    l2*    | l0 |
        assert_eq!(tree.focus, l2);
        assert_eq!(doc_id(&views, l0), Some(doc_l1));
        assert_eq!(doc_id(&views, l1), Some(doc_l2));
        assert_eq!(doc_id(&views, l2), Some(doc_r0));
        assert_eq!(doc_id(&views, r0), Some(doc_l0));

        tree.swap_split_in_direction(&mut views, Direction::Up);
        // | l2* | r0 |    |
        // |    l1    | l0 |
        assert_eq!(tree.focus, l2);
        assert_eq!(doc_id(&views, l0), Some(doc_l2));
        assert_eq!(doc_id(&views, l1), Some(doc_l1));
        assert_eq!(doc_id(&views, l2), Some(doc_r0));
        assert_eq!(doc_id(&views, r0), Some(doc_l0));
    }

    #[test]
    fn all_vertical_views_have_same_width() {
        let tree_area_width = 180;
        let mut views = ViewMap::default();
        let mut tree = Tree::new(
            Rect {
                x: 0,
                y: 0,
                width: tree_area_width,
                height: 80,
            },
            &mut views,
        );
        let mut view = View::new(DocumentId::default(), GutterConfig::default());
        view.area = Rect::new(0, 0, 180, 80);
        tree.insert(&mut views, view);

        let view = View::new(DocumentId::default(), GutterConfig::default());
        tree.split(&mut views, view, Layout::Vertical);

        let view = View::new(DocumentId::default(), GutterConfig::default());
        tree.split(&mut views, view, Layout::Horizontal);

        tree.remove(&mut views, tree.focus);

        let view = View::new(DocumentId::default(), GutterConfig::default());
        tree.split(&mut views, view, Layout::Vertical);

        // Make sure that we only have one level in the tree.
        assert_eq!(3, tree.views(&views).count());
        assert_eq!(
            vec![
                tree_area_width / 3 - 1, // gap here
                tree_area_width / 3 - 1, // gap here
                tree_area_width / 3
            ],
            tree.views(&views)
                .map(|(view, _)| view.area.width)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn vsplit_gap_rounding() {
        let (tree_area_width, tree_area_height) = (80, 24);
        let mut views = ViewMap::default();
        let mut tree = Tree::new(
            Rect {
                x: 0,
                y: 0,
                width: tree_area_width,
                height: tree_area_height,
            },
            &mut views,
        );
        let mut view = View::new(DocumentId::default(), GutterConfig::default());
        view.area = Rect::new(0, 0, tree_area_width, tree_area_height);
        tree.insert(&mut views, view);

        for _ in 0..9 {
            let view = View::new(DocumentId::default(), GutterConfig::default());
            tree.split(&mut views, view, Layout::Vertical);
        }

        assert_eq!(10, tree.views(&views).count());
        assert_eq!(
            std::iter::repeat(7)
                .take(9)
                .chain(Some(8)) // Rounding in `recalculate`.
                .collect::<Vec<_>>(),
            tree.views(&views)
                .map(|(view, _)| view.area.width)
                .collect::<Vec<_>>()
        );
    }
}
