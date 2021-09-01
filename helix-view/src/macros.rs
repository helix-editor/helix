//! These are macros to make getting very nested fields in the `Editor` struct easier
//! These are macros instead of functions because functions will have to take `&mut self`
//! However, rust doesn't know that you only want a partial borrow instead of borrowing the
//! entire struct which `&mut self` says.  This makes it impossible to do other mutable
//! stuff to the struct because it is already borrowed. Because macros are expanded,
//! this circumvents the problem because it is just like indexing fields by hand and then
//! putting a &mut in front of it. This way rust can see that we are only borrowing a
//! part of the struct and not the entire thing.

#[macro_export]
macro_rules! current {
    ( $( $editor:ident ).+ ) => {{
        let view = $crate::view_mut!( $( $editor ).+ );
        let doc = &mut $( $editor ).+ .documents[view.doc];
        (view, doc)
    }};
}

#[macro_export]
macro_rules! doc_mut {
    ( $( $editor:ident ).+ ) => {{
        $crate::current!( $( $editor ).+ ).1
    }};
}

#[macro_export]
macro_rules! view_mut {
    ( $( $editor:ident ).+ ) => {{
        $( $editor ).+ .tree.get_mut($( $editor ).+ .tree.focus)
    }};
}

#[macro_export]
macro_rules! view {
    ( $( $editor:ident ).+ ) => {{
        $( $editor ).+ .tree.get($( $editor ).+ .tree.focus)
    }};
}
